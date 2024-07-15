use crate::{
    button::{ButtonBuilder, ButtonMessage},
    core::{
        pool::Handle,
        reflect::{FieldInfo, FieldValue, Reflect},
        type_traits::prelude::*,
        visitor::prelude::*,
        PhantomDataSendSync,
    },
    define_constructor,
    grid::{Column, GridBuilder, Row},
    inspector::{
        editors::{
            PropertyEditorBuildContext, PropertyEditorDefinition,
            PropertyEditorDefinitionContainer, PropertyEditorInstance,
            PropertyEditorMessageContext, PropertyEditorTranslationContext,
        },
        make_expander_container, make_property_margin, CollectionChanged, FieldKind,
        InspectorEnvironment, InspectorError, ObjectValue, PropertyChanged, PropertyFilter,
    },
    message::{MessageDirection, UiMessage},
    stack_panel::StackPanelBuilder,
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, HorizontalAlignment, Thickness, UiNode, UserInterface,
    VerticalAlignment,
};
use fyrox_graph::BaseSceneGraph;
use std::{
    any::TypeId,
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::Arc,
};

#[derive(Clone, Debug, PartialEq, Default, Visit, Reflect)]
pub struct Item {
    editor_instance: PropertyEditorInstance,
    remove: Handle<UiNode>,
}

pub trait CollectionItem:
    Clone + Reflect + Debug + Default + TypeUuidProvider + Send + 'static
{
}

impl<T> CollectionItem for T where
    T: Clone + Reflect + Debug + Default + TypeUuidProvider + Send + 'static
{
}

#[derive(Debug, Visit, Reflect, ComponentProvider)]
pub struct CollectionEditor<T: CollectionItem> {
    pub widget: Widget,
    pub add: Handle<UiNode>,
    pub items: Vec<Item>,
    pub panel: Handle<UiNode>,
    #[visit(skip)]
    #[reflect(hidden)]
    pub layer_index: usize,
    #[reflect(hidden)]
    #[visit(skip)]
    pub phantom: PhantomData<T>,
}

impl<T: CollectionItem> Clone for CollectionEditor<T> {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.clone(),
            add: self.add,
            items: self.items.clone(),
            panel: self.panel,
            layer_index: self.layer_index,
            phantom: PhantomData,
        }
    }
}

impl<T: CollectionItem> Deref for CollectionEditor<T> {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<T: CollectionItem> DerefMut for CollectionEditor<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum CollectionEditorMessage {
    Items(Vec<Item>),
    ItemChanged { index: usize, message: UiMessage },
}

impl CollectionEditorMessage {
    define_constructor!(CollectionEditorMessage:Items => fn items(Vec<Item>), layout: false);
    define_constructor!(CollectionEditorMessage:ItemChanged => fn item_changed(index: usize, message: UiMessage), layout: false);
}

impl<T: CollectionItem> TypeUuidProvider for CollectionEditor<T> {
    fn type_uuid() -> Uuid {
        combine_uuids(
            uuid!("316b0319-f8ee-4b63-9ed9-3f59a857e2bc"),
            T::type_uuid(),
        )
    }
}

impl<T: CollectionItem> Control for CollectionEditor<T> {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if let Some(index) = self
                .items
                .iter()
                .position(|i| i.remove == message.destination())
            {
                ui.send_message(CollectionChanged::remove(
                    self.handle,
                    MessageDirection::FromWidget,
                    index,
                ));
            }
        } else if let Some(msg) = message.data::<CollectionEditorMessage>() {
            if message.destination == self.handle {
                if let CollectionEditorMessage::Items(items) = msg {
                    let views = create_item_views(items, &mut ui.build_ctx());

                    for old_item in ui.node(self.panel).children() {
                        ui.send_message(WidgetMessage::remove(
                            *old_item,
                            MessageDirection::ToWidget,
                        ));
                    }

                    for view in views {
                        ui.send_message(WidgetMessage::link(
                            view,
                            MessageDirection::ToWidget,
                            self.panel,
                        ));
                    }

                    self.items.clone_from(items);
                }
            }
        } else if let Some(index) = self
            .items
            .iter()
            .position(|i| i.editor_instance.editor() == message.destination())
        {
            ui.send_message(CollectionEditorMessage::item_changed(
                self.handle,
                MessageDirection::FromWidget,
                index,
                message.clone(),
            ));
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == self.add {
                ui.send_message(CollectionChanged::add(
                    self.handle,
                    MessageDirection::FromWidget,
                    ObjectValue {
                        value: Box::<T>::default(),
                    },
                ))
            }
        }
    }
}

