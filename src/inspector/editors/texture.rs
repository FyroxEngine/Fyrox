use crate::inspector::EditorEnvironment;
use crate::{
    gui::{
        BuildContext, CustomWidget, EditorUiMessage, EditorUiNode, Ui, UiMessage, UiNode,
        UiWidgetBuilder,
    },
    make_relative_path,
};
use rg3d::{
    asset::core::{inspect::PropertyInfo, pool::Handle},
    engine::resource_manager::ResourceManager,
    gui::{
        image::ImageBuilder,
        inspector::{
            editors::{PropertyEditorBuildContext, PropertyEditorDefinition},
            InspectorError,
        },
        message::{ImageMessage, MessageDirection, PropertyChanged, UiMessageData, WidgetMessage},
        node::UINode,
        Control,
    },
    resource::texture::Texture,
    utils::into_gui_texture,
};
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
};

#[derive(Clone)]
pub struct TextureEditor {
    widget: CustomWidget,
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
    type Target = CustomWidget;

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

impl Control<EditorUiMessage, EditorUiNode> for TextureEditor {
    fn handle_routed_message(&mut self, ui: &mut Ui, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        match message.data() {
            UiMessageData::Widget(WidgetMessage::Drop(dropped)) => {
                if message.destination() == self.image {
                    if let UiNode::User(EditorUiNode::AssetItem(item)) = ui.node(*dropped) {
                        let relative_path = make_relative_path(&item.path);

                        ui.send_message(UiMessage::user(
                            self.handle(),
                            MessageDirection::ToWidget,
                            EditorUiMessage::TextureEditor(TextureEditorMessage::Texture(Some(
                                self.resource_manager.request_texture(relative_path, None),
                            ))),
                        ));
                    }
                }
            }
            UiMessageData::User(EditorUiMessage::TextureEditor(TextureEditorMessage::Texture(
                texture,
            ))) => {
                if &self.texture != texture && message.direction() == MessageDirection::ToWidget {
                    self.texture = texture.clone();

                    ui.send_message(ImageMessage::texture(
                        self.image,
                        MessageDirection::ToWidget,
                        self.texture.clone().map(|t| into_gui_texture(t)),
                    ));
                }
            }
            _ => {}
        }
    }
}

pub struct TextureEditorBuilder {
    widget_builder: UiWidgetBuilder,
    texture: Option<Texture>,
}

impl TextureEditorBuilder {
    pub fn new(widget_builder: UiWidgetBuilder) -> Self {
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
                image = ImageBuilder::new(UiWidgetBuilder::new().with_allow_drop(true))
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

        ctx.add_node(UiNode::User(EditorUiNode::TextureEditor(editor)))
    }
}

#[derive(Debug)]
pub struct TexturePropertyEditorDefinition;

impl PropertyEditorDefinition<EditorUiMessage, EditorUiNode> for TexturePropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Option<Texture>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext<EditorUiMessage, EditorUiNode>,
    ) -> Result<Handle<UINode<EditorUiMessage, EditorUiNode>>, InspectorError> {
        let value = ctx.property_info.cast_value::<Option<Texture>>()?;

        Ok(
            TextureEditorBuilder::new(UiWidgetBuilder::new().on_row(ctx.row).on_column(ctx.column))
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
        )
    }

    fn create_message(
        &self,
        instance: Handle<UINode<EditorUiMessage, EditorUiNode>>,
        property_info: &PropertyInfo,
    ) -> Result<UiMessage, InspectorError> {
        let value = property_info.cast_value::<Option<Texture>>()?;

        Ok(UiMessage::user(
            instance,
            MessageDirection::ToWidget,
            EditorUiMessage::TextureEditor(TextureEditorMessage::Texture(value.clone())),
        ))
    }

    fn translate_message(
        &self,
        name: &str,
        owner_type_id: TypeId,
        message: &UiMessage,
    ) -> Option<PropertyChanged> {
        if message.direction() == MessageDirection::FromWidget {
            if let UiMessageData::User(EditorUiMessage::TextureEditor(
                TextureEditorMessage::Texture(value),
            )) = message.data()
            {
                return Some(PropertyChanged {
                    owner_type_id,
                    name: name.to_string(),
                    value: Arc::new(value.clone()),
                });
            }
        }
        None
    }
}
