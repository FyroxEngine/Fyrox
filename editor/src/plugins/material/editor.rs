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
        asset::{core::pool::Handle, manager::ResourceManager, state::ResourceState},
        core::{
            color::Color, log::Log, parking_lot::Mutex, reflect::prelude::*,
            type_traits::prelude::*, uuid_provider, visitor::prelude::*, SafeLock,
        },
        graph::SceneGraph,
        gui::{
            brush::Brush,
            button::{ButtonBuilder, ButtonMessage},
            draw::{CommandTexture, Draw, DrawingContext},
            grid::{Column, GridBuilder, Row},
            image::{ImageBuilder, ImageMessage},
            inspector::{
                editors::{
                    PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
                    PropertyEditorMessageContext, PropertyEditorTranslationContext,
                },
                FieldKind, InspectorError, PropertyChanged,
            },
            message::UiMessage,
            text::{TextBuilder, TextMessage},
            utils::{make_asset_preview_tooltip, make_simple_tooltip},
            widget::{Widget, WidgetBuilder, WidgetMessage},
            BuildContext, Control, Thickness, UiNode, UserInterface, VerticalAlignment,
        },
        material::{Material, MaterialResource, MaterialResourceExtension},
    },
    message::MessageSender,
    plugins::inspector::EditorEnvironment,
    utils::make_pick_button,
    Message, MessageDirection,
};
use fyrox::gui::button::Button;
use fyrox::gui::message::MessageData;
use fyrox::gui::text::Text;
use std::{
    any::TypeId,
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
    sync::mpsc::Sender,
};
use fyrox::gui::image::Image;

#[derive(Debug, Clone, PartialEq)]
pub enum MaterialFieldMessage {
    Material(MaterialResource),
}
impl MessageData for MaterialFieldMessage {}

#[derive(Clone, Visit, Reflect, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct MaterialFieldEditor {
    widget: Widget,
    #[visit(skip)]
    #[reflect(hidden)]
    sender: MessageSender,
    text: Handle<Text>,
    edit: Handle<Button>,
    make_unique: Handle<Button>,
    material: MaterialResource,
    image: Handle<Image>,
    image_preview: Handle<Image>,
    asset_selector_mixin: AssetSelectorMixin<Material>,
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
            &self.widget.material,
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
                ui.send(
                    self.handle,
                    MaterialFieldMessage::Material(self.material.deep_copy_as_embedded()),
                );
            }
        } else if let Some(MaterialFieldMessage::Material(material)) = message.data_for(self.handle)
        {
            if &self.material != material {
                self.material = material.clone();

                ui.send(
                    self.text,
                    TextMessage::Text(make_name(
                        &self.asset_selector_mixin.resource_manager,
                        &self.material,
                    )),
                );

                self.asset_selector_mixin
                    .request_preview(self.handle, material);

                ui.send_message(message.reverse());
            }
        } else if let Some(WidgetMessage::Drop(dropped)) = message.data() {
            if let Some(item) = ui.node(*dropped).cast::<AssetItem>() {
                if let Some(material) = item.resource::<Material>() {
                    ui.send(self.handle(), MaterialFieldMessage::Material(material));
                }
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
            .handle_ui_message(Some(&self.material), ui, message);
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        self.asset_selector_mixin
            .preview_ui_message(ui, message, |resource| {
                UiMessage::for_widget(
                    self.handle,
                    MaterialFieldMessage::Material(resource.try_cast::<Material>().unwrap()),
                )
            });
    }
}

pub struct MaterialFieldEditorBuilder {
    widget_builder: WidgetBuilder,
}

