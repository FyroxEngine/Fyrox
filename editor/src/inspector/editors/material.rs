use crate::{
    asset::item::AssetItem,
    fyrox::{
        asset::{core::pool::Handle, state::ResourceState},
        core::{
            color::Color, parking_lot::Mutex, reflect::prelude::*, type_traits::prelude::*,
            uuid_provider, visitor::prelude::*,
        },
        graph::BaseSceneGraph,
        gui::{
            brush::Brush,
            button::{ButtonBuilder, ButtonMessage},
            define_constructor,
            draw::{CommandTexture, Draw, DrawingContext},
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
            widget::{Widget, WidgetBuilder, WidgetMessage},
            BuildContext, Control, Thickness, UiNode, UserInterface, VerticalAlignment,
        },
        material::{Material, MaterialResource, MaterialResourceExtension},
    },
    message::MessageSender,
    Message, MessageDirection,
};
use std::{
    any::TypeId,
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone, PartialEq)]
pub enum MaterialFieldMessage {
    Material(MaterialResource),
}

impl MaterialFieldMessage {
    define_constructor!(MaterialFieldMessage:Material => fn material(MaterialResource), layout: false);
}

#[derive(Clone, Visit, Reflect, ComponentProvider)]
pub struct MaterialFieldEditor {
    widget: Widget,
    #[visit(skip)]
    #[reflect(hidden)]
    sender: MessageSender,
    text: Handle<UiNode>,
    edit: Handle<UiNode>,
    make_unique: Handle<UiNode>,
    material: MaterialResource,
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

uuid_provider!(MaterialFieldEditor = "d3fa0a7c-52d6-4cca-885e-0db8b18542e2");

impl Control for MaterialFieldEditor {
    fn draw(&self, drawing_context: &mut DrawingContext) {
        // Emit transparent geometry for the field to be able to catch mouse events without precise
        // pointing.
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

        if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == self.edit {
                self.sender
                    .send(Message::OpenMaterialEditor(self.material.clone()));
            } else if message.destination() == self.make_unique {
                ui.send_message(MaterialFieldMessage::material(
                    self.handle,
                    MessageDirection::ToWidget,
                    self.material.deep_copy_as_embedded(),
                ));
            }
        } else if let Some(MaterialFieldMessage::Material(material)) = message.data() {
            if message.destination() == self.handle
                && message.direction() == MessageDirection::ToWidget
                && &self.material != material
            {
                self.material = material.clone();

                ui.send_message(TextMessage::text(
                    self.text,
                    MessageDirection::ToWidget,
                    make_name(&self.material),
                ));

                ui.send_message(message.reverse());
            }
        } else if let Some(WidgetMessage::Drop(dropped)) = message.data() {
            if let Some(item) = ui.node(*dropped).cast::<AssetItem>() {
                if let Some(material) = item.resource::<Material>() {
                    ui.send_message(MaterialFieldMessage::material(
                        self.handle(),
                        MessageDirection::ToWidget,
                        material,
                    ));
                }
            }
        }
    }
}

pub struct MaterialFieldEditorBuilder {
    widget_builder: WidgetBuilder,
}

fn make_name(material: &MaterialResource) -> String {
    let header = material.header();
    match header.state {
        ResourceState::Ok(_) => {
            format!("{} - {} uses", header.kind, material.use_count())
        }
        ResourceState::LoadError { ref error, .. } => {
            format!("Loading failed: {:?}", error)
        }
        ResourceState::Pending { .. } => {
            format!("Loading {}", header.kind)
        }
    }
}

impl MaterialFieldEditorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self { widget_builder }
    }

    pub fn build(
        self,
        ctx: &mut BuildContext,
        sender: MessageSender,
        material: MaterialResource,
    ) -> Handle<UiNode> {
        let edit;
        let text;
        let make_unique;
        let make_unique_tooltip = "Creates a deep copy of the material, making a separate version of the material. \
        Useful when you need to change some properties in the material, but only on some nodes that uses the material.";

        let editor = MaterialFieldEditor {
            widget: self
                .widget_builder
                .with_allow_drop(true)
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
    pub sender: Mutex<MessageSender>,
}

impl PropertyEditorDefinition for MaterialPropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<MaterialResource>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<MaterialResource>()?;
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
        let value = ctx.property_info.cast_value::<MaterialResource>()?;
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
