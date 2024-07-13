use crate::fyrox::{
    core::{
        log::Log, parking_lot::Mutex, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        uuid_provider, visitor::prelude::*,
    },
    engine::SerializationContext,
    graph::BaseSceneGraph,
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        define_constructor,
        dropdown_list::{DropdownList, DropdownListMessage},
        grid::{GridBuilder, GridDimension},
        inspector::{
            editors::{
                PropertyEditorBuildContext, PropertyEditorDefinition,
                PropertyEditorDefinitionContainer, PropertyEditorInstance,
                PropertyEditorMessageContext, PropertyEditorTranslationContext,
            },
            make_expander_container, FieldKind, Inspector, InspectorBuilder, InspectorContext,
            InspectorEnvironment, InspectorError, InspectorMessage, PropertyChanged,
            PropertyFilter,
        },
        message::{MessageDirection, UiMessage},
        text::TextBuilder,
        utils::make_simple_tooltip,
        widget::{Widget, WidgetBuilder},
        BuildContext, Control, UiNode, UserInterface,
    },
    script::Script,
};
use crate::{
    gui::make_dropdown_list_option,
    inspector::EditorEnvironment,
    send_sync_message,
    settings::{general::ScriptEditor, SettingsData},
    DropdownListBuilder, MSG_SYNC_FLAG,
};
use std::{
    any::TypeId,
    cell::Cell,
    ops::{Deref, DerefMut},
    sync::Arc,
};

#[derive(Debug, PartialEq, Clone)]
pub enum ScriptPropertyEditorMessage {
    Value(Option<Uuid>),
    PropertyChanged(PropertyChanged),
}

impl ScriptPropertyEditorMessage {
    define_constructor!(ScriptPropertyEditorMessage:Value => fn value(Option<Uuid>), layout: false);
    define_constructor!(ScriptPropertyEditorMessage:PropertyChanged => fn property_changed(PropertyChanged), layout: false);
}

#[derive(Clone, Debug, Visit, Reflect, ComponentProvider)]
pub struct ScriptPropertyEditor {
    widget: Widget,
    inspector: Handle<UiNode>,
    variant_selector: Handle<UiNode>,
    open_in_ide_button: Handle<UiNode>,
    selected_script_uuid: Option<Uuid>,
    need_context_update: Cell<bool>,
}

