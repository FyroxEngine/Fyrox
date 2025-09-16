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
use fyrox_core::pool::NodeVariant;
use crate::{
    core::{
        pool::Handle, reflect::prelude::*, type_traits::prelude::*, uuid_provider,
        visitor::prelude::*, PhantomDataSendSync,
    },
    define_constructor,
    inspector::{
        editors::{
            PropertyEditorBuildContext, PropertyEditorDefinition,
            PropertyEditorDefinitionContainer, PropertyEditorInstance,
            PropertyEditorMessageContext, PropertyEditorTranslationContext,
        },
        make_expander_container, CollectionChanged, FieldKind, InspectorEnvironment,
        InspectorError, PropertyChanged,
    },
    inspector::{make_property_margin, PropertyFilter},
    message::{MessageDirection, UiMessage},
    stack_panel::StackPanelBuilder,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, Thickness, UiNode, UserInterface,
};

use fyrox_graph::BaseSceneGraph;
use std::sync::Arc;
use std::{
    any::TypeId,
    fmt::Debug,
    ops::{Deref, DerefMut},
};

#[derive(Clone, Debug, PartialEq, Visit, Reflect, Default)]
pub struct Item {
    pub editor_instance: PropertyEditorInstance,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ArrayEditorMessage {
    ItemChanged { index: usize, message: UiMessage },
}

impl ArrayEditorMessage {
    define_constructor!(ArrayEditorMessage:ItemChanged => fn item_changed(index: usize, message: UiMessage), layout: false);
}

#[derive(Clone, Debug, Visit, Reflect, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct ArrayEditor {
    pub widget: Widget,
    pub items: Vec<Item>,
}

impl NodeVariant<UiNode> for ArrayEditor {}

crate::define_widget_deref!(ArrayEditor);

uuid_provider!(ArrayEditor = "5c6e4785-8e2d-441f-8478-523900394b93");

impl Control for ArrayEditor {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(index) = self
            .items
            .iter()
            .position(|i| i.editor_instance.editor() == message.destination())
        {
            ui.send_message(ArrayEditorMessage::item_changed(
                self.handle,
                MessageDirection::FromWidget,
                index,
                message.clone(),
            ));
        }
    }
}

pub struct ArrayEditorBuilder<'a, T, I>
where
    T: Reflect,
    I: IntoIterator<Item = &'a T>,
{
    widget_builder: WidgetBuilder,
    collection: Option<I>,
    environment: Option<Arc<dyn InspectorEnvironment>>,
    definition_container: Option<Arc<PropertyEditorDefinitionContainer>>,
    layer_index: usize,
    generate_property_string_values: bool,
    filter: PropertyFilter,
}

fn create_item_views(items: &[Item]) -> Vec<Handle<UiNode>> {
    items
        .iter()
        .map(|item| match item.editor_instance {
            PropertyEditorInstance::Simple { editor } => editor,
            PropertyEditorInstance::Custom { container, .. } => container,
        })
        .collect::<Vec<_>>()
}

