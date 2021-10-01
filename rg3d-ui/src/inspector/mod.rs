use crate::inspector::editors::PropertyEditorMessageContext;
use crate::{
    border::BorderBuilder,
    brush::Brush,
    core::{
        color::Color,
        inspect::{CastError, Inspect, PropertyInfo},
        pool::Handle,
    },
    expander::ExpanderBuilder,
    formatted_text::WrapMode,
    grid::{Column, GridBuilder, Row},
    inspector::editors::{
        Layout, PropertyEditorBuildContext, PropertyEditorDefinition,
        PropertyEditorDefinitionContainer,
    },
    message::{InspectorMessage, MessageDirection, UiMessage, UiMessageData, WidgetMessage},
    stack_panel::StackPanelBuilder,
    text::TextBuilder,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, Thickness, UiNode, UserInterface, VerticalAlignment,
};
use std::fmt::Formatter;
use std::{
    any::{Any, TypeId},
    collections::{hash_map::Entry, HashMap},
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::Arc,
};

pub mod editors;

pub trait InspectorEnvironment: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
}

#[derive(Clone)]
pub struct Inspector {
    widget: Widget,
    stack_panel: Handle<UiNode>,
    context: InspectorContext,
}

crate::define_widget_deref!(Inspector);

impl Inspector {
    pub fn context(&self) -> &InspectorContext {
        &self.context
    }
}

#[derive(Debug)]
pub enum InspectorError {
    CastError(CastError),
    OutOfSync,
    Custom(String),
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
    pub property_editor_definition: Arc<dyn PropertyEditorDefinition>,
    pub property_editor: Handle<UiNode>,
}

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

#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    section: Handle<UiNode>,
    entries: Vec<ContextEntry>,
}

#[derive(Clone)]
pub struct InspectorContext {
    groups: Vec<Group>,
    property_definitions: Arc<PropertyEditorDefinitionContainer>,
}

impl PartialEq for InspectorContext {
    fn eq(&self, other: &Self) -> bool {
        self.groups == other.groups
    }
}

impl Default for InspectorContext {
    fn default() -> Self {
        Self {
            groups: Default::default(),
            property_definitions: Arc::new(PropertyEditorDefinitionContainer::new()),
        }
    }
}

impl Debug for InspectorContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "InspectorContext")
    }
}

fn create_section_header(ctx: &mut BuildContext, text: &str) -> Handle<UiNode> {
    TextBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
        .with_text(text)
        .with_vertical_text_alignment(VerticalAlignment::Center)
        .build(ctx)
}

fn wrap_property(
    title: Handle<UiNode>,
    editor: Handle<UiNode>,
    layout: Layout,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    match layout {
        Layout::Horizontal => {
            ctx[editor].set_row(0).set_column(1);
        }
        Layout::Vertical => {
            ctx[editor].set_row(1).set_column(0);
        }
    }

    GridBuilder::new(WidgetBuilder::new().with_child(title).with_child(editor))
        .add_rows(match layout {
            Layout::Horizontal => {
                vec![Row::strict(26.0)]
            }
            Layout::Vertical => {
                vec![Row::strict(26.0), Row::stretch()]
            }
        })
        .add_columns(match layout {
            Layout::Horizontal => {
                vec![Column::strict(130.0), Column::stretch()]
            }
            Layout::Vertical => {
                vec![Column::stretch()]
            }
        })
        .build(ctx)
}

impl InspectorContext {
    pub fn from_object(
        object: &dyn Inspect,
        ctx: &mut BuildContext,
        definition_container: Arc<PropertyEditorDefinitionContainer>,
        environment: Option<Arc<dyn InspectorEnvironment>>,
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

        let mut sorted_groups = property_groups.into_iter().collect::<Vec<_>>();

        sorted_groups.sort_by_key(|(name, _)| *name);

        let groups = sorted_groups
            .iter()
            .map(|(group, infos)| {
                let mut entries = Vec::new();

                let editors = infos
                    .iter()
                    .enumerate()
                    .map(|(i, info)| {
                        if let Some(definition) = definition_container
                            .definitions()
                            .get(&info.value.type_id())
                        {
                            match definition.create_instance(PropertyEditorBuildContext {
                                build_context: ctx,
                                property_info: info,
                                environment: environment.clone(),
                                definition_container: definition_container.clone(),
                            }) {
                                Ok(instance) => {
                                    entries.push(ContextEntry {
                                        property_editor: instance.editor,
                                        property_editor_definition: definition.clone(),
                                        property_name: info.name.to_string(),
                                        property_owner_type_id: info.owner_type_id,
                                    });

                                    wrap_property(
                                        if instance.title.is_some() {
                                            instance.title
                                        } else {
                                            create_section_header(ctx, info.display_name)
                                        },
                                        instance.editor,
                                        definition.layout(),
                                        ctx,
                                    )
                                }
                                Err(e) => wrap_property(
                                    create_section_header(ctx, info.display_name),
                                    TextBuilder::new(WidgetBuilder::new().on_row(i).on_column(1))
                                        .with_wrap(WrapMode::Word)
                                        .with_vertical_text_alignment(VerticalAlignment::Center)
                                        .with_text(format!(
                                            "Unable to create property \
                                                    editor instance: Reason {:?}",
                                            e
                                        ))
                                        .build(ctx),
                                    Layout::Horizontal,
                                    ctx,
                                ),
                            }
                        } else {
                            wrap_property(
                                create_section_header(ctx, info.display_name),
                                TextBuilder::new(WidgetBuilder::new().on_row(i).on_column(1))
                                    .with_wrap(WrapMode::Word)
                                    .with_vertical_text_alignment(VerticalAlignment::Center)
                                    .with_text("Property Editor Is Missing!")
                                    .build(ctx),
                                Layout::Horizontal,
                                ctx,
                            )
                        }
                    })
                    .collect::<Vec<_>>();

                let section = BorderBuilder::new(
                    WidgetBuilder::new()
                        .with_margin(Thickness::uniform(1.0))
                        .with_child(
                            ExpanderBuilder::new(WidgetBuilder::new())
                                .with_header(create_section_header(ctx, group))
                                .with_content(
                                    StackPanelBuilder::new(
                                        WidgetBuilder::new().with_children(editors),
                                    )
                                    .build(ctx),
                                )
                                .build(ctx),
                        )
                        .with_foreground(Brush::Solid(Color::opaque(130, 130, 130))),
                )
                .build(ctx);

                Group { section, entries }
            })
            .collect::<Vec<_>>();

        Self {
            groups,
            property_definitions: definition_container,
        }
    }

    pub fn sync(
        &self,
        object: &dyn Inspect,
        ui: &mut UserInterface,
        sync_flag: u64,
    ) -> Result<(), Vec<InspectorError>> {
        let mut sync_errors = Vec::new();

        for info in object.properties() {
            if let Some(constructor) = self
                .property_definitions
                .definitions()
                .get(&info.value.type_id())
            {
                let ctx = PropertyEditorMessageContext {
                    sync_flag,
                    instance: self.find_property_editor(info.name),
                    ui,
                    property_info: &info,
                    definition_container: self.property_definitions.clone(),
                };

                match constructor.create_message(ctx) {
                    Ok(message) => {
                        if let Some(mut message) = message {
                            message.flags = sync_flag;
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
        self.groups.iter().map(|g| g.entries.iter()).flatten()
    }

    pub fn find_property_editor(&self, name: &str) -> Handle<UiNode> {
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

impl Control for Inspector {
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
        };
        ctx.add_node(UiNode::new(canvas))
    }
}
