use crate::{gui::make_dropdown_list_option, inspector::EditorEnvironment, DropdownListBuilder};
use fyrox::{
    core::{pool::Handle, uuid::Uuid},
    gui::{
        define_constructor,
        dropdown_list::{DropdownList, DropdownListMessage},
        inspector::{
            editors::{
                PropertyEditorBuildContext, PropertyEditorDefinition,
                PropertyEditorDefinitionContainer, PropertyEditorInstance,
                PropertyEditorMessageContext, PropertyEditorTranslationContext,
            },
            make_expander_container, FieldKind, Inspector, InspectorBuilder, InspectorContext,
            InspectorEnvironment, InspectorError, InspectorMessage, PropertyChanged,
        },
        message::{MessageDirection, UiMessage},
        widget::{Widget, WidgetBuilder},
        BuildContext, Control, UiNode, UserInterface,
    },
    script::{Script, ScriptDefinitionStorage},
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::Arc,
};

#[derive(Debug, PartialEq)]
pub enum ScriptPropertyEditorMessage {
    Value(Option<Uuid>),
    PropertyChanged(PropertyChanged),
}

impl ScriptPropertyEditorMessage {
    define_constructor!(ScriptPropertyEditorMessage:Value => fn value(Option<Uuid>), layout: false);
    define_constructor!(ScriptPropertyEditorMessage:PropertyChanged => fn property_changed(PropertyChanged), layout: false);
}

#[derive(Clone, Debug)]
pub struct ScriptPropertyEditor {
    widget: Widget,
    inspector: Handle<UiNode>,
    variant_selector: Handle<UiNode>,
    selected_script_uuid: Option<Uuid>,
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

impl Control for ScriptPropertyEditor {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(ScriptPropertyEditorMessage::Value(id)) = message.data() {
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::ToWidget
            {
                if self.selected_script_uuid != id.clone() {
                    self.selected_script_uuid = id.clone();
                    ui.send_message(message.reverse());
                }
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
        if let Some(DropdownListMessage::SelectionChanged(Some(i))) = message.data() {
            if message.destination() == self.variant_selector
                && message.direction() == MessageDirection::FromWidget
            {
                let selected_item = ui
                    .node(self.variant_selector)
                    .cast::<DropdownList>()
                    .expect("Must be DropdownList")
                    .items()[*i];

                let new_selected_script_uuid = ui
                    .node(selected_item)
                    .user_data_ref::<Uuid>()
                    .expect("Must be script UUID")
                    .clone();

                ui.send_message(ScriptPropertyEditorMessage::value(
                    self.handle(),
                    MessageDirection::ToWidget,
                    Some(new_selected_script_uuid),
                ));
            }
        }
    }
}

pub struct ScriptPropertyEditorBuilder {
    widget_builder: WidgetBuilder,
}

fn get_editor_environment(
    environment: &Option<Rc<dyn InspectorEnvironment>>,
) -> Option<&EditorEnvironment> {
    environment
        .as_ref()
        .and_then(|e| e.as_any().downcast_ref::<EditorEnvironment>())
}

impl ScriptPropertyEditorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self { widget_builder }
    }

    pub fn build(
        self,
        variant_selector: Handle<UiNode>,
        script_uuid: Option<Uuid>,
        environment: Option<Rc<dyn InspectorEnvironment>>,
        sync_flag: u64,
        layer_index: usize,
        script: &Option<Script>,
        definition_container: Rc<PropertyEditorDefinitionContainer>,
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
            inspector,
        }))
    }
}

fn create_items(
    definition_containers: &[Arc<ScriptDefinitionStorage>],
    ctx: &mut BuildContext,
) -> Vec<Handle<UiNode>> {
    let mut items = vec![{
        let empty = make_dropdown_list_option(ctx, "<No Script>");
        ctx[empty].user_data = Some(Rc::new(Uuid::default()));
        empty
    }];

    items.extend(
        definition_containers
            .iter()
            .flat_map(|c| c.iter())
            .map(|d| {
                let item = make_dropdown_list_option(ctx, &d.name);
                ctx[item].user_data = Some(Rc::new(d.type_uuid.clone()));
                item
            }),
    );

    items
}

