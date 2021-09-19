use crate::{
    core::{algebra::Vector3, pool::Handle},
    expander::ExpanderBuilder,
    formatted_text::WrapMode,
    grid::{Column, GridBuilder, Row},
    message::{
        InspectorMessage, MessageData, MessageDirection, NumericUpDownMessage, PropertyChanged,
        TextBoxMessage, UiMessage, UiMessageData, Vec3EditorMessage, WidgetMessage,
    },
    numeric::NumericUpDownBuilder,
    stack_panel::StackPanelBuilder,
    text::TextBuilder,
    text_box::TextBoxBuilder,
    vec::vec3::Vec3EditorBuilder,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UINode, UserInterface, VerticalAlignment,
};
use std::{
    any::{Any, TypeId},
    collections::{hash_map::Entry, HashMap},
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::Arc,
};

#[derive(Clone)]
pub struct Inspector<M: MessageData, C: Control<M, C>> {
    widget: Widget<M, C>,
    stack_panel: Handle<UINode<M, C>>,
    context: InspectorContext<M, C>,
    property_definitions: PropertyDefinitionContainer<M, C>,
}

crate::define_widget_deref!(Inspector<M, C>);

pub trait PropertyValue: Any + Send + Sync + Debug {
    fn as_any(&self) -> &dyn Any;
}

impl<T: Send + Sync + Debug + 'static> PropertyValue for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct PropertyInfo<'a> {
    pub name: &'a str,
    pub group: &'static str,
    pub value: &'a dyn PropertyValue,
}

impl<'a> PropertyInfo<'a> {
    fn cast_value<T: 'static>(&self) -> Result<&T, InspectorError> {
        match self.value.as_any().downcast_ref::<T>() {
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

#[derive(Debug)]
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

pub trait PropertyEditorDefinition<M: MessageData, C: Control<M, C>>: Debug + Send + Sync {
    fn value_type_id(&self) -> TypeId;

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext<M, C>,
    ) -> Result<Handle<UINode<M, C>>, InspectorError>;

    fn create_message(
        &self,
        instance: Handle<UINode<M, C>>,
        property_info: &PropertyInfo,
    ) -> Result<UiMessage<M, C>, InspectorError>;

    fn translate_message(&self, name: &str, message: &UiMessage<M, C>) -> Option<PropertyChanged>;
}

#[derive(Debug)]
struct F32PropertyEditorDefinition;

impl<M: MessageData, C: Control<M, C>> PropertyEditorDefinition<M, C>
    for F32PropertyEditorDefinition
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<f32>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext<M, C>,
    ) -> Result<Handle<UINode<M, C>>, InspectorError> {
        let value = ctx.property_info.cast_value::<f32>()?;
        Ok(
            NumericUpDownBuilder::new(WidgetBuilder::new().on_row(ctx.row).on_column(ctx.column))
                .with_value(*value)
                .build(ctx.build_context),
        )
    }

    fn create_message(
        &self,
        instance: Handle<UINode<M, C>>,
        property_info: &PropertyInfo,
    ) -> Result<UiMessage<M, C>, InspectorError> {
        let value = property_info.cast_value::<f32>()?;
        Ok(NumericUpDownMessage::value(
            instance,
            MessageDirection::ToWidget,
            *value,
        ))
    }

    fn translate_message(&self, name: &str, message: &UiMessage<M, C>) -> Option<PropertyChanged> {
        if message.direction() == MessageDirection::FromWidget {
            if let UiMessageData::NumericUpDown(NumericUpDownMessage::Value(value)) = message.data()
            {
                return Some(PropertyChanged {
                    name: name.to_string(),
                    value: Arc::new(*value),
                });
            }
        }

        None
    }
}

#[derive(Debug)]
struct I32PropertyEditorDefinition;

impl<M: MessageData, C: Control<M, C>> PropertyEditorDefinition<M, C>
    for I32PropertyEditorDefinition
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<i32>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext<M, C>,
    ) -> Result<Handle<UINode<M, C>>, InspectorError> {
        let value = ctx.property_info.cast_value::<i32>()?;
        Ok(
            NumericUpDownBuilder::new(WidgetBuilder::new().on_row(ctx.row).on_column(ctx.column))
                .with_precision(0)
                .with_step(1.0)
                .with_min_value(-i32::MAX as f32)
                .with_max_value(i32::MAX as f32)
                .with_value(*value as f32)
                .build(ctx.build_context),
        )
    }

    fn create_message(
        &self,
        instance: Handle<UINode<M, C>>,
        property_info: &PropertyInfo,
    ) -> Result<UiMessage<M, C>, InspectorError> {
        let value = property_info.cast_value::<i32>()?;
        Ok(NumericUpDownMessage::value(
            instance,
            MessageDirection::ToWidget,
            *value as f32,
        ))
    }

    fn translate_message(&self, name: &str, message: &UiMessage<M, C>) -> Option<PropertyChanged> {
        if message.direction() == MessageDirection::FromWidget {
            if let UiMessageData::NumericUpDown(NumericUpDownMessage::Value(value)) = message.data()
            {
                return Some(PropertyChanged {
                    name: name.to_string(),
                    value: Arc::new(*value as i32),
                });
            }
        }
        None
    }
}

