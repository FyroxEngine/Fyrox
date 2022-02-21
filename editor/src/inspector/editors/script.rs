use crate::{inspector::EditorEnvironment, DropdownListBuilder};
use fyrox::{
    core::pool::Handle,
    gui::{
        inspector::{
            editors::{
                PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
                PropertyEditorMessageContext,
            },
            make_expander_container, InspectorEnvironment, InspectorError, PropertyChanged,
        },
        message::UiMessage,
        widget::{Widget, WidgetBuilder},
        BuildContext, Control, UiNode, UserInterface,
    },
    script::Script,
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
    rc::Rc,
};

#[derive(Clone, Debug)]
pub struct ScriptPropertyEditor {
    widget: Widget,
    variant_selector: Handle<UiNode>,
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
        self.widget.handle_routed_message(ui, message)
    }
}

pub struct ScriptPropertyEditorBuilder {
    widget_builder: WidgetBuilder,
}

fn get_editor_environment(
    environment: &Option<Rc<dyn InspectorEnvironment>>,
) -> &EditorEnvironment {
    environment
        .as_ref()
        .unwrap()
        .as_any()
        .downcast_ref::<EditorEnvironment>()
        .unwrap()
}

impl ScriptPropertyEditorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self { widget_builder }
    }

    pub fn build(self, variant_selector: Handle<UiNode>, ctx: &mut BuildContext) -> Handle<UiNode> {
        ctx.add_node(UiNode::new(ScriptPropertyEditor {
            widget: self.widget_builder.build(),
            variant_selector,
        }))
    }
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

        let variant_selector =
            DropdownListBuilder::new(WidgetBuilder::new()).build(ctx.build_context);

        let editor;
        let container = make_expander_container(
            ctx.layer_index,
            ctx.property_info.display_name,
            variant_selector,
            {
                editor = ScriptPropertyEditorBuilder::new(WidgetBuilder::new())
                    .build(variant_selector, ctx.build_context);
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
        Ok(None)
    }

    fn translate_message(
        &self,
        name: &str,
        owner_type_id: TypeId,
        message: &UiMessage,
    ) -> Option<PropertyChanged> {
        None
    }
}
