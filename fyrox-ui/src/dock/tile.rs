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
    core::{
        algebra::Vector2, color::Color, math::Rect, pool::Handle, reflect::prelude::*,
        type_traits::prelude::*, visitor::prelude::*,
    },
    define_constructor,
    dock::DockingManager,
    grid::{Column, GridBuilder, Row},
    message::{CursorIcon, MessageDirection, UiMessage},
    tab_control::{TabControl, TabControlBuilder, TabControlMessage, TabDefinition},
    text::TextBuilder,
    widget::{Widget, WidgetBuilder, WidgetMessage},
    window::{Window, WindowMessage},
    BuildContext, Control, Thickness, UiNode, UserInterface,
};

use core::f32;
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
    MultiWindow {
        index: u32,
        windows: Vec<Handle<UiNode>>,
    },
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
    /// True if a window can be docked in a tile that currently has this content.
    pub fn can_dock(&self) -> bool {
        matches!(
            self,
            Self::Empty | Self::Window(_) | Self::MultiWindow { .. }
        )
    }
    pub fn contains_window(&self, window: Handle<UiNode>) -> bool {
        match self {
            Self::Window(handle) => window == *handle,
            Self::MultiWindow { windows, .. } => windows.contains(&window),
            _ => false,
        }
    }
    /// Construct a new tile that adds the given window to this tile.
    /// This tile must be either empty, a window, or a multiwindow, or else panic.
    pub fn plus_window(self, window: Handle<UiNode>) -> Self {
        match self {
            Self::Empty => Self::Window(window),
            Self::Window(handle) => Self::MultiWindow {
                index: 0,
                windows: vec![window, handle],
            },
            Self::MultiWindow { mut windows, .. } => {
                windows.push(window);
                Self::MultiWindow {
                    index: windows.len() as u32 - 1,
                    windows,
                }
            }
            _ => panic!("Cannot add window to split tile"),
        }
    }
    /// Construct a new tile that removes the given window from this tile.
    /// This tile must be either empty, a window, or a multiwindow, or else panic.
    /// If the window does not exist in this tile, then return self.
    pub fn minus_window(self, window: Handle<UiNode>) -> Self {
        match self {
            Self::Empty => Self::Empty,
            Self::Window(handle) => {
                if window == handle {
                    Self::Empty
                } else {
                    self
                }
            }
            Self::MultiWindow { index, mut windows } => {
                let current = windows.get(index as usize).copied();
                windows.retain(|h| h != &window);
                match windows.len() {
                    0 => Self::Empty,
                    1 => Self::Window(windows[0]),
                    _ => {
                        let index = if let Some(current) = current {
                            windows
                                .iter()
                                .position(|w| w == &current)
                                .unwrap_or_default() as u32
                        } else {
                            0
                        };
                        Self::MultiWindow { index, windows }
                    }
                }
            }
            _ => panic!("Cannot subtract window from split tile"),
        }
    }
    /// Construct a new tile that makes the given window active.
    /// If this tile is not a multiwindow or this tile does not contain
    /// the given window, return self.
    pub fn with_active(self, window: Handle<UiNode>) -> Self {
        match self {
            Self::MultiWindow { index, windows } => {
                let index = if let Some(index) = windows.iter().position(|h| h == &window) {
                    index as u32
                } else {
                    index
                };
                Self::MultiWindow { index, windows }
            }
            _ => self,
        }
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
    let handle = match &tile.content {
        TileContent::Window(handle) => handle,
        TileContent::MultiWindow { index, windows } => windows.get(*index as usize)?,
        _ => return None,
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
    pub tabs: Handle<UiNode>,
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
    fn measure_override(
        &self,
        ui: &UserInterface,
        mut available_size: Vector2<f32>,
    ) -> Vector2<f32> {
        if has_one_minimized(ui, &self.content) {
            return self.measure_vertical_with_minimized(ui, available_size);
        }
        ui.measure_node(self.tabs, Vector2::new(available_size.x, f32::INFINITY));
        available_size.y -= ui.node(self.tabs).desired_size().y;
        for &child_handle in self.children() {
            if child_handle == self.tabs {
                continue;
            }
            // Determine available size for each child by its kind:
            // - Every child not in content of tile just takes whole available size.
            // - Every content's child uses specific available measure size.
            // This is a bit weird, but it is how it works.
            let available_size = match &self.content {
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
        match &self.content {
            TileContent::Empty => Vector2::default(),
            TileContent::Window(handle) => ui.node(*handle).desired_size(),
            TileContent::MultiWindow { index, windows } => {
                let tabs = ui.node(self.tabs).desired_size();
                let body = windows
                    .get(*index as usize)
                    .map(|w| ui.node(*w).desired_size())
                    .unwrap_or_default();
                let y = if available_size.y.is_finite() {
                    (available_size.y - tabs.y).max(0.0)
                } else {
                    tabs.y + body.y
                };
                Vector2::new(tabs.x.max(body.x), y)
            }
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

        let tabs_height = ui.node(self.tabs).desired_size().y;
        ui.arrange_node(self.tabs, &Rect::new(0.0, 0.0, final_size.x, tabs_height));
        let full_bounds = Rect::new(0.0, tabs_height, final_size.x, final_size.y - tabs_height);
        for &child_handle in self.children() {
            if child_handle == self.tabs {
                continue;
            }
            let bounds = match &self.content {
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

        if let Some(TabControlMessage::ActiveTabUuid(Some(id))) = message.data() {
            if message.destination() == self.tabs
                && message.direction() == MessageDirection::FromWidget
            {
                self.change_active_tab(id, ui);
            }
        } else if let Some(msg) = message.data::<TileMessage>() {
            if message.destination() == self.handle() {
                match msg {
                    TileMessage::Content(content) => {
                        self.content = content.clone();

                        send_visibility(
                            ui,
                            self.tabs,
                            matches!(self.content, TileContent::MultiWindow { .. }),
                        );

                        match content {
                            TileContent::Empty => {
                                send_visibility(ui, self.splitter, false);
                            }
                            &TileContent::Window(window) => {
                                send_visibility(ui, self.splitter, false);
                                send_visibility(ui, window, true);
                                self.dock(window, ui);
                            }
                            TileContent::MultiWindow { index, windows } => {
                                send_visibility(ui, self.splitter, false);
                                let tabs = ui.node(self.tabs).cast::<TabControl>().unwrap();
                                for tab in tabs.tabs.iter() {
                                    let uuid = tab.uuid;
                                    if !windows.iter().any(|&h| ui.node(h).id == uuid) {
                                        ui.send_message(TabControlMessage::remove_tab_by_uuid(
                                            self.tabs,
                                            MessageDirection::ToWidget,
                                            uuid,
                                        ));
                                    }
                                }
                                for (i, &w) in windows.iter().enumerate() {
                                    let is_active = i as u32 == *index;
                                    let uuid = ui.node(w).id;
                                    let tabs = ui.node(self.tabs).cast::<TabControl>().unwrap();
                                    if tabs.get_tab_by_uuid(uuid).is_none() {
                                        self.add_tab(w, ui);
                                    }
                                    send_visibility(ui, w, is_active);
                                    self.dock(w, ui);
                                }
                                if let Some(&w) = windows.get(*index as usize) {
                                    let uuid = ui.node(w).id;
                                    ui.send_message(TabControlMessage::active_tab_uuid(
                                        self.tabs,
                                        MessageDirection::ToWidget,
                                        Some(uuid),
                                    ));
                                }
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
                        if matches!(
                            self.content,
                            TileContent::Window(_) | TileContent::MultiWindow { .. }
                        ) {
                            self.split(ui, window, direction, first);
                        }
                    }
                }
            }
        } else if let Some(msg) = message.data::<WidgetMessage>() {
            match msg {
                &WidgetMessage::Topmost => {
                    if let TileContent::MultiWindow { ref windows, .. } = self.content {
                        if windows.contains(&message.destination()) {
                            let id = ui.node(message.destination()).id;
                            self.change_active_tab(&id, ui);
                        }
                    }
                }
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
                                            TileContent::MultiWindow { index, ref windows } => {
                                                for &sub_tile_wnd in windows {
                                                    ui.send_message(WidgetMessage::unlink(
                                                        sub_tile_wnd,
                                                        MessageDirection::ToWidget,
                                                    ));
                                                }

                                                ui.send_message(TileMessage::content(
                                                    self.handle,
                                                    MessageDirection::ToWidget,
                                                    TileContent::MultiWindow {
                                                        index,
                                                        windows: windows.clone(),
                                                    },
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
                    let content_moved = self.content.contains_window(message.destination());
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
                    let content_moved = self.content.contains_window(message.destination());

                    if content_moved {
                        if let Some(window) = ui.node(message.destination()).cast::<Window>() {
                            if window.drag_delta.norm() > 20.0 {
                                self.undock(window, ui);
                            }
                        }
                    }
                }
                WindowMessage::Close => match self.content {
                    TileContent::MultiWindow { ref windows, .. } => {
                        if windows.contains(&message.destination()) {
                            let window = ui
                                .node(message.destination())
                                .cast::<Window>()
                                .expect("must be window");
                            self.undock(window, ui);
                        }
                    }
                    TileContent::VerticalTiles { tiles, .. }
                    | TileContent::HorizontalTiles { tiles, .. } => {
                        let closed_window = message.destination();

                        fn tile_has_window(
                            tile: Handle<UiNode>,
                            ui: &UserInterface,
                            window: Handle<UiNode>,
                        ) -> bool {
                            if let Some(tile_ref) = ui.node(tile).query_component::<Tile>() {
                                if let TileContent::Window(tile_window) = tile_ref.content {
                                    tile_window == window
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        }

                        for (tile_a_index, tile_b_index) in [(0, 1), (1, 0)] {
                            let tile_a = tiles[tile_a_index];
                            let tile_b = tiles[tile_b_index];
                            if tile_has_window(tile_a, ui, closed_window) {
                                if let Some(tile_a_ref) = ui.node(tile_a).query_component::<Tile>()
                                {
                                    let window = ui
                                        .node(closed_window)
                                        .cast::<Window>()
                                        .expect("must be window");
                                    tile_a_ref.undock(window, ui);
                                }
                                if let Some(tile_b_ref) = ui.node(tile_b).query_component::<Tile>()
                                {
                                    ui.send_message(WidgetMessage::unlink(
                                        closed_window,
                                        MessageDirection::ToWidget,
                                    ));

                                    tile_b_ref.unlink_content(ui);

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
                            // Window can be docked only if current tile is not split already.
                            if self.content.can_dock() {
                                // Show anchors.
                                for &anchor in &self.anchors() {
                                    send_visibility(ui, anchor, true);
                                }
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
                                match &self.content {
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
                                    TileContent::Window(_) | TileContent::MultiWindow { .. } => {
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
                                        } else if self.drop_anchor.get() == self.center_anchor {
                                            ui.send_message(TileMessage::content(
                                                self.handle,
                                                MessageDirection::ToWidget,
                                                self.content
                                                    .clone()
                                                    .plus_window(message.destination()),
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

fn create_tab_header(label: String, ctx: &mut BuildContext) -> Handle<UiNode> {
    let min_size = Vector2::new(50.0, 12.0);
    let margin = Thickness {
        left: 4.0,
        top: 2.0,
        right: 4.0,
        bottom: 2.0,
    };
    TextBuilder::new(
        WidgetBuilder::new()
            .with_min_size(min_size)
            .with_margin(margin),
    )
    .with_text(label)
    .build(ctx)
}

impl Tile {
    fn change_active_tab(&mut self, id: &Uuid, ui: &mut UserInterface) {
        let TileContent::MultiWindow { index, windows } = &self.content else {
            return;
        };
        let mut window = None;
        for (i, w) in windows.iter().enumerate() {
            let window_id = ui.node(*w).id;
            if &window_id == id {
                if i as u32 == *index {
                    return;
                } else {
                    window = Some(*w);
                    break;
                }
            }
        }
        let Some(window) = window else {
            return;
        };
        let new_content = self.content.clone().with_active(window);
        ui.send_message(TileMessage::content(
            self.handle(),
            MessageDirection::ToWidget,
            new_content,
        ));
    }
    fn unlink_content(&self, ui: &UserInterface) {
        match &self.content {
            TileContent::Empty => {}
            TileContent::Window(window) => {
                ui.send_message(WidgetMessage::unlink(*window, MessageDirection::ToWidget));
            }
            TileContent::MultiWindow { windows, .. } => {
                for tile in windows.iter() {
                    ui.send_message(WidgetMessage::unlink(*tile, MessageDirection::ToWidget));
                }
            }
            TileContent::VerticalTiles {
                tiles: sub_tiles, ..
            }
            | TileContent::HorizontalTiles {
                tiles: sub_tiles, ..
            } => {
                for tile in sub_tiles {
                    ui.send_message(WidgetMessage::unlink(*tile, MessageDirection::ToWidget));
                }
            }
        }
    }
    /// Creates a tab for the window with the given handle.
    fn add_tab(&self, window: Handle<UiNode>, ui: &mut UserInterface) {
        let window = ui.node(window).cast::<Window>().expect("must be window");
        let uuid = window.id;
        let header = create_tab_header(window.tab_label().to_owned(), &mut ui.build_ctx());
        let definition = TabDefinition {
            can_be_closed: false,
            header,
            content: Handle::NONE,
            user_data: None,
        };
        ui.send_message(TabControlMessage::add_tab_with_uuid(
            self.tabs,
            MessageDirection::ToWidget,
            uuid,
            definition,
        ));
    }
    /// Send messages to prepare the window at the given handle for being docked
    /// in this tile.
    fn dock(&self, window: Handle<UiNode>, ui: &UserInterface) {
        ui.send_message(WidgetMessage::link(
            window,
            MessageDirection::ToWidget,
            self.handle(),
        ));

        ui.send_message(WindowMessage::can_resize(
            window,
            MessageDirection::ToWidget,
            false,
        ));

        // Make the window size undefined, so it will be stretched to the tile
        // size correctly.
        send_size(ui, window, f32::NAN, f32::NAN);
    }

    /// Remove window from this tile. When this is called
    /// this tile should have [`TileContent::Window`] and the window
    /// contained in this tile must be given window.
    fn undock(&self, window: &Window, ui: &UserInterface) {
        ui.send_message(TileMessage::content(
            self.handle,
            MessageDirection::ToWidget,
            self.content.clone().minus_window(window.handle()),
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
        let d_1 = ui.node(minimized_handle).desired_size();
        size.y = available_size.y - d_1.y;
        let other_index = if minimized_index == 0 { 1 } else { 0 };
        ui.measure_node(tiles[other_index], size);
        size.y = 0.0;
        ui.measure_node(self.splitter, size);
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

        ui.send_message(TileMessage::content(
            if first { second_tile } else { first_tile },
            MessageDirection::ToWidget,
            std::mem::take(&mut self.content),
        ));

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

        let mut tabs = TabControlBuilder::new(
            WidgetBuilder::new().with_background(Brush::Solid(Color::BLACK).into()),
        )
        .with_tab_drag(true);

        match self.content {
            TileContent::Window(window) => {
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
            TileContent::MultiWindow { ref windows, index } => {
                for (i, &window) in windows.iter().enumerate() {
                    let window = ctx[window].cast_mut::<Window>().expect("must be window");
                    window.can_resize = false;
                    window.width.set_value_and_mark_modified(f32::NAN);
                    window.height.set_value_and_mark_modified(f32::NAN);
                    window.set_visibility(index as usize == i);
                    let id = window.id;
                    let header = create_tab_header(window.tab_label().to_owned(), ctx);
                    let definition = TabDefinition {
                        can_be_closed: false,
                        content: Handle::NONE,
                        user_data: None,
                        header,
                    };
                    tabs = tabs.with_tab_uuid(id, definition);
                }
                tabs = tabs.with_initial_tab(index as usize);
            }
            _ => (),
        }

        let tabs = tabs.build(ctx);

        let children = match &self.content {
            TileContent::Window(window) => vec![*window],
            TileContent::MultiWindow { windows, .. } => windows.clone(),
            TileContent::VerticalTiles { tiles, .. } => vec![tiles[0], tiles[1]],
            TileContent::HorizontalTiles { tiles, .. } => vec![tiles[0], tiles[1]],
            TileContent::Empty => vec![],
        };

        let tile = Tile {
            widget: self
                .widget_builder
                .with_preview_messages(true)
                .with_child(grid)
                .with_child(splitter)
                .with_child(tabs)
                .with_children(children)
                .build(ctx),
            tabs,
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
