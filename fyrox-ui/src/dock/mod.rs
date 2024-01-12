//! Docking manager allows you to dock windows and hold them in-place.
//!
//! # Notes
//!
//! Docking manager can hold any types of UI elements, but dragging works only
//! for windows.

use crate::core::{reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*};
use fyrox_core::uuid_provider;
use std::{
    cell::RefCell,
    ops::{Deref, DerefMut},
};

pub mod config;
mod tile;

use crate::{
    core::pool::Handle,
    define_constructor,
    dock::config::{DockingManagerLayoutDescriptor, FloatingWindowDescriptor, TileDescriptor},
    message::{MessageDirection, UiMessage},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, NodeHandleMapping, UiNode, UserInterface,
};

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

#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider)]
pub struct DockingManager {
    pub widget: Widget,
    pub floating_windows: RefCell<Vec<Handle<UiNode>>>,
}

crate::define_widget_deref!(DockingManager);

uuid_provider!(DockingManager = "b04299f7-3f6b-45f1-89a6-0dce4ad929e1");

impl Control for DockingManager {
    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        node_map.resolve_slice(&mut self.floating_windows.borrow_mut());
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle && message.direction() == MessageDirection::ToWidget
        {
            if let Some(DockingManagerMessage::Layout(layout_descriptor)) = message.data() {
                if let Some(root_tile_handle) = self.children.first().cloned() {
                    let mut windows = Vec::new();
                    let mut stack = vec![root_tile_handle];
                    while let Some(tile_handle) = stack.pop() {
                        if let Some(tile) = ui
                            .try_get_node(tile_handle)
                            .and_then(|n| n.query_component::<Tile>())
                        {
                            match tile.content {
                                TileContent::Window(window) => {
                                    if ui.try_get_node(window).is_some() {
                                        windows.push(window);
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
                        let floating_window = ui.find_by_criteria_down(ui.root(), &|n| {
                            n.name == floating_window_desc.name
                        });
                        if floating_window.is_some() {
                            self.floating_windows.borrow_mut().push(floating_window);

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
                        name: w.name.clone(),
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
