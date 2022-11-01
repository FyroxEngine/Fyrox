use crate::{
    border::BorderBuilder,
    check_box::CheckBoxBuilder,
    core::{algebra::Vector2, pool::Handle, reflect::CastError},
    define_constructor,
    expander::ExpanderBuilder,
    formatted_text::WrapMode,
    grid::{Column, GridBuilder, Row},
    inspector::editors::{
        PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorDefinitionContainer,
        PropertyEditorInstance, PropertyEditorMessageContext, PropertyEditorTranslationContext,
    },
    message::{MessageDirection, UiMessage},
    stack_panel::StackPanelBuilder,
    text::TextBuilder,
    utils::{make_arrow, make_simple_tooltip, ArrowDirection},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, Thickness, UiNode, UserInterface, VerticalAlignment,
};
use fyrox_core::reflect::{Reflect, ResolvePath};
use std::{
    any::{Any, TypeId},
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
    rc::Rc,
};

pub mod editors;

#[derive(Debug, Clone, PartialEq)]
pub enum CollectionChanged {
    /// An item should be added in the collection.
    Add(ObjectValue),
    /// An item in the collection should be removed.
    Remove(usize),
    /// An item in the collection has changed one of its properties.
    ItemChanged {
        /// Index of an item in the collection.
        index: usize,
        property: PropertyChanged,
    },
}

impl CollectionChanged {
    define_constructor!(CollectionChanged:Add => fn add(ObjectValue), layout: false);
    define_constructor!(CollectionChanged:Remove => fn remove(usize), layout: false);
    define_constructor!(CollectionChanged:ItemChanged => fn item_changed(index: usize, property: PropertyChanged), layout: false);
}

#[derive(Debug, Clone)]
pub enum InheritableAction {
    Revert,
}

#[derive(Debug, Clone)]
pub enum FieldKind {
    Collection(Box<CollectionChanged>),
    Inspectable(Box<PropertyChanged>),
    Object(ObjectValue),
    Inheritable(InheritableAction),
}

/// An action for some property.
#[derive(Debug)]
pub enum PropertyAction {
    /// A property needs to be modified with given value.
    Modify {
        /// New value for a property.
        value: Box<dyn Reflect>,
    },
    /// An item needs to be added to a collection property.
    AddItem {
        /// New collection item.
        value: Box<dyn Reflect>,
    },
    /// An item needs to be removed from a collection property.
    RemoveItem {
        /// Index of an item.
        index: usize,
    },
    /// Revert value to parent.
    Revert,
}

impl PropertyAction {
    /// Creates action from a field definition. It is recursive action, it traverses the tree
    /// until there is either FieldKind::Object or FieldKind::Collection. FieldKind::Inspectable
    /// forces new iteration.
    pub fn from_field_kind(field_kind: &FieldKind) -> Self {
        match field_kind {
            FieldKind::Object(ref value) => Self::Modify {
                value: value.clone().into_box_reflect(),
            },
            FieldKind::Collection(ref collection_changed) => match **collection_changed {
                CollectionChanged::Add(ref value) => Self::AddItem {
                    value: value.clone().into_box_reflect(),
                },
                CollectionChanged::Remove(index) => Self::RemoveItem { index },
                CollectionChanged::ItemChanged { ref property, .. } => {
                    Self::from_field_kind(&property.value)
                }
            },
            FieldKind::Inspectable(ref inspectable) => Self::from_field_kind(&inspectable.value),
            FieldKind::Inheritable { .. } => Self::Revert,
        }
    }