fn create_items<'a, 'b, T, I>(
    iter: I,
    environment: Option<Arc<dyn InspectorEnvironment>>,
    definition_container: Arc<PropertyEditorDefinitionContainer>,
    property_info: &FieldRef<'a, 'b>,
    ctx: &mut BuildContext,
    sync_flag: u64,
    layer_index: usize,
    generate_property_string_values: bool,
    filter: PropertyFilter,
    name_column_width: f32,
    base_path: String,
    has_parent_object: bool,
) -> Result<Vec<Item>, InspectorError>
where
    T: Reflect,
    I: IntoIterator<Item = &'a T>,
{
    let mut items = Vec::new();

    for (index, item) in iter.into_iter().enumerate() {
        if let Some(definition) = definition_container.definitions().get(&TypeId::of::<T>()) {
            let name = format!("{}[{index}]", property_info.name);
            let display_name = format!("{}[{index}]", property_info.display_name);

            let metadata = FieldMetadata {
                name: &name,
                display_name: &display_name,
                read_only: property_info.read_only,
                immutable_collection: property_info.immutable_collection,
                min_value: property_info.min_value,
                max_value: property_info.max_value,
                step: property_info.step,
                precision: property_info.precision,
                tag: property_info.tag,
                doc: property_info.doc,
            };

            let proxy_property_info = FieldRef {
                metadata: &metadata,
                value: item,
            };

            let editor =
                definition
                    .property_editor
                    .create_instance(PropertyEditorBuildContext {
                        build_context: ctx,
                        property_info: &proxy_property_info,
                        environment: environment.clone(),
                        definition_container: definition_container.clone(),
                        sync_flag,
                        layer_index: layer_index + 1,
                        generate_property_string_values,
                        filter: filter.clone(),
                        name_column_width,
                        base_path: format!("{base_path}[{index}]"),
                        has_parent_object,
                    })?;

            if let PropertyEditorInstance::Simple { editor } = editor {
                ctx[editor].set_margin(make_property_margin(layer_index + 1));
            }

            items.push(Item {
                editor_instance: editor,
            });
        } else {
            return Err(InspectorError::Custom(format!(
                "Missing property editor of type {}",
                std::any::type_name::<T>()
            )));
        }
    }

    Ok(items)
}

impl<'a, T, I> ArrayEditorBuilder<'a, T, I>
where
    T: Reflect,
    I: IntoIterator<Item = &'a T>,
{
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            collection: None,
            environment: None,
            definition_container: None,
            layer_index: 0,
            generate_property_string_values: false,
            filter: Default::default(),
        }
    }

    pub fn with_collection(mut self, collection: I) -> Self {
        self.collection = Some(collection);
        self
    }

    pub fn with_environment(mut self, environment: Option<Arc<dyn InspectorEnvironment>>) -> Self {
        self.environment = environment;
        self
    }

    pub fn with_generate_property_string_values(
        mut self,
        generate_property_string_values: bool,
    ) -> Self {
        self.generate_property_string_values = generate_property_string_values;
        self
    }

    pub fn with_definition_container(
        mut self,
        definition_container: Arc<PropertyEditorDefinitionContainer>,
    ) -> Self {
        self.definition_container = Some(definition_container);
        self
    }

    pub fn with_layer_index(mut self, layer_index: usize) -> Self {
        self.layer_index = layer_index;
        self
    }

    pub fn with_filter(mut self, filter: PropertyFilter) -> Self {
        self.filter = filter;
        self
    }

    pub fn build(
        self,
        ctx: &mut BuildContext,
        property_info: &FieldRef<'a, '_>,
        sync_flag: u64,
        name_column_width: f32,
        base_path: String,
        has_parent_object: bool,
    ) -> Result<Handle<UiNode>, InspectorError> {
        let definition_container = self
            .definition_container
            .unwrap_or_else(|| Arc::new(PropertyEditorDefinitionContainer::with_default_editors()));

        let environment = self.environment;
        let items = if let Some(collection) = self.collection {
            create_items(
                collection,
                environment,
                definition_container,
                property_info,
                ctx,
                sync_flag,
                self.layer_index + 1,
                self.generate_property_string_values,
                self.filter,
                name_column_width,
                base_path,
                has_parent_object,
            )?
        } else {
            Vec::new()
        };

        let panel =
            StackPanelBuilder::new(WidgetBuilder::new().with_children(create_item_views(&items)))
                .build(ctx);

        let ce = ArrayEditor {
            widget: self.widget_builder.with_child(panel).build(ctx),
            items,
        };

        Ok(ctx.add_node(UiNode::new(ce)))
    }
}

#[derive(Debug)]
pub struct ArrayPropertyEditorDefinition<T, const N: usize>
where
    T: Reflect,
{
    #[allow(dead_code)]
    phantom: PhantomDataSendSync<T>,
}

impl<T, const N: usize> ArrayPropertyEditorDefinition<T, N>
where
    T: Reflect,
{
    pub fn new() -> Self {
        Self::default()
    }
}

