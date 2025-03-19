// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::{
    border::BorderBuilder,
    brush::Brush,
    core::{algebra::Vector2, color::Color, math::Rect, pool::Handle},
    core::{reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*},
    define_constructor,
    dock::DockingManager,
    grid::{Column, GridBuilder, Row},
    message::{CursorIcon, MessageDirection, UiMessage},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    window::{Window, WindowMessage},
    BuildContext, Control, Thickness, UiNode, UserInterface,
};

use fyrox_core::uuid_provider;
use fyrox_graph::constructor::{ConstructorProvider, GraphNodeConstructor};
use fyrox_graph::{BaseSceneGraph, SceneGraph};
use std::{
    cell::Cell,
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

#[derive(Default, Debug, PartialEq, Clone, Visit, Reflect)]
pub enum TileContent {
    #[default]
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

fn send_visibility(ui: &UserInterface, destination: Handle<UiNode>, visible: bool) {
    ui.send_message(WidgetMessage::visibility(
        destination,
        MessageDirection::ToWidget,
        visible,
    ));
}

fn send_size(ui: &UserInterface, destination: Handle<UiNode>, width: f32, height: f32) {
    ui.send_message(WidgetMessage::width(
        destination,
        MessageDirection::ToWidget,
        width,
    ));
    ui.send_message(WidgetMessage::height(
        destination,
        MessageDirection::ToWidget,
        height,
    ));
}

fn send_background(ui: &UserInterface, destination: Handle<UiNode>, color: Color) {
    ui.send_message(WidgetMessage::background(
        destination,
        MessageDirection::ToWidget,
        Brush::Solid(color).into(),
    ));
}

/// The window contained by the tile at the given handle, if the handle points
/// to a tile and the tile has [`TileContent::Window`].
fn get_tile_window(ui: &UserInterface, tile: Handle<UiNode>) -> Option<&Window> {
    let tile = ui.node(tile).cast::<Tile>()?;
    let TileContent::Window(handle) = &tile.content else {
        return None;
    };
    ui.node(*handle).cast::<Window>()
}

/// True if the the given handle points to a tile that has been minimized.
fn is_minimized_window(ui: &UserInterface, tile: Handle<UiNode>) -> bool {
    let Some(window) = get_tile_window(ui, tile) else {
        return false;
    };
    window.minimized()
}

/// True if the given `TileContent` contains exactly one minimized tile as one of its
/// two members. Only [`TileContent::VerticalTiles`] or [`TileContent::HorizontalTiles`]
/// may satisfyin this condition, and only if at least one of its two child tiles
/// is a window tile. This serves to detect the case when a tile needs special layout
/// calculation.
fn has_one_minimized(ui: &UserInterface, content: &TileContent) -> bool {
    let tiles = if let TileContent::VerticalTiles { tiles, .. } = content {
        Some(tiles)
    } else if let TileContent::HorizontalTiles { tiles, .. } = content {
        Some(tiles)
    } else {
        None
    };
    if let Some(tiles) = tiles {
        tiles
            .iter()
            .filter(|h| is_minimized_window(ui, **h))
            .count()
            == 1
    } else {
        false
    }
}

/// Given two tiles and the handle of a window, check that one of the two tiles
/// is a window tile that is holding the given window, and if so then ensure
/// that the other tile is not a minimized window. The idea is to ensure
/// that at most one of the two tiles is minimized at any time.
fn deminimize_other_window(
    this_window: Handle<UiNode>,
    tiles: &[Handle<UiNode>; 2],
    ui: &UserInterface,
) {
    let mut has_this_window = false;
    let mut other_window: Option<Handle<UiNode>> = None;
    for tile in tiles.iter() {
        let Some(window) = get_tile_window(ui, *tile) else {
            return;
        };
        if window.handle() == this_window {
            has_this_window = true;
        } else if window.minimized() {
            other_window = Some(window.handle());
        }
    }
    if !has_this_window {
        return;
    }
    if let Some(handle) = other_window {
        ui.send_message(WindowMessage::minimize(
            handle,
            MessageDirection::ToWidget,
            false,
        ));
    }
}

#[derive(Default, Clone, Debug, Visit, Reflect, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct Tile {
    pub widget: Widget,
    pub left_anchor: Handle<UiNode>,
    pub right_anchor: Handle<UiNode>,
    pub top_anchor: Handle<UiNode>,
    pub bottom_anchor: Handle<UiNode>,
    pub center_anchor: Handle<UiNode>,
    pub content: TileContent,
    pub splitter: Handle<UiNode>,
    pub dragging_splitter: bool,
    pub drop_anchor: Cell<Handle<UiNode>>,
}

impl ConstructorProvider<UiNode, UserInterface> for Tile {
    fn constructor() -> GraphNodeConstructor<UiNode, UserInterface> {
        GraphNodeConstructor::new::<Self>()
            .with_variant("Tile", |ui| {
                TileBuilder::new(WidgetBuilder::new().with_name("Tile"))
                    .build(&mut ui.build_ctx())
                    .into()
            })
            .with_group("Layout")
    }
}

crate::define_widget_deref!(Tile);

uuid_provider!(Tile = "8ed17fa9-890e-4dd7-b4f9-a24660882234");

impl Control for Tile {
    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        if has_one_minimized(ui, &self.content) {
            return self.measure_vertical_with_minimized(ui, available_size);
        }
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
        match self.content {
            TileContent::Empty => Vector2::default(),
            TileContent::Window(handle) => ui.node(handle).desired_size(),
            TileContent::VerticalTiles { tiles, .. } => {
                let mut w = 0.0f32;
                let mut h = DEFAULT_SPLITTER_SIZE;
                for size in tiles.map(|c| ui.node(c).desired_size()) {
                    w = w.max(size.x);
                    h += size.y;
                }
                Vector2::new(w, h)
            }
            TileContent::HorizontalTiles { tiles, .. } => {
                let mut w = DEFAULT_SPLITTER_SIZE;
                let mut h = 0.0f32;
                for size in tiles.map(|c| ui.node(c).desired_size()) {
                    w += size.x;
                    h = h.max(size.y);
                }
                Vector2::new(w, h)
            }
        }
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        let splitter_size = ui.node(self.splitter).desired_size();

        if has_one_minimized(ui, &self.content) {
            return self.arrange_vertical_with_minimized(ui, final_size);
        }

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
                            final_size.y * splitter - DEFAULT_SPLITTER_SIZE * 0.5,
                        )
                    } else if tiles[1] == child_handle {
                        Rect::new(
                            0.0,
                            final_size.y * splitter + splitter_size.y * 0.5,
                            final_size.x,
                            final_size.y * (1.0 - splitter) - DEFAULT_SPLITTER_SIZE * 0.5,
                        )
                    } else if self.splitter == child_handle {
                        Rect::new(
                            0.0,
                            final_size.y * splitter - DEFAULT_SPLITTER_SIZE * 0.5,
                            final_size.x,
                            DEFAULT_SPLITTER_SIZE,
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
                            final_size.x * splitter - DEFAULT_SPLITTER_SIZE * 0.5,
                            final_size.y,
                        )
                    } else if tiles[1] == child_handle {
                        Rect::new(
                            final_size.x * splitter + DEFAULT_SPLITTER_SIZE * 0.5,
                            0.0,
                            final_size.x * (1.0 - splitter) - DEFAULT_SPLITTER_SIZE * 0.5,
                            final_size.y,
                        )
                    } else if self.splitter == child_handle {
                        Rect::new(
                            final_size.x * splitter - DEFAULT_SPLITTER_SIZE * 0.5,
                            0.0,
                            DEFAULT_SPLITTER_SIZE,
                            final_size.y,
                        )
                    } else {
                        full_bounds
                    }
                }
                _ => full_bounds,
            };

            ui.arrange_node(child_handle, &bounds);
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
                                send_visibility(ui, self.splitter, false);
                            }
                            &TileContent::Window(window) => {
                                ui.send_message(WidgetMessage::link(
                                    window,
                                    MessageDirection::ToWidget,
                                    self.handle(),
                                ));

                                send_visibility(ui, self.splitter, false);

                                ui.send_message(WindowMessage::can_resize(
                                    window,
                                    MessageDirection::ToWidget,
                                    false,
                                ));

                                // Make the window size undefined, so it will be stretched to the tile
                                // size correctly.
                                send_size(ui, window, f32::NAN, f32::NAN);
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

                                send_visibility(ui, self.splitter, true);
                                match content {
                                    TileContent::HorizontalTiles { .. } => {
                                        ui.send_message(WidgetMessage::cursor(
                                            self.splitter,
                                            MessageDirection::ToWidget,
                                            Some(CursorIcon::WResize),
                                        ));
                                    }
                                    TileContent::VerticalTiles { .. } => {
                                        ui.send_message(WidgetMessage::cursor(
                                            self.splitter,
                                            MessageDirection::ToWidget,
                                            Some(CursorIcon::NResize),
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
                    if !message.handled()
                        && message.destination() == self.splitter
                        && !has_one_minimized(ui, &self.content)
                    {
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
                                *splitter = ((pos.y - bounds.y()) / bounds.h()).clamp(0.0, 1.0);
                                self.invalidate_layout();
                            }
                            TileContent::HorizontalTiles {
                                ref mut splitter, ..
                            } => {
                                *splitter = ((pos.x - bounds.x()) / bounds.w()).clamp(0.0, 1.0);
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
                                                send_visibility(ui, self.splitter, false);
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
        } else if let Some(msg) = message.data::<WindowMessage>() {
            match msg {
                WindowMessage::Maximize(true) => {
                    // Check if we are maximizing the child window.
                    let content_moved = match self.content {
                        TileContent::Window(window) => window == message.destination(),
                        _ => false,
                    };
                    if content_moved {
                        // Undock the window and re-maximize it, since maximization does nothing to a docked window
                        // because docked windows are not resizable.
                        if let Some(window) = ui.node(message.destination()).cast::<Window>() {
                            self.undock(window, ui);
                            ui.send_message(WindowMessage::maximize(
                                window.handle(),
                                MessageDirection::ToWidget,
                                true,
                            ));
                        }
                    }
                }
                WindowMessage::Minimize(true) => {
                    let tiles = match &self.content {
                        TileContent::VerticalTiles { tiles, .. } => Some(tiles),
                        TileContent::HorizontalTiles { tiles, .. } => Some(tiles),
                        _ => None,
                    };
                    if let Some(tiles) = tiles {
                        deminimize_other_window(message.destination(), tiles, ui);
                    }
                }
                WindowMessage::Move(_) => {
                    // Check if we dragging child window.
                    let content_moved = match self.content {
                        TileContent::Window(window) => window == message.destination(),
                        _ => false,
                    };

                    if content_moved {
                        if let Some(window) = ui.node(message.destination()).cast::<Window>() {
                            if window.drag_delta.norm() > 20.0 {
                                self.undock(window, ui);
                            }
                        }
                    }
                }
                WindowMessage::Close => match self.content {
                    TileContent::VerticalTiles { tiles, .. }
                    | TileContent::HorizontalTiles { tiles, .. } => {
                        let closed_window = message.destination();

                        fn try_get_tile_window(
                            tile: Handle<UiNode>,
                            ui: &UserInterface,
                            window: Handle<UiNode>,
                        ) -> Option<Handle<UiNode>> {
                            if let Some(tile_ref) = ui.node(tile).query_component::<Tile>() {
                                if let TileContent::Window(tile_window) = tile_ref.content {
                                    if tile_window == window {
                                        return Some(tile_window);
                                    }
                                }
                            }
                            None
                        }

                        for (tile_a_index, tile_b_index) in [(0, 1), (1, 0)] {
                            let tile_a = tiles[tile_a_index];
                            let tile_b = tiles[tile_b_index];
                            if let Some(tile_window) =
                                try_get_tile_window(tile_a, ui, closed_window)
                            {
                                if let Some(tile_b_ref) = ui.node(tile_b).query_component::<Tile>()
                                {
                                    ui.send_message(WidgetMessage::unlink(
                                        tile_window,
                                        MessageDirection::ToWidget,
                                    ));

                                    match tile_b_ref.content {
                                        TileContent::Empty => {}
                                        TileContent::Window(window) => {
                                            ui.send_message(WidgetMessage::unlink(
                                                window,
                                                MessageDirection::ToWidget,
                                            ));
                                        }
                                        TileContent::VerticalTiles {
                                            tiles: sub_tiles, ..
                                        }
                                        | TileContent::HorizontalTiles {
                                            tiles: sub_tiles, ..
                                        } => {
                                            for tile in sub_tiles {
                                                ui.send_message(WidgetMessage::unlink(
                                                    tile,
                                                    MessageDirection::ToWidget,
                                                ));
                                            }
                                        }
                                    }

                                    ui.send_message(TileMessage::content(
                                        self.handle,
                                        MessageDirection::ToWidget,
                                        tile_b_ref.content.clone(),
                                    ));

                                    // Destroy tiles.
                                    for &tile in &tiles {
                                        ui.send_message(WidgetMessage::remove(
                                            tile,
                                            MessageDirection::ToWidget,
                                        ));
                                    }

                                    if let Some((_, docking_manager)) =
                                        ui.find_component_up::<DockingManager>(self.parent())
                                    {
                                        docking_manager
                                            .floating_windows
                                            .borrow_mut()
                                            .push(closed_window);
                                    }

                                    break;
                                }
                            }
                        }
                    }
                    _ => {}
                },
                _ => (),
            }
        }
    }

    // We have to use preview_message for docking purposes because dragged window detached
    // from docking manager and handle_routed_message won't receive any messages from window.
    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        if let Some(msg) = message.data::<WindowMessage>() {
            if let Some((_, docking_manager)) =
                ui.find_component_up::<DockingManager>(self.parent())
            {
                // Make sure we are dragging one of floating windows of parent docking manager.
                if message.direction() == MessageDirection::FromWidget
                    && docking_manager
                        .floating_windows
                        .borrow_mut()
                        .contains(&message.destination())
                {
                    match msg {
                        &WindowMessage::Move(_) => {
                            if let TileContent::Empty | TileContent::Window(_) = self.content {
                                // Show anchors.
                                for &anchor in &self.anchors() {
                                    send_visibility(ui, anchor, true);
                                }
                            }

                            // Window can be docked only if current tile is not split already.
                            if let TileContent::Empty | TileContent::Window(_) = self.content {
                                // When window is being dragged, we should check which tile can accept it.
                                let pos = ui.cursor_position;
                                for &anchor in &self.anchors() {
                                    send_background(ui, anchor, DEFAULT_ANCHOR_COLOR);
                                }
                                if ui.node(self.left_anchor).screen_bounds().contains(pos) {
                                    send_background(ui, self.left_anchor, Color::WHITE);
                                    self.drop_anchor.set(self.left_anchor);
                                } else if ui.node(self.right_anchor).screen_bounds().contains(pos) {
                                    send_background(ui, self.right_anchor, Color::WHITE);
                                    self.drop_anchor.set(self.right_anchor);
                                } else if ui.node(self.top_anchor).screen_bounds().contains(pos) {
                                    send_background(ui, self.top_anchor, Color::WHITE);
                                    self.drop_anchor.set(self.top_anchor);
                                } else if ui.node(self.bottom_anchor).screen_bounds().contains(pos)
                                {
                                    send_background(ui, self.bottom_anchor, Color::WHITE);
                                    self.drop_anchor.set(self.bottom_anchor);
                                } else if ui.node(self.center_anchor).screen_bounds().contains(pos)
                                {
                                    send_background(ui, self.center_anchor, Color::WHITE);
                                    self.drop_anchor.set(self.center_anchor);
                                } else {
                                    self.drop_anchor.set(Handle::NONE);
                                }
                            }
                        }
                        WindowMessage::MoveEnd => {
                            // Hide anchors.
                            for &anchor in &self.anchors() {
                                send_visibility(ui, anchor, false);
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
    /// Remove window from this tile. When this is called
    /// this tile should have [`TileContent::Window`] and the window
    /// contained in this tile must be given window.
    fn undock(&mut self, window: &Window, ui: &UserInterface) {
        ui.send_message(TileMessage::content(
            self.handle,
            MessageDirection::ToWidget,
            TileContent::Empty,
        ));

        ui.send_message(WidgetMessage::unlink(
            window.handle(),
            MessageDirection::ToWidget,
        ));

        ui.send_message(WindowMessage::can_resize(
            window.handle(),
            MessageDirection::ToWidget,
            true,
        ));

        let height = if window.minimized() {
            f32::NAN
        } else {
            self.actual_local_size().y
        };

        send_size(ui, window.handle(), self.actual_local_size().x, height);

        if let Some((_, docking_manager)) = ui.find_component_up::<DockingManager>(self.parent()) {
            docking_manager
                .floating_windows
                .borrow_mut()
                .push(window.handle());
        }
    }
    /// Measure the tile in the special case where exactly one of the two child tiles
    /// is a minimized window. The minimized window is put at the top or bottom of the tile
    /// at its natural size, while the unminimized child is made to fill the rest of the tile.
    fn measure_vertical_with_minimized(
        &self,
        ui: &UserInterface,
        available_size: Vector2<f32>,
    ) -> Vector2<f32> {
        let tiles = match self.content {
            TileContent::VerticalTiles { ref tiles, .. } => tiles,
            TileContent::HorizontalTiles { ref tiles, .. } => tiles,
            _ => return Vector2::default(),
        };
        let minimized_index = tiles
            .iter()
            .position(|h| is_minimized_window(ui, *h))
            .unwrap();
        let minimized_handle = tiles[minimized_index];
        let mut size = Vector2::new(available_size.x, f32::INFINITY);
        ui.measure_node(minimized_handle, size);
        size.y = available_size.y;
        let other_index = if minimized_index == 0 { 1 } else { 0 };
        ui.measure_node(tiles[other_index], size);
        size.y = 0.0;
        ui.measure_node(self.splitter, size);
        let d_1 = ui.node(minimized_handle).desired_size();
        let d_2 = ui.node(tiles[other_index]).desired_size();
        Vector2::new(d_1.x.max(d_2.x), d_1.y + d_2.y)
    }
    /// Arrange the tile in the special case where exactly one of the two child tiles
    /// is a minimized window. The minimized window is put at the top or bottom of the tile
    /// at its natural size, while the unminimized child is made to fill the rest of the tile.
    fn arrange_vertical_with_minimized(
        &self,
        ui: &UserInterface,
        final_size: Vector2<f32>,
    ) -> Vector2<f32> {
        let tiles = match self.content {
            TileContent::VerticalTiles { ref tiles, .. } => tiles,
            TileContent::HorizontalTiles { ref tiles, .. } => tiles,
            _ => return final_size,
        };
        let minimized_index = tiles
            .iter()
            .position(|h| is_minimized_window(ui, *h))
            .unwrap();
        let minimized_handle = tiles[minimized_index];
        let height = ui.node(minimized_handle).desired_size().y;
        let mut bounds = if minimized_index == 0 {
            Rect::new(0.0, 0.0, final_size.x, height)
        } else {
            Rect::new(0.0, final_size.y - height, final_size.x, height)
        };
        ui.arrange_node(minimized_handle, &bounds);
        let remaining_height = final_size.y - height;
        bounds.position.y = if minimized_index == 0 {
            height
        } else {
            remaining_height
        };
        bounds.size.y = 0.0;
        ui.arrange_node(self.splitter, &bounds);
        bounds.position.y = if minimized_index == 0 { height } else { 0.0 };
        bounds.size.y = remaining_height;
        let other_index = if minimized_index == 0 { 1 } else { 0 };
        ui.arrange_node(tiles[other_index], &bounds);
        final_size
    }

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
            .with_background(Brush::Solid(DEFAULT_ANCHOR_COLOR).into()),
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
        .with_stroke_thickness(Thickness::uniform(0.0).into())
        .build(ctx);

        if let TileContent::Window(window) = self.content {
            if let Some(window) = ctx[window].cast_mut::<Window>() {
                // Every docked window must be non-resizable (it means that it cannot be resized by user
                // and it still can be resized by a proper message).
                window.can_resize = false;

                // Make the window size undefined, so it will be stretched to the tile
                // size correctly.
                window.width.set_value_and_mark_modified(f32::NAN);
                window.height.set_value_and_mark_modified(f32::NAN);
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
                .build(ctx),
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

#[cfg(test)]
mod test {
    use crate::dock::TileBuilder;
    use crate::{test::test_widget_deletion, widget::WidgetBuilder};

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| TileBuilder::new(WidgetBuilder::new()).build(ctx));
    }
}
