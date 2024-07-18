use crate::fyrox::graph::BaseSceneGraph;
use crate::fyrox::{
    asset::{manager::ResourceManager, state::LoadError, Resource, TypedResourceData},
    core::{
        color::Color, parking_lot::Mutex, pool::Handle, reflect::prelude::*,
        type_traits::prelude::*, uuid::uuid, visitor::prelude::*,
    },
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
        BuildContext, Control, Thickness, UiNode, UserInterface, VerticalAlignment,
    },
};
use crate::{
    asset::item::AssetItem, inspector::EditorEnvironment, load_image, message::MessageSender,
    Message,
};
use fyrox::core::PhantomDataSendSync;
use std::{
    any::TypeId,
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
    path::Path,
    sync::Arc,
};

fn resource_path<T>(resource: &Option<Resource<T>>) -> String
where
    T: TypedResourceData,
{
    resource
        .as_ref()
        .map(|m| m.kind().to_string())
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
pub struct ResourceField<T>
where
    T: TypedResourceData,
{
    widget: Widget,
    name: Handle<UiNode>,
    #[visit(skip)]
    #[reflect(hidden)]
    resource_manager: ResourceManager,
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
            resource_manager: self.resource_manager.clone(),
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
                    resource_path(resource),
                ));

                ui.send_message(message.reverse());
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.locate {
                if let Some(resource) = self.resource.as_ref() {
                    if let Some(path) = resource.kind().into_path() {
                        self.sender.send(Message::ShowInAssetBrowser(path));
                    }
                }
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
        resource_manager: ResourceManager,
    ) -> Handle<UiNode> {
        let name;
        let locate;
        let field = ResourceField {
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
                                        .with_height(16.0)
                                        .with_margin(Thickness::uniform(1.0)),
                                )
                                .with_opt_texture(load_image(include_bytes!(
                                    "../../../resources/sound_source.png"
                                )))
                                .build(ctx),
                            )
                            .with_child({
                                name = TextBuilder::new(
                                    WidgetBuilder::new()
                                        .on_column(1)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_vertical_alignment(VerticalAlignment::Center),
                                )
                                .with_text(resource_path(&self.resource))
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
                            }),
                    )
                    .add_column(Column::auto())
                    .add_column(Column::stretch())
                    .add_column(Column::auto())
                    .add_row(Row::stretch())
                    .build(ctx),
                )
                .with_allow_drop(true)
                .build(),
            name,
            resource_manager,
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

        Ok(PropertyEditorInstance::Simple {
            editor: ResourceFieldBuilder::new(WidgetBuilder::new(), self.sender.clone())
                .with_resource(value.clone())
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
                    owner_type_id: ctx.owner_type_id,
                    name: ctx.name.to_string(),
                    value: FieldKind::object(value.clone()),
                });
            }
        }
        None
    }
}
