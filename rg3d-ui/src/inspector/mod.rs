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
        PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorDefinitionContainer,
    },
    message::{
        InspectorMessage, MessageData, MessageDirection, UiMessage, UiMessageData, WidgetMessage,
    },
    stack_panel::StackPanelBuilder,
    text::TextBuilder,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, Thickness, UINode, UserInterface, VerticalAlignment,
};
use std::any::TypeId;
use std::{
    any::Any,
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
pub struct Inspector<M: MessageData, C: Control<M, C>> {
    widget: Widget<M, C>,
    stack_panel: Handle<UINode<M, C>>,
    context: InspectorContext<M, C>,
    property_definitions: Arc<PropertyEditorDefinitionContainer<M, C>>,
}

crate::define_widget_deref!(Inspector<M, C>);

#[derive(Debug)]
pub enum InspectorError {
    CastError(CastError),
    OutOfSync,
}

impl From<CastError> for InspectorError {
    fn from(e: CastError) -> Self {
        Self::CastError(e)
    }
}

#[derive(Clone, Debug)]
pub struct ContextEntry<M: MessageData, C: Control<M, C>> {
    pub property_name: String,
    pub property_owner_type_id: TypeId,
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

fn create_section_header<M: MessageData, C: Control<M, C>>(
    ctx: &mut BuildContext<M, C>,
    text: &str,
) -> Handle<UINode<M, C>> {
    TextBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
        .with_text(text)
        .with_vertical_text_alignment(VerticalAlignment::Center)
        .build(ctx)
}

fn create_property_title<M: MessageData, C: Control<M, C>>(
    ctx: &mut BuildContext<M, C>,
    row: usize,
    text: &str,
) -> Handle<UINode<M, C>> {
    TextBuilder::new(
        WidgetBuilder::new()
            .on_row(row)
            .on_column(0)
            .with_margin(Thickness::uniform(2.0)),
    )
    .with_text(text)
    .with_vertical_text_alignment(VerticalAlignment::Center)
    .build(ctx)
}

impl<M: MessageData, C: Control<M, C>> InspectorContext<M, C> {
    pub fn from_object(
        object: &dyn Inspect,
        ctx: &mut BuildContext<M, C>,
        definition_container: &PropertyEditorDefinitionContainer<M, C>,
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
                let section = BorderBuilder::new(
                    WidgetBuilder::new()
                        .with_margin(Thickness::uniform(1.0))
                        .with_child(
                            ExpanderBuilder::new(WidgetBuilder::new())
                                .with_header(create_section_header(ctx, group))
                                .with_content(
                                    GridBuilder::new(
                                        WidgetBuilder::new()
                                            .with_children(infos.iter().enumerate().map(
                                                |(i, info)| {
                                                    create_property_title(ctx, i, info.display_name)
                                                },
                                            ))
                                            .with_children(infos.iter().enumerate().map(
                                                |(i, info)| {
                                                    if let Some(definition) = definition_container
                                                        .definitions()
                                                        .get(&info.value.type_id())
                                                    {
                                                        match definition.create_instance(
                                                            PropertyEditorBuildContext {
                                                                build_context: ctx,
                                                                property_info: info,
                                                                row: i,
                                                                column: 1,
                                                                environment: environment.clone(),
                                                            },
                                                        ) {
                                                            Ok(instance) => {
                                                                entries.push(ContextEntry {
                                                                    property_editor: instance,
                                                                    property_editor_definition:
                                                                        definition.clone(),
                                                                    property_name: info
                                                                        .name
                                                                        .to_string(),
                                                                    property_owner_type_id: info
                                                                        .owner_type_id,
                                                                });

                                                                instance
                                                            }
                                                            Err(e) => TextBuilder::new(
                                                                WidgetBuilder::new()
                                                                    .on_row(i)
                                                                    .on_column(1),
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
                                                            WidgetBuilder::new()
                                                                .on_row(i)
                                                                .on_column(1),
                                                        )
                                                        .with_wrap(WrapMode::Word)
                                                        .with_text("Property Editor Is Missing!")
                                                        .build(ctx)
                                                    }
                                                },
                                            )),
                                    )
                                    .add_rows(infos.iter().map(|_| Row::strict(25.0)).collect())
                                    .add_column(Column::strict(130.0))
                                    .add_column(Column::stretch())
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

        Self { groups }
    }

    pub fn sync(
        &self,
        object: &dyn Inspect,
        constructors: &PropertyEditorDefinitionContainer<M, C>,
        ui: &mut UserInterface<M, C>,
        sync_flag: u64,
    ) -> Result<(), InspectorError> {
        for info in object.properties() {
            if let Some(constructor) = constructors.definitions().get(&info.value.type_id()) {
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

pub struct InspectorBuilder<M: MessageData, C: Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    context: InspectorContext<M, C>,
    property_definitions: Option<Arc<PropertyEditorDefinitionContainer<M, C>>>,
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

    pub fn with_property_editor_definitions(
        mut self,
        definitions: Arc<PropertyEditorDefinitionContainer<M, C>>,
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
                .unwrap_or_else(|| Arc::new(PropertyEditorDefinitionContainer::new())),
        };
        ctx.add_node(UINode::Inspector(canvas))
    }
}