    /// Tries to apply the action to a given target.
    pub fn apply(
        self,
        path: &str,
        target: &mut dyn Reflect,
    ) -> Result<Option<Box<dyn Reflect>>, Self> {
        match self {
            PropertyAction::Modify { value } => {
                if let Ok(field) = target.resolve_path_mut(path) {
                    if let Err(value) = field.set(value) {
                        Err(Self::Modify { value })
                    } else {
                        Ok(None)
                    }
                } else {
                    Err(Self::Modify { value })
                }
            }
            PropertyAction::AddItem { value } => {
                if let Ok(field) = target.resolve_path_mut(path) {
                    if let Some(list) = field.as_list_mut() {
                        return if let Err(value) = list.reflect_push(value) {
                            Err(Self::AddItem { value })
                        } else {
                            Ok(None)
                        };
                    }
                }

                Err(Self::AddItem { value })
            }
            PropertyAction::RemoveItem { index } => {
                if let Ok(field) = target.resolve_path_mut(path) {
                    if let Some(list) = field.as_list_mut() {
                        if let Some(value) = list.reflect_remove(index) {
                            return Ok(Some(value));
                        }
                    }
                }

                Err(Self::RemoveItem { index })
            }
            PropertyAction::Revert => {
                // Unsupported due to lack of context (a reference to parent entity).
                Err(Self::Revert)
            }
        }
    }
}

pub trait Value: Reflect + Debug {
    fn clone_box(&self) -> Box<dyn Value>;

    fn into_box_reflect(self: Box<Self>) -> Box<dyn Reflect>;
}

impl<T: Reflect + Clone + Debug> Value for T {
    fn clone_box(&self) -> Box<dyn Value> {
        Box::new(self.clone())
    }

    fn into_box_reflect(self: Box<Self>) -> Box<dyn Reflect> {
        Box::new(*self.into_any().downcast::<T>().unwrap())
    }
}

#[derive(Debug)]
pub struct ObjectValue {
    pub value: Box<dyn Value>,
}

impl Clone for ObjectValue {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone_box(),
        }
    }
}

impl PartialEq for ObjectValue {
    fn eq(&self, other: &Self) -> bool {
        // Cast fat pointers to thin first.
        let ptr_a = &*self.value as *const _ as *const ();
        let ptr_b = &*other.value as *const _ as *const ();
        // Compare thin pointers.
        std::ptr::eq(ptr_a, ptr_b)
    }
}

impl ObjectValue {
    pub fn cast_value<T: 'static>(&self) -> Option<&T> {
        (*self.value).as_any().downcast_ref::<T>()
    }

    pub fn cast_clone<T: Clone + 'static>(&self) -> Option<T> {
        (*self.value).as_any().downcast_ref::<T>().cloned()
    }

    pub fn try_override<T: Clone + 'static>(&self, value: &mut T) -> bool {
        (*self.value)
            .as_any()
            .downcast_ref::<T>()
            .map_or(false, |v| {
                *value = v.clone();
                true
            })
    }

    pub fn into_box_reflect(self) -> Box<dyn Reflect> {
        self.value.into_box_reflect()
    }
}

impl PartialEq for FieldKind {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (FieldKind::Collection(l), FieldKind::Collection(r)) => std::ptr::eq(&**l, &**r),
            (FieldKind::Inspectable(l), FieldKind::Inspectable(r)) => std::ptr::eq(&**l, &**r),
            (FieldKind::Object(l), FieldKind::Object(r)) => l == r,
            _ => false,
        }
    }
}

