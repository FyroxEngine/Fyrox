use crate::Message;
use rg3d::{
    asset::core::pool::Handle,
    gui::{
        button::ButtonBuilder,
        grid::{Column, GridBuilder, Row},
        inspector::{
            editors::{
                Layout, PropertyEditorBuildContext, PropertyEditorDefinition,
                PropertyEditorInstance, PropertyEditorMessageContext,
            },
            InspectorError,
        },
        message::{ButtonMessage, PropertyChanged, UiMessage, UiMessageData},
        text::TextBuilder,
        widget::{Widget, WidgetBuilder},
        BuildContext, Control, Thickness, UiNode, UserInterface, VerticalAlignment,
    },
    material::Material,
};
use std::{
    any::TypeId,
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
    sync::{mpsc::Sender, Arc, Mutex},
};

#[derive(Clone)]
pub struct MaterialFieldEditor {
    widget: Widget,
    sender: Sender<Message>,
    edit: Handle<UiNode>,
    material: Arc<Mutex<Material>>,
}

impl Debug for MaterialFieldEditor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "MaterialFieldEditor")
    }
}

impl Deref for MaterialFieldEditor {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl DerefMut for MaterialFieldEditor {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl Control for MaterialFieldEditor {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.edit {
            if let UiMessageData::Button(ButtonMessage::Click) = message.data() {
                self.sender
                    .send(Message::OpenMaterialEditor(self.material.clone()))
                    .unwrap();
            }
        }
    }
}

pub struct MaterialFieldEditorBuilder {
    widget_builder: WidgetBuilder,
}

impl MaterialFieldEditorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self { widget_builder }
    }

    pub fn build(
        self,
        ctx: &mut BuildContext,
        sender: Sender<Message>,
        material: Arc<Mutex<Material>>,
    ) -> Handle<UiNode> {
        let name = material
            .lock()
            .unwrap()
            .shader()
            .data_ref()
            .definition
            .name
            .clone();

        let edit;
        let editor = MaterialFieldEditor {
            widget: self
                .widget_builder
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .with_height(26.0)
                            .with_child(
                                TextBuilder::new(
                                    WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                                )
                                .with_text(name)
                                .with_vertical_text_alignment(VerticalAlignment::Center)
                                .build(ctx),
                            )
                            .with_child({
                                edit = ButtonBuilder::new(
                                    WidgetBuilder::new().with_width(32.0).on_column(1),
                                )
                                .with_text("...")
                                .build(ctx);
                                edit
                            }),
                    )
                    .add_row(Row::stretch())
                    .add_column(Column::stretch())
                    .add_column(Column::auto())
                    .build(ctx),
                )
                .build(),
            edit,
            sender,
            material,
        };

        ctx.add_node(UiNode::new(editor))
    }
}

#[derive(Debug)]
pub struct MaterialPropertyEditorDefinition {
    pub sender: Mutex<Sender<Message>>,
}

impl PropertyEditorDefinition for MaterialPropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Arc<Mutex<Material>>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<Arc<Mutex<Material>>>()?;
        Ok(PropertyEditorInstance {
            title: Default::default(),
            editor: MaterialFieldEditorBuilder::new(WidgetBuilder::new()).build(
                ctx.build_context,
                self.sender.lock().unwrap().clone(),
                value.clone(),
            ),
        })
    }

    fn create_message(
        &self,
        _ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        Ok(None)
    }

    fn translate_message(
        &self,
        _name: &str,
        _owner_type_id: TypeId,
        _message: &UiMessage,
    ) -> Option<PropertyChanged> {
        None
    }

    fn layout(&self) -> Layout {
        Layout::Horizontal
    }
}