#[derive(Debug)]
struct StringPropertyEditorDefinition;

impl<M: MessageData, C: Control<M, C>> PropertyEditorDefinition<M, C>
    for StringPropertyEditorDefinition
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<String>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext<M, C>,
    ) -> Result<Handle<UINode<M, C>>, InspectorError> {
        let value = ctx.property_info.cast_value::<String>()?;
        Ok(
            TextBoxBuilder::new(WidgetBuilder::new().on_row(ctx.row).on_column(ctx.column))
                .with_text(value)
                .build(ctx.build_context),
        )
    }

    fn create_message(
        &self,
        instance: Handle<UINode<M, C>>,
        property_info: &PropertyInfo,
    ) -> Result<UiMessage<M, C>, InspectorError> {
        let value = property_info.cast_value::<String>()?;
        Ok(TextBoxMessage::text(
            instance,
            MessageDirection::ToWidget,
            value.clone(),
        ))
    }

    fn translate_message(&self, name: &str, message: &UiMessage<M, C>) -> Option<PropertyChanged> {
        if message.direction() == MessageDirection::FromWidget {
            if let UiMessageData::TextBox(TextBoxMessage::Text(value)) = message.data() {
                return Some(PropertyChanged {
                    name: name.to_string(),
                    value: Arc::new(value.clone()),
                });
            }
        }
        None
    }
}

#[derive(Debug)]
struct Vec3PropertyEditorDefinition;

impl<M: MessageData, C: Control<M, C>> PropertyEditorDefinition<M, C>
    for Vec3PropertyEditorDefinition
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Vector3<f32>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext<M, C>,
    ) -> Result<Handle<UINode<M, C>>, InspectorError> {
        let value = ctx.property_info.cast_value::<Vector3<f32>>()?;
        Ok(
            Vec3EditorBuilder::new(WidgetBuilder::new().on_row(ctx.row).on_column(ctx.column))
                .with_value(*value)
                .build(ctx.build_context),
        )
    }

    fn create_message(
        &self,
        instance: Handle<UINode<M, C>>,
        property_info: &PropertyInfo,
    ) -> Result<UiMessage<M, C>, InspectorError> {
        let value = property_info.cast_value::<Vector3<f32>>()?;
        Ok(Vec3EditorMessage::value(
            instance,
            MessageDirection::ToWidget,
            *value,
        ))
    }

    fn translate_message(&self, name: &str, message: &UiMessage<M, C>) -> Option<PropertyChanged> {
        if message.direction() == MessageDirection::FromWidget {
            if let UiMessageData::Vec3Editor(Vec3EditorMessage::Value(value)) = message.data() {
                return Some(PropertyChanged {
                    name: name.to_string(),
                    value: Arc::new(*value),
                });
            }
        }
        None
    }
}

#[derive(Clone)]
pub struct PropertyDefinitionContainer<M: MessageData, C: Control<M, C>> {
    definitions: HashMap<TypeId, Arc<dyn PropertyEditorDefinition<M, C>>>,
}

impl<M: MessageData, C: Control<M, C>> Default for PropertyDefinitionContainer<M, C> {
    fn default() -> Self {
        Self {
            definitions: Default::default(),
        }
    }
}

impl<M: MessageData, C: Control<M, C>> PropertyDefinitionContainer<M, C> {
    pub fn new() -> Self {
        let mut container = Self::default();
        container.insert(Arc::new(F32PropertyEditorDefinition));
        container.insert(Arc::new(I32PropertyEditorDefinition));
        container.insert(Arc::new(StringPropertyEditorDefinition));
        container.insert(Arc::new(Vec3PropertyEditorDefinition));
        container
    }

    pub fn insert(
        &mut self,
        definition: Arc<dyn PropertyEditorDefinition<M, C>>,
    ) -> Option<Arc<dyn PropertyEditorDefinition<M, C>>> {
        self.definitions
            .insert(definition.value_type_id(), definition)
    }
}

#[derive(Clone, Debug)]
pub struct ContextEntry<M: MessageData, C: Control<M, C>> {
    pub property_name: String,
    pub property_editor_definition: Arc<dyn PropertyEditorDefinition<M, C>>,
    pub property_editor: Handle<UINode<M, C>>,
}

