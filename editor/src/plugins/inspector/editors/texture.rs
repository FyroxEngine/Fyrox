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
    asset::{
        item::AssetItem, preview::cache::IconRequest, selector::AssetSelectorMessage,
        selector::AssetSelectorWindowBuilder,
    },
    fyrox::{
        asset::{manager::ResourceManager, untyped::UntypedResource},
        core::{
            algebra::Vector2, make_relative_path, pool::Handle, reflect::prelude::*,
            type_traits::prelude::*, uuid_provider, visitor::prelude::*,
        },
        graph::{BaseSceneGraph, SceneGraph},
        gui::{
            button::ButtonMessage,
            define_constructor,
            grid::{Column, GridBuilder, Row},
            image::{ImageBuilder, ImageMessage},
            inspector::{
                editors::{
                    PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
                    PropertyEditorMessageContext, PropertyEditorTranslationContext,
                },
                FieldKind, InspectorError, PropertyChanged,
            },
            menu::{ContextMenuBuilder, MenuItemBuilder, MenuItemContent, MenuItemMessage},
            message::{MessageDirection, UiMessage},
            popup::{PopupBuilder, PopupMessage},
            stack_panel::StackPanelBuilder,
            widget::{Widget, WidgetBuilder, WidgetMessage},
            window::WindowMessage,
            BuildContext, Control, RcUiNodeHandle, Thickness, UiNode, UserInterface,
        },
        resource::texture::{Texture, TextureResource},
    },
    message::MessageSender,
    plugins::inspector::EditorEnvironment,
    utils, Message,
};
use std::{
    any::TypeId,
    cell::Cell,
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
    sync::mpsc::Sender,
};

#[derive(Clone, Debug, PartialEq)]
struct TextureContextMenu {
    popup: RcUiNodeHandle,
    show_in_asset_browser: Handle<UiNode>,
    unassign: Handle<UiNode>,
}

impl TextureContextMenu {
    fn new(owner: Handle<UiNode>, ctx: &mut BuildContext) -> Self {
        let show_in_asset_browser;
        let unassign;
        let popup = ContextMenuBuilder::new(
            PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
                .with_content(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .with_child({
                                show_in_asset_browser = MenuItemBuilder::new(WidgetBuilder::new())
                                    .with_content(MenuItemContent::text("Show In Asset Browser"))
                                    .build(ctx);
                                show_in_asset_browser
                            })
                            .with_child({
                                unassign = MenuItemBuilder::new(WidgetBuilder::new())
                                    .with_content(MenuItemContent::text("Unassign"))
                                    .build(ctx);
                                unassign
                            }),
                    )
                    .build(ctx),
                )
                .with_owner(owner),
        )
        .build(ctx);
        let popup = RcUiNodeHandle::new(popup, ctx.sender());

        Self {
            popup,
            show_in_asset_browser,
            unassign,
        }
    }
}

#[derive(Clone, Visit, Reflect, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct TextureEditor {
    widget: Widget,
    image: Handle<UiNode>,
    #[visit(skip)]
    #[reflect(hidden)]
    resource_manager: ResourceManager,
    texture: Option<TextureResource>,
    selector: Cell<Handle<UiNode>>,
    select: Handle<UiNode>,
    #[visit(skip)]
    #[reflect(hidden)]
    sender: MessageSender,
    #[visit(skip)]
    #[reflect(hidden)]
    texture_context_menu: Option<TextureContextMenu>,
    #[visit(skip)]
    #[reflect(hidden)]
    icon_request_sender: Sender<IconRequest>,
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
    define_constructor!(TextureEditorMessage:Texture => fn texture(Option<TextureResource>), layout: false);
}

uuid_provider!(TextureEditor = "5db49479-ff89-49b8-a038-0766253d6493");