impl<T, const N: usize> Default for ArrayPropertyEditorDefinition<T, N>
where
    T: Reflect,
{
    fn default() -> Self {
        Self {
            phantom: Default::default(),
        }
    }
}

impl<T, const N: usize> PropertyEditorDefinition for ArrayPropertyEditorDefinition<T, N>
where
    T: Reflect,
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<[T; N]>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<[T; N]>()?;

        let editor;
        let container = make_expander_container(
            ctx.layer_index,
            ctx.property_info.display_name,
            ctx.property_info.doc,
            Handle::NONE,
            {
                editor = ArrayEditorBuilder::new(
                    WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                )
                .with_collection(value.iter())
                .with_environment(ctx.environment.clone())
                .with_layer_index(ctx.layer_index + 1)
                .with_definition_container(ctx.definition_container.clone())
                .with_generate_property_string_values(ctx.generate_property_string_values)
                .with_filter(ctx.filter)
                .build(
                    ctx.build_context,
                    ctx.property_info,
                    ctx.sync_flag,
                    ctx.name_column_width,
                    ctx.base_path.clone(),
                    ctx.has_parent_object,
                )?;
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
        let PropertyEditorMessageContext {
            sync_flag,
            instance,
            ui,
            layer_index,
            generate_property_string_values,
            property_info,
            filter,
            definition_container,
            environment,
            name_column_width,
            base_path,
            has_parent_object,
        } = ctx;

        let instance_ref = if let Some(instance) = ui.node(instance).cast::<ArrayEditor>() {
            instance
        } else {
            return Err(InspectorError::Custom(
                "Property editor is not ArrayEditor!".to_string(),
            ));
        };

        let value = property_info.cast_value::<[T; N]>()?;

        if let Some(definition) = definition_container.definitions().get(&TypeId::of::<T>()) {
            for (index, (item, obj)) in instance_ref
                .items
                .clone()
                .iter()
                .zip(value.iter())
                .enumerate()
            {
                let name = format!("{}[{index}]", property_info.name);
                let display_name = format!("{}[{index}]", property_info.display_name);

                let metadata = FieldMetadata {
                    name: &name,
                    display_name: &display_name,
                    read_only: property_info.read_only,
                    immutable_collection: property_info.immutable_collection,
                    min_value: property_info.min_value,
                    max_value: property_info.max_value,
                    step: property_info.step,
                    precision: property_info.precision,
                    tag: property_info.tag,
                    doc: property_info.doc,
                };

                let proxy_property_info = FieldRef {
                    metadata: &metadata,
                    value: obj,
                };

                if let Some(message) =
                    definition
                        .property_editor
                        .create_message(PropertyEditorMessageContext {
                            property_info: &proxy_property_info,
                            environment: environment.clone(),
                            definition_container: definition_container.clone(),
                            sync_flag,
                            instance: item.editor_instance.editor(),
                            layer_index: layer_index + 1,
                            ui,
                            generate_property_string_values,
                            filter: filter.clone(),
                            name_column_width,
                            base_path: format!("{base_path}[{index}]"),
                            has_parent_object,
                        })?
                {
                    ui.send_message(message.with_flags(ctx.sync_flag))
                }
            }
        }

        Ok(None)
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(ArrayEditorMessage::ItemChanged { index, message }) = ctx.message.data() {
                if let Some(definition) = ctx
                    .definition_container
                    .definitions()
                    .get(&TypeId::of::<T>())
                {
                    return Some(PropertyChanged {
                        name: ctx.name.to_string(),

                        value: FieldKind::Collection(Box::new(CollectionChanged::ItemChanged {
                            index: *index,
                            property: definition
                                .property_editor
                                .translate_message(PropertyEditorTranslationContext {
                                    environment: ctx.environment.clone(),
                                    name: "",
                                    message,
                                    definition_container: ctx.definition_container.clone(),
                                })?
                                .value,
                        })),
                    });
                }
            }
        }
        None
    }
}
