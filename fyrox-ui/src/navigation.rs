//! A widget, that handles keyboard navigation on its descendant widgets using Tab key. See [`NavigationLayer`]
//! docs for more info and usage examples.

#![warn(missing_docs)]

use crate::{
    core::{
        pool::Handle, reflect::prelude::*, type_traits::prelude::*, variable::InheritableVariable,
        visitor::prelude::*,
    },
    message::{KeyCode, MessageDirection, UiMessage},
    scroll_viewer::{ScrollViewer, ScrollViewerMessage},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, UiNode, UserInterface,
};
use fyrox_graph::{BaseSceneGraph, SceneGraph};
use std::ops::{Deref, DerefMut};

/// A widget, that handles keyboard navigation on its descendant widgets using Tab key. It should
/// be used as a root widget for an hierarchy, that should support Tab key navigation:
///
/// ```rust
/// use fyrox_ui::{
///     button::ButtonBuilder, navigation::NavigationLayerBuilder, stack_panel::StackPanelBuilder,
///     text::TextBuilder, widget::WidgetBuilder, BuildContext,
/// };
///
/// fn create_navigation_layer(ctx: &mut BuildContext) {
///     NavigationLayerBuilder::new(
///         WidgetBuilder::new().with_child(
///             StackPanelBuilder::new(
///                 WidgetBuilder::new()
///                     .with_child(
///                         // This widget won't participate in Tab key navigation.
///                         TextBuilder::new(WidgetBuilder::new())
///                             .with_text("Do something?")
///                             .build(ctx),
///                     )
///                     // The keyboard focus for the following two buttons can be cycled using Tab/Shift+Tab.
///                     .with_child(
///                         ButtonBuilder::new(WidgetBuilder::new().with_tab_index(Some(0)))
///                             .with_text("OK")
///                             .build(ctx),
///                     )
///                     .with_child(
///                         ButtonBuilder::new(WidgetBuilder::new().with_tab_index(Some(1)))
///                             .with_text("Cancel")
///                             .build(ctx),
///                     ),
///             )
///             .build(ctx),
///         ),
///     )
///     .build(ctx);
/// }
/// ```
///
/// This example shows how to create a simple confirmation dialog, that allows a user to use Tab key
/// to cycle from one button to another. A focused button then can be "clicked" using Enter key.
#[derive(Default, Clone, Visit, Reflect, Debug, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "135d347b-5019-4743-906c-6df5c295a3be")]
pub struct NavigationLayer {
    /// Base widget of the navigation layer.
    pub widget: Widget,
    /// A flag, that defines whether the navigation layer should search for a [`crate::scroll_viewer::ScrollViewer`]
    /// parent widget and send [`crate::scroll_viewer::ScrollViewerMessage::BringIntoView`] message
    /// to a newly focused widget.
    pub bring_into_view: InheritableVariable<bool>,
}

crate::define_widget_deref!(NavigationLayer);

#[derive(Debug)]
struct OrderedHandle {
    tab_index: usize,
    handle: Handle<UiNode>,
}

impl Control for NavigationLayer {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(WidgetMessage::KeyDown(KeyCode::Tab)) = message.data() {
            // Collect all descendant widgets, that supports Tab navigation.
            let mut tab_list = Vec::new();
            for &child in self.children() {
                for descendant in ui.traverse_handle_iter(child) {
                    let descendant_ref = ui.node(descendant);

                    if !*descendant_ref.tab_stop && descendant_ref.is_globally_visible() {
                        if let Some(tab_index) = *descendant_ref.tab_index {
                            tab_list.push(OrderedHandle {
                                tab_index,
                                handle: descendant,
                            });
                        }
                    }
                }
            }

            if !tab_list.is_empty() {
                tab_list.sort_by_key(|entry| entry.tab_index);

                let focused_index = tab_list
                    .iter()
                    .position(|entry| entry.handle == ui.keyboard_focus_node)
                    .unwrap_or_default();

                let next_focused_node_index = if ui.keyboard_modifiers.shift {
                    let count = tab_list.len() as isize;
                    let mut prev = (focused_index as isize).saturating_sub(1);
                    if prev < 0 {
                        prev += count;
                    }
                    (prev % count) as usize
                } else {
                    focused_index.saturating_add(1) % tab_list.len()
                };

                if let Some(entry) = tab_list.get(next_focused_node_index) {
                    ui.send_message(WidgetMessage::focus(
                        entry.handle,
                        MessageDirection::ToWidget,
                    ));

                    if *self.bring_into_view {
                        // Find a parent scroll viewer.
                        if let Some((scroll_viewer, _)) =
                            ui.find_component_up::<ScrollViewer>(entry.handle)
                        {
                            ui.send_message(ScrollViewerMessage::bring_into_view(
                                scroll_viewer,
                                MessageDirection::ToWidget,
                                entry.handle,
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Navigation layer builder creates new [`NavigationLayer`] widget instances and adds them to the user interface.
pub struct NavigationLayerBuilder {
    widget_builder: WidgetBuilder,
    bring_into_view: bool,
}

impl NavigationLayerBuilder {
    /// Creates new builder instance.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            bring_into_view: true,
        }
    }

    /// Finishes navigation layer widget building and adds the instance to the user interface and
    /// returns its handle.
    pub fn build(self, ui: &mut BuildContext) -> Handle<UiNode> {
        let navigation_layer = NavigationLayer {
            widget: self.widget_builder.build(),
            bring_into_view: self.bring_into_view.into(),
        };
        ui.add_node(UiNode::new(navigation_layer))
    }
}