impl FieldKind {
    pub fn object<T: Value>(value: T) -> Self {
        Self::Object(ObjectValue {
            value: Box::new(value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropertyChanged {
    pub name: String,
    pub owner_type_id: TypeId,
    pub value: FieldKind,
}

impl PropertyChanged {
    pub fn path(&self) -> String {
        let mut path = self.name.clone();
        match self.value {
            FieldKind::Collection(ref collection_changed) => {
                if let CollectionChanged::ItemChanged {
                    ref property,
                    index,
                } = **collection_changed
                {
                    path += format!("[{}].{}", index, property.path()).as_ref();
                }
            }
            FieldKind::Inspectable(ref inspectable) => {
                path += format!(".{}", inspectable.path()).as_ref();
            }
            FieldKind::Object(_) | FieldKind::Inheritable { .. } => {}
        }
        path
    }

    pub fn is_inheritable(&self) -> bool {
        match self.value {
            FieldKind::Collection(ref collection_changed) => match **collection_changed {
                CollectionChanged::Add(_) => false,
                CollectionChanged::Remove(_) => false,
                CollectionChanged::ItemChanged { ref property, .. } => property.is_inheritable(),
            },
            FieldKind::Inspectable(ref inspectable) => inspectable.is_inheritable(),
            FieldKind::Object(_) => false,
            FieldKind::Inheritable(_) => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InspectorMessage {
    Context(InspectorContext),
    PropertyChanged(PropertyChanged),
}

impl InspectorMessage {
    define_constructor!(InspectorMessage:Context => fn context(InspectorContext), layout: false);
    define_constructor!(InspectorMessage:PropertyChanged => fn property_changed(PropertyChanged), layout: false);
}

pub trait InspectorEnvironment: Any {
    fn as_any(&self) -> &dyn Any;
}

#[derive(Clone)]
pub struct Inspector {
    pub widget: Widget,
    pub context: InspectorContext,
}

crate::define_widget_deref!(Inspector);

impl Inspector {
    pub fn context(&self) -> &InspectorContext {
        &self.context
    }
}

pub const NAME_COLUMN_WIDTH: f32 = 150.0;
pub const HEADER_MARGIN: Thickness = Thickness {
    left: 2.0,
    top: 1.0,
    right: 4.0,
    bottom: 1.0,
};

#[derive(Debug)]
pub enum InspectorError {
    CastError(CastError),
    OutOfSync,
    Custom(String),
    Group(Vec<InspectorError>),
}

impl From<CastError> for InspectorError {
    fn from(e: CastError) -> Self {
        Self::CastError(e)
    }
}

#[derive(Clone, Debug)]
pub struct ContextEntry {
    pub property_name: String,
    pub property_owner_type_id: TypeId,
    pub property_editor_definition: Rc<dyn PropertyEditorDefinition>,
    pub property_editor: Handle<UiNode>,
}

impl PartialEq for ContextEntry {
    fn eq(&self, other: &Self) -> bool {
        // Cast fat pointers to thin first.
        let ptr_a = &*self.property_editor_definition as *const _ as *const ();
        let ptr_b = &*other.property_editor_definition as *const _ as *const ();

        self.property_editor == other.property_editor
            && self.property_name == other.property_name
            // Compare thin pointers.
            && std::ptr::eq(ptr_a, ptr_b)
    }
}

#[derive(Clone)]
pub struct InspectorContext {
    pub stack_panel: Handle<UiNode>,
    pub entries: Vec<ContextEntry>,
    pub property_definitions: Rc<PropertyEditorDefinitionContainer>,
    pub environment: Option<Rc<dyn InspectorEnvironment>>,
    pub sync_flag: u64,
}

impl PartialEq for InspectorContext {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}

impl Default for InspectorContext {
    fn default() -> Self {
        Self {
            stack_panel: Default::default(),
            entries: Default::default(),
            property_definitions: Rc::new(PropertyEditorDefinitionContainer::new()),
            environment: None,
            sync_flag: 0,
        }
    }
}

impl Debug for InspectorContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "InspectorContext")
    }
}

pub fn make_property_margin(layer_index: usize) -> Thickness {
    let mut margin = HEADER_MARGIN;
    margin.left += 10.0 + layer_index as f32 * 10.0;
    margin
}

fn make_expander_margin(layer_index: usize) -> Thickness {
    let mut margin = HEADER_MARGIN;
    margin.left += layer_index as f32 * 10.0;
    margin
}

fn make_expander_check_box(
    layer_index: usize,
    property_name: &str,
    property_description: &str,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    CheckBoxBuilder::new(
        WidgetBuilder::new()
            .with_vertical_alignment(VerticalAlignment::Center)
            .with_margin(make_expander_margin(layer_index)),
    )
    .with_background(
        BorderBuilder::new(
            WidgetBuilder::new()
                .with_vertical_alignment(VerticalAlignment::Center)
                .with_min_size(Vector2::new(4.0, 4.0)),
        )
        .with_stroke_thickness(Thickness::zero())
        .build(ctx),
    )
    .with_content(
        TextBuilder::new(
            WidgetBuilder::new()
                .with_tooltip(make_tooltip(ctx, property_description))
                .with_height(16.0)
                .with_margin(Thickness::left(2.0)),
        )
        .with_vertical_text_alignment(VerticalAlignment::Center)
        .with_text(property_name)
        .build(ctx),
    )
    .checked(Some(true))
    .with_check_mark(make_arrow(ctx, ArrowDirection::Bottom, 8.0))
    .with_uncheck_mark(make_arrow(ctx, ArrowDirection::Right, 8.0))
    .build(ctx)
}

pub fn make_expander_container(
    layer_index: usize,
    property_name: &str,
    description: &str,
    header: Handle<UiNode>,
    content: Handle<UiNode>,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    ExpanderBuilder::new(WidgetBuilder::new())
        .with_checkbox(make_expander_check_box(
            layer_index,
            property_name,
            description,
            ctx,
        ))
        .with_expander_column(Column::strict(NAME_COLUMN_WIDTH))
        .with_expanded(true)
        .with_header(header)
        .with_content(content)
        .build(ctx)
}

fn create_header(ctx: &mut BuildContext, text: &str, layer_index: usize) -> Handle<UiNode> {
    TextBuilder::new(WidgetBuilder::new().with_margin(make_property_margin(layer_index)))
        .with_text(text)
        .with_vertical_text_alignment(VerticalAlignment::Center)
        .build(ctx)
}

fn make_tooltip(ctx: &mut BuildContext, text: &str) -> Rc<Handle<UiNode>> {
    if text.is_empty() {
        Rc::new(Handle::NONE)
    } else {
        make_simple_tooltip(ctx, text)
    }
}

fn make_simple_property_container(
    title: Handle<UiNode>,
    editor: Handle<UiNode>,
    description: &str,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    ctx[editor].set_row(0).set_column(1);

    let tooltip = make_tooltip(ctx, description);
    ctx[title].set_tooltip(tooltip);

    GridBuilder::new(WidgetBuilder::new().with_child(title).with_child(editor))
        .add_row(Row::auto())
        .add_columns(vec![Column::strict(NAME_COLUMN_WIDTH), Column::stretch()])
        .build(ctx)
}

impl InspectorContext {
    pub fn from_object(
        object: &dyn Reflect,
        ctx: &mut BuildContext,
        definition_container: Rc<PropertyEditorDefinitionContainer>,
        environment: Option<Rc<dyn InspectorEnvironment>>,
        sync_flag: u64,
        layer_index: usize,
    ) -> Self {
        let mut entries = Vec::new();

        let editors = object
            .fields_info()
            .iter()
            .enumerate()
            .map(|(i, info)| {
                let description = if info.description.is_empty() {
                    info.display_name.to_string()
                } else {
                    format!("{}\n\n{}", info.display_name, info.description)
                };

                if let Some(definition) = definition_container
                    .definitions()
                    .get(&info.value.type_id())
                {
                    match definition.create_instance(PropertyEditorBuildContext {
                        build_context: ctx,
                        property_info: info,
                        environment: environment.clone(),
                        definition_container: definition_container.clone(),
                        sync_flag,
                        layer_index,
                    }) {
                        Ok(instance) => {
                            let (container, editor) = match instance {
                                PropertyEditorInstance::Simple { editor } => (
                                    make_simple_property_container(
                                        create_header(ctx, info.display_name, layer_index),
                                        editor,
                                        &description,
                                        ctx,
                                    ),
                                    editor,
                                ),
                                PropertyEditorInstance::Custom { container, editor } => {
                                    (container, editor)
                                }
                            };
                            entries.push(ContextEntry {
                                property_editor: editor,
                                property_editor_definition: definition.clone(),
                                property_name: info.name.to_string(),
                                property_owner_type_id: info.owner_type_id,
                            });

                            if info.read_only {
                                ctx[editor].set_enabled(false);
                            }

                            container
                        }
                        Err(e) => make_simple_property_container(
                            create_header(ctx, info.display_name, layer_index),
                            TextBuilder::new(WidgetBuilder::new().on_row(i).on_column(1))
                                .with_wrap(WrapMode::Word)
                                .with_vertical_text_alignment(VerticalAlignment::Center)
                                .with_text(format!(
                                    "Unable to create property \
                                                    editor instance: Reason {:?}",
                                    e
                                ))
                                .build(ctx),
                            &description,
                            ctx,
                        ),
                    }
                } else {
                    make_simple_property_container(
                        create_header(ctx, info.display_name, layer_index),
                        TextBuilder::new(WidgetBuilder::new().on_row(i).on_column(1))
                            .with_wrap(WrapMode::Word)
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .with_text("Property Editor Is Missing!")
                            .build(ctx),
                        &description,
                        ctx,
                    )
                }
            })
            .collect::<Vec<_>>();

        let stack_panel =
            StackPanelBuilder::new(WidgetBuilder::new().with_children(editors)).build(ctx);

        Self {
            stack_panel,
            entries,
            property_definitions: definition_container,
            sync_flag,
            environment,
        }
    }

    pub fn sync(
        &self,
        object: &dyn Reflect,
        ui: &mut UserInterface,
        layer_index: usize,
    ) -> Result<(), Vec<InspectorError>> {
        let mut sync_errors = Vec::new();

        for info in object.fields_info() {
            if let Some(constructor) = self
                .property_definitions
                .definitions()
                .get(&info.value.type_id())
            {
                if let Some(property_editor) = self.find_property_editor(info.name) {
                    let ctx = PropertyEditorMessageContext {
                        sync_flag: self.sync_flag,
                        instance: property_editor.property_editor,
                        ui,
                        property_info: &info,
                        definition_container: self.property_definitions.clone(),
                        layer_index,
                        environment: self.environment.clone(),
                    };

                    match constructor.create_message(ctx) {
                        Ok(message) => {
                            if let Some(mut message) = message {
                                message.flags = self.sync_flag;
                                ui.send_message(message);
                            }
                        }
                        Err(e) => sync_errors.push(e),
                    }
                }
            }
        }

        if sync_errors.is_empty() {
            Ok(())
        } else {
            Err(sync_errors)
        }
    }

    pub fn property_editors(&self) -> impl Iterator<Item = &ContextEntry> + '_ {
        self.entries.iter()
    }

    pub fn find_property_editor(&self, name: &str) -> Option<&ContextEntry> {
        self.entries.iter().find(|e| e.property_name == name)
    }

    pub fn find_property_editor_widget(&self, name: &str) -> Handle<UiNode> {
        self.find_property_editor(name)
            .map(|e| e.property_editor)
            .unwrap_or_default()
    }
}

impl Control for Inspector {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle && message.direction() == MessageDirection::ToWidget
        {
            if let Some(InspectorMessage::Context(ctx)) = message.data::<InspectorMessage>() {
                // Remove previous content.
                for child in self.children() {
                    ui.send_message(WidgetMessage::remove(*child, MessageDirection::ToWidget));
                }

                // Link new panel.
                ui.send_message(WidgetMessage::link(
                    ctx.stack_panel,
                    MessageDirection::ToWidget,
                    self.handle,
                ));

                self.context = ctx.clone();
            }
        }

        // Check each message from descendant widget and try to translate it to
        // PropertyChanged message.
        if message.flags != self.context.sync_flag {
            let env = self.context.environment.clone();
            for entry in self.context.entries.iter() {
                if message.destination() == entry.property_editor {
                    if let Some(args) = entry.property_editor_definition.translate_message(
                        PropertyEditorTranslationContext {
                            environment: env.clone(),
                            name: &entry.property_name,
                            owner_type_id: entry.property_owner_type_id,
                            message,
                            definition_container: self.context.property_definitions.clone(),
                        },
                    ) {
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

pub struct InspectorBuilder {
    widget_builder: WidgetBuilder,
    context: InspectorContext,
}

impl InspectorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            context: Default::default(),
        }
    }

    pub fn with_context(mut self, context: InspectorContext) -> Self {
        self.context = context;
        self
    }

    pub fn with_opt_context(mut self, context: Option<InspectorContext>) -> Self {
        if let Some(context) = context {
            self.context = context;
        }
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let canvas = Inspector {
            widget: self
                .widget_builder
                .with_child(self.context.stack_panel)
                .build(),
            context: self.context,
        };
        ctx.add_node(UiNode::new(canvas))
    }
}
