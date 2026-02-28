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
        item::{AssetItem, AssetItemMessage},
        preview::cache::IconRequest,
        selector::AssetSelectorMixin,
    },
    fyrox::{
        asset::manager::ResourceManager,
        core::{
            futures::executor::block_on, log::Log, make_relative_path, pool::Handle,
            reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*,
        },
        graph::SceneGraph,
        gui::{
            brush::Brush,
            button::{Button, ButtonBuilder, ButtonMessage},
            define_widget_deref,
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
            stack_panel::StackPanelBuilder,
            text::{Text, TextBuilder, TextMessage},
            utils::{make_asset_preview_tooltip, ImageButtonBuilder},
            widget::{Widget, WidgetBuilder, WidgetMessage},
            BuildContext, Control, Orientation, Thickness, UiNode, UserInterface,
        },
        scene::mesh::surface::{SurfaceData, SurfaceResource},
    },
    load_image,
    message::MessageSender,
    plugins::inspector::EditorEnvironment,
    utils::make_pick_button,
    Message,
};
use std::{any::TypeId, sync::mpsc::Sender};

#[derive(Debug, PartialEq, Clone)]
pub enum SurfaceDataPropertyEditorMessage {
    Value(SurfaceResource),
}
impl MessageData for SurfaceDataPropertyEditorMessage {}

#[derive(Clone, Visit, Reflect, Debug, ComponentProvider, TypeUuidProvider)]
#[type_uuid(id = "8461a183-4fd4-4f74-a4f4-7fd8e84bf423")]
#[reflect(derived_type = "UiNode")]
#[allow(dead_code)]
pub struct SurfaceDataPropertyEditor {
    widget: Widget,
    view: Handle<Button>,
    locate: Handle<Button>,
    surface_resource: SurfaceResource,
    text: Handle<Text>,
    image: Handle<Image>,
    image_preview: Handle<Image>,
    #[visit(skip)]
    #[reflect(hidden)]
    sender: Option<MessageSender>,
    asset_selector_mixin: AssetSelectorMixin<SurfaceData>,
}

define_widget_deref!(SurfaceDataPropertyEditor);

impl Control for SurfaceDataPropertyEditor {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination == self.view {
                if let Some(sender) = self.sender.as_ref() {
                    sender.send(Message::ViewSurfaceData(self.surface_resource.clone()));
                }
            } else if message.destination() == self.locate {
                if let Some(path) = self
                    .asset_selector_mixin
                    .resource_manager
                    .resource_path(&self.surface_resource)
                {
                    if let Some(sender) = self.sender.as_ref() {
                        sender.send(Message::ShowInAssetBrowser(path));
                    }
                }
            }
        } else if let Some(WidgetMessage::Drop(dropped)) = message.data() {
            if message.destination() == self.handle() {
                if let Some(item) = ui.node(*dropped).cast::<AssetItem>() {
                    let path = if self
                        .asset_selector_mixin
                        .resource_manager
                        .state()
                        .built_in_resources
                        .contains_key(&item.path)
                    {
                        Ok(item.path.clone())
                    } else {
                        make_relative_path(&item.path)
                    };

                    if let Ok(path) = path {
                        if let Some(request) = self
                            .asset_selector_mixin
                            .resource_manager
                            .try_request::<SurfaceData>(path)
                        {
                            if let Ok(value) = block_on(request) {
                                ui.send(
                                    self.handle(),
                                    SurfaceDataPropertyEditorMessage::Value(value),
                                );
                            }
                        }
                    }
                }
            }
        } else if let Some(SurfaceDataPropertyEditorMessage::Value(surface_resource)) =
            message.data_for(self.handle)
        {
            if &self.surface_resource != surface_resource {
                self.surface_resource = surface_resource.clone();
                ui.send_message(message.reverse());

                ui.send(
                    self.text,
                    TextMessage::Text(surface_data_info(
                        &self.asset_selector_mixin.resource_manager,
                        surface_resource,
                    )),
                );

                self.asset_selector_mixin
                    .request_preview(self.handle, surface_resource);
            }
        } else if let Some(AssetItemMessage::Icon {
            texture,
            flip_y,
            color,
        }) = message.data_for(self.handle)
        {
            for widget in [self.image, self.image_preview] {
                ui.send(widget, ImageMessage::Texture(texture.clone()));
                ui.send(widget, ImageMessage::Flip(*flip_y));
                ui.send(
                    widget,
                    WidgetMessage::Background(Brush::Solid(*color).into()),
                )
            }
        }

        self.asset_selector_mixin
            .handle_ui_message(Some(&self.surface_resource), ui, message);
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        self.asset_selector_mixin
            .preview_ui_message(ui, message, |resource| {
                UiMessage::for_widget(
                    self.handle,
                    SurfaceDataPropertyEditorMessage::Value(
                        resource.try_cast::<SurfaceData>().unwrap(),
                    ),
                )
            })
    }
}

