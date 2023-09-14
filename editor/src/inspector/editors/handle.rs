use crate::message::MessageSender;
use crate::{
    world::graph::item::SceneItem, Message, UiMessage, UiNode, UserInterface, VerticalAlignment,
};
use fyrox::gui::draw::{CommandTexture, Draw, DrawingContext};
use fyrox::{
    core::{color::Color, pool::Handle},
    gui::{
        brush::Brush,
        button::{ButtonBuilder, ButtonMessage},
        define_constructor,
        grid::{Column, GridBuilder, Row},
        inspector::{
            editors::{
                PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
                PropertyEditorMessageContext, PropertyEditorTranslationContext,
            },
            FieldKind, InspectorError, PropertyChanged,
        },
        message::MessageDirection,
        text::{TextBuilder, TextMessage},
        utils::make_simple_tooltip,
        widget::{Widget, WidgetBuilder, WidgetMessage},
        BuildContext, Control,
    },
    scene::node::Node,
};
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::Mutex,
};

#[derive(Debug, PartialEq, Eq)]
pub enum HandlePropertyEditorMessage {
    Value(Handle<Node>),
    Name(Option<String>),
}

impl HandlePropertyEditorMessage {
    define_constructor!(Self:Value => fn value(Handle<Node>), layout: false);
    define_constructor!(Self:Name => fn name(Option<String>), layout: false);
}

#[derive(Debug)]
pub struct HandlePropertyEditor {
    widget: Widget,
    text: Handle<UiNode>,
    locate: Handle<UiNode>,
    select: Handle<UiNode>,
    make_unassigned: Handle<UiNode>,
    value: Handle<Node>,
    sender: MessageSender,
}

impl Clone for HandlePropertyEditor {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.clone(),
            text: self.text,
            value: self.value,
            sender: self.sender.clone(),
            locate: self.locate,
            select: self.select,
            make_unassigned: self.make_unassigned,
        }
    }
}

impl Deref for HandlePropertyEditor {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl DerefMut for HandlePropertyEditor {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl Control for HandlePropertyEditor {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        // Emit transparent geometry for the field to be able to catch mouse events without precise pointing at the
        // node name letters.
        drawing_context.push_rect_filled(&self.bounding_rect(), None);
        drawing_context.commit(
            self.clip_bounds(),
            Brush::Solid(Color::TRANSPARENT),
            CommandTexture::None,
            None,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<HandlePropertyEditorMessage>() {
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::ToWidget
            {
                match msg {
                    HandlePropertyEditorMessage::Value(handle) => {
                        if self.value != *handle {
                            self.value = *handle;
                            ui.send_message(message.reverse());
                        }

                        // Sync name in any case, because it may be changed.
                        request_name_sync(&self.sender, self.handle, self.value);
                    }
                    HandlePropertyEditorMessage::Name(value) => {
                        // Handle messages from the editor, it will respond to requests and provide
                        // node names in efficient way.
                        let value = if let Some(value) = value {
                            Some(value.as_str())
                        } else if self.value.is_none() {
                            Some("Unassigned")
                        } else {
                            None
                        };

                        if let Some(value) = value {
                            ui.send_message(TextMessage::text(
                                self.text,
                                MessageDirection::ToWidget,
                                format!("{} ({})", value, self.value),
                            ));

                            ui.send_message(WidgetMessage::foreground(
                                self.text,
                                MessageDirection::ToWidget,
                                Brush::Solid(fyrox::gui::COLOR_FOREGROUND),
                            ));
                        } else {
                            ui.send_message(TextMessage::text(
                                self.text,
                                MessageDirection::ToWidget,
                                format!("<Invalid handle!> ({})", self.value),
                            ));

                            ui.send_message(WidgetMessage::foreground(
                                self.text,
                                MessageDirection::ToWidget,
                                Brush::Solid(Color::RED),
                            ));
                        };
                    }
                }
            }
        } else if let Some(WidgetMessage::Drop(dropped)) = message.data() {
            if message.destination() == self.handle() {
                if let Some(item) = ui.node(*dropped).cast::<SceneItem>() {
                    ui.send_message(HandlePropertyEditorMessage::value(
                        self.handle(),
                        MessageDirection::ToWidget,
                        item.entity_handle,
                    ))
                }
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination == self.locate {
                self.sender.send(Message::LocateObject {
                    type_id: TypeId::of::<Node>(),
                    handle: self.value.into(),
                });
            } else if message.destination == self.select {
                self.sender.send(Message::SelectObject {
                    type_id: TypeId::of::<Node>(),
                    handle: self.value.into(),
                });
            } else if message.destination == self.make_unassigned {
                ui.send_message(HandlePropertyEditorMessage::value(
                    self.handle,
                    MessageDirection::ToWidget,
                    Handle::NONE,
                ));
            }
        }
    }
}

struct HandlePropertyEditorBuilder {
    widget_builder: WidgetBuilder,
    value: Handle<Node>,
    sender: MessageSender,
}

impl HandlePropertyEditorBuilder {
    pub fn new(widget_builder: WidgetBuilder, sender: MessageSender) -> Self {
        Self {
            widget_builder,
            sender,
            value: Default::default(),
        }
    }

