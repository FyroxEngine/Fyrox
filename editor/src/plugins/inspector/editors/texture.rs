// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::{
    asset::{item::AssetItem, preview::cache::IconRequest, selector::AssetSelectorMixin},
    fyrox::{
        asset::{manager::ResourceManager, untyped::UntypedResource},
        core::{
            algebra::Vector2, color::Color, make_relative_path, pool::Handle, reflect::prelude::*,
            type_traits::prelude::*, uuid_provider, visitor::prelude::*,
        },
        graph::SceneGraph,
        gui::{
            button::{Button, ButtonMessage},
            grid::{Column, GridBuilder, Row},
            image::{Image, ImageBuilder, ImageMessage},
            inspector::{
                editors::{
                    PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
                    PropertyEditorMessageContext, PropertyEditorTranslationContext,
                },
                FieldAction, InspectorError, PropertyChanged,
            },
            message::{MessageData, MessageDirection, UiMessage},
            text::{Text, TextBuilder, TextMessage},
            utils::{make_asset_preview_tooltip, ImageButtonBuilder},
            widget::{Widget, WidgetBuilder, WidgetMessage},
            BuildContext, Control, Thickness, UiNode, UserInterface, VerticalAlignment,
        },
        resource::texture::{Texture, TextureResource},
    },
    load_image,
    message::MessageSender,
    plugins::inspector::EditorEnvironment,
    utils, Message,
};
use std::{
    any::TypeId,
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
    sync::mpsc::Sender,
};

#[derive(Clone, Visit, Reflect, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct TextureEditor {
    widget: Widget,
    image: Handle<Image>,
    path: Handle<Text>,
    texture: Option<TextureResource>,
    unassign: Handle<Button>,
    locate: Handle<Button>,
    selector_mixin: AssetSelectorMixin<Texture>,
    #[visit(skip)]
    #[reflect(hidden)]
    sender: MessageSender,
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
impl MessageData for TextureEditorMessage {}

uuid_provider!(TextureEditor = "5db49479-ff89-49b8-a038-0766253d6493");

fn texture_name(texture: Option<&TextureResource>, resource_manager: &ResourceManager) -> String {
    match texture.and_then(|tex| resource_manager.resource_path(tex.as_ref())) {
        None => "Unassigned".to_string(),
        Some(path) => path.to_string_lossy().to_string(),
    }
}

impl Control for TextureEditor {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(WidgetMessage::Drop(dropped)) = message.data::<WidgetMessage>() {
            if message.destination() == self.image {
                if let Some(item) = ui.node(*dropped).cast::<AssetItem>() {
                    if let Ok(relative_path) = make_relative_path(&item.path) {
                        ui.send(
                            self.handle(),
                            TextureEditorMessage::Texture(
                                self.selector_mixin
                                    .resource_manager
                                    .try_request::<Texture>(relative_path),
                            ),
                        );
                    }
                }
            }
        } else if let Some(TextureEditorMessage::Texture(texture)) = message.data_for(self.handle) {
            if &self.texture != texture {
                self.texture.clone_from(texture);

                ui.send(self.image, ImageMessage::Texture(self.texture.clone()));
                ui.send(
                    self.path,
                    TextMessage::Text(texture_name(
                        self.texture.as_ref(),
                        &self.selector_mixin.resource_manager,
                    )),
                );

                ui.send_message(message.reverse());
            }
        } else if let Some(ButtonMessage::Click) = message.data_from(self.locate) {
            if let Some(path) = self.texture.as_ref().and_then(|t| {
                self.selector_mixin
                    .resource_manager
                    .resource_path(t.as_ref())
            }) {
                self.sender.send(Message::ShowInAssetBrowser(path));
            }
        } else if let Some(ButtonMessage::Click) = message.data_from(self.unassign) {
            ui.send(self.handle, TextureEditorMessage::Texture(None));
        }

        self.selector_mixin
            .handle_ui_message(self.texture.as_ref(), ui, message);
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        self.selector_mixin
            .preview_ui_message(ui, message, |resource| {
                UiMessage::for_widget(
                    self.handle,
                    TextureEditorMessage::Texture(resource.try_cast::<Texture>()),
                )
            });
    }
}

