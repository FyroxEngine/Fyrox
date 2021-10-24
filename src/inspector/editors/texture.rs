use crate::{asset::AssetItem, inspector::EditorEnvironment, make_relative_path};
use rg3d::gui::Thickness;
use rg3d::{
    asset::core::pool::Handle,
    engine::resource_manager::ResourceManager,
    gui::{
        image::ImageBuilder,
        inspector::{
            editors::{
                PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
                PropertyEditorMessageContext,
            },
            InspectorError,
        },
        message::{
            FieldKind, ImageMessage, MessageDirection, PropertyChanged, UiMessage, UiMessageData,
            WidgetMessage,
        },
        widget::{Widget, WidgetBuilder},
        BuildContext, Control, UiNode, UserInterface,
    },
    resource::texture::Texture,
    utils::into_gui_texture,
};
use std::{
    any::TypeId,
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
};

#[derive(Clone)]
pub struct TextureEditor {
    widget: Widget,
    image: Handle<UiNode>,
    resource_manager: ResourceManager,
    texture: Option<Texture>,
}

impl Debug for TextureEditor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "TextureEditor")
    }
}

impl Deref for TextureEditor {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl DerefMut for TextureEditor {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum TextureEditorMessage {
    Texture(Option<Texture>),
}

impl Control for TextureEditor {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        match message.data() {
            UiMessageData::Widget(WidgetMessage::Drop(dropped)) => {
                if message.destination() == self.image {
                    if let Some(item) = ui.node(*dropped).cast::<AssetItem>() {
                        let relative_path = make_relative_path(&item.path);

                        ui.send_message(UiMessage::user(
                            self.handle(),
                            MessageDirection::ToWidget,
                            Box::new(TextureEditorMessage::Texture(Some(
                                self.resource_manager.request_texture(relative_path, None),
                            ))),
                        ));
                    }
                }
            }
            UiMessageData::User(msg) => {
                if let Some(TextureEditorMessage::Texture(texture)) =
                    msg.cast::<TextureEditorMessage>()
                {
                    if &self.texture != texture && message.direction() == MessageDirection::ToWidget
                    {
                        self.texture = texture.clone();

                        ui.send_message(ImageMessage::texture(
                            self.image,
                            MessageDirection::ToWidget,
                            self.texture.clone().map(|t| into_gui_texture(t)),
                        ));

                        ui.send_message(message.reverse());
                    }
                }
            }
            _ => {}
        }
    }
}

pub struct TextureEditorBuilder {
    widget_builder: WidgetBuilder,
    texture: Option<Texture>,
}

impl TextureEditorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            texture: None,
        }
    }

    pub fn with_texture(mut self, texture: Option<Texture>) -> Self {
        self.texture = texture;
        self
    }

    pub fn build(
        self,
        ctx: &mut BuildContext,
        resource_manager: ResourceManager,
    ) -> Handle<UiNode> {
        let image;
        let widget = self
            .widget_builder
            .with_child({
                image = ImageBuilder::new(
                    WidgetBuilder::new()
                        .with_margin(Thickness::uniform(1.0))
                        .with_allow_drop(true),
                )
                .with_opt_texture(self.texture.clone().map(|t| into_gui_texture(t)))
                .build(ctx);
                image
            })
            .build();

        let editor = TextureEditor {
            widget,
            image,
            resource_manager,
            texture: None,
        };

        ctx.add_node(UiNode::new(editor))
    }
}

#[derive(Debug)]
pub struct TexturePropertyEditorDefinition;

impl PropertyEditorDefinition for TexturePropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Option<Texture>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<Option<Texture>>()?;

        Ok(PropertyEditorInstance {
            title: Default::default(),
            editor: TextureEditorBuilder::new(WidgetBuilder::new())
                .with_texture(value.clone())
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
        let value = ctx.property_info.cast_value::<Option<Texture>>()?;

        Ok(Some(UiMessage::user(
            ctx.instance,
            MessageDirection::ToWidget,
            Box::new(TextureEditorMessage::Texture(value.clone())),
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
                if let Some(TextureEditorMessage::Texture(value)) =
                    msg.cast::<TextureEditorMessage>()
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