    pub fn with_value(mut self, value: Handle<Node>) -> Self {
        self.value = value;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let text;
        let locate;
        let select;
        let make_unassigned;
        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child({
                    text = TextBuilder::new(WidgetBuilder::new().on_column(0))
                        .with_vertical_text_alignment(VerticalAlignment::Center)
                        .with_text(if self.value.is_none() {
                            "Unassigned".to_owned()
                        } else {
                            "Err: Desync!".to_owned()
                        })
                        .build(ctx);
                    text
                })
                .with_child({
                    locate = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .with_tooltip(make_simple_tooltip(ctx, "Locate Object"))
                            .with_width(20.0)
                            .with_height(20.0)
                            .on_column(1),
                    )
                    .with_text(">>")
                    .build(ctx);
                    locate
                })
                .with_child({
                    select = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .with_tooltip(make_simple_tooltip(ctx, "Select Object"))
                            .with_width(20.0)
                            .with_height(20.0)
                            .on_column(2),
                    )
                    .with_text("*")
                    .build(ctx);
                    select
                })
                .with_child({
                    make_unassigned = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .with_tooltip(make_simple_tooltip(ctx, "Make Unassigned"))
                            .with_width(20.0)
                            .with_height(20.0)
                            .on_column(3),
                    )
                    .with_text("X")
                    .build(ctx);
                    make_unassigned
                }),
        )
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .add_column(Column::auto())
        .add_column(Column::auto())
        .build(ctx);

        let editor = HandlePropertyEditor {
            widget: self
                .widget_builder
                .with_tooltip(make_simple_tooltip(
                    ctx,
                    "Use <Alt+Mouse Drag> in World Viewer to assign the value here.",
                ))
                .with_allow_drop(true)
                .with_child(grid)
                .build(),
            text,
            value: self.value,
            sender: self.sender,
            locate,
            select,
            make_unassigned,
        };

        ctx.add_node(UiNode::new(editor))
    }
}

#[derive(Debug)]
pub struct NodeHandlePropertyEditorDefinition {
    sender: Mutex<MessageSender>,
}

impl NodeHandlePropertyEditorDefinition {
    pub fn new(sender: MessageSender) -> Self {
        Self {
            sender: Mutex::new(sender),
        }
    }
}

impl PropertyEditorDefinition for NodeHandlePropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Handle<Node>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<Handle<Node>>()?;

        let sender = self.sender.lock().unwrap().clone();

        let editor = HandlePropertyEditorBuilder::new(WidgetBuilder::new(), sender.clone())
            .with_value(*value)
            .build(ctx.build_context);

        request_name_sync(&sender, editor, *value);

        Ok(PropertyEditorInstance::Simple { editor })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<Handle<Node>>()?;

        Ok(Some(HandlePropertyEditorMessage::value(
            ctx.instance,
            MessageDirection::ToWidget,
            *value,
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(HandlePropertyEditorMessage::Value(value)) =
                ctx.message.data::<HandlePropertyEditorMessage>()
            {
                return Some(PropertyChanged {
                    owner_type_id: ctx.owner_type_id,
                    name: ctx.name.to_string(),
                    value: FieldKind::object(*value),
                });
            }
        }
        None
    }
}

fn request_name_sync(sender: &MessageSender, editor: Handle<UiNode>, handle: Handle<Node>) {
    // It is not possible to **effectively** provide information about node names here,
    // instead we ask the editor to provide such information in a deferred manner - by
    // sending a message.
    sender.send(Message::SyncNodeHandleName {
        view: editor,
        handle,
    });
}
