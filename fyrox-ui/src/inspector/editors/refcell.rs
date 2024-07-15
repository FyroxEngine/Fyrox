use crate::{
    core::{reflect::prelude::*, reflect::FieldValue},
    inspector::{
        editors::{
            PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext, PropertyEditorTranslationContext,
        },
        InspectorError, PropertyChanged,
    },
    message::UiMessage,
};
use fyrox_core::PhantomDataSendSync;
use std::{
    any::TypeId,
    cell::RefCell,
    fmt::{Debug, Formatter},
};

pub struct RefCellPropertyEditorDefinition<T>
where
    T: FieldValue,
{
    #[allow(dead_code)]
    phantom: PhantomDataSendSync<T>,
}

impl<T> RefCellPropertyEditorDefinition<T>
where
    T: FieldValue,
{
    pub fn new() -> Self {
        Self {
            phantom: Default::default(),
        }
    }
}

impl<T> Debug for RefCellPropertyEditorDefinition<T>
where
    T: Reflect + FieldValue,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "RefCellPropertyEditorDefinition")
    }
}

fn make_proxy<'a, 'b, 'c, T>(
    value: &'a T,
    property_info: &'b FieldInfo<'a, 'c>,
) -> Result<FieldInfo<'a, 'c>, InspectorError>
where
    T: Reflect + FieldValue,
    'b: 'a,
{
    Ok(FieldInfo {
        owner_type_id: TypeId::of::<T>(),
        name: property_info.name,
        display_name: property_info.display_name,
        value,
        reflect_value: value,
        read_only: property_info.read_only,
        immutable_collection: property_info.immutable_collection,
        min_value: property_info.min_value,
        max_value: property_info.max_value,
        step: property_info.step,
        precision: property_info.precision,
        description: property_info.description,
        type_name: property_info.type_name,
        doc: property_info.doc,
    })
}

impl<T> PropertyEditorDefinition for RefCellPropertyEditorDefinition<T>
where
    T: Reflect + FieldValue,
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<RefCell<T>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        if let Some(definition) = ctx
            .definition_container
            .definitions()
            .get(&TypeId::of::<T>())
        {
            let value = ctx.property_info.cast_value::<RefCell<T>>()?.borrow();
            definition
                .property_editor
                .create_instance(PropertyEditorBuildContext {
                    build_context: ctx.build_context,
                    property_info: &make_proxy::<T>(&value, ctx.property_info)?,
                    environment: ctx.environment.clone(),
                    definition_container: ctx.definition_container.clone(),
                    sync_flag: ctx.sync_flag,
                    layer_index: ctx.layer_index,
                    generate_property_string_values: ctx.generate_property_string_values,
                    filter: ctx.filter,
                    name_column_width: ctx.name_column_width,
                })
        } else {
            Err(InspectorError::Custom("No editor!".to_string()))
        }
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        if let Some(definition) = ctx
            .definition_container
            .definitions()
            .get(&TypeId::of::<T>())
        {
            let value = ctx.property_info.cast_value::<RefCell<T>>()?.borrow();
            return definition
                .property_editor
                .create_message(PropertyEditorMessageContext {
                    property_info: &make_proxy::<T>(&value, ctx.property_info)?,
                    environment: ctx.environment.clone(),
                    definition_container: ctx.definition_container.clone(),
                    sync_flag: ctx.sync_flag,
                    instance: ctx.instance,
                    layer_index: ctx.layer_index,
                    ui: ctx.ui,
                    generate_property_string_values: ctx.generate_property_string_values,
                    filter: ctx.filter,
                    name_column_width: ctx.name_column_width,
                });
        }

        Err(InspectorError::Custom("No editor!".to_string()))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        // Try translate other messages using inner property editor.
        if let Some(definition) = ctx
            .definition_container
            .definitions()
            .get(&TypeId::of::<T>())
        {
            return definition.property_editor.translate_message(
                PropertyEditorTranslationContext {
                    environment: ctx.environment.clone(),
                    name: ctx.name,
                    owner_type_id: ctx.owner_type_id,
                    message: ctx.message,
                    definition_container: ctx.definition_container.clone(),
                },
            );
        }

        None
    }
}
