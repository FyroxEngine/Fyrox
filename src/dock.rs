//! Docking manager allows you to dock windows and hold them in-place.
//!
//! # Notes
//!
//! Docking manager can hold any types of UI elements, but dragging works only
//! for windows.

use std::{
    ops::{Deref, DerefMut},
};
use crate::{
    grid::{GridBuilder, Row, Column},
    border::BorderBuilder,
    brush::Brush,
    node::UINode,
    core::{
        math::{
            vec2::Vec2,
            Rect,
        },
        pool::Handle,
        color::Color,
    },
    message::{
        UiMessage,
        UiMessageData,
        WindowMessage,
    },
    widget::{
        Widget,
        WidgetBuilder,
    },
    Control,
    UserInterface,
    Thickness,
    message::WidgetMessage,
};

pub enum TileContent<M: 'static, C: 'static + Control<M, C>> {
    Empty,
    Window(Handle<UINode<M, C>>),
    VerticalTiles {
        splitter: f32,
        /// Docking system requires tiles to be handles to Tile instances.
        /// However any node handle is acceptable, but in this case docking
        /// will most likely not work.
        tiles: [Handle<UINode<M, C>>; 2],
    },
    HorizontalTiles {
        splitter: f32,
        /// Docking system requires tiles to be handles to Tile instances.
        /// However any node handle is acceptable, but in this case docking
        /// will most likely not work.
        tiles: [Handle<UINode<M, C>>; 2],
    },
}

impl<M: 'static, C: 'static + Control<M, C>> Clone for TileContent<M, C> {
    fn clone(&self) -> Self {
        match self {
            TileContent::Empty => TileContent::Empty,
            TileContent::Window(v) => TileContent::Window(*v),
            TileContent::VerticalTiles { splitter, tiles } => TileContent::VerticalTiles {
                splitter: *splitter,
                tiles: *tiles,
            },
            TileContent::HorizontalTiles { splitter, tiles } => TileContent::HorizontalTiles {
                splitter: *splitter,
                tiles: *tiles,
            },
        }
    }
}

pub struct Tile<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    left_anchor: Handle<UINode<M, C>>,
    right_anchor: Handle<UINode<M, C>>,
    top_anchor: Handle<UINode<M, C>>,
    bottom_anchor: Handle<UINode<M, C>>,
    center_anchor: Handle<UINode<M, C>>,
    content: TileContent<M, C>,
    splitter: Handle<UINode<M, C>>,
    dragging_splitter: bool,
    drop_anchor: Handle<UINode<M, C>>,
}

impl<M: 'static, C: 'static + Control<M, C>> Clone for Tile<M, C> {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.raw_copy(),
            left_anchor: self.left_anchor,
            right_anchor: self.right_anchor,
            top_anchor: self.top_anchor,
            bottom_anchor: self.bottom_anchor,
            center_anchor: self.center_anchor,
            content: self.content.clone(),
            splitter: self.splitter,
            dragging_splitter: self.dragging_splitter,
            drop_anchor: Default::default(),
        }
    }
}

