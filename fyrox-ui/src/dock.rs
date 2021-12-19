//! Docking manager allows you to dock windows and hold them in-place.
//!
//! # Notes
//!
//! Docking manager can hold any types of UI elements, but dragging works only
//! for windows.

use crate::{
    border::BorderBuilder,
    brush::Brush,
    core::{algebra::Vector2, color::Color, math::Rect, pool::Handle},
    define_constructor,
    grid::{Column, GridBuilder, Row},
    message::{CursorIcon, MessageDirection, UiMessage},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    window::{Window, WindowMessage},
    BuildContext, Control, NodeHandleMapping, Thickness, UiNode, UserInterface,
};
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone, PartialEq)]
pub enum TileMessage {
    Content(TileContent),
    /// Internal. Do not use.
    Split {
        window: Handle<UiNode>,
        direction: SplitDirection,
        first: bool,
    },
}

impl TileMessage {
    define_constructor!(TileMessage:Content => fn content(TileContent), layout: false);
    define_constructor!(TileMessage:Split => fn split(window: Handle<UiNode>,
        direction: SplitDirection,
        first: bool), layout: false);
}

#[derive(Debug, PartialEq, Clone)]
pub enum TileContent {
    Empty,
    Window(Handle<UiNode>),
    VerticalTiles {
        splitter: f32,
        /// Docking system requires tiles to be handles to Tile instances.
        /// However any node handle is acceptable, but in this case docking
        /// will most likely not work.
        tiles: [Handle<UiNode>; 2],
    },
    HorizontalTiles {
        splitter: f32,
        /// Docking system requires tiles to be handles to Tile instances.
        /// However any node handle is acceptable, but in this case docking
        /// will most likely not work.
        tiles: [Handle<UiNode>; 2],
    },
}

impl TileContent {
    pub fn is_empty(&self) -> bool {
        matches!(self, TileContent::Empty)
    }
}

#[derive(Clone)]
pub struct Tile {
    widget: Widget,
    left_anchor: Handle<UiNode>,
    right_anchor: Handle<UiNode>,
    top_anchor: Handle<UiNode>,
    bottom_anchor: Handle<UiNode>,
    center_anchor: Handle<UiNode>,
    content: TileContent,
    splitter: Handle<UiNode>,
    dragging_splitter: bool,
    drop_anchor: Cell<Handle<UiNode>>,
}

crate::define_widget_deref!(Tile);

