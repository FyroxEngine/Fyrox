use crate::{
    core::pool::Handle,
    expander::ExpanderBuilder,
    grid::{Column, GridBuilder, Row},
    message::{
        InspectorMessage, MessageData, MessageDirection, NumericUpDownMessage, TextBoxMessage,
        UiMessage, UiMessageData, WidgetMessage,
    },
    numeric::NumericUpDownBuilder,
    stack_panel::StackPanelBuilder,
    text::TextBuilder,
    text_box::TextBoxBuilder,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UINode, UserInterface, VerticalAlignment,
};
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    ops::{Deref, DerefMut},
};

#[derive(Clone)]
pub struct Inspector<M: MessageData, C: Control<M, C>> {
    widget: Widget<M, C>,
    stack_panel: Handle<UINode<M, C>>,
    context: InspectorContext<M, C>,
}

crate::define_widget_deref!(Inspector<M, C>);

pub const INSPECTOR_SYNC_FLAG: u64 = u64::MAX - 1;

pub fn mark_message<M: MessageData, C: Control<M, C>>(mut msg: UiMessage<M, C>) -> UiMessage<M, C> {
    msg.flags = INSPECTOR_SYNC_FLAG;
    msg
}

pub struct PropertyInfo<'a> {
    pub name: &'a str,
    pub group: &'static str,
    pub value: &'a dyn Any,
}

impl<'a> PropertyInfo<'a> {
    fn cast_value<T: 'static>(&self) -> Result<&T, InspectorError> {
        match self.value.downcast_ref::<T>() {
            Some(value) => Ok(value),
            None => Err(InspectorError::TypeMismatch {
                property_name: self.name.to_string(),
                expected_type_id: TypeId::of::<T>(),
                actual_type_id: self.value.type_id(),
            }),
        }
    }
}

pub trait Inspect {
    fn properties(&self) -> Vec<PropertyInfo<'_>>;
}

pub enum InspectorError {
    TypeMismatch {
        property_name: String,
        expected_type_id: TypeId,
        actual_type_id: TypeId,
    },
    OutOfSync,
}

pub struct PropertyEditorBuildContext<'a, 'b, 'c, M: MessageData, C: Control<M, C>> {
    build_context: &'a mut BuildContext<'c, M, C>,
    property_info: &'b PropertyInfo<'b>,
    row: usize,
    column: usize,
}

pub struct PropertyEditorConstructor<M: MessageData, C: Control<M, C>> {
    type_id: TypeId,
    builder: Box<
        dyn FnMut(PropertyEditorBuildContext<M, C>) -> Result<Handle<UINode<M, C>>, InspectorError>,
    >,
    sync_message_builder: Box<
        dyn FnMut(Handle<UINode<M, C>>, &PropertyInfo) -> Result<UiMessage<M, C>, InspectorError>,
    >,
}

impl<M: MessageData, C: Control<M, C>> PropertyEditorConstructor<M, C> {
    pub fn f32_editor() -> Self {
        Self {
            type_id: TypeId::of::<f32>(),
            builder: Box::new(|ctx| {
                let value = ctx.property_info.cast_value::<f32>()?;
                Ok(NumericUpDownBuilder::new(
                    WidgetBuilder::new().on_row(ctx.row).on_column(ctx.column),
                )
                .with_value(*value)
                .build(ctx.build_context))
            }),
            sync_message_builder: Box::new(|dest, property_info| {
                let value = property_info.cast_value::<f32>()?;
                Ok(NumericUpDownMessage::value(
                    dest,
                    MessageDirection::ToWidget,
                    *value,
                ))
            }),
        }
    }

    pub fn i32_editor() -> Self {
        Self {
            type_id: TypeId::of::<i32>(),
            builder: Box::new(|ctx| {
                let value = ctx.property_info.cast_value::<i32>()?;
                Ok(NumericUpDownBuilder::new(
                    WidgetBuilder::new().on_row(ctx.row).on_column(ctx.column),
                )
                .with_precision(0)
                .with_min_value(-i32::MAX as f32)
                .with_max_value(i32::MAX as f32)
                .with_value(*value as f32)
                .build(ctx.build_context))
            }),
            sync_message_builder: Box::new(|dest, property_info| {
                let value = property_info.cast_value::<i32>()?;
                Ok(NumericUpDownMessage::value(
                    dest,
                    MessageDirection::ToWidget,
                    *value as f32,
                ))
            }),
        }
    }

    pub fn string_editor() -> Self {
        Self {
            type_id: TypeId::of::<String>(),
            builder: Box::new(|ctx| {
                let value = ctx.property_info.cast_value::<String>()?;
                Ok(
                    TextBoxBuilder::new(WidgetBuilder::new().on_row(ctx.row).on_column(ctx.column))
                        .with_text(value)
                        .build(ctx.build_context),
                )
            }),
            sync_message_builder: Box::new(|dest, property_info| {
                let value = property_info.cast_value::<String>()?;
                Ok(TextBoxMessage::text(
                    dest,
                    MessageDirection::ToWidget,
                    value.clone(),
                ))
            }),
        }
    }
}

pub struct ConstructorContainer<M: MessageData, C: Control<M, C>> {
    constructors: HashMap<TypeId, RefCell<PropertyEditorConstructor<M, C>>>,
}

impl<M: MessageData, C: Control<M, C>> Default for ConstructorContainer<M, C> {
    fn default() -> Self {
        Self {
            constructors: Default::default(),
        }
    }
}

impl<M: MessageData, C: Control<M, C>> ConstructorContainer<M, C> {
    pub fn new() -> Self {
        let mut container = Self::default();
        container.insert(PropertyEditorConstructor::f32_editor());
        container.insert(PropertyEditorConstructor::i32_editor());
        container.insert(PropertyEditorConstructor::string_editor());
        container
    }

