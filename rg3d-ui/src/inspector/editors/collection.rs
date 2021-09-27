use crate::{
    border::BorderBuilder,
    brush::Brush,
    button::ButtonBuilder,
    core::{
        color::Color,
        inspect::{Inspect, PropertyInfo},
        pool::Handle,
    },
    expander::ExpanderBuilder,
    grid::{Column, GridBuilder, Row},
    inspector::{
        editors::{
            Layout, PropertyEditorBuildContext, PropertyEditorDefinition,
            PropertyEditorDefinitionContainer,
        },
        InspectorBuilder, InspectorContext, InspectorEnvironment, InspectorError,
    },
    message::{
        ButtonMessage, CollectionChanged, FieldKind, InspectorMessage, MessageDirection,
        PropertyChanged, UiMessage, UiMessageData,
    },
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
    sync::Arc,
};

#[derive(Clone, Debug)]
struct Item {
    inspector: Handle<UiNode>,
    remove: Handle<UiNode>,
}

#[derive(Clone, Debug)]
pub struct CollectionEditor {
    widget: Widget,
    add: Handle<UiNode>,
    items: Vec<Item>,
}

crate::define_widget_deref!(CollectionEditor);

impl Control for CollectionEditor {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn clone_boxed(&self) -> Box<dyn Control> {
        Box::new(self.clone())
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        match message.data() {
            UiMessageData::Inspector(InspectorMessage::PropertyChanged(p)) => {
                if let Some(index) = self
                    .items
                    .iter()
                    .position(|i| i.inspector == message.destination())
                {
                    ui.send_message(UiMessage::user(
                        self.handle,
                        MessageDirection::FromWidget,
                        Box::new(CollectionChanged::ItemChanged {
                            index,
                            property: p.clone(),
                        }),
                    ))
                }
            }
            UiMessageData::Button(ButtonMessage::Click) => {
                if let Some(index) = self
                    .items
                    .iter()
                    .position(|i| i.remove == message.destination())
                {
                    ui.send_message(UiMessage::user(
                        self.handle,
                        MessageDirection::FromWidget,
                        Box::new(CollectionChanged::Remove(index)),
                    ));
                }
            }
            _ => {}
        }
    }
}

pub struct CollectionEditorBuilder<'a, T, I>
where
    T: Inspect + 'static,
    I: IntoIterator<Item = &'a T>,
{
    widget_builder: WidgetBuilder,
    collection: Option<I>,
    environment: Option<Arc<dyn InspectorEnvironment>>,
    definition_container: Option<Arc<PropertyEditorDefinitionContainer>>,
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
                                GridBuilder::new(
                                    WidgetBuilder::new()
                                        .with_child(
                                            TextBuilder::new(WidgetBuilder::new())
                                                .with_vertical_text_alignment(
                                                    VerticalAlignment::Center,
                                                )
                                                .with_text(format!("Item {}", n))
                                                .build(ctx),
                                        )
                                        .with_child(item.remove),
                                )
                                .add_column(Column::stretch())
                                .add_column(Column::strict(16.0))
                                .add_row(Row::strict(26.0))
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

impl<'a, T, I> CollectionEditorBuilder<'a, T, I>
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

    pub fn with_definition_container(
        mut self,
        definition_container: Arc<PropertyEditorDefinitionContainer>,
    ) -> Self {
        self.definition_container = Some(definition_container);
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let definition_container = self
            .definition_container
            .unwrap_or_else(|| Arc::new(PropertyEditorDefinitionContainer::new()));

        let environment = self.environment;
        let items = self
            .collection
            .map(|collection| {
                collection
                    .into_iter()
                    .map(|entry| {
                        let inspector_context = InspectorContext::from_object(
                            entry,
                            ctx,
                            definition_container.clone(),
                            environment.clone(),
                        );

                        let inspector = InspectorBuilder::new(WidgetBuilder::new())
                            .with_context(inspector_context)
                            .with_property_editor_definitions(definition_container.clone())
                            .build(ctx);

                        let remove = ButtonBuilder::new(
                            WidgetBuilder::new()
                                .with_margin(Thickness::uniform(1.0))
                                .with_vertical_alignment(VerticalAlignment::Center)
                                .on_column(1)
                                .with_width(16.0)
                                .with_height(16.0),
                        )
                        .with_text("x")
                        .build(ctx);

                        Item { inspector, remove }
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let add;
        let ce = CollectionEditor {
            widget: self
                .widget_builder
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .with_child({
                                add = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_margin(Thickness::uniform(1.0))
                                        .on_row(0),
                                )
                                .with_text("Add New")
                                .build(ctx);
                                add
                            })
                            .with_child(
                                StackPanelBuilder::new(
                                    WidgetBuilder::new()
                                        .on_row(1)
                                        .with_children(create_item_views(&items, ctx)),
                                )
                                .build(ctx),
                            ),
                    )
                    .add_row(Row::strict(26.0))
                    .add_row(Row::stretch())
                    .add_column(Column::stretch())
                    .build(ctx),
                )
                .build(),
            add,
            items,
        };

        ctx.add_node(UiNode::new(ce))
    }
}

#[derive(Debug)]
pub struct VecCollectionPropertyEditorDefinition<T>
where
    T: Inspect + Debug + Send + Sync + 'static,
{
    phantom: PhantomData<T>,
}

impl<T> VecCollectionPropertyEditorDefinition<T>
where
    T: Inspect + Debug + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            phantom: PhantomData::default(),
        }
    }
}

impl<T> PropertyEditorDefinition for VecCollectionPropertyEditorDefinition<T>
where
    T: Inspect + Debug + Send + Sync + 'static,
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Vec<T>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<Handle<UiNode>, InspectorError> {
        let value = ctx.property_info.cast_value::<Vec<T>>()?;
        Ok(
            CollectionEditorBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
                .with_collection(value.iter())
                .with_environment(ctx.environment.clone())
                .with_definition_container(ctx.definition_container.clone())
                .build(ctx.build_context),
        )
    }

    fn create_message(
        &self,
        _instance: Handle<UiNode>,
        _property_info: &PropertyInfo,
    ) -> Result<UiMessage, InspectorError> {
        Err(InspectorError::OutOfSync)
    }

    fn translate_message(
        &self,
        name: &str,
        owner_type_id: TypeId,
        message: &UiMessage,
    ) -> Option<PropertyChanged> {
        if message.direction() == MessageDirection::FromWidget {
            if let UiMessageData::User(msg) = message.data() {
                if let Some(collection_changed) = msg.cast::<CollectionChanged>() {
                    return Some(PropertyChanged {
                        name: name.to_string(),
                        owner_type_id,
                        value: FieldKind::Collection(Arc::new(collection_changed.clone())),
                    });
                }
            }
        }
        None
    }

    fn layout(&self) -> Layout {
        Layout::Vertical
    }
}