fn surface_data_info(resource_manager: &ResourceManager, data: &SurfaceResource) -> String {
    let use_count = data.use_count();
    let id = data.key();
    let kind = resource_manager
        .resource_path(data.as_ref())
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "External".to_string());
    if data.is_ok() {
        format!("{} - {} uses; Id: {}", kind, use_count, id)
    } else {
        format!("{}Not loaded - {} uses; Id: {}", kind, use_count, id)
    }
}

impl SurfaceDataPropertyEditor {
    pub fn build(
        ctx: &mut BuildContext,
        surface_resource: SurfaceResource,
        sender: MessageSender,
        icon_request_sender: Sender<IconRequest>,
        resource_manager: ResourceManager,
    ) -> Handle<SurfaceDataPropertyEditor> {
        let view = ButtonBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .on_row(0)
                .on_column(3)
                .with_width(55.0)
                .with_height(22.0),
        )
        .with_text("View...")
        .build(ctx);

        let select = make_pick_button(1, ctx);

        let text = TextBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
            .with_text(surface_data_info(&resource_manager, &surface_resource))
            .build(ctx);

        let (image_preview_tooltip, image_preview) = make_asset_preview_tooltip(None, ctx);

        let image = ImageBuilder::new(
            WidgetBuilder::new()
                .with_width(52.0)
                .with_height(52.0)
                .with_tooltip(image_preview_tooltip)
                .with_margin(Thickness::uniform(1.0)),
        )
        .build(ctx);

        let locate = ImageButtonBuilder::default()
            .with_size(22.0)
            .with_image(load_image!("../../../../resources/locate.png"))
            .with_tooltip("Show In Asset Browser")
            .build_button(ctx);

        let content = GridBuilder::new(
            WidgetBuilder::new()
                .on_column(1)
                .with_child(text)
                .with_child(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .on_row(1)
                            .with_child(select)
                            .with_child(locate)
                            .with_child(view),
                    )
                    .with_orientation(Orientation::Horizontal)
                    .build(ctx),
                ),
        )
        .add_column(Column::stretch())
        .add_row(Row::auto())
        .add_row(Row::auto())
        .build(ctx);

        let widget = WidgetBuilder::new()
            .with_preview_messages(true)
            .with_child(
                GridBuilder::new(WidgetBuilder::new().with_child(image).with_child(content))
                    .add_column(Column::auto())
                    .add_column(Column::stretch())
                    .add_row(Row::auto())
                    .build(ctx),
            )
            .with_allow_drop(true)
            .build(ctx);

        let editor = Self {
            widget,
            surface_resource: surface_resource.clone(),
            view,
            sender: Some(sender),
            asset_selector_mixin: AssetSelectorMixin::new(
                select,
                icon_request_sender.clone(),
                resource_manager,
            ),
            text,
            image,
            image_preview,
            locate,
        };

        let handle = ctx.add(editor);

        Log::verify(icon_request_sender.send(IconRequest {
            widget_handle: handle.to_base(),
            resource: surface_resource.into_untyped(),
            force_update: false,
        }));

        handle
    }
}

#[derive(Debug)]
pub struct SurfaceDataPropertyEditorDefinition {
    pub sender: MessageSender,
}

impl PropertyEditorDefinition for SurfaceDataPropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<SurfaceResource>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<SurfaceResource>()?;
        let environment = EditorEnvironment::try_get_from(&ctx.environment)?;

        Ok(PropertyEditorInstance::simple(
            SurfaceDataPropertyEditor::build(
                ctx.build_context,
                value.clone(),
                self.sender.clone(),
                environment.icon_request_sender.clone(),
                environment.resource_manager.clone(),
            ),
        ))
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<SurfaceResource>()?;

        Ok(Some(UiMessage::for_widget(
            ctx.instance,
            SurfaceDataPropertyEditorMessage::Value(value.clone()),
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(SurfaceDataPropertyEditorMessage::Value(value)) = ctx.message.data() {
                return Some(PropertyChanged {
                    name: ctx.name.to_string(),
                    action: FieldAction::object(value.clone()),
                });
            }
        }
        None
    }
}
