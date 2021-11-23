use crate::inspector::editors::PropertyEditorInstance;
use crate::{
    border::BorderBuilder,
    brush::Brush,
    core::{
        algebra::Vector2,
        color::Color,
        inspect::{CastError, Inspect, PropertyValue},
        pool::Handle,
    },
    define_constructor,
    formatted_text::WrapMode,
    grid::{Column, GridBuilder, Row},
    inspector::editors::{
        PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorDefinitionContainer,
        PropertyEditorMessageContext,
    },
    message::{MessageDirection, UiMessage},
    stack_panel::StackPanelBuilder,
    text::TextBuilder,
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, Thickness, UiNode, UserInterface, VerticalAlignment,
};
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
    Add,
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
    define_constructor!(CollectionChanged:Add => fn add(), layout: false);
    define_constructor!(CollectionChanged:Remove => fn remove(usize), layout: false);
    define_constructor!(CollectionChanged:ItemChanged => fn item_changed(index: usize, property: PropertyChanged), layout: false);
}

#[derive(Debug, Clone)]
pub enum FieldKind {
    Collection(Box<CollectionChanged>),
    Inspectable(Box<PropertyChanged>),
    Object(ObjectValue),
}

#[derive(Debug, Clone)]
pub struct ObjectValue {
    value: Rc<dyn PropertyValue>,
}

#[allow(clippy::vtable_address_comparisons)]
impl PartialEq for ObjectValue {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(&*self.value, &*other.value)
    }
}

impl ObjectValue {
    pub fn cast_value<T: 'static>(&self) -> Option<&T> {
        (*self.value).as_any().downcast_ref::<T>()
    }

    pub fn cast_value_cloned<T: Clone + 'static>(&self) -> Option<T> {
        (*self.value).as_any().downcast_ref::<T>().cloned()
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
    pub fn object<T: PropertyValue>(value: T) -> Self {
        Self::Object(ObjectValue {
            value: Rc::new(value),
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
            FieldKind::Object(_) => {}
        }
        path
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

pub trait InspectorEnvironment: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
}

#[derive(Clone)]
pub struct Inspector {
    widget: Widget,
    context: InspectorContext,
}

crate::define_widget_deref!(Inspector);

impl Inspector {
    pub fn context(&self) -> &InspectorContext {
        &self.context
    }
}

pub const NAME_COLUMN_WIDTH: f32 = 150.0;
pub const HEADER_MARGIN: Thickness = Thickness {
    left: 4.0,
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

#[allow(clippy::vtable_address_comparisons)]
impl PartialEq for ContextEntry {
    fn eq(&self, other: &Self) -> bool {
        self.property_editor == other.property_editor
            && self.property_name == other.property_name
            && std::ptr::eq(
                &*self.property_editor_definition,
                &*other.property_editor_definition,
            )
    }
}

#[derive(Clone)]
pub struct InspectorContext {
    stack_panel: Handle<UiNode>,
    entries: Vec<ContextEntry>,
    property_definitions: Rc<PropertyEditorDefinitionContainer>,
    environment: Option<Rc<dyn InspectorEnvironment>>,
    sync_flag: u64,
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

pub fn make_layer_margin(layer_index: usize) -> Thickness {
    let mut margin = HEADER_MARGIN;
    margin.left += layer_index as f32 * 10.0;
    margin
}

pub fn make_expander_margin(layer_index: usize) -> Thickness {
    let mut margin = make_layer_margin(layer_index);
    margin.left = (margin.left - 20.0).max(0.0);
    margin
}

fn create_header(ctx: &mut BuildContext, text: &str, layer_index: usize) -> Handle<UiNode> {
    TextBuilder::new(WidgetBuilder::new().with_margin(make_layer_margin(layer_index)))
        .with_text(text)
        .with_vertical_text_alignment(VerticalAlignment::Center)
        .build(ctx)
}

fn make_tooltip(ctx: &mut BuildContext, text: &str) -> Handle<UiNode> {
    if text.is_empty() {
        Handle::NONE
    } else {
        BorderBuilder::new(
            WidgetBuilder::new()
                .with_visibility(false)
                .with_foreground(Brush::Solid(Color::opaque(160, 160, 160)))
                .with_max_size(Vector2::new(250.0, f32::INFINITY))
                .with_child(
                    TextBuilder::new(WidgetBuilder::new())
                        .with_wrap(WrapMode::Word)
                        .with_text(text)
                        .build(ctx),
                ),
        )
        .build(ctx)
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
        .add_rows(vec![Row::strict(26.0)])
        .add_columns(vec![Column::strict(NAME_COLUMN_WIDTH), Column::stretch()])
        .build(ctx)
}

impl InspectorContext {
    pub fn from_object(
        object: &dyn Inspect,
        ctx: &mut BuildContext,
        definition_container: Rc<PropertyEditorDefinitionContainer>,
        environment: Option<Rc<dyn InspectorEnvironment>>,
        sync_flag: u64,
        layer_index: usize,
    ) -> Self {
        let mut entries = Vec::new();

        let editors = object
            .properties()
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
        object: &dyn Inspect,
        ui: &mut UserInterface,
        layer_index: usize,
    ) -> Result<(), Vec<InspectorError>> {
        let mut sync_errors = Vec::new();

        for info in object.properties() {
            if let Some(constructor) = self
                .property_definitions
                .definitions()
                .get(&info.value.type_id())
            {
                let ctx = PropertyEditorMessageContext {
                    sync_flag: self.sync_flag,
                    instance: self.find_property_editor(info.name),
                    ui,
                    property_info: &info,
                    definition_container: self.property_definitions.clone(),
                    layer_index,
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

        if sync_errors.is_empty() {
            Ok(())
        } else {
            Err(sync_errors)
        }
    }

    pub fn property_editors(&self) -> impl Iterator<Item = &ContextEntry> + '_ {
        self.entries.iter()
    }

    pub fn find_property_editor(&self, name: &str) -> Handle<UiNode> {
        if let Some(property_editor) = self
            .entries
            .iter()
            .find(|e| e.property_name == name)
            .map(|e| e.property_editor)
        {
            return property_editor;
        }

        Default::default()
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
            for entry in self.context.entries.iter() {
                if message.destination() == entry.property_editor {
                    if let Some(args) = entry.property_editor_definition.translate_message(
                        &entry.property_name,
                        entry.property_owner_type_id,
                        message,
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
