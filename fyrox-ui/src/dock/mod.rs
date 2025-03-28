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

//! Docking manager allows you to dock windows and hold them in-place.
//!
//! # Notes
//!
//! Docking manager can hold any types of UI elements, but dragging works only
//! for windows.

use crate::{
    core::{
        log::Log, pool::Handle, reflect::prelude::*, type_traits::prelude::*, uuid_provider,
        visitor::prelude::*,
    },
    define_constructor,
    dock::config::{DockingManagerLayoutDescriptor, FloatingWindowDescriptor, TileDescriptor},
    message::{MessageDirection, UiMessage},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    window::WindowMessage,
    BuildContext, Control, UiNode, UserInterface,
};

use fyrox_graph::constructor::{ConstructorProvider, GraphNodeConstructor};
use fyrox_graph::{BaseSceneGraph, SceneGraph};
use std::{
    cell::RefCell,
    ops::{Deref, DerefMut},
};

pub mod config;
mod tile;

pub use tile::*;

/// Supported docking manager-specific messages.
#[derive(Debug, Clone, PartialEq)]
pub enum DockingManagerMessage {
    Layout(DockingManagerLayoutDescriptor),
    AddFloatingWindow(Handle<UiNode>),
    RemoveFloatingWindow(Handle<UiNode>),
}

impl DockingManagerMessage {
    define_constructor!(
        /// Creates a new [Self::Layout] message.
        DockingManagerMessage:Layout => fn layout(DockingManagerLayoutDescriptor), layout: false
    );
    define_constructor!(
        /// Creates a new [Self::AddFloatingWindow] message.
        DockingManagerMessage:AddFloatingWindow => fn add_floating_window(Handle<UiNode>), layout: false
    );
    define_constructor!(
        /// Creates a new [Self::RemoveFloatingWindow] message.
        DockingManagerMessage:RemoveFloatingWindow => fn remove_floating_window(Handle<UiNode>), layout: false
    );
}

#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct DockingManager {
    pub widget: Widget,
    pub floating_windows: RefCell<Vec<Handle<UiNode>>>,
}

impl ConstructorProvider<UiNode, UserInterface> for DockingManager {
    fn constructor() -> GraphNodeConstructor<UiNode, UserInterface> {
        GraphNodeConstructor::new::<Self>()
            .with_variant("Docking Manager", |ui| {
                DockingManagerBuilder::new(WidgetBuilder::new().with_name("Docking Manager"))
                    .build(&mut ui.build_ctx())
                    .into()
            })
            .with_group("Layout")
    }
}

crate::define_widget_deref!(DockingManager);

uuid_provider!(DockingManager = "b04299f7-3f6b-45f1-89a6-0dce4ad929e1");

impl Control for DockingManager {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle && message.direction() == MessageDirection::ToWidget
        {
            if let Some(msg) = message.data() {
                match msg {
                    DockingManagerMessage::Layout(layout_descriptor) => {
                        self.set_layout(layout_descriptor, ui);
                    }
                    DockingManagerMessage::AddFloatingWindow(window) => {
                        self.add_floating_window(*window)
                    }
                    DockingManagerMessage::RemoveFloatingWindow(window) => {
                        self.remove_floating_window(*window)
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
                    ui.try_get(*h).map(|w| FloatingWindowDescriptor {
                        name: w.name.clone(),
                        position: w.actual_local_position(),
                        size: w.actual_local_size(),
                        is_open: w.is_globally_visible(),
                    })
                })
                .collect::<Vec<_>>(),
            root_tile_descriptor: self
                .children()
                .first()
                .map(|c| TileDescriptor::from_tile_handle(*c, ui)),
        }
    }