    pub fn insert(
        &mut self,
        constructor: PropertyEditorConstructor<M, C>,
    ) -> Option<PropertyEditorConstructor<M, C>> {
        self.constructors
            .insert(constructor.type_id, RefCell::new(constructor))
            .map(|c| c.into_inner())
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ContextEntry<M: MessageData, C: Control<M, C>> {
    name: String,
    property_editor: Handle<UINode<M, C>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InspectorContext<M: MessageData, C: Control<M, C>> {
    groups: HashMap<Handle<UINode<M, C>>, Vec<ContextEntry<M, C>>>,
}

impl<M: MessageData, C: Control<M, C>> Default for InspectorContext<M, C> {
    fn default() -> Self {
        Self {
            groups: Default::default(),
        }
    }
}

impl<M: MessageData, C: Control<M, C>> InspectorContext<M, C> {
    pub fn from_object<I: Inspect>(
        object: I,
        ctx: &mut BuildContext<M, C>,
        constructors: &ConstructorContainer<M, C>,
    ) -> Self {
        let mut property_groups = HashMap::<&'static str, Vec<PropertyInfo>>::new();
        for info in object.properties() {
            match property_groups.entry(info.group) {
                Entry::Vacant(e) => {
                    e.insert(vec![info]);
                }
                Entry::Occupied(e) => {
                    e.into_mut().push(info);
                }
            }
        }

        let groups = property_groups
            .iter()
            .map(|(&group, infos)| {
                let mut entries = Vec::new();
                let section = ExpanderBuilder::new(WidgetBuilder::new())
                    .with_header(
                        TextBuilder::new(WidgetBuilder::new())
                            .with_text(group)
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ctx),
                    )
                    .with_content(
                        GridBuilder::new(
                            WidgetBuilder::new()
                                .with_children(infos.iter().enumerate().map(|(i, info)| {
                                    TextBuilder::new(WidgetBuilder::new().on_row(i).on_column(0))
                                        .with_text(info.name)
                                        .with_vertical_text_alignment(VerticalAlignment::Center)
                                        .build(ctx)
                                }))
                                .with_children(infos.iter().enumerate().map(|(i, info)| {
                                    let property_editor = constructors
                                        .constructors
                                        .get(&info.value.type_id())
                                        .and_then(|constructor| {
                                            ((constructor.borrow_mut().builder)(
                                                PropertyEditorBuildContext {
                                                    build_context: ctx,
                                                    property_info: info,
                                                    row: i,
                                                    column: 1,
                                                },
                                            ))
                                            .ok()
                                        })
                                        .unwrap_or_else(|| {
                                            TextBuilder::new(
                                                WidgetBuilder::new().on_row(i).on_column(1),
                                            )
                                            .with_text("Property Editor Is Missing!")
                                            .build(ctx)
                                        });

                                    entries.push(ContextEntry {
                                        property_editor,
                                        name: info.name.to_string(),
                                    });

                                    property_editor
                                })),
                        )
                        .add_rows(infos.iter().map(|_| Row::strict(25.0)).collect())
                        .add_column(Column::strict(200.0))
                        .add_column(Column::stretch())
                        .build(ctx),
                    )
                    .build(ctx);
                (section, entries)
            })
            .collect::<HashMap<_, _>>();

        Self { groups }
    }

    pub fn sync<I: Inspect>(
        &self,
        object: I,
        constructors: &ConstructorContainer<M, C>,
        ui: &mut UserInterface<M, C>,
    ) -> Result<(), InspectorError> {
        for info in object.properties() {
            if let Some(constructor) = constructors.constructors.get(&info.value.type_id()) {
                ui.send_message(mark_message((constructor
                    .borrow_mut()
                    .sync_message_builder)(
                    self.find_property_editor(info.name), &info
                )?))
            }
        }

        Ok(())
    }

    pub fn find_property_editor(&self, name: &str) -> Handle<UINode<M, C>> {
        for group in self.groups.values() {
            if let Some(property_editor) = group
                .iter()
                .find(|e| e.name == name)
                .map(|e| e.property_editor)
            {
                return property_editor;
            }
        }
        Default::default()
    }
}

impl<M: MessageData, C: Control<M, C>> Control<M, C> for Inspector<M, C> {
    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle && message.direction() == MessageDirection::ToWidget
        {
            if let UiMessageData::Inspector(InspectorMessage::Context(ctx)) = message.data() {
                // Remove previous content.
                for child in ui.node(self.stack_panel).children() {
                    ui.send_message(WidgetMessage::remove(*child, MessageDirection::ToWidget));
                }

                // Link new sections to the panel.
                for group in ctx.groups.keys() {
                    ui.send_message(WidgetMessage::link(
                        *group,
                        MessageDirection::ToWidget,
                        self.stack_panel,
                    ));
                }

                self.context = ctx.clone();
            }
        }
    }
}

pub struct InspectorBuilder<M: MessageData, C: Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    context: InspectorContext<M, C>,
}

impl<M: MessageData, C: Control<M, C>> InspectorBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            context: Default::default(),
        }
    }

    pub fn with_context(mut self, context: InspectorContext<M, C>) -> Self {
        self.context = context;
        self
    }

    pub fn build(self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        let sections = self.context.groups.keys().cloned().collect::<Vec<_>>();

        let stack_panel =
            StackPanelBuilder::new(WidgetBuilder::new().with_children(sections)).build(ctx);

        let canvas = Inspector {
            widget: self.widget_builder.with_child(stack_panel).build(),
            stack_panel,
            context: self.context,
        };
        ctx.add_node(UINode::Inspector(canvas))
    }
}
