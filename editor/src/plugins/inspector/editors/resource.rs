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
        item::AssetItem,
        preview::cache::IconRequest,
        selector::{AssetSelectorMessage, AssetSelectorWindowBuilder},
    },
    fyrox::{
        asset::{manager::ResourceManager, state::LoadError, Resource, TypedResourceData},
        core::{
            color::Color, parking_lot::Mutex, pool::Handle, reflect::prelude::*,
            type_traits::prelude::*, uuid::uuid, visitor::prelude::*, PhantomDataSendSync,
        },
        graph::BaseSceneGraph,
        gui::{
            brush::Brush,
            button::{ButtonBuilder, ButtonMessage},
            define_constructor,
            draw::{CommandTexture, Draw, DrawingContext},
            grid::{Column, GridBuilder, Row},
            image::ImageBuilder,
            inspector::{
                editors::{
                    PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
                    PropertyEditorMessageContext, PropertyEditorTranslationContext,
                },
                FieldKind, InspectorError, PropertyChanged,
            },
            message::{MessageDirection, UiMessage},
            text::{TextBuilder, TextMessage},
            widget::{Widget, WidgetBuilder, WidgetMessage},
            window::WindowMessage,
            BuildContext, Control, Thickness, UiNode, UserInterface, VerticalAlignment,
        },
    },
    load_image,
    message::MessageSender,
    plugins::inspector::EditorEnvironment,
    utils, Message,
};
use std::{
    any::TypeId,
    cell::Cell,
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
    path::Path,
    sync::{mpsc::Sender, Arc},
};

fn resource_path<T>(resource_manager: &ResourceManager, resource: &Option<Resource<T>>) -> String
where
    T: TypedResourceData,
{
    resource
        .as_ref()
        .and_then(|m| {
            resource_manager
                .resource_path(m.as_ref())
                .map(|p| p.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "None".to_string())
}

#[derive(Debug)]
pub enum ResourceFieldMessage<T>
where
    T: TypedResourceData,
{
    Value(Option<Resource<T>>),
}

impl<T: TypedResourceData> Clone for ResourceFieldMessage<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Value(value) => Self::Value(value.clone()),
        }
    }
}

impl<T> PartialEq for ResourceFieldMessage<T>
where
    T: TypedResourceData,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ResourceFieldMessage::Value(left), ResourceFieldMessage::Value(right)) => {
                left == right
            }
        }
    }
}

impl<T> ResourceFieldMessage<T>
where
    T: TypedResourceData,
{
    define_constructor!(ResourceFieldMessage:Value => fn value(Option<Resource<T>>), layout: false);
}

pub type ResourceLoaderCallback<T> = Arc<
    Mutex<
        dyn for<'a> Fn(&'a ResourceManager, &'a Path) -> Option<Result<Resource<T>, LoadError>>
            + Send,
    >,
>;

#[derive(Visit, Reflect, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct ResourceField<T>
where
    T: TypedResourceData,
{
    widget: Widget,
    name: Handle<UiNode>,
    select: Handle<UiNode>,
    selector: Cell<Handle<UiNode>>,
    #[visit(skip)]
    #[reflect(hidden)]
    resource_manager: ResourceManager,
    #[visit(skip)]
    #[reflect(hidden)]
    icon_request_sender: Sender<IconRequest>,
    #[visit(skip)]
    #[reflect(hidden)]
    resource: Option<Resource<T>>,
    locate: Handle<UiNode>,
    #[visit(skip)]
    #[reflect(hidden)]
    sender: MessageSender,
}

impl<T> Debug for ResourceField<T>
where
    T: TypedResourceData,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "ResourceField")
    }
}

impl<T> Clone for ResourceField<T>
where
    T: TypedResourceData,
{
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.clone(),
            name: self.name,
            select: self.select,
            selector: self.selector.clone(),
            resource_manager: self.resource_manager.clone(),
            icon_request_sender: self.icon_request_sender.clone(),
            resource: self.resource.clone(),
            locate: self.locate,
            sender: self.sender.clone(),
        }
    }
}

