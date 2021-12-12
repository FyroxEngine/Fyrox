use crate::{
    world::graph::item::SceneItem, Message, UiMessage, UiNode, UserInterface, VerticalAlignment,
};
use rg3d::gui::button::ButtonMessage;
use rg3d::{
    core::pool::Handle,
    gui::{
        button::ButtonBuilder,
        define_constructor,
        grid::{Column, GridBuilder, Row},
        inspector::{
            editors::{
                PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
                PropertyEditorMessageContext,
            },
            FieldKind, InspectorError, PropertyChanged,
        },
        message::MessageDirection,
        text::{TextBuilder, TextMessage},
        utils::make_simple_tooltip,
        widget::{Widget, WidgetBuilder, WidgetMessage},
        BuildContext, Control,
    },
};
use std::{
    any::{Any, TypeId},
    fmt::{Debug, Formatter},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::{mpsc::Sender, Mutex},
};

pub enum HandlePropertyEditorMessage<T> {
    Value(Handle<T>),
}

impl<T: 'static> HandlePropertyEditorMessage<T> {
    define_constructor!(HandlePropertyEditorMessage:Value => fn value(Handle<T>), layout: false);
}

impl<T> PartialEq for HandlePropertyEditorMessage<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Value(a), Self::Value(b)) => *a == *b,
        }
    }
}

impl<T> Debug for HandlePropertyEditorMessage<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "HandlePropertyEditorMessage")
    }
}

#[derive(Debug)]
pub struct HandlePropertyEditor<T> {
    widget: Widget,
    text: Handle<UiNode>,
    locate: Handle<UiNode>,
    select: Handle<UiNode>,
    value: Handle<T>,
    sender: Sender<Message>,
}

impl<T> Clone for HandlePropertyEditor<T> {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.clone(),
            text: self.text,
            value: self.value,
            sender: self.sender.clone(),
            locate: self.locate,
            select: self.select,
        }
    }
}

impl<T> Deref for HandlePropertyEditor<T> {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<T> DerefMut for HandlePropertyEditor<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<T: 'static> Control for HandlePropertyEditor<T> {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(HandlePropertyEditorMessage::Value(handle)) =
            message.data::<HandlePropertyEditorMessage<T>>()
        {
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::ToWidget
                && self.value != *handle
            {
                self.value = *handle;

                ui.send_message(TextMessage::text(
                    self.text,
                    MessageDirection::ToWidget,
                    format!("{}", *handle),
                ));

                ui.send_message(message.reverse());
            }
        } else if let Some(WidgetMessage::Drop(dropped)) = message.data() {
            if message.destination() == self.handle() {
                if let Some(item) = ui.node(*dropped).cast::<SceneItem<T>>() {
                    ui.send_message(HandlePropertyEditorMessage::value(
                        self.handle(),
                        MessageDirection::ToWidget,
                        item.entity_handle,
                    ))
                }
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination == self.locate {
                self.sender
                    .send(Message::LocateObject {
                        type_id: TypeId::of::<T>(),
                        handle: self.value.into(),
                    })
                    .unwrap();
            } else if message.destination == self.select {
                self.sender
                    .send(Message::SelectObject {
                        type_id: TypeId::of::<T>(),
                        handle: self.value.into(),
                    })
                    .unwrap();
            }
        }
    }
}

struct HandlePropertyEditorBuilder<T> {
    widget_builder: WidgetBuilder,
    value: Handle<T>,
    sender: Sender<Message>,
}

impl<T: 'static> HandlePropertyEditorBuilder<T> {
    pub fn new(widget_builder: WidgetBuilder, sender: Sender<Message>) -> Self {
        Self {
            widget_builder,
            sender,
            value: Default::default(),
        }
    }

    pub fn with_value(mut self, value: Handle<T>) -> Self {
        self.value = value;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let text;
        let locate;
        let select;
        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child({
                    text = TextBuilder::new(WidgetBuilder::new().on_column(0))
                        .with_vertical_text_alignment(VerticalAlignment::Center)
                        .with_text(if self.value.is_none() {
                            "Unassigned".to_owned()
                        } else {
                            format!("{}", self.value)
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
                }),
        )
        .add_row(Row::stretch())
        .add_column(Column::stretch())
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
        };

        ctx.add_node(UiNode::new(editor))
    }
}

pub struct HandlePropertyEditorDefinition<T> {
    phantom: PhantomData<T>,
    sender: Mutex<Sender<Message>>,
}

impl<T> HandlePropertyEditorDefinition<T> {
    pub fn new(sender: Sender<Message>) -> Self {
        Self {
            phantom: PhantomData,
            sender: Mutex::new(sender),
        }
    }
}

impl<T> Debug for HandlePropertyEditorDefinition<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "HandlePropertyEditorDefinition")
    }
}

impl<T: 'static> PropertyEditorDefinition for HandlePropertyEditorDefinition<T> {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Handle<T>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<Handle<T>>()?;

        Ok(PropertyEditorInstance::Simple {
            editor: HandlePropertyEditorBuilder::new(
                WidgetBuilder::new(),
                self.sender.lock().unwrap().clone(),
            )
            .with_value(*value)
            .build(ctx.build_context),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<Handle<T>>()?;

        Ok(Some(HandlePropertyEditorMessage::value(
            ctx.instance,
            MessageDirection::ToWidget,
            *value,
        )))
    }

    fn translate_message(
        &self,
        name: &str,
        owner_type_id: TypeId,
        message: &UiMessage,
    ) -> Option<PropertyChanged> {
        if message.direction() == MessageDirection::FromWidget {
            if let Some(HandlePropertyEditorMessage::Value(value)) =
                message.data::<HandlePropertyEditorMessage<T>>()
            {
                return Some(PropertyChanged {
                    owner_type_id,
                    name: name.to_string(),
                    value: FieldKind::object(*value),
                });
            }
        }
        None
    }
}