impl Control for Tile {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        node_map.resolve_cell(&mut self.drop_anchor);
        node_map.resolve(&mut self.splitter);
        node_map.resolve(&mut self.center_anchor);
        node_map.resolve(&mut self.bottom_anchor);
        node_map.resolve(&mut self.top_anchor);
        node_map.resolve(&mut self.right_anchor);
        node_map.resolve(&mut self.left_anchor);
        match &mut self.content {
            TileContent::Empty => {}
            TileContent::Window(window) => node_map.resolve(window),
            TileContent::VerticalTiles { tiles, .. }
            | TileContent::HorizontalTiles { tiles, .. } => {
                for tile in tiles {
                    node_map.resolve(tile);
                }
            }
        }
    }

    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        for &child_handle in self.children() {
            // Determine available size for each child by its kind:
            // - Every child not in content of tile just takes whole available size.
            // - Every content's child uses specific available measure size.
            // This is a bit weird, but it is how it works.
            let available_size = match self.content {
                TileContent::VerticalTiles {
                    splitter,
                    ref tiles,
                } => {
                    if tiles[0] == child_handle {
                        Vector2::new(available_size.x, available_size.y * splitter)
                    } else if tiles[1] == child_handle {
                        Vector2::new(available_size.x, available_size.y * (1.0 - splitter))
                    } else {
                        available_size
                    }
                }
                TileContent::HorizontalTiles {
                    splitter,
                    ref tiles,
                } => {
                    if tiles[0] == child_handle {
                        Vector2::new(available_size.x * splitter, available_size.y)
                    } else if tiles[1] == child_handle {
                        Vector2::new(available_size.x * (1.0 - splitter), available_size.y)
                    } else {
                        available_size
                    }
                }
                _ => available_size,
            };

            ui.measure_node(child_handle, available_size);
        }

        available_size
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        let splitter_size = ui.node(self.splitter).desired_size();

        for &child_handle in self.children() {
            let full_bounds = Rect::new(0.0, 0.0, final_size.x, final_size.y);

            let bounds = match self.content {
                TileContent::VerticalTiles {
                    splitter,
                    ref tiles,
                } => {
                    if tiles[0] == child_handle {
                        Rect::new(
                            0.0,
                            0.0,
                            final_size.x,
                            final_size.y * splitter - splitter_size.y * 0.5,
                        )
                    } else if tiles[1] == child_handle {
                        Rect::new(
                            0.0,
                            final_size.y * splitter + splitter_size.y * 0.5,
                            final_size.x,
                            final_size.y * (1.0 - splitter) - splitter_size.y,
                        )
                    } else if self.splitter == child_handle {
                        Rect::new(
                            0.0,
                            final_size.y * splitter - splitter_size.y * 0.5,
                            final_size.x,
                            splitter_size.y,
                        )
                    } else {
                        full_bounds
                    }
                }
                TileContent::HorizontalTiles {
                    splitter,
                    ref tiles,
                } => {
                    if tiles[0] == child_handle {
                        Rect::new(
                            0.0,
                            0.0,
                            final_size.x * splitter - splitter_size.x * 0.5,
                            final_size.y,
                        )
                    } else if tiles[1] == child_handle {
                        Rect::new(
                            final_size.x * splitter + splitter_size.x * 0.5,
                            0.0,
                            final_size.x * (1.0 - splitter) - splitter_size.x * 0.5,
                            final_size.y,
                        )
                    } else if self.splitter == child_handle {
                        Rect::new(
                            final_size.x * splitter - splitter_size.x * 0.5,
                            0.0,
                            splitter_size.x,
                            final_size.y,
                        )
                    } else {
                        full_bounds
                    }
                }
                _ => full_bounds,
            };

            ui.arrange_node(child_handle, &bounds);

            // Main difference between tile arrangement and other arrangement methods in
            // library is that tile has to explicitly set width of child windows, otherwise
            // layout will be weird - window will most likely will stay at its previous size.
            if child_handle != self.splitter {
                ui.send_message(WidgetMessage::width(
                    child_handle,
                    MessageDirection::ToWidget,
                    bounds.w(),
                ));
                ui.send_message(WidgetMessage::height(
                    child_handle,
                    MessageDirection::ToWidget,
                    bounds.h(),
                ));
            }
        }

        final_size
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<TileMessage>() {
            if message.destination() == self.handle() {
                match msg {
                    TileMessage::Content(content) => {
                        self.content = content.clone();

                        match content {
                            TileContent::Empty => {
                                ui.send_message(WidgetMessage::visibility(
                                    self.splitter,
                                    MessageDirection::ToWidget,
                                    false,
                                ));
                            }
                            &TileContent::Window(window) => {
                                ui.send_message(WidgetMessage::link(
                                    window,
                                    MessageDirection::ToWidget,
                                    self.handle(),
                                ));

                                ui.send_message(WidgetMessage::visibility(
                                    self.splitter,
                                    MessageDirection::ToWidget,
                                    false,
                                ));

                                ui.send_message(WindowMessage::can_resize(
                                    window,
                                    MessageDirection::ToWidget,
                                    false,
                                ));
                            }
                            TileContent::VerticalTiles { tiles, .. }
                            | TileContent::HorizontalTiles { tiles, .. } => {
                                for &tile in tiles {
                                    ui.send_message(WidgetMessage::link(
                                        tile,
                                        MessageDirection::ToWidget,
                                        self.handle(),
                                    ));
                                }

                                ui.send_message(WidgetMessage::visibility(
                                    self.splitter,
                                    MessageDirection::ToWidget,
                                    true,
                                ));

                                match content {
                                    TileContent::HorizontalTiles { .. } => {
                                        ui.send_message(WidgetMessage::width(
                                            self.splitter,
                                            MessageDirection::ToWidget,
                                            DEFAULT_SPLITTER_SIZE,
                                        ));
                                        ui.send_message(WidgetMessage::height(
                                            self.splitter,
                                            MessageDirection::ToWidget,
                                            f32::INFINITY,
                                        ));
                                    }
                                    TileContent::VerticalTiles { .. } => {
                                        ui.send_message(WidgetMessage::width(
                                            self.splitter,
                                            MessageDirection::ToWidget,
                                            f32::INFINITY,
                                        ));
                                        ui.send_message(WidgetMessage::height(
                                            self.splitter,
                                            MessageDirection::ToWidget,
                                            DEFAULT_SPLITTER_SIZE,
                                        ));
                                    }
                                    _ => (),
                                }
                            }
                        }
                    }
                    &TileMessage::Split {
                        window,
                        direction,
                        first,
                    } => {
                        if matches!(self.content, TileContent::Window(_)) {
                            self.split(ui, window, direction, first);
                        }
                    }
                }
            }
        } else if let Some(msg) = message.data::<WidgetMessage>() {
            match msg {
                &WidgetMessage::MouseDown { .. } => {
                    if !message.handled() && message.destination() == self.splitter {
                        message.set_handled(true);
                        self.dragging_splitter = true;
                        ui.capture_mouse(self.splitter);
                    }
                }
                &WidgetMessage::MouseUp { .. } => {
                    if !message.handled() && message.destination() == self.splitter {
                        message.set_handled(true);
                        self.dragging_splitter = false;
                        ui.release_mouse_capture();
                    }
                }
                &WidgetMessage::MouseMove { pos, .. } => {
                    if self.dragging_splitter {
                        let bounds = self.screen_bounds();
                        match self.content {
                            TileContent::VerticalTiles {
                                ref mut splitter, ..
                            } => {
                                *splitter = ((pos.y - bounds.y()) / bounds.h()).max(0.0).min(1.0);
                                self.invalidate_layout();
                            }
                            TileContent::HorizontalTiles {
                                ref mut splitter, ..
                            } => {
                                *splitter = ((pos.x - bounds.x()) / bounds.w()).max(0.0).min(1.0);
                                self.invalidate_layout();
                            }
                            _ => (),
                        }
                    }
                }
                WidgetMessage::Unlink => {
                    // Check if this tile can be removed: only if it is split and sub-tiles are empty.
                    match self.content {
                        TileContent::VerticalTiles { tiles, .. }
                        | TileContent::HorizontalTiles { tiles, .. } => {
                            let mut has_empty_sub_tile = false;
                            for &tile in &tiles {
                                if let Some(sub_tile) = ui.node(tile).cast::<Tile>() {
                                    if let TileContent::Empty = sub_tile.content {
                                        has_empty_sub_tile = true;
                                        break;
                                    }
                                }
                            }
                            if has_empty_sub_tile {
                                for &tile in &tiles {
                                    if let Some(sub_tile) = ui.node(tile).cast::<Tile>() {
                                        match sub_tile.content {
                                            TileContent::Window(sub_tile_wnd) => {
                                                // If we have only a tile with a window, then detach window and schedule
                                                // linking with current tile.
                                                ui.send_message(WidgetMessage::unlink(
                                                    sub_tile_wnd,
                                                    MessageDirection::ToWidget,
                                                ));

                                                ui.send_message(TileMessage::content(
                                                    self.handle,
                                                    MessageDirection::ToWidget,
                                                    TileContent::Window(sub_tile_wnd),
                                                ));
                                                // Splitter must be hidden.
                                                ui.send_message(WidgetMessage::visibility(
                                                    self.splitter,
                                                    MessageDirection::ToWidget,
                                                    false,
                                                ));
                                            }
                                            // In case if we have a split tile (vertically or horizontally) left in current tile
                                            // (which is split too) we must set content of current tile to content of sub tile.
                                            TileContent::VerticalTiles {
                                                splitter,
                                                tiles: sub_tiles,
                                            } => {
                                                for &sub_tile in &sub_tiles {
                                                    ui.send_message(WidgetMessage::unlink(
                                                        sub_tile,
                                                        MessageDirection::ToWidget,
                                                    ));
                                                }
                                                // Transfer sub tiles to current tile.
                                                ui.send_message(TileMessage::content(
                                                    self.handle,
                                                    MessageDirection::ToWidget,
                                                    TileContent::VerticalTiles {
                                                        splitter,
                                                        tiles: sub_tiles,
                                                    },
                                                ));
                                            }
                                            TileContent::HorizontalTiles {
                                                splitter,
                                                tiles: sub_tiles,
                                            } => {
                                                for &sub_tile in &sub_tiles {
                                                    ui.send_message(WidgetMessage::unlink(
                                                        sub_tile,
                                                        MessageDirection::ToWidget,
                                                    ));
                                                }
                                                // Transfer sub tiles to current tile.
                                                ui.send_message(TileMessage::content(
                                                    self.handle,
                                                    MessageDirection::ToWidget,
                                                    TileContent::HorizontalTiles {
                                                        splitter,
                                                        tiles: sub_tiles,
                                                    },
                                                ));
                                            }
                                            _ => {}
                                        }
                                    }
                                }

                                // Destroy tiles.
                                for &tile in &tiles {
                                    ui.send_message(WidgetMessage::remove(
                                        tile,
                                        MessageDirection::ToWidget,
                                    ));
                                }
                            }
                        }
                        _ => (),
                    }
                }
                _ => {}
            }
            // We can catch any message from window while it docked.
        } else if let Some(WindowMessage::Move(_)) = message.data::<WindowMessage>() {
            // Check if we dragging child window.
            let content_moved = match self.content {
                TileContent::Window(window) => window == message.destination(),
                _ => false,
            };

            if content_moved {
                if let Some(window) = ui.node(message.destination()).cast::<Window>() {
                    if window.drag_delta().norm() > 20.0 {
                        ui.send_message(TileMessage::content(
                            self.handle,
                            MessageDirection::ToWidget,
                            TileContent::Empty,
                        ));

                        ui.send_message(WidgetMessage::unlink(
                            message.destination(),
                            MessageDirection::ToWidget,
                        ));

                        ui.send_message(WindowMessage::can_resize(
                            message.destination(),
                            MessageDirection::ToWidget,
                            true,
                        ));

                        if let Some((_, docking_manager)) =
                            ui.try_borrow_by_type_up::<DockingManager>(self.parent())
                        {
                            docking_manager
                                .floating_windows
                                .borrow_mut()
                                .push(message.destination());
                        }
                    }
                }
            }
        }
    }

    // We have to use preview_message for docking purposes because dragged window detached
    // from docking manager and handle_routed_message won't receive any messages from window.
    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        if let Some(WidgetMessage::Unlink) = message.data::<WidgetMessage>() {
            if let TileContent::Empty | TileContent::Window(_) = self.content {
                // Show anchors.
                for &anchor in &self.anchors() {
                    ui.send_message(WidgetMessage::visibility(
                        anchor,
                        MessageDirection::ToWidget,
                        true,
                    ));
                }
            }
        } else if let Some(msg) = message.data::<WindowMessage>() {
            if let Some((_, docking_manager)) =
                ui.try_borrow_by_type_up::<DockingManager>(self.parent())
            {
                // Make sure we are dragging one of floating windows of parent docking manager.
                if docking_manager
                    .floating_windows
                    .borrow_mut()
                    .contains(&message.destination())
                {
                    match msg {
                        &WindowMessage::Move(_) => {
                            // Window can be docked only if current tile is not split already.
                            if let TileContent::Empty | TileContent::Window(_) = self.content {
                                // When window is being dragged, we should check which tile can accept it.
                                let pos = ui.cursor_position;
                                for &anchor in &self.anchors() {
                                    ui.send_message(WidgetMessage::background(
                                        anchor,
                                        MessageDirection::ToWidget,
                                        Brush::Solid(DEFAULT_ANCHOR_COLOR),
                                    ))
                                }
                                if ui.node(self.left_anchor).screen_bounds().contains(pos) {
                                    ui.send_message(WidgetMessage::background(
                                        self.left_anchor,
                                        MessageDirection::ToWidget,
                                        Brush::Solid(Color::WHITE),
                                    ));
                                    self.drop_anchor.set(self.left_anchor);
                                } else if ui.node(self.right_anchor).screen_bounds().contains(pos) {
                                    ui.send_message(WidgetMessage::background(
                                        self.right_anchor,
                                        MessageDirection::ToWidget,
                                        Brush::Solid(Color::WHITE),
                                    ));
                                    self.drop_anchor.set(self.right_anchor);
                                } else if ui.node(self.top_anchor).screen_bounds().contains(pos) {
                                    ui.send_message(WidgetMessage::background(
                                        self.top_anchor,
                                        MessageDirection::ToWidget,
                                        Brush::Solid(Color::WHITE),
                                    ));
                                    self.drop_anchor.set(self.top_anchor);
                                } else if ui.node(self.bottom_anchor).screen_bounds().contains(pos)
                                {
                                    ui.send_message(WidgetMessage::background(
                                        self.bottom_anchor,
                                        MessageDirection::ToWidget,
                                        Brush::Solid(Color::WHITE),
                                    ));
                                    self.drop_anchor.set(self.bottom_anchor);
                                } else if ui.node(self.center_anchor).screen_bounds().contains(pos)
                                {
                                    ui.send_message(WidgetMessage::background(
                                        self.center_anchor,
                                        MessageDirection::ToWidget,
                                        Brush::Solid(Color::WHITE),
                                    ));
                                    self.drop_anchor.set(self.center_anchor);
                                } else {
                                    self.drop_anchor.set(Handle::NONE);
                                }
                            }
                        }
                        WindowMessage::MoveStart => {
                            if let TileContent::Empty | TileContent::Window(_) = self.content {
                                // Show anchors.
                                for &anchor in &self.anchors() {
                                    ui.send_message(WidgetMessage::visibility(
                                        anchor,
                                        MessageDirection::ToWidget,
                                        true,
                                    ));
                                }
                            }
                        }
                        WindowMessage::MoveEnd => {
                            // Hide anchors.
                            for &anchor in &self.anchors() {
                                ui.send_message(WidgetMessage::visibility(
                                    anchor,
                                    MessageDirection::ToWidget,
                                    false,
                                ));
                            }

                            // Drop if has any drop anchor.
                            if self.drop_anchor.get().is_some() {
                                match self.content {
                                    TileContent::Empty => {
                                        if self.drop_anchor.get() == self.center_anchor {
                                            ui.send_message(TileMessage::content(
                                                self.handle,
                                                MessageDirection::ToWidget,
                                                TileContent::Window(message.destination()),
                                            ));
                                            ui.send_message(WidgetMessage::link(
                                                message.destination(),
                                                MessageDirection::ToWidget,
                                                self.handle,
                                            ));
                                        }
                                    }
                                    TileContent::Window(_) => {
                                        if self.drop_anchor.get() == self.left_anchor {
                                            // Split horizontally, dock to left.
                                            ui.send_message(TileMessage::split(
                                                self.handle,
                                                MessageDirection::ToWidget,
                                                message.destination(),
                                                SplitDirection::Horizontal,
                                                true,
                                            ));
                                        } else if self.drop_anchor.get() == self.right_anchor {
                                            // Split horizontally, dock to right.
                                            ui.send_message(TileMessage::split(
                                                self.handle,
                                                MessageDirection::ToWidget,
                                                message.destination(),
                                                SplitDirection::Horizontal,
                                                false,
                                            ));
                                        } else if self.drop_anchor.get() == self.top_anchor {
                                            // Split vertically, dock to top.
                                            ui.send_message(TileMessage::split(
                                                self.handle,
                                                MessageDirection::ToWidget,
                                                message.destination(),
                                                SplitDirection::Vertical,
                                                true,
                                            ));
                                        } else if self.drop_anchor.get() == self.bottom_anchor {
                                            // Split vertically, dock to bottom.
                                            ui.send_message(TileMessage::split(
                                                self.handle,
                                                MessageDirection::ToWidget,
                                                message.destination(),
                                                SplitDirection::Vertical,
                                                false,
                                            ));
                                        }
                                    }
                                    // Rest cannot accept windows.
                                    _ => (),
                                }
                            }
                        }
                        _ => (),
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

impl Tile {
    pub fn anchors(&self) -> [Handle<UiNode>; 5] {
        [
            self.left_anchor,
            self.right_anchor,
            self.top_anchor,
            self.bottom_anchor,
            self.center_anchor,
        ]
    }

    fn split(
        &mut self,
        ui: &mut UserInterface,
        window: Handle<UiNode>,
        direction: SplitDirection,
        first: bool,
    ) {
        let existing_content = match self.content {
            TileContent::Window(existing_window) => existing_window,
            _ => Handle::NONE,
        };

        let first_tile = TileBuilder::new(WidgetBuilder::new())
            .with_content({
                if first {
                    TileContent::Window(window)
                } else {
                    TileContent::Empty
                }
            })
            .build(&mut ui.build_ctx());

        let second_tile = TileBuilder::new(WidgetBuilder::new())
            .with_content({
                if first {
                    TileContent::Empty
                } else {
                    TileContent::Window(window)
                }
            })
            .build(&mut ui.build_ctx());

        if existing_content.is_some() {
            ui.send_message(TileMessage::content(
                if first { second_tile } else { first_tile },
                MessageDirection::ToWidget,
                TileContent::Window(existing_content),
            ));
        }

        ui.send_message(TileMessage::content(
            self.handle,
            MessageDirection::ToWidget,
            match direction {
                SplitDirection::Horizontal => TileContent::HorizontalTiles {
                    tiles: [first_tile, second_tile],
                    splitter: 0.5,
                },
                SplitDirection::Vertical => TileContent::VerticalTiles {
                    tiles: [first_tile, second_tile],
                    splitter: 0.5,
                },
            },
        ));
    }
}

#[derive(Clone)]
pub struct DockingManager {
    widget: Widget,
    floating_windows: RefCell<Vec<Handle<UiNode>>>,
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

pub const DEFAULT_SPLITTER_SIZE: f32 = 4.0;
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
                })
                .with_margin(Thickness::uniform(1.0)),
        )
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
