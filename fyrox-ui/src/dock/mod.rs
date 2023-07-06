//! Docking manager allows you to dock windows and hold them in-place.
//!
//! # Notes
//!
//! Docking manager can hold any types of UI elements, but dragging works only
//! for windows.

use std::collections::HashMap;
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    ops::{Deref, DerefMut},
};

pub mod config;
mod tile;

use crate::{
    border::BorderBuilder,
    brush::Brush,
    core::{color::Color, pool::Handle},
    define_constructor,
    dock::config::DockingManagerLayoutDescriptor,
    grid::{Column, GridBuilder, Row},
    message::MessageDirection,
    message::{CursorIcon, UiMessage},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    window::Window,
    BuildContext, Control, NodeHandleMapping, Thickness, UiNode, UserInterface,
};

use crate::dock::config::{FloatingWindowDescriptor, TileDescriptor};
pub use tile::*;

/// Supported docking manager-specific messages.
#[derive(Debug, Clone, PartialEq)]
pub enum DockingManagerMessage {
    Layout(DockingManagerLayoutDescriptor),
}

impl DockingManagerMessage {
    define_constructor!(
        /// Creates a new [Self::Layout] message.
        DockingManagerMessage:Layout => fn layout(DockingManagerLayoutDescriptor), layout: false
    );
}

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

        if message.destination() == self.handle && message.direction() == MessageDirection::ToWidget
        {
            if let Some(DockingManagerMessage::Layout(layout_descriptor)) = message.data() {
                if let Some(root_tile_handle) = self.children.first().cloned() {
                    // Detach the content of all tiles first.
                    let mut content = HashMap::new();
                    let mut stack = vec![root_tile_handle];
                    while let Some(tile_handle) = stack.pop() {
                        if let Some(tile) = ui
                            .try_get_node(tile_handle)
                            .and_then(|n| n.query_component::<Tile>())
                        {
                            match tile.content {
                                TileContent::Window(window) => {
                                    if let Some(window_ref) = ui.try_get_node(window) {
                                        ui.send_message(WidgetMessage::unlink(
                                            window,
                                            MessageDirection::ToWidget,
                                        ));
                                        content.insert(window_ref.id, window);
                                    }
                                }
                                TileContent::VerticalTiles { tiles, .. }
                                | TileContent::HorizontalTiles { tiles, .. } => {
                                    stack.extend_from_slice(&tiles);
                                }
                                _ => (),
                            }
                        }
                    }

                    // Destroy the root tile with all descendant tiles.
                    ui.send_message(WidgetMessage::remove(
                        root_tile_handle,
                        MessageDirection::ToWidget,
                    ));

                    // Re-create the tiles according to the layout and attach it to the docking manager.
                    if let Some(root_tile_descriptor) =
                        layout_descriptor.root_tile_descriptor.as_ref()
                    {
                        let root_tile = root_tile_descriptor.create_tile(ui);
                        ui.send_message(WidgetMessage::link(
                            root_tile,
                            MessageDirection::ToWidget,
                            self.handle,
                        ));
                    }

                    // Restore floating windows.
                    self.floating_windows.borrow_mut().clear();
                    for floating_window_desc in layout_descriptor.floating_windows.iter() {
                        let floating_window = ui
                            .find_by_criteria_down(ui.root(), &|n| n.id == floating_window_desc.id);
                        if floating_window.is_some() {
                            self.floating_windows.borrow_mut().push(floating_window);

                            ui.send_message(WidgetMessage::desired_position(
                                floating_window,
                                MessageDirection::ToWidget,
                                floating_window_desc.position,
                            ));

                            ui.send_message(WidgetMessage::width(
                                floating_window,
                                MessageDirection::ToWidget,
                                floating_window_desc.size.x,
                            ));
                            ui.send_message(WidgetMessage::height(
                                floating_window,
                                MessageDirection::ToWidget,
                                floating_window_desc.size.y,
                            ));
                        }
                    }
                }
            }
        }
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

impl DockingManager {
    pub fn layout(&self, ui: &UserInterface) -> DockingManagerLayoutDescriptor {
        DockingManagerLayoutDescriptor {
            floating_windows: self
                .floating_windows
                .borrow()
                .iter()
                .filter_map(|h| {
                    ui.try_get_node(*h).map(|w| FloatingWindowDescriptor {
                        id: w.id,
                        position: w.actual_local_position(),
                        size: w.actual_local_size(),
                    })
                })
                .collect::<Vec<_>>(),
            root_tile_descriptor: self
                .children()
                .first()
                .map(|c| TileDescriptor::from_tile_handle(*c, ui)),
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