fn selected_script(
    definition_containers: &[Arc<ScriptDefinitionStorage>],
    value: &Option<Script>,
) -> Option<usize> {
    value.as_ref().and_then(|s| {
        definition_containers
            .iter()
            .flat_map(|c| c.iter())
            .position(|d| d.type_uuid == s.type_uuid())
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

    let editor_environment = get_editor_environment(&environment);

    editor_environment.map(|e| create_items(&e.script_definitions, &mut ui.build_ctx()))
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

        let environment =
            get_editor_environment(&ctx.environment).expect("Must have editor environment!");

        let items = create_items(&environment.script_definitions, ctx.build_context);

        let variant_selector = DropdownListBuilder::new(WidgetBuilder::new())
            .with_selected(selected_script(&environment.script_definitions, value).unwrap_or(0))
            .with_items(items)
            .build(ctx.build_context);

        let editor;
        let container = make_expander_container(
            ctx.layer_index,
            ctx.property_info.display_name,
            variant_selector,
            {
                editor = ScriptPropertyEditorBuilder::new(WidgetBuilder::new()).build(
                    variant_selector,
                    value.as_ref().map(|s| s.type_uuid()),
                    ctx.environment.clone(),
                    ctx.sync_flag,
                    ctx.layer_index,
                    value,
                    ctx.definition_container.clone(),
                    ctx.build_context,
                );
                editor
            },
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

        let environment = ctx
            .ui
            .node(instance_ref.inspector)
            .cast::<Inspector>()
            .expect("Must be Inspector!")
            .context()
            .environment
            .clone();

        let editor_environment = get_editor_environment(&environment);

        let variant_selector_ref = ctx
            .ui
            .node(instance_ref.variant_selector)
            .cast::<DropdownList>()
            .expect("Must be a DropDownList");

        // Script list might change over time if some plugins were reloaded.
        if Some(variant_selector_ref.items().len())
            != editor_environment
                .map(|e| e.script_definitions.iter().flat_map(|c| c.iter()).count())
        {
            if let Some(items) = new_script_definitions_items {
                ctx.ui.send_message(DropdownListMessage::items(
                    instance_ref.variant_selector,
                    MessageDirection::ToWidget,
                    items,
                ));
                ctx.ui.send_message(ScriptPropertyEditorMessage::value(
                    ctx.instance,
                    MessageDirection::ToWidget,
                    value.as_ref().map(|s| s.type_uuid()).clone(),
                ))
            }
        }

        if instance_ref.selected_script_uuid != value.as_ref().map(|s| s.type_uuid()) {
            ctx.ui.send_message(ScriptPropertyEditorMessage::value(
                ctx.instance,
                MessageDirection::ToWidget,
                value.as_ref().map(|s| s.type_uuid()).clone(),
            ));

            let inspector = instance_ref.inspector;

            let context = InspectorContext::from_object(
                value,
                &mut ctx.ui.build_ctx(),
                ctx.definition_container.clone(),
                environment,
                ctx.sync_flag,
                ctx.layer_index + 1,
            );

            Ok(Some(InspectorMessage::context(
                inspector,
                MessageDirection::ToWidget,
                context,
            )))
        } else {
            let layer_index = ctx.layer_index;
            let inspector_ctx = ctx
                .ui
                .node(instance_ref.inspector)
                .cast::<Inspector>()
                .expect("Must be Inspector!")
                .context()
                .clone();

            if let Err(e) = inspector_ctx.sync(value, ctx.ui, layer_index + 1) {
                Err(InspectorError::Group(e))
            } else {
                Ok(None)
            }
        }
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(message) = ctx.message.data::<ScriptPropertyEditorMessage>() {
                match message {
                    ScriptPropertyEditorMessage::Value(value) => {
                        if let Some(env) = get_editor_environment(&ctx.environment) {
                            let script = value.and_then(|uuid| {
                                env.script_definitions
                                    .iter()
                                    .flat_map(|s| s.iter())
                                    .find(|d| d.type_uuid == uuid)
                                    .map(|d| Script((d.constructor)()))
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
                            name: ctx.name.to_string(),
                            owner_type_id: ctx.owner_type_id,
                            value: FieldKind::Inspectable(Box::new(property_changed.clone())),
                        })
                    }
                }
            }
        }
        None
    }
}
