// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

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
            let property_info = ctx.property_info;

            let value = property_info.cast_value::<RefCell<T>>()?.borrow();

            let proxy_property_info = FieldRef {
                metadata: &FieldMetadata {
                    name: property_info.name,
                    display_name: property_info.display_name,
                    read_only: property_info.read_only,
                    immutable_collection: property_info.immutable_collection,
                    min_value: property_info.min_value,
                    max_value: property_info.max_value,
                    step: property_info.step,
                    precision: property_info.precision,
                    description: property_info.description,
                    tag: property_info.tag,
                    doc: property_info.doc,
                },
                value: &*value,
            };

            definition
                .property_editor
                .create_instance(PropertyEditorBuildContext {
                    build_context: ctx.build_context,
                    property_info: &proxy_property_info,
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
            let property_info = ctx.property_info;

            let value = ctx.property_info.cast_value::<RefCell<T>>()?.borrow();

            let proxy_property_info = FieldRef {
                metadata: &FieldMetadata {
                    name: property_info.name,
                    display_name: property_info.display_name,
                    read_only: property_info.read_only,
                    immutable_collection: property_info.immutable_collection,
                    min_value: property_info.min_value,
                    max_value: property_info.max_value,
                    step: property_info.step,
                    precision: property_info.precision,
                    description: property_info.description,
                    tag: property_info.tag,
                    doc: property_info.doc,
                },
                value: &*value,
            };

            return definition
                .property_editor
                .create_message(PropertyEditorMessageContext {
                    property_info: &proxy_property_info,
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

                    message: ctx.message,
                    definition_container: ctx.definition_container.clone(),
                },
            );
        }

        None
    }
}
