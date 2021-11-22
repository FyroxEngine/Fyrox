use crate::{
    border::BorderBuilder,
    brush::Brush,
    core::{color::Color, inspect::Inspect, pool::Handle},
    expander::ExpanderBuilder,
    inspector::{
        editors::{
            Layout, PropertyEditorBuildContext, PropertyEditorDefinition,
            PropertyEditorDefinitionContainer, PropertyEditorInstance,
            PropertyEditorMessageContext,
        },
        CollectionChanged, FieldKind, Inspector, InspectorBuilder, InspectorContext,
        InspectorEnvironment, InspectorError, InspectorMessage, PropertyChanged, HEADER_MARGIN,
    },
    message::{MessageDirection, UiMessage},
    stack_panel::StackPanelBuilder,
    text::TextBuilder,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, Thickness, UiNode, UserInterface, VerticalAlignment,
};
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    rc::Rc,
};

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    inspector: Handle<UiNode>,
}

#[derive(Clone, Debug)]
pub struct ArrayEditor {
    widget: Widget,
    items: Vec<Item>,
    panel: Handle<UiNode>,
}

crate::define_widget_deref!(ArrayEditor);

impl Control for ArrayEditor {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(InspectorMessage::PropertyChanged(p)) = message.data::<InspectorMessage>() {
            if let Some(index) = self
                .items
                .iter()
                .position(|i| i.inspector == message.destination())
            {
                ui.send_message(CollectionChanged::item_changed(
                    self.handle,
                    MessageDirection::FromWidget,
                    index,
                    p.clone(),
                ))
            }
        }
    }
}

pub struct ArrayEditorBuilder<'a, T, I>
where
    T: Inspect + 'static,
    I: IntoIterator<Item = &'a T>,
{
    widget_builder: WidgetBuilder,
    collection: Option<I>,
    environment: Option<Rc<dyn InspectorEnvironment>>,
    definition_container: Option<Rc<PropertyEditorDefinitionContainer>>,
    layer_index: usize,
}

fn create_item_views(items: &[Item], ctx: &mut BuildContext) -> Vec<Handle<UiNode>> {
    items
        .iter()
        .enumerate()
        .map(|(n, item)| {
            BorderBuilder::new(
                WidgetBuilder::new()
                    .with_child(
                        ExpanderBuilder::new(WidgetBuilder::new())
                            .with_header(
                                TextBuilder::new(WidgetBuilder::new())
                                    .with_vertical_text_alignment(VerticalAlignment::Center)
                                    .with_text(format!("Item {}", n))
                                    .build(ctx),
                            )
                            .with_content(item.inspector)
                            .build(ctx),
                    )
                    .with_foreground(Brush::Solid(Color::opaque(130, 130, 130))),
            )
            .build(ctx)
        })
        .collect::<Vec<_>>()
}

fn create_items<'a, T, I>(
    iter: I,
    environment: Option<Rc<dyn InspectorEnvironment>>,
    definition_container: Rc<PropertyEditorDefinitionContainer>,
    ctx: &mut BuildContext,
    sync_flag: u64,
    layer_index: usize,
) -> Vec<Item>
where
    T: Inspect + 'static,
    I: IntoIterator<Item = &'a T>,
{
    iter.into_iter()
        .map(|entry| {
            let inspector_context = InspectorContext::from_object(
                entry,
                ctx,
                definition_container.clone(),
                environment.clone(),
                sync_flag,
                true,
                layer_index,
            );

            let inspector = InspectorBuilder::new(WidgetBuilder::new())
                .with_context(inspector_context)
                .build(ctx);

            Item { inspector }
        })
        .collect::<Vec<_>>()
}