    fn set_layout(
        &mut self,
        layout_descriptor: &DockingManagerLayoutDescriptor,
        ui: &mut UserInterface,
    ) {
        if let Some(root_tile_handle) = self.children.first().cloned() {
            let mut windows = Vec::new();
            let mut stack = vec![root_tile_handle];
            while let Some(tile_handle) = stack.pop() {
                if let Some(tile) = ui
                    .try_get(tile_handle)
                    .and_then(|n| n.query_component::<Tile>())
                {
                    match tile.content {
                        TileContent::Window(window) => {
                            if ui.is_valid_handle(window) {
                                // Detach the window from the tile, this is needed to prevent
                                // deletion of the window when the tile is deleted.
                                ui.unlink_node(window);

                                windows.push(window);
                            }
                        }
                        TileContent::MultiWindow {
                            windows: ref tile_windows,
                            ..
                        } => {
                            for w in tile_windows.clone() {
                                ui.unlink_node(w);
                                windows.push(w);
                            }
                        }
                        TileContent::VerticalTiles { tiles, .. }
                        | TileContent::HorizontalTiles { tiles, .. } => {
                            stack.extend_from_slice(&tiles);
                        }
                        TileContent::Empty => (),
                    }
                }
            }

            // Destroy the root tile with all descendant tiles.
            ui.send_message(WidgetMessage::remove(
                root_tile_handle,
                MessageDirection::ToWidget,
            ));

            // Re-create the tiles according to the layout and attach it to the docking manager.
            if let Some(root_tile_descriptor) = layout_descriptor.root_tile_descriptor.as_ref() {
                let root_tile = root_tile_descriptor.create_tile(ui, &windows);
                ui.send_message(WidgetMessage::link(
                    root_tile,
                    MessageDirection::ToWidget,
                    self.handle,
                ));
            }

            // Restore floating windows.
            self.floating_windows.borrow_mut().clear();
            for floating_window_desc in layout_descriptor.floating_windows.iter() {
                if floating_window_desc.name.is_empty() {
                    Log::warn(
                        "Floating window name is empty, wrong widget will be used as a \
                        floating window. Assign a unique name to the floating window used in a docking \
                        manager!",
                    );
                }

                let floating_window =
                    ui.find_handle(ui.root(), &mut |n| n.name == floating_window_desc.name);
                if floating_window.is_some() {
                    self.floating_windows.borrow_mut().push(floating_window);

                    if floating_window_desc.is_open {
                        ui.send_message(WindowMessage::open(
                            floating_window,
                            MessageDirection::ToWidget,
                            false,
                            false,
                        ));
                    } else {
                        ui.send_message(WindowMessage::close(
                            floating_window,
                            MessageDirection::ToWidget,
                        ));
                    }

                    ui.send_message(WidgetMessage::desired_position(
                        floating_window,
                        MessageDirection::ToWidget,
                        floating_window_desc.position,
                    ));

                    if floating_window_desc.size.x != 0.0 {
                        ui.send_message(WidgetMessage::width(
                            floating_window,
                            MessageDirection::ToWidget,
                            floating_window_desc.size.x,
                        ));
                    }

                    if floating_window_desc.size.y != 0.0 {
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

    fn add_floating_window(&mut self, window: Handle<UiNode>) {
        let mut windows = self.floating_windows.borrow_mut();
        if !windows.contains(&window) {
            windows.push(window);
        }
    }

    fn remove_floating_window(&mut self, window: Handle<UiNode>) {
        let mut windows = self.floating_windows.borrow_mut();
        if let Some(position) = windows.iter().position(|&w| w == window) {
            windows.remove(position);
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
            widget: self.widget_builder.with_preview_messages(true).build(ctx),
            floating_windows: RefCell::new(self.floating_windows),
        };

        ctx.add_node(UiNode::new(docking_manager))
    }
}

#[cfg(test)]
mod test {
    use crate::dock::DockingManagerBuilder;
    use crate::{test::test_widget_deletion, widget::WidgetBuilder};

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| DockingManagerBuilder::new(WidgetBuilder::new()).build(ctx));
    }
}
