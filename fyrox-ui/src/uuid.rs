//! UUID editor is used to show an arbitrary UUID and give an ability to generate a new value. See [`UuidEditor`] docs for
//! more info and usage examples.

#![warn(missing_docs)]

use crate::{
    button::{ButtonBuilder, ButtonMessage},
    core::{pool::Handle, reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*},
    define_constructor, define_widget_deref,
    grid::{Column, GridBuilder, Row},
    message::{MessageDirection, UiMessage},
    text::{TextBuilder, TextMessage},
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, Thickness, UiNode, UserInterface, VerticalAlignment,
};
use fyrox_core::uuid_provider;
use std::ops::{Deref, DerefMut};

/// A set of messages that is used to fetch or modify values of [`UuidEditor`] widgets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UuidEditorMessage {
    /// Fetches or modifies a value of a [`UuidEditor`] widget.
    Value(Uuid),
}

impl UuidEditorMessage {
    define_constructor!(
        /// Creates [`UuidEditorMessage::Value`] message.
        UuidEditorMessage:Value => fn value(Uuid), layout: false
    );
}

/// UUID editor is used to show an arbitrary UUID and give an ability to generate a new value. It is widely used in
/// [`crate::inspector::Inspector`] to show and edit UUIDs.
///
/// ## Example
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::{pool::Handle, uuid::Uuid},
/// #     uuid::UuidEditorBuilder,
/// #     widget::WidgetBuilder,
/// #     BuildContext, UiNode,
/// # };
/// fn create_uuid_editor(ctx: &mut BuildContext) -> Handle<UiNode> {
///     UuidEditorBuilder::new(WidgetBuilder::new())
///         .with_value(Uuid::new_v4())
///         .build(ctx)
/// }
/// ```
#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider)]
pub struct UuidEditor {
    widget: Widget,
    value: Uuid,
    text: Handle<UiNode>,
    generate: Handle<UiNode>,
}

define_widget_deref!(UuidEditor);

uuid_provider!(UuidEditor = "667f7f48-2448-42da-91dd-cd743ca7117e");

impl Control for UuidEditor {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle && message.direction() == MessageDirection::ToWidget
        {
            if let Some(UuidEditorMessage::Value(value)) = message.data() {
                if self.value != *value {
                    self.value = *value;
                    ui.send_message(message.reverse());

                    ui.send_message(TextMessage::text(
                        self.text,
                        MessageDirection::ToWidget,
                        value.to_string(),
                    ));
                }
            }
        } else if message.destination() == self.generate {
            if let Some(ButtonMessage::Click) = message.data() {
                ui.send_message(UuidEditorMessage::value(
                    self.handle,
                    MessageDirection::ToWidget,
                    Uuid::new_v4(),
                ));
            }
        }
    }
}

/// Creates [`UuidEditor`] widgets and add them to the user interface.
pub struct UuidEditorBuilder {
    widget_builder: WidgetBuilder,
    value: Uuid,
}

impl UuidEditorBuilder {
    /// Creates new builder instance.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            value: Default::default(),
        }
    }

    /// Sets a desired value of the [`UuidEditor`].
    pub fn with_value(mut self, value: Uuid) -> Self {
        self.value = value;
        self
    }

    /// Finishes widget building.
    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let text;
        let generate;
        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child({
                    text = TextBuilder::new(
                        WidgetBuilder::new()
                            .on_column(0)
                            .on_row(0)
                            .with_margin(Thickness::uniform(1.0))
                            .with_vertical_alignment(VerticalAlignment::Center),
                    )
                    .with_text(self.value.to_string())
                    .build(ctx);
                    text
                })
                .with_child({
                    generate = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .on_column(1)
                            .on_row(0)
                            .with_width(24.0)
                            .with_margin(Thickness::uniform(1.0)),
                    )
                    .with_text("^/v")
                    .build(ctx);
                    generate
                }),
        )
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .build(ctx);

        let uuid_editor = UuidEditor {
            widget: self.widget_builder.with_child(grid).build(),
            value: self.value,
            text,
            generate,
        };

        ctx.add_node(UiNode::new(uuid_editor))
    }
}
