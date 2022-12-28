use crate::{
    button::{ButtonBuilder, ButtonMessage},
    core::{pool::Handle, uuid::Uuid},
    define_constructor, define_widget_deref,
    grid::{Column, GridBuilder, Row},
    message::{MessageDirection, UiMessage},
    text::{TextBuilder, TextMessage},
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, Thickness, UiNode, UserInterface,
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UuidEditorMessage {
    Value(Uuid),
}

impl UuidEditorMessage {
    define_constructor!(UuidEditorMessage:Value => fn value(Uuid), layout: false);
}

#[derive(Clone)]
pub struct UuidEditor {
    widget: Widget,
    value: Uuid,
    text: Handle<UiNode>,
    generate: Handle<UiNode>,
}

define_widget_deref!(UuidEditor);

impl Control for UuidEditor {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

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

pub struct UuidEditorBuilder {
    widget_builder: WidgetBuilder,
    value: Uuid,
}

impl UuidEditorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            value: Default::default(),
        }
    }

    pub fn with_value(mut self, value: Uuid) -> Self {
        self.value = value;
        self
    }

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
                            .with_margin(Thickness::uniform(1.0)),
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
