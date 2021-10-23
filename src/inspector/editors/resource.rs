use crate::{asset::AssetItem, inspector::EditorEnvironment, load_image, make_relative_path};
use rg3d::{
    asset::{Resource, ResourceData, ResourceLoadError},
    core::{futures::executor::block_on, pool::Handle},
    engine::resource_manager::ResourceManager,
    gui::{
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        inspector::{
            editors::{
                PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
                PropertyEditorMessageContext,
            },
            InspectorError,
        },
        message::{
            FieldKind, MessageDirection, PropertyChanged, TextMessage, UiMessage, UiMessageData,
            WidgetMessage,
        },
        text::TextBuilder,
        widget::{Widget, WidgetBuilder},
        BuildContext, Control, UiNode, UserInterface, VerticalAlignment,
    },
    resource::model::Model,
    sound::buffer::SoundBufferResource,
};
use std::{
    any::TypeId,
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
};

#[derive(Debug)]
pub struct ModelResourcePropertyEditorDefinition;

fn resource_path<T, S, E>(resource: &Option<T>) -> String
where
    T: Deref<Target = Resource<S, E>>,
    S: ResourceData,
    E: ResourceLoadError,
{
    resource
        .as_ref()
        .map(|m| m.state().path().to_string_lossy().to_string())
        .unwrap_or_else(|| "None".to_string())
}

impl PropertyEditorDefinition for ModelResourcePropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Option<Model>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<Option<Model>>()?;

        Ok(PropertyEditorInstance {
            title: Default::default(),
            editor: TextBuilder::new(WidgetBuilder::new())
                .with_text(resource_path(value))
                .with_vertical_text_alignment(VerticalAlignment::Center)
                .build(ctx.build_context),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<Option<Model>>()?;

        Ok(Some(TextMessage::text(
            ctx.instance,
            MessageDirection::ToWidget,
            resource_path(value),
        )))
    }

    fn translate_message(
        &self,
        _name: &str,
        _owner_type_id: TypeId,
        _message: &UiMessage,
    ) -> Option<PropertyChanged> {
        None
    }
}

#[derive(Debug, PartialEq)]
pub enum SoundBufferFieldMessage {
    Value(Option<SoundBufferResource>),
}

impl SoundBufferFieldMessage {
    pub fn value(
        destination: Handle<UiNode>,
        direction: MessageDirection,
        value: Option<SoundBufferResource>,
    ) -> UiMessage {
        UiMessage::user(
            destination,
            direction,
            Box::new(SoundBufferFieldMessage::Value(value)),
        )
    }
}

#[derive(Clone)]
pub struct SoundBufferField {
    widget: Widget,
    name: Handle<UiNode>,
    resource_manager: ResourceManager,
    sound_buffer: Option<SoundBufferResource>,
}

impl Debug for SoundBufferField {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "SoundBufferField")
    }
}

impl Deref for SoundBufferField {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl DerefMut for SoundBufferField {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl Control for SoundBufferField {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        match message.data() {
            UiMessageData::Widget(WidgetMessage::Drop(dropped)) => {
                if message.destination() == self.handle() {
                    if let Some(item) = ui.node(*dropped).cast::<AssetItem>() {
                        let relative_path = make_relative_path(&item.path);

                        if let Ok(value) = block_on(
                            self.resource_manager
                                .request_sound_buffer(relative_path, false),
                        ) {
                            ui.send_message(UiMessage::user(
                                self.handle(),
                                MessageDirection::ToWidget,
                                Box::new(SoundBufferFieldMessage::Value(Some(value))),
                            ));
                        }
                    }
                }
            }
            UiMessageData::User(msg) => {
                if let Some(SoundBufferFieldMessage::Value(sound_buffer)) =
                    msg.cast::<SoundBufferFieldMessage>()
                {
                    if &self.sound_buffer != sound_buffer
                        && message.destination() == self.handle()
                        && message.direction() == MessageDirection::ToWidget
                    {
                        self.sound_buffer = sound_buffer.clone();

                        ui.send_message(TextMessage::text(
                            self.name,
                            MessageDirection::ToWidget,
                            resource_path(sound_buffer),
                        ));

                        ui.send_message(message.reverse());
                    }
                }
            }
            _ => (),
        }
    }
}

pub struct SoundBufferFieldBuilder {
    widget_builder: WidgetBuilder,
    sound_buffer: Option<SoundBufferResource>,
}

impl SoundBufferFieldBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            sound_buffer: Default::default(),
        }
    }

    pub fn with_sound_buffer(mut self, sound_buffer: Option<SoundBufferResource>) -> Self {
        self.sound_buffer = sound_buffer;
        self
    }

    pub fn build(
        self,
        ctx: &mut BuildContext,
        resource_manager: ResourceManager,
    ) -> Handle<UiNode> {
        let name;
        let field = SoundBufferField {
            widget: self
                .widget_builder
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .with_child(
                                ImageBuilder::new(
                                    WidgetBuilder::new()
                                        .on_column(0)
                                        .with_width(16.0)
                                        .with_height(16.0),
                                )
                                .with_opt_texture(load_image(include_bytes!(
                                    "../../../resources/embed/sound_source.png"
                                )))
                                .build(ctx),
                            )
                            .with_child({
                                name = TextBuilder::new(WidgetBuilder::new().on_column(1))
                                    .with_text(resource_path(&self.sound_buffer))
                                    .with_vertical_text_alignment(VerticalAlignment::Center)
                                    .build(ctx);
                                name
                            }),
                    )
                    .add_column(Column::auto())
                    .add_column(Column::stretch())
                    .add_row(Row::stretch())
                    .build(ctx),
                )
                .with_allow_drop(true)
                .build(),
            name,
            resource_manager,
            sound_buffer: self.sound_buffer,
        };

        ctx.add_node(UiNode::new(field))
    }
}

#[derive(Debug)]
pub struct SoundBufferResourcePropertyEditorDefinition;

impl PropertyEditorDefinition for SoundBufferResourcePropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Option<SoundBufferResource>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx
            .property_info
            .cast_value::<Option<SoundBufferResource>>()?;

        Ok(PropertyEditorInstance {
            title: Default::default(),
            editor: SoundBufferFieldBuilder::new(WidgetBuilder::new())
                .with_sound_buffer(value.clone())
                .build(
                    ctx.build_context,
                    ctx.environment
                        .as_ref()
                        .unwrap()
                        .as_any()
                        .downcast_ref::<EditorEnvironment>()
                        .map(|e| e.resource_manager.clone())
                        .unwrap(),
                ),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx
            .property_info
            .cast_value::<Option<SoundBufferResource>>()?;

        Ok(Some(SoundBufferFieldMessage::value(
            ctx.instance,
            MessageDirection::ToWidget,
            value.clone(),
        )))
    }

    fn translate_message(
        &self,
        name: &str,
        owner_type_id: TypeId,
        message: &UiMessage,
    ) -> Option<PropertyChanged> {
        if message.direction() == MessageDirection::FromWidget {
            if let UiMessageData::User(msg) = message.data() {
                if let Some(SoundBufferFieldMessage::Value(value)) =
                    msg.cast::<SoundBufferFieldMessage>()
                {
                    return Some(PropertyChanged {
                        owner_type_id,
                        name: name.to_string(),
                        value: FieldKind::object(value.clone()),
                    });
                }
            }
        }
        None
    }
}