impl<T> Deref for ResourceField<T>
where
    T: TypedResourceData,
{
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<T> DerefMut for ResourceField<T>
where
    T: TypedResourceData,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<T: TypedResourceData> TypeUuidProvider for ResourceField<T> {
    fn type_uuid() -> Uuid {
        uuid!("5179b3b9-855f-43a6-b23a-831129fee1cf")
    }
}

impl<T> Control for ResourceField<T>
where
    T: TypedResourceData,
{
    fn draw(&self, drawing_context: &mut DrawingContext) {
        // Emit transparent geometry for the field to be able to catch mouse events without precise pointing at the
        // node name letters.
        drawing_context.push_rect_filled(&self.bounding_rect(), None);
        drawing_context.commit(
            self.clip_bounds(),
            Brush::Solid(Color::TRANSPARENT),
            CommandTexture::None,
            &self.material,
            None,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(WidgetMessage::Drop(dropped)) = message.data::<WidgetMessage>() {
            if message.destination() == self.handle() {
                if let Some(item) = ui.node(*dropped).cast::<AssetItem>() {
                    if let Some(value) = item.resource::<T>() {
                        ui.send_message(ResourceFieldMessage::value(
                            self.handle(),
                            MessageDirection::ToWidget,
                            Some(value),
                        ));
                    }
                }
            }
        } else if let Some(ResourceFieldMessage::Value(resource)) = message.data() {
            if &self.resource != resource
                && message.destination() == self.handle()
                && message.direction() == MessageDirection::ToWidget
            {
                self.resource.clone_from(resource);

                ui.send_message(TextMessage::text(
                    self.name,
                    MessageDirection::ToWidget,
                    resource_path(&self.resource_manager, resource),
                ));

                ui.send_message(message.reverse());
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.locate {
                if let Some(resource) = self.resource.as_ref() {
                    if let Some(path) = self.resource_manager.resource_path(resource.as_ref()) {
                        self.sender.send(Message::ShowInAssetBrowser(path));
                    }
                }
            } else if message.destination() == self.select {
                self.selector
                    .set(AssetSelectorWindowBuilder::build_for_type_and_open::<T>(
                        self.icon_request_sender.clone(),
                        self.resource_manager.clone(),
                        ui,
                    ));
            }
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        if message.destination() == self.selector.get() {
            if let Some(WindowMessage::Close) = message.data() {
                self.selector.set(Handle::NONE);
            } else if let Some(AssetSelectorMessage::Select(resource)) = message.data() {
                ui.send_message(ResourceFieldMessage::value(
                    self.handle,
                    MessageDirection::ToWidget,
                    resource.try_cast::<T>(),
                ))
            }
        }
    }
}

pub struct ResourceFieldBuilder<T>
where
    T: TypedResourceData,
{
    widget_builder: WidgetBuilder,
    resource: Option<Resource<T>>,
    sender: MessageSender,
}

impl<T> ResourceFieldBuilder<T>
where
    T: TypedResourceData,
{
    pub fn new(widget_builder: WidgetBuilder, sender: MessageSender) -> Self {
        Self {
            widget_builder,
            resource: None,
            sender,
        }
    }

    pub fn with_resource(mut self, resource: Option<Resource<T>>) -> Self {
        self.resource = resource;
        self
    }

    pub fn build(
        self,
        ctx: &mut BuildContext,
        icon_request_sender: Sender<IconRequest>,
        resource_manager: ResourceManager,
    ) -> Handle<UiNode> {
        let name;
        let locate;
        let select;
        let field = ResourceField {
            widget: self
                .widget_builder
                .with_preview_messages(true)
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .with_child(
                                ImageBuilder::new(
                                    WidgetBuilder::new()
                                        .on_column(0)
                                        .with_width(16.0)
                                        .with_height(16.0)
                                        .with_margin(Thickness::uniform(1.0)),
                                )
                                .with_opt_texture(load_image!(
                                    "../../../../resources/sound_source.png"
                                ))
                                .build(ctx),
                            )
                            .with_child({
                                name = TextBuilder::new(
                                    WidgetBuilder::new()
                                        .on_column(1)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_vertical_alignment(VerticalAlignment::Center),
                                )
                                .with_text(resource_path(&resource_manager, &self.resource))
                                .build(ctx);
                                name
                            })
                            .with_child({
                                locate = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_width(24.0)
                                        .on_column(2)
                                        .with_margin(Thickness::uniform(1.0)),
                                )
                                .with_text("<<")
                                .build(ctx);
                                locate
                            })
                            .with_child({
                                select = utils::make_pick_button(3, ctx);
                                select
                            }),
                    )
                    .add_column(Column::auto())
                    .add_column(Column::stretch())
                    .add_column(Column::auto())
                    .add_column(Column::auto())
                    .add_row(Row::auto())
                    .build(ctx),
                )
                .with_allow_drop(true)
                .build(ctx),
            name,
            select,
            selector: Default::default(),
            resource_manager,
            icon_request_sender,
            resource: self.resource,
            locate,
            sender: self.sender,
        };

        ctx.add_node(UiNode::new(field))
    }
}

pub struct ResourceFieldPropertyEditorDefinition<T>
where
    T: TypedResourceData,
{
    sender: MessageSender,
    #[allow(dead_code)]
    phantom: PhantomDataSendSync<T>,
}

impl<T> ResourceFieldPropertyEditorDefinition<T>
where
    T: TypedResourceData,
{
    pub fn new(sender: MessageSender) -> Self {
        Self {
            sender,
            phantom: Default::default(),
        }
    }
}

impl<T> Debug for ResourceFieldPropertyEditorDefinition<T>
where
    T: TypedResourceData,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "ResourceFieldPropertyEditorDefinition")
    }
}

impl<T> PropertyEditorDefinition for ResourceFieldPropertyEditorDefinition<T>
where
    T: TypedResourceData,
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Option<Resource<T>>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<Option<Resource<T>>>()?;
        let environment = EditorEnvironment::try_get_from(&ctx.environment)?;
        Ok(PropertyEditorInstance::Simple {
            editor: ResourceFieldBuilder::new(WidgetBuilder::new(), self.sender.clone())
                .with_resource(value.clone())
                .build(
                    ctx.build_context,
                    environment.icon_request_sender.clone(),
                    environment.resource_manager.clone(),
                ),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<Option<Resource<T>>>()?;

        Ok(Some(ResourceFieldMessage::value(
            ctx.instance,
            MessageDirection::ToWidget,
            value.clone(),
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(ResourceFieldMessage::Value(value)) =
                ctx.message.data::<ResourceFieldMessage<T>>()
            {
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
    use crate::plugins::inspector::editors::resource::ResourceFieldBuilder;
    use fyrox::asset::io::FsResourceIo;
    use fyrox::asset::manager::ResourceManager;
    use fyrox::resource::model::Model;
    use fyrox::{gui::test::test_widget_deletion, gui::widget::WidgetBuilder};
    use std::sync::mpsc::channel;
    use std::sync::Arc;

    #[test]
    fn test_deletion() {
        let (sender, _) = channel();
        test_widget_deletion(|ctx| {
            ResourceFieldBuilder::<Model>::new(WidgetBuilder::new(), Default::default()).build(
                ctx,
                sender,
                ResourceManager::new(Arc::new(FsResourceIo), Default::default()),
            )
        });
    }
}