fn make_name(resource_manager: &ResourceManager, material: &MaterialResource) -> String {
    let resource_uuid = material.resource_uuid();
    let header = material.header();
    match header.state {
        ResourceState::Ok { .. } => {
            if let Some(path) = resource_manager
                .state()
                .resource_registry
                .safe_lock()
                .uuid_to_path_buf(resource_uuid)
            {
                format!(
                    "{} - {} uses; id - {}",
                    path.display(),
                    material.use_count(),
                    material.key()
                )
            } else {
                format!(
                    "Embedded - {} uses; id - {}",
                    material.use_count(),
                    material.key()
                )
            }
        }
        ResourceState::Unloaded => "Material not loading".into(),
        ResourceState::LoadError { ref error, .. } => {
            format!("Loading failed: {error:?}")
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
        icon_request_sender: Sender<IconRequest>,
        resource_manager: ResourceManager,
    ) -> Handle<UiNode> {
        let edit;
        let text;
        let select;
        let make_unique;
        let make_unique_tooltip = "Creates a deep copy of the material, making a separate version of the material. \
        Useful when you need to change some properties in the material, but only on some nodes that uses the material.";

        let buttons = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .with_child({
                    select = make_pick_button(0, ctx);
                    select
                })
                .with_child({
                    edit = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .with_width(40.0)
                            .with_margin(Thickness::uniform(1.0))
                            .on_column(1),
                    )
                    .with_text("Edit...")
                    .build(ctx);
                    edit
                })
                .with_child({
                    make_unique = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .on_column(2)
                            .with_tooltip(make_simple_tooltip(ctx, make_unique_tooltip)),
                    )
                    .with_text("Make Unique")
                    .build(ctx);
                    make_unique
                }),
        )
        .add_row(Row::strict(20.0))
        .add_column(Column::auto())
        .add_column(Column::auto())
        .add_column(Column::auto())
        .build(ctx);

        let (image_preview_tooltip, image_preview) = make_asset_preview_tooltip(None, ctx);

        let image = ImageBuilder::new(
            WidgetBuilder::new()
                .on_column(0)
                .with_width(52.0)
                .with_height(52.0)
                .with_tooltip(image_preview_tooltip)
                .with_margin(Thickness::uniform(1.0)),
        )
        .build(ctx);

        let content = GridBuilder::new(
            WidgetBuilder::new()
                .on_column(1)
                .with_child({
                    text =
                        TextBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
                            .with_text(make_name(&resource_manager, &material))
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ctx);
                    text
                })
                .with_child(buttons),
        )
        .add_row(Row::auto())
        .add_row(Row::auto())
        .add_column(Column::auto())
        .build(ctx);

        let editor = MaterialFieldEditor {
            widget: self
                .widget_builder
                .with_preview_messages(true)
                .with_allow_drop(true)
                .with_child(
                    GridBuilder::new(WidgetBuilder::new().with_child(image).with_child(content))
                        .add_column(Column::auto())
                        .add_column(Column::stretch())
                        .add_row(Row::auto())
                        .build(ctx),
                )
                .build(ctx),
            edit,
            sender,
            material: material.clone(),
            text,
            make_unique,
            asset_selector_mixin: AssetSelectorMixin::new(
                select,
                icon_request_sender.clone(),
                resource_manager,
            ),
            image,
            image_preview,
        };

        let handle = ctx.add_node(UiNode::new(editor));

        Log::verify(icon_request_sender.send(IconRequest {
            widget_handle: handle,
            resource: material.into_untyped(),
            force_update: false,
        }));

        handle
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
        let environment = EditorEnvironment::try_get_from(&ctx.environment)?;
        Ok(PropertyEditorInstance::Simple {
            editor: MaterialFieldEditorBuilder::new(WidgetBuilder::new()).build(
                ctx.build_context,
                self.sender.safe_lock().clone(),
                value.clone(),
                environment.icon_request_sender.clone(),
                environment.resource_manager.clone(),
            ),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<MaterialResource>()?;
        Ok(Some(UiMessage::for_widget(
            ctx.instance,
            MaterialFieldMessage::Material(value.clone()),
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(MaterialFieldMessage::Material(value)) = ctx.message.data() {
                return Some(PropertyChanged {
                    name: ctx.name.to_string(),
                    value: FieldKind::object(value.clone()),
                });
            }
        }
        None
    }
}

#[cfg(test)]
mod test {
    use crate::plugins::material::editor::MaterialFieldEditorBuilder;
    use fyrox::asset::io::FsResourceIo;
    use fyrox::asset::manager::ResourceManager;
    use fyrox::core::task::TaskPool;
    use fyrox::{gui::test::test_widget_deletion, gui::widget::WidgetBuilder};
    use std::sync::mpsc::channel;
    use std::sync::Arc;

    #[test]
    fn test_deletion() {
        let resource_manager =
            ResourceManager::new(Arc::new(FsResourceIo), Arc::new(TaskPool::new()));
        let (sender, _) = channel();
        test_widget_deletion(|ctx| {
            MaterialFieldEditorBuilder::new(WidgetBuilder::new()).build(
                ctx,
                Default::default(),
                Default::default(),
                sender,
                resource_manager,
            )
        });
    }
}