impl<'a, T, I> ArrayEditorBuilder<'a, T, I>
where
    T: Inspect + 'static,
    I: IntoIterator<Item = &'a T>,
{
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            collection: None,
            environment: None,
            definition_container: None,
            layer_index: 0,
        }
    }

    pub fn with_collection(mut self, collection: I) -> Self {
        self.collection = Some(collection);
        self
    }

    pub fn with_environment(mut self, environment: Option<Rc<dyn InspectorEnvironment>>) -> Self {
        self.environment = environment;
        self
    }

    pub fn with_definition_container(
        mut self,
        definition_container: Rc<PropertyEditorDefinitionContainer>,
    ) -> Self {
        self.definition_container = Some(definition_container);
        self
    }

    pub fn with_layer_index(mut self, layer_index: usize) -> Self {
        self.layer_index = layer_index;
        self
    }

    pub fn build(self, ctx: &mut BuildContext, sync_flag: u64) -> Handle<UiNode> {
        let definition_container = self
            .definition_container
            .unwrap_or_else(|| Rc::new(PropertyEditorDefinitionContainer::new()));

        let environment = self.environment;
        let items = self
            .collection
            .map(|collection| {
                create_items(
                    collection,
                    environment,
                    definition_container,
                    ctx,
                    sync_flag,
                    self.layer_index,
                )
            })
            .unwrap_or_default();

        let panel = StackPanelBuilder::new(
            WidgetBuilder::new().with_children(create_item_views(&items, ctx)),
        )
        .build(ctx);

        let ce = ArrayEditor {
            widget: self.widget_builder.with_child(panel).build(),
            items,
            panel,
        };

        ctx.add_node(UiNode::new(ce))
    }
}

#[derive(Debug)]
pub struct ArrayPropertyEditorDefinition<T, const N: usize>
where
    T: Inspect + Debug + 'static,
{
    phantom: PhantomData<T>,
}

impl<T, const N: usize> ArrayPropertyEditorDefinition<T, N>
where
    T: Inspect + Debug + 'static,
{
    pub fn new() -> Self {
        Self::default()
    }
}

impl<T, const N: usize> Default for ArrayPropertyEditorDefinition<T, N>
where
    T: Inspect + Debug + 'static,
{
    fn default() -> Self {
        Self {
            phantom: PhantomData::default(),
        }
    }
}

impl<T, const N: usize> PropertyEditorDefinition for ArrayPropertyEditorDefinition<T, N>
where
    T: Inspect + Debug + 'static,
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<[T; N]>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<[T; N]>()?;

        Ok(PropertyEditorInstance {
            title: TextBuilder::new(WidgetBuilder::new().with_margin(HEADER_MARGIN))
                .with_text(ctx.property_info.display_name)
                .with_vertical_text_alignment(VerticalAlignment::Center)
                .build(ctx.build_context),
            editor: ArrayEditorBuilder::new(
                WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
            )
            .with_collection(value.iter())
            .with_environment(ctx.environment.clone())
            .with_layer_index(ctx.layer_index + 1)
            .with_definition_container(ctx.definition_container.clone())
            .build(ctx.build_context, ctx.sync_flag),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let PropertyEditorMessageContext {
            instance,
            ui,
            property_info,
            ..
        } = ctx;

        let instance_ref = if let Some(instance) = ui.node(instance).cast::<ArrayEditor>() {
            instance
        } else {
            return Err(InspectorError::Custom(
                "Property editor is not ArrayEditor!".to_string(),
            ));
        };

        let value = property_info.cast_value::<[T; N]>()?;

        let mut error_group = Vec::new();

        // Just sync inspector of every item.
        for (item, obj) in instance_ref.items.clone().iter().zip(value.iter()) {
            let layer_index = ctx.layer_index;
            let ctx = ui
                .node(item.inspector)
                .cast::<Inspector>()
                .expect("Must be Inspector!")
                .context()
                .clone();
            if let Err(e) = ctx.sync(obj, ui, layer_index + 1) {
                error_group.extend(e.into_iter())
            }
        }

        if error_group.is_empty() {
            Ok(None)
        } else {
            Err(InspectorError::Group(error_group))
        }
    }

    fn translate_message(
        &self,
        name: &str,
        owner_type_id: TypeId,
        message: &UiMessage,
    ) -> Option<PropertyChanged> {
        if message.direction() == MessageDirection::FromWidget {
            if let Some(collection_changed) = message.data::<CollectionChanged>() {
                return Some(PropertyChanged {
                    name: name.to_string(),
                    owner_type_id,
                    value: FieldKind::Collection(Box::new(collection_changed.clone())),
                });
            }
        }
        None
    }

    fn layout(&self) -> Layout {
        Layout::Vertical
    }
}
