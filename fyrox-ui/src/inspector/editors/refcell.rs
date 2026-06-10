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
    core::{pool::Handle, reflect::prelude::*, PhantomDataSendSync},
    inspector::{
        editors::{
            PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext, PropertyEditorTranslationContext,
        },
        make_expander_container, FieldAction, Inspector, InspectorBuilder, InspectorContext,
        InspectorContextArgs, InspectorError, InspectorMessage, PropertyChanged,
    },
    message::{MessageDirection, UiMessage},
    widget::WidgetBuilder,
    UiNode,
};
use fyrox_graph::SceneGraph;
use std::{
    any::TypeId,
    cell::RefCell,
    fmt::{Debug, Formatter},
};

pub struct RefCellPropertyEditorDefinition<T>
where
    T: Reflect + Clone,
{
    #[allow(dead_code)]
    phantom: PhantomDataSendSync<T>,
}

impl<T> RefCellPropertyEditorDefinition<T>
where
    T: Reflect + Clone,
{
    pub fn new() -> Self {
        Self {
            phantom: Default::default(),
        }
    }
}

impl<T> Debug for RefCellPropertyEditorDefinition<T>
where
    T: Reflect + Clone,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "RefCellPropertyEditorDefinition")
    }
}

impl<T> PropertyEditorDefinition for RefCellPropertyEditorDefinition<T>
where
    T: Reflect + Clone + PartialEq,
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<RefCell<T>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<RefCell<T>>()?;

        let inspector_context = InspectorContext::from_object(InspectorContextArgs {
            object: value,
            ctx: ctx.build_context,
            definition_container: ctx.definition_container.clone(),
            environment: ctx.environment.clone(),
            layer_index: ctx.layer_index + 1,
            generate_property_string_values: ctx.generate_property_string_values,
            filter: ctx.filter,
            name_column_width: ctx.name_column_width,
            hide_name_column: false,
            base_path: ctx.base_path.clone(),
            has_parent_object: ctx.has_parent_object,
        });

        let editor;
        let container = make_expander_container(
            ctx.layer_index,
            ctx.property_info.display_name,
            ctx.property_info.doc,
            Handle::<UiNode>::NONE,
            {
                editor = InspectorBuilder::new(WidgetBuilder::new())
                    .with_context(inspector_context)
                    .build(ctx.build_context)
                    .to_base();
                editor
            },
            ctx.name_column_width,
            ctx.hide_name_column,
            ctx.build_context,
        );

        Ok(PropertyEditorInstance::Custom { container, editor })
    }

    /// Instead of creating a message to update its widget,
    /// call [InspectorContext::sync] to directly send whatever messages are necessary
    /// and return None.
    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<RefCell<T>>()?;

        let mut error_group = Vec::new();

        let inspector_context = ctx
            .ui
            .node(ctx.instance)
            .cast::<Inspector>()
            .expect("Must be Inspector!")
            .context()
            .clone();
        if let Err(e) = inspector_context.sync(
            value,
            ctx.ui,
            ctx.layer_index + 1,
            ctx.generate_property_string_values,
            ctx.filter,
            ctx.base_path.clone(),
        ) {
            error_group.extend(e)
        }

        if error_group.is_empty() {
            Ok(None)
        } else {
            Err(InspectorError::Group(error_group))
        }
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if let Some(InspectorMessage::PropertyChanged(msg)) = ctx.message.data::<InspectorMessage>()
        {
            if ctx.message.direction() == MessageDirection::FromWidget {
                return Some(PropertyChanged {
                    name: ctx.name.to_owned(),
                    action: FieldAction::InspectableAction(Box::new(msg.clone())),
                });
            }
        }

        None
    }
}
