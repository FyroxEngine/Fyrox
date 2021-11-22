use crate::inspector::{make_expander_margin, make_layer_margin};
use crate::{
    button::{ButtonBuilder, ButtonMessage},
    core::{inspect::Inspect, pool::Handle},
    define_constructor,
    expander::ExpanderBuilder,
    grid::{Column, GridBuilder, Row},
    inspector::{
        editors::{
            Layout, PropertyEditorBuildContext, PropertyEditorDefinition,
            PropertyEditorDefinitionContainer, PropertyEditorInstance,
            PropertyEditorMessageContext,
        },
        CollectionChanged, FieldKind, Inspector, InspectorBuilder, InspectorContext,
        InspectorEnvironment, InspectorError, InspectorMessage, PropertyChanged, NAME_COLUMN_WIDTH,
    },
    message::{MessageDirection, UiMessage},
    stack_panel::StackPanelBuilder,
    text::TextBuilder,
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, HorizontalAlignment, Thickness, UiNode, UserInterface,
    VerticalAlignment,
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
    remove: Handle<UiNode>,
}

#[derive(Clone, Debug)]
pub struct CollectionEditor {
    widget: Widget,
    add: Handle<UiNode>,
    items: Vec<Item>,
    panel: Handle<UiNode>,
    layer_index: usize,
}

crate::define_widget_deref!(CollectionEditor);

#[derive(Debug, PartialEq, Clone)]
pub enum CollectionEditorMessage {
    Items(Vec<Item>),
}

impl CollectionEditorMessage {
    define_constructor!(CollectionEditorMessage:Items => fn items(Vec<Item>), layout: false);
}

impl Control for CollectionEditor {
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
        } else if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
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
            match msg {
                CollectionEditorMessage::Items(items) => {
                    let views = create_item_views(items, &mut ui.build_ctx(), self.layer_index + 1);

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

                    self.items = items.clone();
                }
            }
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == self.add {
                ui.send_message(CollectionChanged::add(
                    self.handle,
                    MessageDirection::FromWidget,
                ))
            }
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
    environment: Option<Rc<dyn InspectorEnvironment>>,
    definition_container: Option<Rc<PropertyEditorDefinitionContainer>>,
    add: Handle<UiNode>,
    layer_index: usize,
}

fn create_item_views(
    items: &[Item],
    ctx: &mut BuildContext,
    layer_index: usize,
) -> Vec<Handle<UiNode>> {
    items
        .iter()
        .enumerate()
        .map(|(n, item)| {
            ExpanderBuilder::new(WidgetBuilder::new())
                .with_header(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .with_child(
                                TextBuilder::new(WidgetBuilder::new())
                                    .with_vertical_text_alignment(VerticalAlignment::Center)
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
                .with_expander_margin(make_expander_margin(layer_index))
                .with_content(item.inspector)
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
                layer_index,
            );

            let inspector = InspectorBuilder::new(WidgetBuilder::new())
                .with_context(inspector_context)
                .build(ctx);

            let remove = ButtonBuilder::new(
                WidgetBuilder::new()
                    .with_margin(Thickness::uniform(1.0))
                    .with_vertical_alignment(VerticalAlignment::Center)
                    .on_column(1)
                    .with_width(16.0)
                    .with_height(16.0),
            )
            .with_text("-")
            .build(ctx);

            Item { inspector, remove }
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
            add: Default::default(),
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

    pub fn with_add(mut self, add: Handle<UiNode>) -> Self {
        self.add = add;
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
                    self.layer_index + 1,
                )
            })
            .unwrap_or_default();

        let panel = StackPanelBuilder::new(WidgetBuilder::new().with_children(create_item_views(
            &items,
            ctx,
            self.layer_index,
        )))
        .build(ctx);

        let ce = CollectionEditor {
            widget: self
                .widget_builder
                .with_preview_messages(true)
                .with_child(panel)
                .build(),
            add: self.add,
            items,
            panel,
            layer_index: self.layer_index,
        };

        ctx.add_node(UiNode::new(ce))
    }
}

#[derive(Debug)]
pub struct VecCollectionPropertyEditorDefinition<T>
where
    T: Inspect + Debug + 'static,
{
    phantom: PhantomData<T>,
}

impl<T> VecCollectionPropertyEditorDefinition<T>
where
    T: Inspect + Debug + 'static,
{
    pub fn new() -> Self {
        Self::default()
    }
}

impl<T> Default for VecCollectionPropertyEditorDefinition<T>
where
    T: Inspect + Debug + 'static,
{
    fn default() -> Self {
        Self {
            phantom: PhantomData::default(),
        }
    }
}

impl<T> PropertyEditorDefinition for VecCollectionPropertyEditorDefinition<T>
where
    T: Inspect + Debug + 'static,
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
                .with_horizontal_alignment(HorizontalAlignment::Right)
                .with_width(16.0)
                .with_height(16.0)
                .on_column(1)
                .with_margin(Thickness::uniform(1.0)),
        )
        .with_text("+")
        .build(ctx.build_context);

        Ok(PropertyEditorInstance {
            title: GridBuilder::new(
                WidgetBuilder::new()
                    .with_child(
                        TextBuilder::new(
                            WidgetBuilder::new().with_margin(make_layer_margin(ctx.layer_index)),
                        )
                        .with_text(ctx.property_info.display_name)
                        .with_vertical_text_alignment(VerticalAlignment::Center)
                        .build(ctx.build_context),
                    )
                    .with_child(add),
            )
            .add_column(Column::strict(NAME_COLUMN_WIDTH))
            .add_column(Column::stretch())
            .add_row(Row::stretch())
            .build(ctx.build_context),
            editor: CollectionEditorBuilder::new(
                WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
            )
            .with_add(add)
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
            sync_flag,
            instance,
            ui,
            property_info,
            definition_container,
            layer_index,
        } = ctx;

        let instance_ref = if let Some(instance) = ui.node(instance).cast::<CollectionEditor>() {
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
                None,
                definition_container,
                &mut ui.build_ctx(),
                sync_flag,
                layer_index + 1,
            );

            Ok(Some(CollectionEditorMessage::items(
                instance,
                MessageDirection::ToWidget,
                items,
            )))
        } else {
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
