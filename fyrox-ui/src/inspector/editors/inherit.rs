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

//! Property editor for [`InheritableVariable`]. It acts like a proxy to inner property, but also
//!  adds a special "revert" button that is used to revert value to its parent's value.

use crate::{
    button::{Button, ButtonBuilder, ButtonMessage},
    core::{
        pool::Handle, reflect::prelude::*, variable::InheritableVariable, visitor::prelude::*,
        PhantomDataSendSync,
    },
    define_widget_deref,
    grid::{Column, GridBuilder, Row},
    image::ImageBuilder,
    inspector::{
        editors::{
            PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext, PropertyEditorTranslationContext,
        },
        FieldAction, InheritableAction, Inspector, InspectorBuilder, InspectorContext,
        InspectorContextArgs, InspectorError, InspectorMessage, PropertyChanged,
    },
    message::{MessageData, UiMessage},
    resources::REVERT_ICON,
    style::{resource::StyleResourceExt, Style},
    utils::make_simple_tooltip,
    widget::WidgetBuilder,
    BuildContext, Control, MessageDirection, Thickness, UiNode, UserInterface, VerticalAlignment,
    Widget, WidgetMessage,
};
use fyrox_graph::SceneGraph;
use std::{
    any::TypeId,
    fmt::{Debug, Formatter},
};

#[derive(Debug, Clone, PartialEq)]
pub enum InheritablePropertyEditorMessage {
    Revert,
    Modified(bool),
    PropertyChanged(PropertyChanged),
}
impl MessageData for InheritablePropertyEditorMessage {}

#[derive(Debug, Clone, Visit, Reflect)]
#[reflect(type_uuid = "d5dce72c-a54b-4754-96a3-2e923eaa802f")]
#[reflect(derived_type = "UiNode")]
pub struct InheritablePropertyEditor {
    widget: Widget,
    revert: Handle<Button>,
    inspector: Handle<UiNode>,
}

define_widget_deref!(InheritablePropertyEditor);

impl Control for InheritablePropertyEditor {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(ButtonMessage::Click) = message.data_from(self.revert) {
            ui.post(self.handle, InheritablePropertyEditorMessage::Revert);
        } else if let Some(InheritablePropertyEditorMessage::Modified(modified)) = message.data() {
            if message.destination() == self.handle {
                ui.send(self.revert, WidgetMessage::Visibility(*modified));
            }
        } else if let Some(InspectorMessage::PropertyChanged(property_changed)) =
            message.data_from(self.inspector)
        {
            ui.post(
                self.handle(),
                InheritablePropertyEditorMessage::PropertyChanged(property_changed.clone()),
            )
        }
    }
}

struct InheritablePropertyEditorBuilder {
    widget_builder: WidgetBuilder,
    inspector: Handle<UiNode>,
    container: Handle<UiNode>,
    modified: bool,
}

impl InheritablePropertyEditorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            inspector: Handle::NONE,
            container: Handle::NONE,
            modified: false,
        }
    }

    pub fn with_inner_editor(mut self, inner_editor: Handle<UiNode>) -> Self {
        self.inspector = inner_editor;
        self
    }

    pub fn with_container(mut self, container: Handle<UiNode>) -> Self {
        self.container = container;
        self
    }

    pub fn with_modified(mut self, modified: bool) -> Self {
        self.modified = modified;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<InheritablePropertyEditor> {
        let revert = ButtonBuilder::new(
            WidgetBuilder::new()
                .with_visibility(self.modified)
                .with_width(22.0)
                .with_height(22.0)
                .with_vertical_alignment(VerticalAlignment::Top)
                .with_tooltip(make_simple_tooltip(ctx, "Revert To Parent"))
                .with_margin(Thickness::uniform(1.0))
                .on_column(1),
        )
        .with_content(
            ImageBuilder::new(
                WidgetBuilder::new()
                    .with_background(ctx.style.property(Style::BRUSH_BRIGHTEST))
                    .with_margin(Thickness::uniform(3.0))
                    .with_width(16.0)
                    .with_height(16.0),
            )
            .with_opt_texture(REVERT_ICON.clone())
            .build(ctx),
        )
        .build(ctx);

        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(self.container)
                .with_child(revert),
        )
        .add_row(Row::auto())
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .build(ctx);

        ctx.add(InheritablePropertyEditor {
            widget: self.widget_builder.with_child(grid).build(ctx),
            revert,
            inspector: self.inspector,
        })
    }
}

pub struct InheritablePropertyEditorDefinition<T>
where
    T: Reflect + Clone + PartialEq,
{
    #[allow(dead_code)]
    phantom: PhantomDataSendSync<T>,
}

impl<T> InheritablePropertyEditorDefinition<T>
where
    T: Reflect + Clone + PartialEq,
{
    pub fn new() -> Self {
        Self {
            phantom: Default::default(),
        }
    }
}

impl<T> Debug for InheritablePropertyEditorDefinition<T>
where
    T: Reflect + Clone + PartialEq,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "InheritablePropertyEditorDefinition")
    }
}

impl<T> PropertyEditorDefinition for InheritablePropertyEditorDefinition<T>
where
    T: Reflect + Clone + PartialEq,
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<InheritableVariable<T>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let property_info = ctx.property_info;

        let value = property_info.cast_value::<InheritableVariable<T>>()?;

        let inspector_context = InspectorContext::from_object(InspectorContextArgs {
            object: value,
            ctx: ctx.build_context,
            definition_container: ctx.definition_container.clone(),
            environment: ctx.environment.clone(),
            layer_index: ctx.layer_index + 1,
            generate_property_string_values: ctx.generate_property_string_values,
            filter: ctx.filter,
            name_column_width: ctx.name_column_width,
            hide_name_column: true,
            base_path: ctx.base_path.clone(),
            has_parent_object: ctx.has_parent_object,
        });

        let editor = InspectorBuilder::new(WidgetBuilder::new())
            .with_context(inspector_context)
            .build(ctx.build_context)
            .to_base();

        let wrapper = InheritablePropertyEditorBuilder::new(WidgetBuilder::new())
            .with_container(editor)
            .with_inner_editor(editor)
            .with_modified(
                ctx.has_parent_object
                    && ctx
                        .property_info
                        .cast_value::<InheritableVariable<T>>()?
                        .is_modified(),
            )
            .build(ctx.build_context)
            .to_base();

        Ok(PropertyEditorInstance::Simple { editor: wrapper })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let instance = ctx
            .ui
            .node(ctx.instance)
            .cast::<InheritablePropertyEditor>()
            .unwrap();

        let is_modified = ctx.has_parent_object
            && ctx
                .property_info
                .cast_value::<InheritableVariable<T>>()?
                .is_modified();
        ctx.ui.send_sync(
            instance.handle,
            InheritablePropertyEditorMessage::Modified(is_modified),
        );

        let property_info = ctx.property_info;

        let value = property_info.cast_value::<InheritableVariable<T>>()?;

        let mut error_group = Vec::new();

        let inspector_context = ctx
            .ui
            .node(instance.inspector)
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
        if let Some(InheritablePropertyEditorMessage::Revert) = ctx.message.data() {
            return Some(PropertyChanged {
                name: ctx.name.to_string(),
                action: FieldAction::InheritableAction(InheritableAction::Revert),
            });
        }

        if let Some(InheritablePropertyEditorMessage::PropertyChanged(msg)) = ctx.message.data() {
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