impl TextureEditor {
    pub fn texture(&self) -> Option<&TextureResource> {
        self.texture.as_ref()
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
        sender: MessageSender,
        icon_request_sender: Sender<IconRequest>,
        resource_manager: ResourceManager,
    ) -> Handle<TextureEditor> {
        let image = ImageBuilder::new(
            WidgetBuilder::new()
                .on_column(0)
                .with_margin(Thickness::uniform(1.0))
                .with_allow_drop(true)
                .with_width(32.0)
                .with_height(32.0),
        )
        .with_sync_with_texture_size(false)
        .with_checkerboard_background(true)
        .with_opt_texture(self.texture.clone())
        .build(ctx);

        let (tooltip, _) = make_asset_preview_tooltip(self.texture.clone(), ctx);

        let select = utils::make_pick_button(2, ctx);

        let path = TextBuilder::new(
            WidgetBuilder::new()
                .on_column(1)
                .with_margin(Thickness::uniform(1.0))
                .with_vertical_alignment(VerticalAlignment::Center),
        )
        .with_text(texture_name(self.texture.as_ref(), &resource_manager))
        .build(ctx);

        let locate = ImageButtonBuilder::default()
            .on_column(3)
            .with_image_size(14.0)
            .with_size(22.0)
            .with_image(load_image!("../../../../resources/locate.png"))
            .with_tooltip("Show In Asset Browser")
            .build_button(ctx);

        let unassign = ImageButtonBuilder::default()
            .on_column(4)
            .with_image_size(14.0)
            .with_size(22.0)
            .with_image_color(Color::opaque(180, 0, 0))
            .with_image(load_image!("../../../../resources/cross.png"))
            .with_tooltip("Unassign. Fallback will be used instead.")
            .build_button(ctx);

        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(image)
                .with_child(path)
                .with_child(select)
                .with_child(locate)
                .with_child(unassign),
        )
        .add_row(Row::auto())
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .add_column(Column::auto())
        .add_column(Column::auto())
        .build(ctx);

        let widget = self
            .widget_builder
            .with_tooltip(tooltip)
            .with_preview_messages(true)
            .with_child(content)
            .build(ctx);

        let editor = TextureEditor {
            widget,
            image,
            path,
            texture: None,
            unassign,
            locate,
            selector_mixin: AssetSelectorMixin::new(select, icon_request_sender, resource_manager),
            sender,
        };

        ctx.add(editor)
    }
}

#[derive(Debug)]
pub struct TexturePropertyEditorDefinition {
    pub untyped: bool,
}

impl TexturePropertyEditorDefinition {
    fn value(&self, field_info: &FieldRef) -> Result<Option<TextureResource>, InspectorError> {
        if self.untyped {
            let value = field_info.cast_value::<Option<UntypedResource>>()?;
            let casted = value.as_ref().and_then(|r| r.try_cast::<Texture>());
            Ok(casted)
        } else {
            Ok(field_info.cast_value::<Option<TextureResource>>()?.clone())
        }
    }
}

impl PropertyEditorDefinition for TexturePropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        if self.untyped {
            TypeId::of::<Option<UntypedResource>>()
        } else {
            TypeId::of::<Option<TextureResource>>()
        }
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = self.value(ctx.property_info)?;
        let environment = EditorEnvironment::try_get_from(&ctx.environment)?;

        Ok(PropertyEditorInstance::simple(
            TextureEditorBuilder::new(WidgetBuilder::new().with_min_size(Vector2::new(0.0, 17.0)))
                .with_texture(value.clone())
                .build(
                    ctx.build_context,
                    environment.sender.clone(),
                    environment.icon_request_sender.clone(),
                    environment.resource_manager.clone(),
                ),
        ))
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = self.value(ctx.property_info)?;

        Ok(Some(UiMessage::for_widget(
            ctx.instance,
            TextureEditorMessage::Texture(value.clone()),
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(TextureEditorMessage::Texture(value)) =
                ctx.message.data::<TextureEditorMessage>()
            {
                return Some(PropertyChanged {
                    name: ctx.name.to_string(),
                    action: if self.untyped {
                        FieldAction::object(value.clone().map(|r| r.into_untyped()))
                    } else {
                        FieldAction::object(value.clone())
                    },
                });
            }
        }
        None
    }
}
