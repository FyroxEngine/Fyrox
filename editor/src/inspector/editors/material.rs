use crate::{Message, MessageDirection};
use fyrox::{
    asset::core::pool::Handle,
    core::parking_lot::Mutex,
    gui::{
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
        message::UiMessage,
        text::{TextBuilder, TextMessage},
        utils::make_simple_tooltip,
        widget::{Widget, WidgetBuilder},
        BuildContext, Control, Thickness, UiNode, UserInterface, VerticalAlignment,
    },
    material::SharedMaterial,
};
use std::{
    any::{Any, TypeId},
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
    sync::mpsc::Sender,
};

#[derive(Debug, Clone, PartialEq)]
pub enum MaterialFieldMessage {
    Material(SharedMaterial),
}

impl MaterialFieldMessage {
    define_constructor!(MaterialFieldMessage:Material => fn material(SharedMaterial), layout: false);
}

#[derive(Clone)]
pub struct MaterialFieldEditor {
    widget: Widget,
    sender: Sender<Message>,
    text: Handle<UiNode>,
    edit: Handle<UiNode>,
    material: SharedMaterial,
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
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == self.edit {
                self.sender
                    .send(Message::OpenMaterialEditor(self.material.clone()))
                    .unwrap();
            } else if message.destination() == self.make_unique {
                let deep_copy = self.material.lock().clone();

                ui.send_message(MaterialFieldMessage::material(
                    self.handle,
                    MessageDirection::ToWidget,
                    Arc::new(Mutex::new(deep_copy)),
                ));
            }
        } else if let Some(MaterialFieldMessage::Material(material)) = message.data() {
            if message.destination() == self.handle {
                if !Arc::ptr_eq(&self.material, material) {
                    self.material = material.clone();

                    ui.send_message(TextMessage::text(
                        self.text,
                        MessageDirection::ToWidget,
                        make_name(&self.material),
                    ));
                }
            }
        }
    }
}

pub struct MaterialFieldEditorBuilder {
    widget_builder: WidgetBuilder,
}

fn make_name(material: &SharedMaterial) -> String {
    let name = material.lock().shader().data_ref().definition.name.clone();
    format!("{} - {} uses", name, material.use_count())
}

impl MaterialFieldEditorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self { widget_builder }
    }

    pub fn build(
        self,
        ctx: &mut BuildContext,
        sender: Sender<Message>,
        material: SharedMaterial,
    ) -> Handle<UiNode> {
        let edit;
        let text;
        let make_unique;
        let make_unique_tooltip = "Creates a deep copy of the material, making a separate version of the material. \
        Useful when you need to change some properties in the material, but only on some nodes that uses the material.";

        let editor = MaterialFieldEditor {
            widget: self
                .widget_builder
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .with_child({
                                text = TextBuilder::new(
                                    WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                                )
                                .with_text(make_name(&material))
                                .with_vertical_text_alignment(VerticalAlignment::Center)
                                .build(ctx);
                                text
                            })
                            .with_child(
                                GridBuilder::new(
                                    WidgetBuilder::new()
                                        .on_row(1)
                                        .with_child({
                                            edit = ButtonBuilder::new(
                                                WidgetBuilder::new()
                                                    .with_width(40.0)
                                                    .with_margin(Thickness::uniform(1.0)),
                                            )
                                            .with_text("Edit...")
                                            .build(ctx);
                                            edit
                                        })
                                        .with_child({
                                            make_unique = ButtonBuilder::new(
                                                WidgetBuilder::new()
                                                    .with_margin(Thickness::uniform(1.0))
                                                    .on_column(1)
                                                    .with_tooltip(make_simple_tooltip(
                                                        ctx,
                                                        make_unique_tooltip,
                                                    )),
                                            )
                                            .with_text("Make Unique")
                                            .build(ctx);
                                            make_unique
                                        }),
                                )
                                .add_row(Row::strict(20.0))
                                .add_column(Column::auto())
                                .add_column(Column::stretch())
                                .build(ctx),
                            ),
                    )
                    .add_row(Row::auto())
                    .add_row(Row::auto())
                    .add_column(Column::auto())
                    .build(ctx),
                )
                .build(),
            edit,
            sender,
            material,
            text,
            make_unique,
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
        TypeId::of::<SharedMaterial>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<SharedMaterial>()?;
        Ok(PropertyEditorInstance::Simple {
            editor: MaterialFieldEditorBuilder::new(WidgetBuilder::new()).build(
                ctx.build_context,
                self.sender.lock().clone(),
                value.clone(),
            ),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<SharedMaterial>()?;
        Ok(Some(MaterialFieldMessage::material(
            ctx.instance,
            MessageDirection::ToWidget,
            value.clone(),
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(MaterialFieldMessage::Material(value)) = ctx.message.data() {
                return Some(PropertyChanged {
                    name: ctx.name.to_string(),
                    owner_type_id: ctx.owner_type_id,
                    value: FieldKind::object(value.clone()),
                });
            }
        }
        None
    }
}