impl<M: MessageData, C: Control<M, C>> PartialEq for ContextEntry<M, C> {
    fn eq(&self, other: &Self) -> bool {
        self.property_editor == other.property_editor
            && self.property_name == other.property_name
            && std::ptr::eq(
                &*self.property_editor_definition,
                &*other.property_editor_definition,
            )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Group<M: MessageData, C: Control<M, C>> {
    section: Handle<UINode<M, C>>,
    entries: Vec<ContextEntry<M, C>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InspectorContext<M: MessageData, C: Control<M, C>> {
    groups: Vec<Group<M, C>>,
}

impl<M: MessageData, C: Control<M, C>> Default for InspectorContext<M, C> {
    fn default() -> Self {
        Self {
            groups: Default::default(),
        }
    }
}

impl<M: MessageData, C: Control<M, C>> InspectorContext<M, C> {
    pub fn from_object(
        object: &dyn Inspect,
        ctx: &mut BuildContext<M, C>,
        definition_container: &PropertyDefinitionContainer<M, C>,
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
                                    if let Some(definition) =
                                        definition_container.definitions.get(&info.value.type_id())
                                    {
                                        match definition.create_instance(
                                            PropertyEditorBuildContext {
                                                build_context: ctx,
                                                property_info: info,
                                                row: i,
                                                column: 1,
                                            },
                                        ) {
                                            Ok(instance) => {
                                                entries.push(ContextEntry {
                                                    property_editor: instance,
                                                    property_editor_definition: definition.clone(),
                                                    property_name: info.name.to_string(),
                                                });

                                                instance
                                            }
                                            Err(e) => TextBuilder::new(
                                                WidgetBuilder::new().on_row(i).on_column(1),
                                            )
                                            .with_wrap(WrapMode::Word)
                                            .with_text(format!(
                                                "Unable to create property \
                                                    editor instance: Reason {:?}",
                                                e
                                            ))
                                            .build(ctx),
                                        }
                                    } else {
                                        TextBuilder::new(
                                            WidgetBuilder::new().on_row(i).on_column(1),
                                        )
                                        .with_wrap(WrapMode::Word)
                                        .with_text("Property Editor Is Missing!")
                                        .build(ctx)
                                    }
                                })),
                        )
                        .add_rows(infos.iter().map(|_| Row::strict(25.0)).collect())
                        .add_column(Column::strict(200.0))
                        .add_column(Column::stretch())
                        .build(ctx),
                    )
                    .build(ctx);
                Group { section, entries }
            })
            .collect::<Vec<_>>();

        Self { groups }
    }

    pub fn sync(
        &self,
        object: &dyn Inspect,
        constructors: &PropertyDefinitionContainer<M, C>,
        ui: &mut UserInterface<M, C>,
        sync_flag: u64,
    ) -> Result<(), InspectorError> {
        for info in object.properties() {
            if let Some(constructor) = constructors.definitions.get(&info.value.type_id()) {
                let mut message =
                    constructor.create_message(self.find_property_editor(info.name), &info)?;

                message.flags = sync_flag;

                ui.send_message(message);
            }
        }

        Ok(())
    }

    pub fn property_editors(&self) -> impl Iterator<Item = &ContextEntry<M, C>> + '_ {
        self.groups.iter().map(|g| g.entries.iter()).flatten()
    }

    pub fn find_property_editor(&self, name: &str) -> Handle<UINode<M, C>> {
        for group in self.groups.iter() {
            if let Some(property_editor) = group
                .entries
                .iter()
                .find(|e| e.property_name == name)
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
                for group in ctx.groups.iter() {
                    ui.send_message(WidgetMessage::link(
                        group.section,
                        MessageDirection::ToWidget,
                        self.stack_panel,
                    ));
                }

                self.context = ctx.clone();
            }
        }

        // Check each message from descendant widget and try to translate it to
        // PropertyChanged message.
        for group in self.context.groups.iter() {
            for entry in group.entries.iter() {
                if message.destination() == entry.property_editor {
                    if let Some(args) = entry
                        .property_editor_definition
                        .translate_message(&entry.property_name, message)
                    {
                        ui.send_message(InspectorMessage::property_changed(
                            self.handle,
                            MessageDirection::FromWidget,
                            args,
                        ));
                    }
                }
            }
        }
    }
}

pub struct InspectorBuilder<M: MessageData, C: Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    context: InspectorContext<M, C>,
    property_definitions: Option<PropertyDefinitionContainer<M, C>>,
}

impl<M: MessageData, C: Control<M, C>> InspectorBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            context: Default::default(),
            property_definitions: None,
        }
    }

    pub fn with_context(mut self, context: InspectorContext<M, C>) -> Self {
        self.context = context;
        self
    }

    pub fn with_property_definitions(
        mut self,
        definitions: PropertyDefinitionContainer<M, C>,
    ) -> Self {
        self.property_definitions = Some(definitions);
        self
    }

    pub fn build(self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        let sections = self
            .context
            .groups
            .iter()
            .map(|g| g.section)
            .collect::<Vec<_>>();

        let stack_panel =
            StackPanelBuilder::new(WidgetBuilder::new().with_children(sections)).build(ctx);

        let canvas = Inspector {
            widget: self.widget_builder.with_child(stack_panel).build(),
            stack_panel,
            context: self.context,
            property_definitions: self
                .property_definitions
                .unwrap_or_else(|| PropertyDefinitionContainer::new()),
        };
        ctx.add_node(UINode::Inspector(canvas))
    }
}
