use crate::{asset::item::AssetItem, inspector::EditorEnvironment, load_image};
use fyrox::{
    asset::{Resource, ResourceData, ResourceLoadError},
    core::{make_relative_path, pool::Handle},
    engine::resource_manager::ResourceManager,
    gui::{
        define_constructor,
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        inspector::editors::PropertyEditorTranslationContext,
        inspector::{
            editors::{
                PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
                PropertyEditorMessageContext,
            },
            FieldKind, InspectorError, PropertyChanged,
        },
        message::{MessageDirection, UiMessage},
        text::{TextBuilder, TextMessage},
        widget::{Widget, WidgetBuilder, WidgetMessage},
        BuildContext, Control, UiNode, UserInterface, VerticalAlignment,
    },
};
use std::sync::Arc;
use std::{
    any::{Any, TypeId},
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
    path::Path,
    rc::Rc,
};

fn resource_path<T, S, E>(resource: &Option<T>) -> String
where
    T: Deref<Target = Resource<S, E>>,
    S: ResourceData,
    E: ResourceLoadError,
{
    resource
        .as_ref()
        .map(|m| m.state().path().to_string_lossy().to_string())
        .unwrap_or_else(|| "None".to_string())
}

#[derive(Debug)]
pub enum ResourceFieldMessage<T, S, E>
where
    T: Deref<Target = Resource<S, E>>,
    S: ResourceData,
    E: ResourceLoadError,
{
    Value(Option<T>),
}

impl<T, S, E> PartialEq for ResourceFieldMessage<T, S, E>
where
    T: Deref<Target = Resource<S, E>> + PartialEq,
    S: ResourceData,
    E: ResourceLoadError,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ResourceFieldMessage::Value(left), ResourceFieldMessage::Value(right)) => {
                left == right
            }
        }
    }
}

impl<T, S, E> ResourceFieldMessage<T, S, E>
where
    T: Deref<Target = Resource<S, E>> + Debug + PartialEq + 'static,
    S: ResourceData,
    E: ResourceLoadError,
{
    define_constructor!(ResourceFieldMessage:Value => fn value(Option<T>), layout: false);
}

pub type ResourceLoaderCallback<T, E> =
    Rc<dyn Fn(&ResourceManager, &Path) -> Result<T, Option<Arc<E>>>>;

pub struct ResourceField<T, S, E>
where
    T: Deref<Target = Resource<S, E>>,
    S: ResourceData,
    E: ResourceLoadError,
{
    widget: Widget,
    name: Handle<UiNode>,
    resource_manager: ResourceManager,
    resource: Option<T>,
    loader: ResourceLoaderCallback<T, E>,
}

impl<T, S, E> Debug for ResourceField<T, S, E>
where
    T: Deref<Target = Resource<S, E>>,
    S: ResourceData,
    E: ResourceLoadError,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "ResourceField")
    }
}

impl<T, S, E> Clone for ResourceField<T, S, E>
where
    T: Deref<Target = Resource<S, E>> + Clone,
    S: ResourceData,
    E: ResourceLoadError,
{
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.clone(),
            name: self.name,
            resource_manager: self.resource_manager.clone(),
            resource: self.resource.clone(),
            loader: self.loader.clone(),
        }
    }
}

impl<T, S, E> Deref for ResourceField<T, S, E>
where
    T: Deref<Target = Resource<S, E>>,
    S: ResourceData,
    E: ResourceLoadError,
{
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<T, S, E> DerefMut for ResourceField<T, S, E>
where
    T: Deref<Target = Resource<S, E>>,
    S: ResourceData,
    E: ResourceLoadError,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<T, S, E> Control for ResourceField<T, S, E>
where
    T: Deref<Target = Resource<S, E>> + Clone + PartialEq + Debug + 'static,
    S: ResourceData,
    E: ResourceLoadError,
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
                    let relative_path = make_relative_path(&item.path);

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
        }
    }
}

pub struct ResourceFieldBuilder<T, S, E>
where
    T: Deref<Target = Resource<S, E>> + PartialEq,
    S: ResourceData,
    E: ResourceLoadError,
{
    widget_builder: WidgetBuilder,
    resource: Option<T>,
    loader: ResourceLoaderCallback<T, E>,
}

impl<T, S, E> ResourceFieldBuilder<T, S, E>
where
    T: Deref<Target = Resource<S, E>> + PartialEq + Debug + Clone + 'static,
    S: ResourceData,
    E: ResourceLoadError,
{
    pub fn new(widget_builder: WidgetBuilder, loader: ResourceLoaderCallback<T, E>) -> Self {
        Self {
            widget_builder,
            resource: None,
            loader,
        }
    }

    pub fn with_resource(mut self, resource: Option<T>) -> Self {
        self.resource = resource;
        self
    }

    pub fn build(
        self,
        ctx: &mut BuildContext,
        resource_manager: ResourceManager,
    ) -> Handle<UiNode> {
        let name;
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
                            }),
                    )
                    .add_column(Column::auto())
                    .add_column(Column::stretch())
                    .add_row(Row::stretch())
                    .build(ctx),
                )
                .with_allow_drop(true)
                .build(),
            name,
            resource_manager,
            resource: self.resource,
            loader: self.loader,
        };

        ctx.add_node(UiNode::new(field))
    }
}

pub struct ResourceFieldPropertyEditorDefinition<T, S, E>
where
    T: Deref<Target = Resource<S, E>>,
    S: ResourceData,
    E: ResourceLoadError,
{
    loader: ResourceLoaderCallback<T, E>,
}

impl<T, S, E> ResourceFieldPropertyEditorDefinition<T, S, E>
where
    T: Deref<Target = Resource<S, E>>,
    S: ResourceData,
    E: ResourceLoadError,
{
    pub fn new(loader: ResourceLoaderCallback<T, E>) -> Self {
        Self { loader }
    }
}

impl<T, S, E> Debug for ResourceFieldPropertyEditorDefinition<T, S, E>
where
    T: Deref<Target = Resource<S, E>>,
    S: ResourceData,
    E: ResourceLoadError,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "ResourceFieldPropertyEditorDefinition")
    }
}

impl<T, S, E> PropertyEditorDefinition for ResourceFieldPropertyEditorDefinition<T, S, E>
where
    T: Deref<Target = Resource<S, E>> + Clone + PartialEq + Debug + 'static,
    S: ResourceData,
    E: ResourceLoadError,
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Option<T>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<Option<T>>()?;

        Ok(PropertyEditorInstance::Simple {
            editor: ResourceFieldBuilder::new(WidgetBuilder::new(), self.loader.clone())
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
        let value = ctx.property_info.cast_value::<Option<T>>()?;

        Ok(Some(ResourceFieldMessage::value(
            ctx.instance,
            MessageDirection::ToWidget,
            value.clone(),
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(ResourceFieldMessage::Value(value)) =
                ctx.message.data::<ResourceFieldMessage<T, S, E>>()
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