impl<M: 'static, C: 'static + Control<M, C>> Deref for Tile<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> DerefMut for Tile<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> Control<M, C> for Tile<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        UINode::Tile(self.clone())
    }

    fn measure_override(&self, ui: &UserInterface<M, C>, available_size: Vec2) -> Vec2 {
        for &child_handle in self.children() {
            // Determine available size for each child by its kind:
            // - Every child not in content of tile just takes whole available size.
            // - Every content's child uses specific available measure size.
            // This is a bit weird, but it is how it works.
            let available_size = match self.content {
                TileContent::VerticalTiles { splitter, ref tiles } => {
                    if tiles[0] == child_handle {
                        Vec2::new(available_size.x, available_size.y * splitter)
                    } else if tiles[1] == child_handle {
                        Vec2::new(available_size.x, available_size.y * (1.0 - splitter))
                    } else {
                        available_size
                    }
                }
                TileContent::HorizontalTiles { splitter, ref tiles } => {
                    if tiles[0] == child_handle {
                        Vec2::new(available_size.x * splitter, available_size.y)
                    } else if tiles[1] == child_handle {
                        Vec2::new(available_size.x * (1.0 - splitter), available_size.y)
                    } else {
                        available_size
                    }
                }
                _ => available_size
            };

            ui.node(child_handle).measure(ui, available_size);
        }

        available_size
    }

    fn arrange_override(&self, ui: &UserInterface<M, C>, final_size: Vec2) -> Vec2 {
        let splitter_size = ui.node(self.splitter).desired_size();

        for &child_handle in self.children() {
            let full_bounds = Rect {
                x: 0.0,
                y: 0.0,
                w: final_size.x,
                h: final_size.y,
            };

            let bounds = match self.content {
                TileContent::VerticalTiles { splitter, ref tiles } => {
                    if tiles[0] == child_handle {
                        Rect {
                            x: 0.0,
                            y: 0.0,
                            w: final_size.x,
                            h: final_size.y * splitter - splitter_size.y * 0.5,
                        }
                    } else if tiles[1] == child_handle {
                        Rect {
                            x: 0.0,
                            y: final_size.y * splitter + splitter_size.y * 0.5,
                            w: final_size.x,
                            h: final_size.y * (1.0 - splitter) - splitter_size.y,
                        }
                    } else if self.splitter == child_handle {
                        Rect {
                            x: 0.0,
                            y: final_size.y * splitter - splitter_size.y * 0.5,
                            w: final_size.x,
                            h: splitter_size.y,
                        }
                    } else {
                        full_bounds
                    }
                }
                TileContent::HorizontalTiles { splitter, ref tiles } => {
                    if tiles[0] == child_handle {
                        Rect {
                            x: 0.0,
                            y: 0.0,
                            w: final_size.x * splitter - splitter_size.x * 0.5,
                            h: final_size.y,
                        }
                    } else if tiles[1] == child_handle {
                        Rect {
                            x: final_size.x * splitter + splitter_size.x * 0.5,
                            y: 0.0,
                            w: final_size.x * (1.0 - splitter) - splitter_size.x * 0.5,
                            h: final_size.y,
                        }
                    } else if self.splitter == child_handle {
                        Rect {
                            x: final_size.x * splitter - splitter_size.x * 0.5,
                            y: 0.0,
                            w: splitter_size.x,
                            h: final_size.y,
                        }
                    } else {
                        full_bounds
                    }
                }
                _ => full_bounds
            };

            ui.node(child_handle).arrange(ui, &bounds);

            // Main difference between tile arrangement and other arrangement methods in
            // library is that tile has to explicitly set width of child windows, otherwise
            // layout will be weird - window will most likely will stay at its previous size.
            if let UINode::Window(window) = ui.node(child_handle) {
                window.set_width(bounds.w);
                window.set_height(bounds.h);
            }
        }

        final_size
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_routed_message(ui, message);

        match &message.data {
            UiMessageData::Widget(msg) => {
                match msg {
                    &WidgetMessage::MouseDown { .. } => {
                        if !message.handled && message.destination == self.splitter {
                            message.handled = true;
                            self.dragging_splitter = true;
                            ui.capture_mouse(self.splitter);
                        }
                    }
                    &WidgetMessage::MouseUp { .. } => {
                        if !message.handled && message.destination == self.splitter {
                            message.handled = true;
                            self.dragging_splitter = false;
                            ui.release_mouse_capture();
                        }
                    }
                    &WidgetMessage::MouseMove { pos, .. } => {
                        if self.dragging_splitter {
                            let bounds = self.screen_bounds();
                            match self.content {
                                TileContent::VerticalTiles { ref mut splitter, .. } => {
                                    *splitter = ((pos.y - bounds.y) / bounds.h).max(0.0).min(1.0);
                                    self.invalidate_layout();
                                }
                                TileContent::HorizontalTiles { ref mut splitter, .. } => {
                                    *splitter = ((pos.x - bounds.x) / bounds.w).max(0.0).min(1.0);
                                    self.invalidate_layout();
                                }
                                _ => ()
                            }
                        }
                    }
                    WidgetMessage::Unlink => {
                        // Check if this tile can be removed: only if it is split and sub-tiles are empty.
                        match self.content {
                            TileContent::VerticalTiles { tiles, .. } | TileContent::HorizontalTiles { tiles, .. } => {
                                let mut empty_count = 0;
                                for &tile in &tiles {
                                    if let UINode::Tile(sub_tile) = ui.node(tile) {
                                        if let TileContent::Empty = sub_tile.content {
                                            empty_count += 1;
                                        }
                                    }
                                }

                                if empty_count == 2 {
                                    self.content = TileContent::Empty;

                                    ui.node_mut(self.splitter).set_visibility(false);

                                    for &tile in &tiles {
                                        // Remove sub-tiles.
                                        ui.send_message(UiMessage {
                                            handled: false,
                                            data: UiMessageData::Widget(WidgetMessage::Remove),
                                            destination: tile,
                                        })
                                    }
                                }
                            }
                            _ => ()
                        }
                    }
                    _ => {}
                }
            }
            // We can catch any message from window while it docked.
            UiMessageData::Window(msg) => {
                if let WindowMessage::Move(_) = msg {
                    // Check if we dragging child window.
                    let content_moved = match self.content {
                        TileContent::Window(window) => window == message.destination,
                        _ => false,
                    };

                    if content_moved {
                        if let UINode::Window(window) = ui.node(message.destination) {
                            if window.drag_delta().len() > 20.0 {
                                // Schedule unlink, we can't unlink node here directly because it attached
                                // to node that currently moved out of pool, and if we'd try we'd get panic.
                                ui.send_message(UiMessage {
                                    handled: false,
                                    data: UiMessageData::Widget(WidgetMessage::Unlink),
                                    destination: message.destination,
                                });

                                self.content = TileContent::Empty;

                                let docking_manager = ui.borrow_by_criteria_up_mut(self.parent(), |n| {
                                    if let UINode::DockingManager(_) = n { true } else { false }
                                });
                                if let UINode::DockingManager(docking_manager) = docking_manager {
                                    docking_manager.floating_windows.push(message.destination);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // We have to use preview_message for docking purposes because dragged window detached
    // from docking manager and handle_routed_message won't receive any messages from window.
    fn preview_message(&mut self, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        match &message.data {
            UiMessageData::Widget(msg) => {
                if let WidgetMessage::Unlink = msg {
                    if let TileContent::Empty | TileContent::Window(_) = self.content {
                        // Show anchors.
                        for &anchor in &self.anchors() {
                            ui.node_mut(anchor).set_visibility(true);
                        }
                    }
                }
            }
            UiMessageData::Window(msg) => {
                // Can panic if tile is used not as part of docking manager.
                let docking_manager = ui.borrow_by_criteria_up(self.parent(), |n| {
                    if let UINode::DockingManager(_) = n { true } else { false }
                });

                if let UINode::DockingManager(docking_manager) = docking_manager {
                    // Make sure we are dragging one of floating windows of parent docking manager.
                    if docking_manager.floating_windows.contains(&message.destination) {
                        match msg {
                            &WindowMessage::Move(_) => {
                                // Window can be docked only if current tile is not split already.
                                if let TileContent::Empty | TileContent::Window(_) = self.content {
                                    // When window is being dragged, we should check which tile can accept it.
                                    let pos = ui.cursor_position;
                                    for &anchor in &self.anchors() {
                                        ui.node_mut(anchor).set_background(Brush::Solid(DEFAULT_ANCHOR_COLOR));
                                    }
                                    if ui.node(self.left_anchor).screen_bounds().contains(pos.x, pos.y) {
                                        ui.node_mut(self.left_anchor).set_background(Brush::Solid(Color::WHITE));
                                        self.drop_anchor = self.left_anchor;
                                    } else if ui.node(self.right_anchor).screen_bounds().contains(pos.x, pos.y) {
                                        ui.node_mut(self.right_anchor).set_background(Brush::Solid(Color::WHITE));
                                        self.drop_anchor = self.right_anchor;
                                    } else if ui.node(self.top_anchor).screen_bounds().contains(pos.x, pos.y) {
                                        ui.node_mut(self.top_anchor).set_background(Brush::Solid(Color::WHITE));
                                        self.drop_anchor = self.top_anchor;
                                    } else if ui.node(self.bottom_anchor).screen_bounds().contains(pos.x, pos.y) {
                                        ui.node_mut(self.bottom_anchor).set_background(Brush::Solid(Color::WHITE));
                                        self.drop_anchor = self.bottom_anchor;
                                    } else if ui.node(self.center_anchor).screen_bounds().contains(pos.x, pos.y) {
                                        ui.node_mut(self.center_anchor).set_background(Brush::Solid(Color::WHITE));
                                        self.drop_anchor = self.center_anchor;
                                    } else {
                                        self.drop_anchor = Handle::NONE;
                                    }
                                }
                            }
                            WindowMessage::MoveStart => {
                                if let TileContent::Empty | TileContent::Window(_) = self.content {
                                    // Show anchors.
                                    for &anchor in &self.anchors() {
                                        ui.node_mut(anchor).set_visibility(true);
                                    }
                                }
                            }
                            WindowMessage::MoveEnd => {
                                // Hide anchors.
                                for &anchor in &self.anchors() {
                                    ui.node_mut(anchor).set_visibility(false);
                                }

                                // Drop if has any drop anchor.
                                if self.drop_anchor.is_some() {
                                    match self.content {
                                        TileContent::Empty | TileContent::Window(_) => {
                                            if self.drop_anchor == self.center_anchor {
                                                if let TileContent::Window(_) = self.content {
                                                    // TODO: This most likely will require some sort of tab control to
                                                    //  be able to choose windows.
                                                } else {
                                                    self.content = TileContent::Window(message.destination);
                                                    ui.send_message(UiMessage {
                                                        handled: false,
                                                        data: UiMessageData::Widget(WidgetMessage::LinkWith(self.handle)),
                                                        destination: message.destination,
                                                    });
                                                }
                                            } else if self.drop_anchor == self.left_anchor {
                                                // Split horizontally, dock to left.
                                                self.split(ui, message.destination, SplitDirection::Horizontal, true);
                                            } else if self.drop_anchor == self.right_anchor {
                                                // Split horizontally, dock to right.
                                                self.split(ui, message.destination, SplitDirection::Horizontal, false);
                                            } else if self.drop_anchor == self.top_anchor {
                                                // Split vertically, dock to top.
                                                self.split(ui, message.destination, SplitDirection::Vertical, true);
                                            } else if self.drop_anchor == self.bottom_anchor {
                                                // Split vertically, dock to bottom.
                                                self.split(ui, message.destination, SplitDirection::Vertical, false);
                                            }
                                        }
                                        // Rest cannot accept windows.
                                        _ => ()
                                    }
                                }
                            }
                            _ => ()
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

enum SplitDirection {
    Horizontal,
    Vertical,
}

impl<M: 'static, C: 'static + Control<M, C>> Tile<M, C> {
    pub fn anchors(&self) -> [Handle<UINode<M, C>>; 5] {
        [self.left_anchor, self.right_anchor, self.top_anchor, self.bottom_anchor, self.center_anchor]
    }

    fn split(&mut self, ui: &mut UserInterface<M, C>, window: Handle<UINode<M, C>>, direction: SplitDirection, first: bool) {
        let existing_content = match self.content {
            TileContent::Window(existing_window) => existing_window,
            _ => Handle::NONE
        };

        let first_tile = TileBuilder::new(WidgetBuilder::new())
            .with_content({
                if first {
                    TileContent::Window(window)
                } else {
                    TileContent::Empty
                }
            })
            .build(ui);

        let second_tile = TileBuilder::new(WidgetBuilder::new())
            .with_content({
                if first {
                    TileContent::Empty
                } else {
                    TileContent::Window(window)
                }
            })
            .build(ui);

        if !first && existing_content.is_some() {
            // We can't set content directly, so use deferred call.
            if let UINode::Tile(first_tile) = ui.node_mut(first_tile) {
                first_tile.content = TileContent::Window(existing_content);
            }
            ui.send_message(UiMessage {
                handled: false,
                data: UiMessageData::Widget(WidgetMessage::LinkWith(first_tile)),
                destination: existing_content,
            });
        }

        if first && existing_content.is_some() {
            // We can't set content directly, so use deferred call.
            if let UINode::Tile(second_tile) = ui.node_mut(second_tile) {
                second_tile.content = TileContent::Window(existing_content);
            }
            ui.send_message(UiMessage {
                handled: false,
                data: UiMessageData::Widget(WidgetMessage::LinkWith(second_tile)),
                destination: existing_content,
            });
        }

        self.content = match direction {
            SplitDirection::Horizontal => {
                TileContent::HorizontalTiles {
                    tiles: [first_tile, second_tile],
                    splitter: 0.5,
                }
            }
            SplitDirection::Vertical => {
                TileContent::VerticalTiles {
                    tiles: [first_tile, second_tile],
                    splitter: 0.5,
                }
            }
        };

        // All messages must be sent *after* all nodes are created, otherwise it will panic!
        ui.send_message(UiMessage {
            handled: false,
            data: UiMessageData::Widget(WidgetMessage::LinkWith(self.handle)),
            destination: first_tile,
        });
        ui.send_message(UiMessage {
            handled: false,
            data: UiMessageData::Widget(WidgetMessage::LinkWith(self.handle)),
            destination: second_tile,
        });

        let splitter = ui.node_mut(self.splitter);

        splitter.set_visibility(true);
        match direction {
            SplitDirection::Horizontal => {
                splitter.set_width_mut(DEFAULT_SPLITTER_SIZE)
                    .set_height(std::f32::INFINITY);
            }
            SplitDirection::Vertical => {
                splitter.set_height_mut(DEFAULT_SPLITTER_SIZE)
                    .set_width(std::f32::INFINITY);
            }
        }
    }
}

pub struct DockingManager<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    floating_windows: Vec<Handle<UINode<M, C>>>,
}

impl<M: 'static, C: 'static + Control<M, C>> Deref for DockingManager<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> DerefMut for DockingManager<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> Clone for DockingManager<M, C> {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.raw_copy(),
            floating_windows: self.floating_windows.clone(),
        }
    }
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for DockingManager<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        UINode::DockingManager(self.clone())
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_routed_message(ui, message);
    }

    fn preview_message(&mut self, _ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        if let UiMessageData::Widget(msg) = &message.data {
            if let WidgetMessage::LinkWith(_) = msg {
                if let Some(pos) = self.floating_windows.iter().position(|&i| i == message.destination) {
                    self.floating_windows.remove(pos);
                }
            }
        }
    }
}

pub struct DockingManagerBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    floating_windows: Vec<Handle<UINode<M, C>>>,
}

impl<M, C: 'static + Control<M, C>> DockingManagerBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            floating_windows: Default::default(),
        }
    }

    pub fn with_floating_windows(mut self, windows: Vec<Handle<UINode<M, C>>>) -> Self {
        self.floating_windows = windows;
        self
    }

    pub fn build(self, ui: &mut UserInterface<M, C>) -> Handle<UINode<M, C>> {
        let docking_manager = DockingManager {
            widget: self.widget_builder
                .build(ui.sender()),
            floating_windows: self.floating_windows,
        };

        let handle = ui.add_node(UINode::DockingManager(docking_manager));

        ui.flush_messages();

        handle
    }
}

pub struct TileBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    content: TileContent<M, C>,
}

pub const DEFAULT_SPLITTER_SIZE: f32 = 6.0;
pub const DEFAULT_ANCHOR_COLOR: Color = Color::opaque(150, 150, 150);

pub fn make_default_anchor<M, C: 'static + Control<M, C>>(ui: &mut UserInterface<M, C>) -> Handle<UINode<M, C>> {
    let default_anchor_size = 30.0;
    BorderBuilder::new(WidgetBuilder::new()
        .with_width(default_anchor_size)
        .with_height(default_anchor_size)
        .with_visibility(false)
        .on_row(2)
        .on_column(1)
        .with_draw_on_top(true)
        .with_background(Brush::Solid(DEFAULT_ANCHOR_COLOR)))
        .build(ui)
}

impl<M, C: 'static + Control<M, C>> TileBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            content: TileContent::Empty,
        }
    }

    pub fn with_content(mut self, content: TileContent<M, C>) -> Self {
        self.content = content;
        self
    }

    pub fn build(self, ui: &mut UserInterface<M, C>) -> Handle<UINode<M, C>> {
        let left_anchor = make_default_anchor(ui);
        ui.node_mut(left_anchor).set_row(2).set_column(1);

        let right_anchor = make_default_anchor(ui);
        ui.node_mut(right_anchor).set_row(2).set_column(3);

        let dock_anchor = make_default_anchor(ui);
        ui.node_mut(dock_anchor).set_row(2).set_column(2);

        let top_anchor = make_default_anchor(ui);
        ui.node_mut(top_anchor).set_row(1).set_column(2);

        let bottom_anchor = make_default_anchor(ui);
        ui.node_mut(bottom_anchor).set_row(3).set_column(2);

        let grid = GridBuilder::new(WidgetBuilder::new()
            .with_child(left_anchor)
            .with_child(dock_anchor)
            .with_child(right_anchor)
            .with_child(top_anchor)
            .with_child(bottom_anchor))
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
            .build(ui);

        let splitter = BorderBuilder::new(WidgetBuilder::new()
            .with_width({
                if let TileContent::HorizontalTiles { .. } = self.content {
                    DEFAULT_SPLITTER_SIZE
                } else {
                    std::f32::INFINITY
                }
            })
            .with_height({
                if let TileContent::VerticalTiles { .. } = self.content {
                    DEFAULT_SPLITTER_SIZE
                } else {
                    std::f32::INFINITY
                }
            })
            .with_visibility(match self.content {
                TileContent::VerticalTiles { .. } | TileContent::HorizontalTiles { .. } => true,
                _ => false
            })
            .with_margin(Thickness::uniform(1.0)))
            .build(ui);

        let children = match self.content {
            TileContent::Window(window) => vec![window],
            TileContent::VerticalTiles { tiles, .. } => vec![tiles[0], tiles[1]],
            TileContent::HorizontalTiles { tiles, .. } => vec![tiles[0], tiles[1]],
            _ => vec![],
        };

        let tile = Tile {
            widget: self.widget_builder
                .with_child(grid)
                .with_child(splitter)
                .with_children(&children)
                .build(ui.sender()),
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

        let handle = ui.add_node(UINode::Tile(tile));

        ui.flush_messages();

        handle
    }
}