//! Docking manager allows you to dock windows and hold them in-place.
//!
//! # Notes
//!
//! Docking manager can hold any types of UI elements, but dragging works only
//! for windows.

use crate::{
    border::BorderBuilder,
    brush::Brush,
    core::{color::Color, pool::Handle},
    grid::{Column, GridBuilder, Row},
    message::{CursorIcon, UiMessage},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    window::Window,
    BuildContext, Control, NodeHandleMapping, Thickness, UiNode, UserInterface,
};
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    ops::{Deref, DerefMut},
};

pub mod config;
mod tile;

pub use tile::*;

#[derive(Clone)]
pub struct DockingManager {
    pub widget: Widget,
    pub floating_windows: RefCell<Vec<Handle<UiNode>>>,
}

crate::define_widget_deref!(DockingManager);

impl Control for DockingManager {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        node_map.resolve_slice(&mut self.floating_windows.borrow_mut());
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);
    }

    fn preview_message(&self, _ui: &UserInterface, message: &mut UiMessage) {
        if let Some(WidgetMessage::LinkWith(_)) = message.data::<WidgetMessage>() {
            let pos = self
                .floating_windows
                .borrow()
                .iter()
                .position(|&i| i == message.destination());
            if let Some(pos) = pos {
                self.floating_windows.borrow_mut().remove(pos);
            }
        }
    }
}

pub struct DockingManagerBuilder {
    widget_builder: WidgetBuilder,
    floating_windows: Vec<Handle<UiNode>>,
}

impl DockingManagerBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            floating_windows: Default::default(),
        }
    }

    pub fn with_floating_windows(mut self, windows: Vec<Handle<UiNode>>) -> Self {
        self.floating_windows = windows;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let docking_manager = DockingManager {
            widget: self.widget_builder.with_preview_messages(true).build(),
            floating_windows: RefCell::new(self.floating_windows),
        };

        ctx.add_node(UiNode::new(docking_manager))
    }
}

pub struct TileBuilder {
    widget_builder: WidgetBuilder,
    content: TileContent,
}

pub const DEFAULT_SPLITTER_SIZE: f32 = 5.0;
pub const DEFAULT_ANCHOR_COLOR: Color = Color::opaque(150, 150, 150);

pub fn make_default_anchor(ctx: &mut BuildContext, row: usize, column: usize) -> Handle<UiNode> {
    let default_anchor_size = 30.0;
    BorderBuilder::new(
        WidgetBuilder::new()
            .with_width(default_anchor_size)
            .with_height(default_anchor_size)
            .with_visibility(false)
            .on_row(row)
            .on_column(column)
            .with_draw_on_top(true)
            .with_background(Brush::Solid(DEFAULT_ANCHOR_COLOR)),
    )
    .build(ctx)
}

impl TileBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            content: TileContent::Empty,
        }
    }

    pub fn with_content(mut self, content: TileContent) -> Self {
        self.content = content;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let left_anchor = make_default_anchor(ctx, 2, 1);
        let right_anchor = make_default_anchor(ctx, 2, 3);
        let dock_anchor = make_default_anchor(ctx, 2, 2);
        let top_anchor = make_default_anchor(ctx, 1, 2);
        let bottom_anchor = make_default_anchor(ctx, 3, 2);

        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(left_anchor)
                .with_child(dock_anchor)
                .with_child(right_anchor)
                .with_child(top_anchor)
                .with_child(bottom_anchor),
        )
        .add_row(Row::stretch())
        .add_row(Row::auto())
        .add_row(Row::auto())
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .add_column(Column::auto())
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .build(ctx);

        let splitter = BorderBuilder::new(
            WidgetBuilder::new()
                .with_width({
                    if let TileContent::HorizontalTiles { .. } = self.content {
                        DEFAULT_SPLITTER_SIZE
                    } else {
                        f32::INFINITY
                    }
                })
                .with_height({
                    if let TileContent::VerticalTiles { .. } = self.content {
                        DEFAULT_SPLITTER_SIZE
                    } else {
                        f32::INFINITY
                    }
                })
                .with_visibility(matches!(
                    self.content,
                    TileContent::VerticalTiles { .. } | TileContent::HorizontalTiles { .. }
                ))
                .with_cursor(match self.content {
                    TileContent::HorizontalTiles { .. } => Some(CursorIcon::WResize),
                    TileContent::VerticalTiles { .. } => Some(CursorIcon::NResize),
                    _ => None,
                }),
        )
        .with_stroke_thickness(Thickness::uniform(0.0))
        .build(ctx);

        if let TileContent::Window(window) = self.content {
            if let Some(window) = ctx[window].cast_mut::<Window>() {
                // Every docked window must be non-resizable (it means that it cannot be resized by user
                // and it still can be resized by a proper message).
                window.set_can_resize(false);
            }
        }

        let children = match self.content {
            TileContent::Window(window) => vec![window],
            TileContent::VerticalTiles { tiles, .. } => vec![tiles[0], tiles[1]],
            TileContent::HorizontalTiles { tiles, .. } => vec![tiles[0], tiles[1]],
            _ => vec![],
        };

        let tile = Tile {
            widget: self
                .widget_builder
                .with_preview_messages(true)
                .with_child(grid)
                .with_child(splitter)
                .with_children(children)
                .build(),
            left_anchor,
            right_anchor,
            top_anchor,
            bottom_anchor,
            center_anchor: dock_anchor,
            content: self.content,
            splitter,
            dragging_splitter: false,
            drop_anchor: Default::default(),
        };

        ctx.add_node(UiNode::new(tile))
    }
}
