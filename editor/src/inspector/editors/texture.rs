use crate::{asset::item::AssetItem, inspector::EditorEnvironment};
use fyrox::{
    asset::manager::ResourceManager,
    core::{algebra::Vector2, make_relative_path, pool::Handle},
    gui::{
        define_constructor,
        image::{ImageBuilder, ImageMessage},
        inspector::{
            editors::{
                PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
                PropertyEditorMessageContext, PropertyEditorTranslationContext,
            },
            FieldKind, InspectorError, PropertyChanged,
        },
        message::{MessageDirection, UiMessage},
        widget::{Widget, WidgetBuilder, WidgetMessage},
        BuildContext, Control, Thickness, UiNode, UserInterface,
    },
    resource::texture::{Texture, TextureResource},
    utils::into_gui_texture,
};
use std::{
    any::{Any, TypeId},
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
};

#[derive(Clone)]
pub struct TextureEditor {
    widget: Widget,
    image: Handle<UiNode>,
    resource_manager: ResourceManager,
    texture: Option<TextureResource>,
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

#[derive(Debug, PartialEq, Clone, Eq)]
pub enum TextureEditorMessage {
    Texture(Option<TextureResource>),
}

impl TextureEditorMessage {
    define_constructor!(Self:Texture => fn texture(Option<TextureResource>), layout: false);
}

impl Control for TextureEditor {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(WidgetMessage::Drop(dropped)) = message.data::<WidgetMessage>() {
            if message.destination() == self.image {
                if let Some(item) = ui.node(*dropped).cast::<AssetItem>() {
                    if let Ok(relative_path) = make_relative_path(&item.path) {
                        ui.send_message(TextureEditorMessage::texture(
                            self.handle(),
                            MessageDirection::ToWidget,
                            Some(self.resource_manager.request::<Texture, _>(relative_path)),
                        ));
                    }
                }
            }
        } else if let Some(TextureEditorMessage::Texture(texture)) =
            message.data::<TextureEditorMessage>()
        {
            if &self.texture != texture && message.direction() == MessageDirection::ToWidget {
                self.texture = texture.clone();

                ui.send_message(ImageMessage::texture(
                    self.image,
                    MessageDirection::ToWidget,
                    self.texture.clone().map(into_gui_texture),
                ));

                ui.send_message(message.reverse());
            }
        }
    }
}

pub struct TextureEditorBuilder {
    widget_builder: WidgetBuilder,
    texture: Option<TextureResource>,
}

impl TextureEditorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            texture: None,
        }
    }

    pub fn with_texture(mut self, texture: Option<TextureResource>) -> Self {
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
                .with_checkerboard_background(true)
                .with_opt_texture(self.texture.map(into_gui_texture))
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
        TypeId::of::<Option<TextureResource>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<Option<TextureResource>>()?;

        Ok(PropertyEditorInstance::Simple {
            editor: TextureEditorBuilder::new(
                WidgetBuilder::new().with_min_size(Vector2::new(0.0, 17.0)),
            )
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
        let value = ctx.property_info.cast_value::<Option<TextureResource>>()?;

        Ok(Some(TextureEditorMessage::texture(
            ctx.instance,
            MessageDirection::ToWidget,
            value.clone(),
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(TextureEditorMessage::Texture(value)) =
                ctx.message.data::<TextureEditorMessage>()
            {
                return Some(PropertyChanged {
                    owner_type_id: ctx.owner_type_id,
                    name: ctx.name.to_string(),
                    value: FieldKind::object(value.clone()),
                });
            }
        }
        None
    }
}