pub struct CollectionEditorBuilder<'a, T, I>
where
    T: CollectionItem,
    I: IntoIterator<Item = &'a T>,
{
    widget_builder: WidgetBuilder,
    collection: Option<I>,
    environment: Option<Arc<dyn InspectorEnvironment>>,
    definition_container: Option<Arc<PropertyEditorDefinitionContainer>>,
    add: Handle<UiNode>,
    layer_index: usize,
    generate_property_string_values: bool,
    filter: PropertyFilter,
    immutable_collection: bool,
}

fn create_item_views(items: &[Item], ctx: &mut BuildContext) -> Vec<Handle<UiNode>> {
    items
        .iter()
        .map(|item| {
            GridBuilder::new(
                WidgetBuilder::new()
                    .with_child(match item.editor_instance {
                        PropertyEditorInstance::Simple { editor } => editor,
                        PropertyEditorInstance::Custom { container, .. } => container,
                    })
                    .with_child(item.remove),
            )
            .add_row(Row::stretch())
            .add_column(Column::stretch())
            .add_column(Column::auto())
            .build(ctx)
        })
        .collect::<Vec<_>>()
}

fn make_proxy<'a, 'b, T>(
    collection_property_info: &'b FieldInfo<'a, 'b>,
    item: &'a T,
    name: &'b str,
    display_name: &'b str,
) -> Result<FieldInfo<'a, 'b>, InspectorError>
where
    T: Reflect + FieldValue,
    'b: 'a,
{
    Ok(FieldInfo {
        owner_type_id: TypeId::of::<T>(),
        name,
        display_name,
        value: item,
        reflect_value: item,
        read_only: collection_property_info.read_only,
        immutable_collection: collection_property_info.immutable_collection,
        min_value: collection_property_info.min_value,
        max_value: collection_property_info.max_value,
        step: collection_property_info.step,
        precision: collection_property_info.precision,
        description: collection_property_info.description,
        type_name: collection_property_info.type_name,
        doc: collection_property_info.doc,
    })
}

fn create_items<'a, 'b, T, I>(
    iter: I,
    environment: Option<Arc<dyn InspectorEnvironment>>,
    definition_container: Arc<PropertyEditorDefinitionContainer>,
    property_info: &FieldInfo<'a, 'b>,
    ctx: &mut BuildContext,
    sync_flag: u64,
    layer_index: usize,
    generate_property_string_values: bool,
    filter: PropertyFilter,
    immutable_collection: bool,
    name_column_width: f32,
) -> Result<Vec<Item>, InspectorError>
where
    T: CollectionItem,
    I: IntoIterator<Item = &'a T>,
{
    let mut items = Vec::new();

    for (index, item) in iter.into_iter().enumerate() {
        if let Some(definition) = definition_container.definitions().get(&TypeId::of::<T>()) {
            let name = format!("{}[{index}]", property_info.name);
            let display_name = format!("{}[{index}]", property_info.display_name);

            let editor =
                definition
                    .property_editor
                    .create_instance(PropertyEditorBuildContext {
                        build_context: ctx,
                        property_info: &make_proxy::<T>(property_info, item, &name, &display_name)?,
                        environment: environment.clone(),
                        definition_container: definition_container.clone(),
                        sync_flag,
                        layer_index: layer_index + 1,
                        generate_property_string_values,
                        filter: filter.clone(),
                        name_column_width,
                    })?;

            if let PropertyEditorInstance::Simple { editor } = editor {
                ctx[editor].set_margin(make_property_margin(layer_index + 1));
            }

            let remove = ButtonBuilder::new(
                WidgetBuilder::new()
                    .with_visibility(!immutable_collection)
                    .with_margin(Thickness::uniform(1.0))
                    .with_vertical_alignment(VerticalAlignment::Top)
                    .with_horizontal_alignment(HorizontalAlignment::Right)
                    .on_column(1)
                    .with_width(16.0)
                    .with_height(16.0),
            )
            .with_text("-")
            .build(ctx);

            items.push(Item {
                editor_instance: editor,
                remove,
            });
        } else {
            return Err(InspectorError::Custom(format!(
                "Missing property editor of type {}",
                std::any::type_name::<T>()
            )));
        }
    }

    Ok(items)
}