impl Deref for ScriptPropertyEditor {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl DerefMut for ScriptPropertyEditor {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

uuid_provider!(ScriptPropertyEditor = "f43c3bfb-8b39-4cc0-be77-04141a45822e");

impl Control for ScriptPropertyEditor {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(ScriptPropertyEditorMessage::Value(id)) = message.data() {
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::ToWidget
                && self.selected_script_uuid != *id
            {
                self.selected_script_uuid = *id;
                self.need_context_update.set(true);
                ui.send_message(message.reverse());
            }
        } else if let Some(InspectorMessage::PropertyChanged(property_changed)) =
            message.data::<InspectorMessage>()
        {
            if message.destination() == self.inspector
                && message.direction() == MessageDirection::FromWidget
            {
                ui.send_message(ScriptPropertyEditorMessage::property_changed(
                    self.handle(),
                    MessageDirection::FromWidget,
                    property_changed.clone(),
                ))
            }
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.open_in_ide_button
                && message.direction() == MessageDirection::FromWidget
            {
                if let Some(uuid) = self.selected_script_uuid {
                    if let Some(selected_item) = ui
                        .node(self.variant_selector)
                        .cast::<DropdownList>()
                        .expect("Must be DropdownList")
                        .items
                        .iter()
                        .map(|el| {
                            ui.node(*el)
                                .user_data_cloned::<(Uuid, Option<String>)>()
                                .expect("Must be script (UUID, Option<String>)")
                        })
                        .find(|el| el.0 == uuid)
                    {
                        let (_, script_path) = selected_item;

                        let cd = std::env::current_dir().expect("Must be current directory");

                        if let (cd, Some(script_path)) = (cd, script_path) {
                            if let Some(cd) = cd.to_str() {
                                let script_full_path = format!("{cd}/{script_path}");

                                let settings = SettingsData::load();

                                let script_editor = settings
                                    .expect("Must be editor settings data")
                                    .general
                                    .script_editor;

                                let script_editor = match &script_editor {
                                    ScriptEditor::VSCode => {
                                        #[cfg(target_os = "macos")]
                                        let app_name = "Visual Studio Code";
                                        #[cfg(not(target_os = "macos"))]
                                        let app_name = "code";

                                        Some(app_name)
                                    }
                                    ScriptEditor::XCode => Some("xcode"),
                                    ScriptEditor::Emacs => Some("emacs"),
                                    ScriptEditor::SystemDefault => None,
                                };

                                let open_result = if let Some(editor) = script_editor {
                                    open::with(&script_full_path, editor)
                                } else {
                                    open::that(&script_full_path)
                                };

                                if let Err(e) = open_result {
                                    Log::err(format!(
                                        "Error opening script {script_full_path} in external editor: {e}"
                                    ))
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(DropdownListMessage::SelectionChanged(Some(i))) = message.data() {
            if message.destination() == self.variant_selector
                && message.direction() == MessageDirection::FromWidget
            {
                let selected_item = ui
                    .node(self.variant_selector)
                    .cast::<DropdownList>()
                    .expect("Must be DropdownList")
                    .items[*i];

                let new_selected_script_data = ui
                    .node(selected_item)
                    .user_data_cloned::<(Uuid, Option<String>)>()
                    .expect("Must be script (UUID, Option<String>)");

                ui.send_message(ScriptPropertyEditorMessage::value(
                    self.handle(),
                    MessageDirection::ToWidget,
                    Some(new_selected_script_data.0),
                ));
            }
        }
    }
}

pub struct ScriptPropertyEditorBuilder {
    widget_builder: WidgetBuilder,
}

impl ScriptPropertyEditorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self { widget_builder }
    }

    pub fn build(
        self,
        open_in_ide_button: Handle<UiNode>,
        variant_selector: Handle<UiNode>,
        script_uuid: Option<Uuid>,
        environment: Option<Arc<dyn InspectorEnvironment>>,
        sync_flag: u64,
        layer_index: usize,
        generate_property_string_values: bool,
        filter: PropertyFilter,
        script: &Option<Script>,
        definition_container: Arc<PropertyEditorDefinitionContainer>,
        name_column_width: f32,
        ctx: &mut BuildContext,
    ) -> Handle<UiNode> {
        let context = script.as_ref().map(|script| {
            InspectorContext::from_object(
                script,
                ctx,
                definition_container,
                environment,
                sync_flag,
                layer_index,
                generate_property_string_values,
                filter,
                name_column_width,
            )
        });

        let inspector = InspectorBuilder::new(WidgetBuilder::new())
            .with_opt_context(context)
            .build(ctx);

        ctx.add_node(UiNode::new(ScriptPropertyEditor {
            widget: self
                .widget_builder
                .with_preview_messages(true)
                .with_child(inspector)
                .build(),
            selected_script_uuid: script_uuid,
            variant_selector,
            open_in_ide_button,
            inspector,
            need_context_update: Cell::new(false),
        }))
    }
}

fn create_items(
    serialization_context: Arc<SerializationContext>,
    ctx: &mut BuildContext,
) -> Vec<Handle<UiNode>> {
    let mut items = vec![{
        let empty = make_dropdown_list_option(ctx, "<No Script>");
        ctx[empty].user_data = Some(Arc::new(Mutex::new((
            Uuid::default(),
            Option::<String>::None,
        ))));
        empty
    }];

    items.extend(serialization_context.script_constructors.map().iter().map(
        |(type_uuid, constructor)| {
            let item = make_dropdown_list_option(ctx, &constructor.name);

            ctx[item].user_data = Some(Arc::new(Mutex::new((
                *type_uuid,
                Some(constructor.source_path.to_string()),
            ))));

            item
        },
    ));

    items
}

fn selected_script(
    serialization_context: Arc<SerializationContext>,
    value: &Option<Script>,
) -> Option<usize> {
    value
        .as_ref()
        .and_then(|s| {
            serialization_context
                .script_constructors
                .map()
                .iter()
                .position(|(type_uuid, _)| *type_uuid == s.id())
        })
        .map(|n| {
            // Because the list has `<No Script>` element
            n + 1
        })
}

fn fetch_script_definitions(
    instance: Handle<UiNode>,
    ui: &mut UserInterface,
) -> Option<Vec<Handle<UiNode>>> {
    let instance_ref = ui
        .node(instance)
        .cast::<ScriptPropertyEditor>()
        .expect("Must be ScriptPropertyEditor!");

    let environment = ui
        .node(instance_ref.inspector)
        .cast::<Inspector>()
        .expect("Must be Inspector!")
        .context()
        .environment
        .clone();

    let editor_environment = EditorEnvironment::try_get_from(&environment);

    editor_environment.map(|e| create_items(e.serialization_context.clone(), &mut ui.build_ctx()))
}

#[derive(Debug)]
pub struct ScriptPropertyEditorDefinition {}

impl PropertyEditorDefinition for ScriptPropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Option<Script>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<Option<Script>>()?;

        let environment = EditorEnvironment::try_get_from(&ctx.environment)
            .expect("Must have editor environment!");

        let items = create_items(environment.serialization_context.clone(), ctx.build_context);

        let variant_selector = DropdownListBuilder::new(WidgetBuilder::new())
            .with_selected(
                selected_script(environment.serialization_context.clone(), value).unwrap_or(0),
            )
            .with_items(items)
            .build(ctx.build_context);

        let open_in_ide = ButtonBuilder::new(
            WidgetBuilder::new()
                .on_column(1)
                .with_tooltip(make_simple_tooltip(ctx.build_context, "Open in IDE")),
        )
        .with_content(
            TextBuilder::new(WidgetBuilder::new())
                .with_text("Edit...")
                .build(ctx.build_context),
        )
        .build(ctx.build_context);

        let script_selector_panel = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(variant_selector)
                .with_child(open_in_ide),
        )
        .add_row(GridDimension::stretch())
        .add_column(GridDimension::stretch())
        .add_column(GridDimension::auto())
        .build(ctx.build_context);

        let editor;
        let container = make_expander_container(
            ctx.layer_index,
            ctx.property_info.display_name,
            ctx.property_info.description,
            script_selector_panel,
            {
                editor = ScriptPropertyEditorBuilder::new(WidgetBuilder::new()).build(
                    open_in_ide,
                    variant_selector,
                    value.as_ref().map(|s| s.id()),
                    ctx.environment.clone(),
                    ctx.sync_flag,
                    ctx.layer_index,
                    ctx.generate_property_string_values,
                    ctx.filter,
                    value,
                    ctx.definition_container.clone(),
                    ctx.name_column_width,
                    ctx.build_context,
                );
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
        let value = ctx.property_info.cast_value::<Option<Script>>()?;

        let new_script_definitions_items = fetch_script_definitions(ctx.instance, ctx.ui);

        let instance_ref = ctx
            .ui
            .node(ctx.instance)
            .cast::<ScriptPropertyEditor>()
            .expect("Must be EnumPropertyEditor!");

        let editor_environment =
            EditorEnvironment::try_get_from(&ctx.environment).expect("Environment must be set!");

        let variant_selector_ref = ctx
            .ui
            .node(instance_ref.variant_selector)
            .cast::<DropdownList>()
            .expect("Must be a DropDownList");

        // Script list might change over time if some plugins were reloaded.
        if variant_selector_ref.items.len()
            != editor_environment
                .serialization_context
                .script_constructors
                .map()
                .values()
                .count()
        {
            if let Some(items) = new_script_definitions_items {
                send_sync_message(
                    ctx.ui,
                    DropdownListMessage::items(
                        instance_ref.variant_selector,
                        MessageDirection::ToWidget,
                        items,
                    ),
                );
                send_sync_message(
                    ctx.ui,
                    ScriptPropertyEditorMessage::value(
                        ctx.instance,
                        MessageDirection::ToWidget,
                        value.as_ref().map(|s| s.id()),
                    ),
                );
            }
        }

        if instance_ref.selected_script_uuid != value.as_ref().map(|s| s.id())
            || instance_ref.need_context_update.get()
        {
            instance_ref.need_context_update.set(false);

            send_sync_message(
                ctx.ui,
                ScriptPropertyEditorMessage::value(
                    ctx.instance,
                    MessageDirection::ToWidget,
                    value.as_ref().map(|s| s.id()),
                ),
            );

            let inspector = instance_ref.inspector;

            let context = value
                .as_ref()
                .map(|script| {
                    InspectorContext::from_object(
                        script,
                        &mut ctx.ui.build_ctx(),
                        ctx.definition_container.clone(),
                        ctx.environment.clone(),
                        ctx.sync_flag,
                        ctx.layer_index + 1,
                        ctx.generate_property_string_values,
                        ctx.filter,
                        ctx.name_column_width,
                    )
                })
                .unwrap_or_default();

            let mut msg = InspectorMessage::context(inspector, MessageDirection::ToWidget, context);
            msg.flags = MSG_SYNC_FLAG;
            Ok(Some(msg))
        } else {
            let layer_index = ctx.layer_index;
            let inspector_ctx = ctx
                .ui
                .node(instance_ref.inspector)
                .cast::<Inspector>()
                .expect("Must be Inspector!")
                .context()
                .clone();

            if let Some(value) = value.as_ref() {
                if let Err(e) = inspector_ctx.sync(
                    value,
                    ctx.ui,
                    layer_index + 1,
                    ctx.generate_property_string_values,
                    ctx.filter,
                ) {
                    Err(InspectorError::Group(e))
                } else {
                    Ok(None)
                }
            } else {
                // This is not an error, because we can actually have None variant here, because script can be unassigned.
                Ok(None)
            }
        }
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(message) = ctx.message.data::<ScriptPropertyEditorMessage>() {
                match message {
                    ScriptPropertyEditorMessage::Value(value) => {
                        if let Some(env) = EditorEnvironment::try_get_from(&ctx.environment) {
                            let script = value.and_then(|uuid| {
                                env.serialization_context
                                    .script_constructors
                                    .try_create(&uuid)
                            });

                            return Some(PropertyChanged {
                                owner_type_id: ctx.owner_type_id,
                                name: ctx.name.to_string(),
                                value: FieldKind::object(script),
                            });
                        }
                    }
                    ScriptPropertyEditorMessage::PropertyChanged(property_changed) => {
                        return Some(PropertyChanged {
                            // Mimic Option<Script> path by adding `.Some@0` suffix to property path.
                            // It is needed because we're editing compound type in this editor.
                            name: ctx.name.to_string() + ".Some@0",
                            owner_type_id: ctx.owner_type_id,
                            value: FieldKind::Inspectable(Box::new(property_changed.clone())),
                        });
                    }
                }
            }
        }
        None
    }
}