impl Control for TextureEditor {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(WidgetMessage::Drop(dropped)) = message.data::<WidgetMessage>() {
            if message.destination() == self.image {
                if let Some(item) = ui.node(*dropped).cast::<AssetItem>() {
                    if let Ok(relative_path) = make_relative_path(&item.path) {
                        ui.send_message(TextureEditorMessage::texture(
                            self.handle(),
                            MessageDirection::ToWidget,
                            self.resource_manager.try_request::<Texture>(relative_path),
                        ));
                    }
                }
            }
        } else if let Some(TextureEditorMessage::Texture(texture)) = message.data() {
            if &self.texture != texture && message.direction() == MessageDirection::ToWidget {
                self.texture.clone_from(texture);

                ui.send_message(ImageMessage::texture(
                    self.image,
                    MessageDirection::ToWidget,
                    self.texture.clone(),
                ));

                ui.send_message(message.reverse());
            }
        } else if let Some(PopupMessage::RelayedMessage(message)) = message.data() {
            let context_menu = self.texture_context_menu.as_mut().unwrap();
            if let Some(MenuItemMessage::Click) = message.data() {
                if message.destination() == context_menu.show_in_asset_browser {
                    if let Some(path) = self
                        .texture
                        .as_ref()
                        .and_then(|t| self.resource_manager.resource_path(t.as_ref()))
                    {
                        self.sender.send(Message::ShowInAssetBrowser(path));
                    }
                } else if message.destination() == context_menu.unassign {
                    ui.send_message(TextureEditorMessage::texture(
                        self.handle,
                        MessageDirection::ToWidget,
                        None,
                    ));
                }
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.select {
                self.selector.set(
                    AssetSelectorWindowBuilder::build_for_type_and_open::<Texture>(
                        self.icon_request_sender.clone(),
                        self.resource_manager.clone(),
                        ui,
                    ),
                );
            }
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        if message.destination() == self.selector.get() {
            if let Some(WindowMessage::Close) = message.data() {
                self.selector.set(Handle::NONE);
            } else if let Some(AssetSelectorMessage::Select(resource)) = message.data() {
                ui.send_message(TextureEditorMessage::texture(
                    self.handle,
                    MessageDirection::ToWidget,
                    resource.try_cast::<Texture>(),
                ))
            }
        }
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
    ) -> Handle<UiNode> {
        let image = ImageBuilder::new(
            WidgetBuilder::new()
                .on_column(0)
                .with_margin(Thickness::uniform(1.0))
                .with_allow_drop(true),
        )
        .with_sync_with_texture_size(false)
        .with_checkerboard_background(true)
        .with_opt_texture(self.texture)
        .build(ctx);

        let select = utils::make_pick_button(1, ctx);

        let content = GridBuilder::new(WidgetBuilder::new().with_child(image).with_child(select))
            .add_row(Row::auto())
            .add_column(Column::stretch())
            .add_column(Column::auto())
            .build(ctx);

        let widget = self
            .widget_builder
            .with_preview_messages(true)
            .with_child(content)
            .build(ctx);

        let editor = TextureEditor {
            widget,
            image,
            resource_manager,
            texture: None,
            selector: Default::default(),
            select,
            sender,
            texture_context_menu: None,
            icon_request_sender,
        };

        let editor = ctx.add_node(UiNode::new(editor));

        let texture_context_menu = TextureContextMenu::new(editor, ctx);
        let editor_mut = ctx
            .inner_mut()
            .try_get_mut_of_type::<TextureEditor>(editor)
            .unwrap();
        editor_mut.context_menu = Some(texture_context_menu.popup.clone());
        editor_mut.texture_context_menu = Some(texture_context_menu);

        editor
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

        Ok(PropertyEditorInstance::Simple {
            editor: TextureEditorBuilder::new(
                WidgetBuilder::new().with_min_size(Vector2::new(0.0, 17.0)),
            )
            .with_texture(value.clone())
            .build(
                ctx.build_context,
                environment.sender.clone(),
                environment.icon_request_sender.clone(),
                environment.resource_manager.clone(),
            ),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = self.value(ctx.property_info)?;

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
                    name: ctx.name.to_string(),
                    value: if self.untyped {
                        FieldKind::object(value.clone().map(|r| r.into_untyped()))
                    } else {
                        FieldKind::object(value.clone())
                    },
                });
            }
        }
        None
    }
}