impl<'a, T, I> CollectionEditorBuilder<'a, T, I>
where
    T: CollectionItem,
    I: IntoIterator<Item = &'a T>,
{
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            collection: None,
            environment: None,
            definition_container: None,
            add: Default::default(),
            layer_index: 0,
            generate_property_string_values: false,
            filter: Default::default(),
            immutable_collection: false,
        }
    }

    pub fn with_collection(mut self, collection: I) -> Self {
        self.collection = Some(collection);
        self
    }

    pub fn with_environment(mut self, environment: Option<Arc<dyn InspectorEnvironment>>) -> Self {
        self.environment = environment;
        self
    }

    pub fn with_add(mut self, add: Handle<UiNode>) -> Self {
        self.add = add;
        self
    }

    pub fn with_definition_container(
        mut self,
        definition_container: Arc<PropertyEditorDefinitionContainer>,
    ) -> Self {
        self.definition_container = Some(definition_container);
        self
    }

    pub fn with_layer_index(mut self, layer_index: usize) -> Self {
        self.layer_index = layer_index;
        self
    }

    pub fn with_generate_property_string_values(
        mut self,
        generate_property_string_values: bool,
    ) -> Self {
        self.generate_property_string_values = generate_property_string_values;
        self
    }

    pub fn with_filter(mut self, filter: PropertyFilter) -> Self {
        self.filter = filter;
        self
    }

    pub fn with_immutable_collection(mut self, immutable_collection: bool) -> Self {
        self.immutable_collection = immutable_collection;
        self
    }

    pub fn build(
        self,
        ctx: &mut BuildContext,
        property_info: &FieldInfo<'a, '_>,
        sync_flag: u64,
        name_column_width: f32,
    ) -> Result<Handle<UiNode>, InspectorError> {
        let definition_container = self
            .definition_container
            .unwrap_or_else(|| Arc::new(PropertyEditorDefinitionContainer::with_default_editors()));

        let environment = self.environment;
        let items = if let Some(collection) = self.collection {
            create_items(
                collection,
                environment,
                definition_container,
                property_info,
                ctx,
                sync_flag,
                self.layer_index + 1,
                self.generate_property_string_values,
                self.filter,
                self.immutable_collection,
                name_column_width,
            )?
        } else {
            Vec::new()
        };

        let panel = StackPanelBuilder::new(
            WidgetBuilder::new().with_children(create_item_views(&items, ctx)),
        )
        .build(ctx);

        let ce = CollectionEditor::<T> {
            widget: self
                .widget_builder
                .with_preview_messages(true)
                .with_child(panel)
                .build(),
            add: self.add,
            items,
            panel,
            layer_index: self.layer_index,
            phantom: PhantomData,
        };

        Ok(ctx.add_node(UiNode::new(ce)))
    }
}

#[derive(Debug)]
pub struct VecCollectionPropertyEditorDefinition<T>
where
    T: CollectionItem,
{
    #[allow(dead_code)]
    phantom: PhantomDataSendSync<T>,
}

impl<T> VecCollectionPropertyEditorDefinition<T>
where
    T: CollectionItem,
{
    pub fn new() -> Self {
        Self::default()
    }
}

impl<T> Default for VecCollectionPropertyEditorDefinition<T>
where
    T: CollectionItem,
{
    fn default() -> Self {
        Self {
            phantom: Default::default(),
        }
    }
}

