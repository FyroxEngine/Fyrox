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
    dock::config::{DockingManagerLayoutDescriptor, FloatingWindowDescriptor, TileDescriptor},
    message::UiMessage,
    widget::{Widget, WidgetBuilder, WidgetMessage},
    window::WindowMessage,
    BuildContext, Control, UiNode, UserInterface,
};

use fyrox_graph::constructor::{ConstructorProvider, GraphNodeConstructor};
use fyrox_graph::SceneGraph;
use std::cell::RefCell;

pub mod config;
mod tile;

use crate::message::MessageData;
use crate::window::{Window, WindowAlignment};
pub use tile::*;

/// Supported docking manager-specific messages.
#[derive(Debug, Clone, PartialEq)]
pub enum DockingManagerMessage {
    Layout(DockingManagerLayoutDescriptor),
    AddFloatingWindow(Handle<Window>),
    RemoveFloatingWindow(Handle<Window>),
}
impl MessageData for DockingManagerMessage {}

/// Docking manager is a special container widget, that holds a bunch of children widgets in-place
/// using [`Tile`]s and a bunch of floating windows. Any window can be undocked and become a floating
/// window and vice versa. Docking manager is typically used to "pack" multiple windows into a
/// rectangular. The most notable use case is IDEs where you can drag,
/// dock, undock, stack windows.
///
/// ## Tiles
///
/// The main element of the docking manager is the [`Tile`] widget, which can be in two major states:
///
/// 1) It can hold a window
/// 2) It can be split into two more sub-tiles (either vertically or horizontally), which can in
/// their turn either contain some other window or a sub-tile.
///
/// This structure essentially forms a tree of pretty much unlimited depth. This approach basically
/// allows you to "pack" multiple windows in a rectangular area with no free space between the tiles.
/// Split tiles have a special parameter called splitter, which is simply a fraction that shows how
/// much space each half takes. In the case of a horizontal tile, if the splitter is 0.25, then the left
/// tile will take 25% of the width of the tile and the right tile will take the rest 75% of the
/// width.
///
/// ## Floating Windows
///
/// The docking manager can control an unlimited number of floating windows, floating windows can be
/// docked and vice versa. When a window is undocked, it is automatically placed into a list of floating
/// windows. Only the windows from this list can be docked.
///
/// ## Example
///
/// The following example shows how to create a docking manager with one root tile split vertically
/// into two smaller tiles where each tile holds a separate window.
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle,
/// #     dock::{DockingManagerBuilder, TileBuilder, TileContent},
/// #     widget::WidgetBuilder,
/// #     window::{WindowBuilder, WindowTitle},
/// #     BuildContext, UiNode,
/// # };
/// #
/// fn create_docking_manager(ctx: &mut BuildContext) -> Handle<UiNode> {
///     let top_window = WindowBuilder::new(WidgetBuilder::new())
///         .with_title(WindowTitle::text("Top Window"))
///         .build(ctx);
///
///     let bottom_window = WindowBuilder::new(WidgetBuilder::new())
///         .with_title(WindowTitle::text("Bottom Window"))
///         .build(ctx);
///
///     let root_tile = TileBuilder::new(WidgetBuilder::new())
///         .with_content(TileContent::VerticalTiles {
///             splitter: 0.5,
///             tiles: [
///                 TileBuilder::new(WidgetBuilder::new())
///                     // Note that you have to put the window into a separate tile, otherwise
///                     // you'll get unexpected results.
///                     .with_content(TileContent::Window(top_window))
///                     .build(ctx),
///                 TileBuilder::new(WidgetBuilder::new())
///                     .with_content(TileContent::Window(bottom_window))
///                     .build(ctx),
///             ],
///         })
///         .build(ctx);
///
///     DockingManagerBuilder::new(
///         WidgetBuilder::new()
///             .with_child(root_tile)
///             .with_width(500.0)
///             .with_height(500.0),
///     )
///     .build(ctx)
/// }
/// ```
///
/// ## Layout
///
/// The current docking manager layout can be saved and restored later if needed. This is a very useful
/// option for customizable user interfaces, where users can adjust the interface as they like,
/// save it and then load on the next session. Use the following code to save the layout:
///
/// ```rust
/// # use fyrox_ui::{
/// #     dock::config::DockingManagerLayoutDescriptor, dock::DockingManager, UiNode, UserInterface,
/// # };
/// # use fyrox_core::pool::Handle;
/// # use fyrox_graph::SceneGraph;
/// #
/// fn save_layout(
///     ui: &UserInterface,
///     docking_manager_handle: Handle<UiNode>,
/// ) -> Option<DockingManagerLayoutDescriptor> {
///     ui.try_get_of_type::<DockingManager>(docking_manager_handle)
///         .as_ref().ok()
///         .map(|docking_manager| docking_manager.layout(ui))
/// }
/// ```
///
/// The layout can be restored by sending a [`DockingManagerMessage::Layout`] message to the docking
/// manager. Use [`DockingManagerMessage::layout`] builder method to make one.
///
/// To be able to restore the layout to its defaults, just create a desired layout from code,
/// save the layout and use the returned layout descriptor when you need to restore the layout
/// to its defaults.
#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct DockingManager {
    pub widget: Widget,
    pub floating_windows: RefCell<Vec<Handle<Window>>>,
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

        if let Some(msg) = message.data_for(self.handle) {
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

    fn preview_message(&self, _ui: &UserInterface, message: &mut UiMessage) {
        if let Some(WidgetMessage::LinkWith(_)) = message.data::<WidgetMessage>() {
            let pos = self
                .floating_windows
                .borrow()
                .iter()
                .position(|&i| message.destination() == i);
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
                    ui.try_get(*h).ok().map(|w| FloatingWindowDescriptor {
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
                if let Ok(tile) = ui.try_get_of_type::<Tile>(tile_handle) {
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
            ui.send(root_tile_handle, WidgetMessage::Remove);

            // Re-create the tiles according to the layout and attach it to the docking manager.
            if let Some(root_tile_descriptor) = layout_descriptor.root_tile_descriptor.as_ref() {
                let root_tile = root_tile_descriptor.create_tile(ui, &windows);
                ui.send(root_tile, WidgetMessage::LinkWith(self.handle));
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

                let floating_window = ui
                    .find_handle(ui.root(), &mut |n| {
                        n.has_component::<Window>() && n.name == floating_window_desc.name
                    })
                    .to_variant();
                if floating_window.is_some() {
                    self.floating_windows.borrow_mut().push(floating_window);

                    if floating_window_desc.is_open {
                        ui.send(
                            floating_window,
                            WindowMessage::Open {
                                alignment: WindowAlignment::None,
                                modal: false,
                                focus_content: false,
                            },
                        );
                    } else {
                        ui.send(floating_window, WindowMessage::Close);
                    }

                    ui.send(
                        floating_window,
                        WidgetMessage::DesiredPosition(floating_window_desc.position),
                    );

                    if floating_window_desc.size.x != 0.0 {
                        ui.send(
                            floating_window,
                            WidgetMessage::Width(floating_window_desc.size.x),
                        );
                    }

                    if floating_window_desc.size.y != 0.0 {
                        ui.send(
                            floating_window,
                            WidgetMessage::Height(floating_window_desc.size.y),
                        );
                    }
                }
            }
        }
    }

    fn add_floating_window(&mut self, window: Handle<Window>) {
        let mut windows = self.floating_windows.borrow_mut();
        if !windows.contains(&window) {
            windows.push(window);
        }
    }

    fn remove_floating_window(&mut self, window: Handle<Window>) {
        let mut windows = self.floating_windows.borrow_mut();
        if let Some(position) = windows.iter().position(|&w| w == window) {
            windows.remove(position);
        }
    }
}

pub struct DockingManagerBuilder {
    widget_builder: WidgetBuilder,
    floating_windows: Vec<Handle<Window>>,
}

impl DockingManagerBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            floating_windows: Default::default(),
        }
    }

    pub fn with_floating_windows(mut self, windows: Vec<Handle<Window>>) -> Self {
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
