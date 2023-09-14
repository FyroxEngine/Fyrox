use crate::{
    asset::item::AssetItem, inspector::EditorEnvironment, load_image, message::MessageSender,
    Message,
};
use fyrox::{
    asset::{manager::ResourceManager, Resource, ResourceData, ResourceLoadError},
    core::{make_relative_path, pool::Handle, TypeUuidProvider},
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        define_constructor,
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
        BuildContext, Control, UiNode, UserInterface, VerticalAlignment,
    },
};
use std::{
    any::{Any, TypeId},
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
    path::Path,
    rc::Rc,
    sync::Arc,
};

fn resource_path<T>(resource: &Option<Resource<T>>) -> String
where
    T: ResourceData + TypeUuidProvider,
{
    resource
        .as_ref()
        .map(|m| m.path().to_string_lossy().to_string())
        .unwrap_or_else(|| "None".to_string())
}

#[derive(Debug)]
pub enum ResourceFieldMessage<T>
where
    T: ResourceData + TypeUuidProvider,
{
    Value(Option<Resource<T>>),
}

impl<T> PartialEq for ResourceFieldMessage<T>
where
    T: ResourceData + TypeUuidProvider,
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
    T: ResourceData + TypeUuidProvider,
{
    define_constructor!(Self:Value => fn value(Option<Resource<T>>), layout: false);
}

pub type ResourceLoaderCallback<T> =
    Rc<dyn Fn(&ResourceManager, &Path) -> Result<Resource<T>, Option<Arc<dyn ResourceLoadError>>>>;

pub struct ResourceField<T>
where
    T: ResourceData + TypeUuidProvider,
{
    widget: Widget,
    name: Handle<UiNode>,
    resource_manager: ResourceManager,
    resource: Option<Resource<T>>,
    loader: ResourceLoaderCallback<T>,
    locate: Handle<UiNode>,
    sender: MessageSender,
}

impl<T> Debug for ResourceField<T>
where
    T: ResourceData + TypeUuidProvider,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "ResourceField")
    }
}

impl<T> Clone for ResourceField<T>
where
    T: ResourceData + TypeUuidProvider,
{
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.clone(),
            name: self.name,
            resource_manager: self.resource_manager.clone(),
            resource: self.resource.clone(),
            loader: self.loader.clone(),
            locate: self.locate,
            sender: self.sender.clone(),
        }
    }
}

impl<T> Deref for ResourceField<T>
where
    T: ResourceData + TypeUuidProvider,
{
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<T> DerefMut for ResourceField<T>
where
    T: ResourceData + TypeUuidProvider,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<T> Control for ResourceField<T>
where
    T: ResourceData + TypeUuidProvider,
{
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
            if message.destination() == self.handle() {
                if let Some(item) = ui.node(*dropped).cast::<AssetItem>() {
                    if let Ok(relative_path) = make_relative_path(&item.path) {
                        if let Ok(value) =
                            (self.loader)(&self.resource_manager, relative_path.as_path())
                        {
                            ui.send_message(ResourceFieldMessage::value(
                                self.handle(),
                                MessageDirection::ToWidget,
                                Some(value),
                            ));
                        }
                    }
                }
            }
        } else if let Some(ResourceFieldMessage::Value(resource)) = message.data() {
            if &self.resource != resource
                && message.destination() == self.handle()
                && message.direction() == MessageDirection::ToWidget
            {
                self.resource = resource.clone();

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
                    self.sender
                        .send(Message::ShowInAssetBrowser(resource.path()));
                }
            }
        }
    }
}

pub struct ResourceFieldBuilder<T>
where
    T: ResourceData + TypeUuidProvider,
{
    widget_builder: WidgetBuilder,
    resource: Option<Resource<T>>,
    loader: ResourceLoaderCallback<T>,
    sender: MessageSender,
}

impl<T> ResourceFieldBuilder<T>
where
    T: ResourceData + TypeUuidProvider,
{
    pub fn new(
        widget_builder: WidgetBuilder,
        loader: ResourceLoaderCallback<T>,
        sender: MessageSender,
    ) -> Self {
        Self {
            widget_builder,
            resource: None,
            loader,
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
                                        .with_height(16.0),
                                )
                                .with_opt_texture(load_image(include_bytes!(
                                    "../../../resources/embed/sound_source.png"
                                )))
                                .build(ctx),
                            )
                            .with_child({
                                name = TextBuilder::new(WidgetBuilder::new().on_column(1))
                                    .with_text(resource_path(&self.resource))
                                    .with_vertical_text_alignment(VerticalAlignment::Center)
                                    .build(ctx);
                                name
                            })
                            .with_child({
                                locate = ButtonBuilder::new(
                                    WidgetBuilder::new().with_width(24.0).on_column(2),
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
            loader: self.loader,
            locate,
            sender: self.sender,
        };

        ctx.add_node(UiNode::new(field))
    }
}

pub struct ResourceFieldPropertyEditorDefinition<T>
where
    T: ResourceData + TypeUuidProvider,
{
    loader: ResourceLoaderCallback<T>,
    sender: MessageSender,
}

impl<T> ResourceFieldPropertyEditorDefinition<T>
where
    T: ResourceData + TypeUuidProvider,
{
    pub fn new(loader: ResourceLoaderCallback<T>, sender: MessageSender) -> Self {
        Self { loader, sender }
    }
}

impl<T> Debug for ResourceFieldPropertyEditorDefinition<T>
where
    T: ResourceData + TypeUuidProvider,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "ResourceFieldPropertyEditorDefinition")
    }
}

impl<T> PropertyEditorDefinition for ResourceFieldPropertyEditorDefinition<T>
where
    T: ResourceData + TypeUuidProvider,
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
            editor: ResourceFieldBuilder::new(
                WidgetBuilder::new(),
                self.loader.clone(),
                self.sender.clone(),
            )
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
