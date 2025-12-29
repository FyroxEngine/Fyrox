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
    fyrox::{
        core::dyntype::{DynTypeConstructorContainer, DynTypeContainer},
        graph::BaseSceneGraph,
        gui::{
            core::{
                parking_lot::Mutex, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
                uuid_provider, visitor::prelude::*,
            },
            dropdown_list::{DropdownList, DropdownListBuilder, DropdownListMessage},
            grid::{GridBuilder, GridDimension},
            inspector::InspectorContextArgs,
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
            message::MessageData,
            message::{MessageDirection, UiMessage},
            utils::make_dropdown_list_option,
            widget::{Widget, WidgetBuilder},
            BuildContext, Control, UiNode, UserInterface,
        },
    },
    plugins::inspector::EditorEnvironment,
};
use fyrox::core::dyntype::DynTypeWrapper;
use std::{
    any::TypeId,
    cell::Cell,
    ops::{Deref, DerefMut},
    sync::Arc,
};

#[derive(Debug, PartialEq, Clone)]
pub enum DynTypePropertyEditorMessage {
    Value(Option<Uuid>),
    PropertyChanged(PropertyChanged),
}
impl MessageData for DynTypePropertyEditorMessage {}

#[derive(Clone, Debug, Visit, Reflect, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct DynTypePropertyEditor {
    widget: Widget,
    inspector: Handle<UiNode>,
    variant_selector: Handle<UiNode>,
    selected_dyn_type_uuid: Option<Uuid>,
    need_context_update: Cell<bool>,
}

impl Deref for DynTypePropertyEditor {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl DerefMut for DynTypePropertyEditor {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

uuid_provider!(DynTypePropertyEditor = "f43c3bfb-8b39-4cc0-be77-04141a45822e");

impl Control for DynTypePropertyEditor {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(DynTypePropertyEditorMessage::Value(id)) = message.data_for(self.handle()) {
            if self.selected_dyn_type_uuid != *id {
                self.selected_dyn_type_uuid = *id;
                self.need_context_update.set(true);
                ui.send_message(message.reverse());
            }
        } else if let Some(InspectorMessage::PropertyChanged(property_changed)) =
            message.data_from(self.inspector)
        {
            ui.post(
                self.handle(),
                DynTypePropertyEditorMessage::PropertyChanged(property_changed.clone()),
            )
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        if let Some(DropdownListMessage::Selection(Some(i))) =
            message.data_from(self.variant_selector)
        {
            let selected_item = ui
                .node(self.variant_selector)
                .cast::<DropdownList>()
                .expect("Must be DropdownList")
                .items[*i];

            let id = ui
                .node(selected_item)
                .user_data_cloned::<Uuid>()
                .expect("Must be dyn type UUID");

            ui.send(self.handle(), DynTypePropertyEditorMessage::Value(Some(id)));
        }
    }
}

pub struct DynTypePropertyEditorBuilder {
    widget_builder: WidgetBuilder,
}

impl DynTypePropertyEditorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self { widget_builder }
    }

    pub fn build(
        self,
        variant_selector: Handle<UiNode>,
        dyn_type_uuid: Option<Uuid>,
        environment: Option<Arc<dyn InspectorEnvironment>>,
        layer_index: usize,
        generate_property_string_values: bool,
        filter: PropertyFilter,
        dyntype_container: &DynTypeContainer,
        definition_container: Arc<PropertyEditorDefinitionContainer>,
        name_column_width: f32,
        has_parent_object: bool,
        ctx: &mut BuildContext,
    ) -> Handle<UiNode> {
        let context = dyntype_container.value_ref().as_ref().map(|dyn_type| {
            InspectorContext::from_object(InspectorContextArgs {
                object: *dyn_type,
                ctx,
                definition_container,
                environment,
                layer_index,
                generate_property_string_values,
                filter,
                name_column_width,
                base_path: Default::default(),
                has_parent_object,
            })
        });

        let inspector = InspectorBuilder::new(WidgetBuilder::new())
            .with_opt_context(context)
            .build(ctx);

        ctx.add_node(UiNode::new(DynTypePropertyEditor {
            widget: self
                .widget_builder
                .with_preview_messages(true)
                .with_child(inspector)
                .build(ctx),
            selected_dyn_type_uuid: dyn_type_uuid,
            variant_selector,
            inspector,
            need_context_update: Cell::new(false),
        }))
    }
}

fn create_items(
    constructors: Arc<DynTypeConstructorContainer>,
    ctx: &mut BuildContext,
) -> Vec<Handle<UiNode>> {
    let mut items = vec![{
        let empty = make_dropdown_list_option(ctx, "<Not Set>");
        ctx[empty].user_data = Some(Arc::new(Mutex::new(Uuid::default())));
        empty
    }];

    items.extend(constructors.inner().iter().map(|(type_uuid, constructor)| {
        let item = make_dropdown_list_option(ctx, &constructor.name);
        ctx[item].user_data = Some(Arc::new(Mutex::new(*type_uuid)));
        item
    }));

    items
}

fn selected_dyn_type(
    serialization_context: Arc<DynTypeConstructorContainer>,
    value: &DynTypeContainer,
) -> Option<usize> {
    value
        .value_ref()
        .as_ref()
        .and_then(|s| {
            serialization_context
                .inner()
                .iter()
                .position(|(type_uuid, _)| *type_uuid == s.type_uuid())
        })
        .map(|n| {
            // Because the list has `<Not Set>` element
            n + 1
        })
}