impl<T> PropertyEditorDefinition for VecCollectionPropertyEditorDefinition<T>
where
    T: CollectionItem,
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Vec<T>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<Vec<T>>()?;

        let add = ButtonBuilder::new(
            WidgetBuilder::new()
                .with_visibility(!ctx.property_info.immutable_collection)
                .with_horizontal_alignment(HorizontalAlignment::Right)
                .with_width(16.0)
                .with_height(16.0)
                .on_column(1)
                .with_margin(Thickness::uniform(1.0)),
        )
        .with_text("+")
        .build(ctx.build_context);

        let editor;
        let container = make_expander_container(
            ctx.layer_index,
            ctx.property_info.display_name,
            ctx.property_info.description,
            add,
            {
                editor = CollectionEditorBuilder::new(
                    WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                )
                .with_add(add)
                .with_collection(value.iter())
                .with_environment(ctx.environment.clone())
                .with_layer_index(ctx.layer_index + 1)
                .with_definition_container(ctx.definition_container.clone())
                .with_generate_property_string_values(ctx.generate_property_string_values)
                .with_filter(ctx.filter)
                .with_immutable_collection(ctx.property_info.immutable_collection)
                .build(
                    ctx.build_context,
                    ctx.property_info,
                    ctx.sync_flag,
                    ctx.name_column_width,
                )?;
                editor
            },
            ctx.name_column_width,
            ctx.build_context,
        );

        Ok(PropertyEditorInstance::Custom { container, editor })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let PropertyEditorMessageContext {
            sync_flag,
            instance,
            ui,
            property_info,
            definition_container,
            layer_index,
            environment,
            generate_property_string_values,
            filter,
            name_column_width,
        } = ctx;

        let instance_ref = if let Some(instance) = ui.node(instance).cast::<CollectionEditor<T>>() {
            instance
        } else {
            return Err(InspectorError::Custom(
                "Property editor is not CollectionEditor!".to_string(),
            ));
        };

        let value = property_info.cast_value::<Vec<T>>()?;

        if value.len() != instance_ref.items.len() {
            // Re-create items.
            let items = create_items(
                value.iter(),
                environment,
                definition_container,
                property_info,
                &mut ui.build_ctx(),
                sync_flag,
                layer_index + 1,
                generate_property_string_values,
                filter,
                property_info.immutable_collection,
                name_column_width,
            )?;

            Ok(Some(CollectionEditorMessage::items(
                instance,
                MessageDirection::ToWidget,
                items,
            )))
        } else {
            if let Some(definition) = definition_container.definitions().get(&TypeId::of::<T>()) {
                for (index, (item, obj)) in instance_ref
                    .items
                    .clone()
                    .iter()
                    .zip(value.iter())
                    .enumerate()
                {
                    let name = format!("{}[{index}]", property_info.name);
                    let display_name = format!("{}[{index}]", property_info.display_name);

                    if let Some(message) =
                        definition
                            .property_editor
                            .create_message(PropertyEditorMessageContext {
                                property_info: &make_proxy::<T>(
                                    property_info,
                                    obj,
                                    &name,
                                    &display_name,
                                )?,
                                environment: environment.clone(),
                                definition_container: definition_container.clone(),
                                sync_flag,
                                instance: item.editor_instance.editor(),
                                layer_index: layer_index + 1,
                                ui,
                                generate_property_string_values,
                                filter: filter.clone(),
                                name_column_width,
                            })?
                    {
                        ui.send_message(message.with_flags(ctx.sync_flag))
                    }
                }
            }

            Ok(None)
        }
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(collection_changed) = ctx.message.data::<CollectionChanged>() {
                return Some(PropertyChanged {
                    name: ctx.name.to_string(),
                    owner_type_id: ctx.owner_type_id,
                    value: FieldKind::Collection(Box::new(collection_changed.clone())),
                });
            } else if let Some(CollectionEditorMessage::ItemChanged { index, message }) =
                ctx.message.data()
            {
                if let Some(definition) = ctx
                    .definition_container
                    .definitions()
                    .get(&TypeId::of::<T>())
                {
                    return Some(PropertyChanged {
                        name: ctx.name.to_string(),
                        owner_type_id: ctx.owner_type_id,
                        value: FieldKind::Collection(Box::new(CollectionChanged::ItemChanged {
                            index: *index,
                            property: definition
                                .property_editor
                                .translate_message(PropertyEditorTranslationContext {
                                    environment: ctx.environment.clone(),
                                    name: "",
                                    owner_type_id: ctx.owner_type_id,
                                    message,
                                    definition_container: ctx.definition_container.clone(),
                                })?
                                .value,
                        })),
                    });
                }
            }
        }

        None
    }
}