#[derive(Debug)]
pub struct DynTypePropertyEditorDefinition {}

impl PropertyEditorDefinition for DynTypePropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<DynTypeContainer>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<DynTypeContainer>()?;
        let environment = EditorEnvironment::try_get_from(&ctx.environment)?;

        let items = create_items(environment.dyn_type_constructors.clone(), ctx.build_context);

        let variant_selector = DropdownListBuilder::new(WidgetBuilder::new())
            .with_selected(
                selected_dyn_type(environment.dyn_type_constructors.clone(), value).unwrap_or(0),
            )
            .with_items(items)
            .build(ctx.build_context);

        let dyn_type_selector_panel =
            GridBuilder::new(WidgetBuilder::new().with_child(variant_selector))
                .add_row(GridDimension::stretch())
                .add_column(GridDimension::stretch())
                .add_column(GridDimension::auto())
                .build(ctx.build_context);

        let editor;
        let container = make_expander_container(
            ctx.layer_index,
            ctx.property_info.display_name,
            ctx.property_info.doc,
            dyn_type_selector_panel,
            {
                editor = DynTypePropertyEditorBuilder::new(WidgetBuilder::new()).build(
                    variant_selector,
                    value.value_ref().map(|s| s.type_uuid()),
                    ctx.environment.clone(),
                    ctx.layer_index,
                    ctx.generate_property_string_values,
                    ctx.filter,
                    value,
                    ctx.definition_container.clone(),
                    ctx.name_column_width,
                    ctx.has_parent_object,
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
        let value = ctx.property_info.cast_value::<DynTypeContainer>()?;

        let editor_environment = EditorEnvironment::try_get_from(&ctx.environment)?;

        let new_dyn_type_definitions_items = create_items(
            editor_environment.dyn_type_constructors.clone(),
            &mut ctx.ui.build_ctx(),
        );

        let instance_ref = ctx
            .ui
            .node(ctx.instance)
            .cast::<DynTypePropertyEditor>()
            .ok_or(InspectorError::Custom("Must be EnumPropertyEditor!".into()))?;

        let variant_selector_ref = ctx
            .ui
            .node(instance_ref.variant_selector)
            .cast::<DropdownList>()
            .ok_or(InspectorError::Custom("Must be a DropDownList".into()))?;

        // Dyn types list might change over time if some plugins were reloaded.
        if variant_selector_ref.items.len()
            != editor_environment
                .dyn_type_constructors
                .inner()
                .values()
                .count()
        {
            ctx.ui.send_sync(
                instance_ref.variant_selector,
                DropdownListMessage::Items(new_dyn_type_definitions_items),
            );
            ctx.ui.send_sync(
                ctx.instance,
                DynTypePropertyEditorMessage::Value(value.value_ref().map(|s| s.type_uuid())),
            );
        }

        if instance_ref.selected_dyn_type_uuid != value.value_ref().map(|s| s.type_uuid())
            || instance_ref.need_context_update.get()
        {
            instance_ref.need_context_update.set(false);

            ctx.ui.send_sync(
                ctx.instance,
                DynTypePropertyEditorMessage::Value(value.value_ref().map(|s| s.type_uuid())),
            );

            let inspector = instance_ref.inspector;

            let context = value
                .value_ref()
                .map(|dyn_type| {
                    InspectorContext::from_object(InspectorContextArgs {
                        object: dyn_type,
                        ctx: &mut ctx.ui.build_ctx(),
                        definition_container: ctx.definition_container.clone(),
                        environment: ctx.environment.clone(),
                        layer_index: ctx.layer_index + 1,
                        generate_property_string_values: ctx.generate_property_string_values,
                        filter: ctx.filter,
                        name_column_width: ctx.name_column_width,
                        base_path: Default::default(),
                        has_parent_object: ctx.has_parent_object,
                    })
                })
                .unwrap_or_default();

            Ok(Some(UiMessage::for_widget(
                inspector,
                InspectorMessage::Context(context),
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

            if let Some(value) = value.value_ref() {
                if let Err(e) = inspector_ctx.sync(
                    value,
                    ctx.ui,
                    layer_index + 1,
                    ctx.generate_property_string_values,
                    ctx.filter,
                    ctx.base_path.clone(),
                ) {
                    Err(InspectorError::Group(e))
                } else {
                    Ok(None)
                }
            } else {
                // This is not an error, because we can actually have None variant here, because a
                // dyn type can be unassigned.
                Ok(None)
            }
        }
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(message) = ctx.message.data::<DynTypePropertyEditorMessage>() {
                match message {
                    DynTypePropertyEditorMessage::Value(value) => {
                        if let Ok(env) = EditorEnvironment::try_get_from(&ctx.environment) {
                            let dyn_type = value
                                .and_then(|uuid| env.dyn_type_constructors.try_create(&uuid))
                                .map(DynTypeWrapper);

                            return Some(PropertyChanged {
                                name: ctx.name.to_string(),
                                value: FieldKind::object(DynTypeContainer(dyn_type)),
                            });
                        }
                    }
                    DynTypePropertyEditorMessage::PropertyChanged(property_changed) => {
                        return Some(PropertyChanged {
                            // Mimic Option<DynTypeWrapper> path by adding `.Some@0` suffix to property path.
                            // It is needed because we're editing compound type in this editor.
                            name: ctx.name.to_string() + ".Some@0",
                            value: FieldKind::Inspectable(Box::new(property_changed.clone())),
                        });
                    }
                }
            }
        }
        None
    }
}
