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

//! Property editor for [`StyledProperty`]. It acts like a proxy to inner property, but also
//! adds a special "bind" button used to change style binding of the property.

use crate::{
    button::{ButtonBuilder, ButtonMessage},
    core::{
        pool::Handle, reflect::prelude::*, reflect::FieldValue, type_traits::prelude::*,
        visitor::prelude::*, ImmutableString, PhantomDataSendSync,
    },
    define_widget_deref,
    grid::{Column, GridBuilder, Row},
    image::ImageBuilder,
    inspector::{
        editors::{
            PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext, PropertyEditorTranslationContext,
        },
        InspectorError, PropertyChanged,
    },
    message::UiMessage,
    style::{resource::StyleResourceExt, Style, StyledProperty},
    utils::{load_image, make_simple_tooltip},
    widget::WidgetBuilder,
    BuildContext, Control, MessageDirection, Thickness, UiNode, UserInterface, VerticalAlignment,
    Widget,
};

use fyrox_graph::BaseSceneGraph;
use fyrox_texture::TextureResource;
use std::{
    any::TypeId,
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
    sync::LazyLock,
};

static BIND_ICON: LazyLock<Option<TextureResource>> =
    LazyLock::new(|| load_image(include_bytes!("../../resources/chain.png")));

#[derive(Debug, Clone, Visit, Reflect, ComponentProvider, TypeUuidProvider)]
#[type_uuid(id = "1b8fb74a-3911-4b44-bb71-1a0382ebb9a7")]
#[reflect(derived_type = "UiNode")]
pub struct StyledPropertyEditor {
    widget: Widget,
    bind: Handle<UiNode>,
    inner_editor: Handle<UiNode>,
}

define_widget_deref!(StyledPropertyEditor);

impl Control for StyledPropertyEditor {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.bind {
                // TODO. Add a window to be able to select a property to bind to.
            }
        }

        // Re-cast messages from inner editor as message from this editor.
        // If anything is listening to messages from this editor, let them hear the messages from the inner
        // editor as if they were coming from this editor, but *do not* re-cast messages to the inner editor
        // to this editor. Particularly, when the inner editor is made invisible, that does not mean that
        // this editor should be invisible.
        if message.destination() == self.inner_editor
            && message.direction == MessageDirection::FromWidget
        {
            let mut clone = message.clone();
            clone.destination = self.handle;
            ui.send_message(clone);
        }
    }
}

struct StyledPropertyEditorBuilder {
    widget_builder: WidgetBuilder,
    inner_editor: Handle<UiNode>,
    container: Handle<UiNode>,
    style_property_name: ImmutableString,
}

impl StyledPropertyEditorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            inner_editor: Handle::NONE,
            container: Handle::NONE,
            style_property_name: Default::default(),
        }
    }

    pub fn with_inner_editor(mut self, inner_editor: Handle<UiNode>) -> Self {
        self.inner_editor = inner_editor;
        self
    }

    pub fn with_container(mut self, container: Handle<UiNode>) -> Self {
        self.container = container;
        self
    }

    pub fn with_style_property_name(mut self, style_property_name: ImmutableString) -> Self {
        self.style_property_name = style_property_name;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let is_bound = !self.style_property_name.is_empty();
        let brush = if is_bound {
            ctx.style.property(Style::BRUSH_BRIGHT_BLUE)
        } else {
            ctx.style.property(Style::BRUSH_BRIGHTEST)
        };

        let tooltip = if is_bound {
            &format!("Bound To `{}` Property", self.style_property_name)
        } else {
            "Bind To Style Property"
        };

        let bind;
        let grid = GridBuilder::new(WidgetBuilder::new().with_child(self.container).with_child({
            bind = ButtonBuilder::new(
                WidgetBuilder::new()
                    .on_column(1)
                    .with_tooltip(make_simple_tooltip(ctx, tooltip))
                    .with_margin(Thickness::uniform(1.0))
                    .with_vertical_alignment(VerticalAlignment::Top)
                    .with_width(18.0)
                    .with_height(18.0),
            )
            .with_content(
                ImageBuilder::new(
                    WidgetBuilder::new()
                        .with_background(brush)
                        .with_margin(Thickness::uniform(2.0))
                        .with_width(16.0)
                        .with_height(16.0),
                )
                .with_opt_texture(BIND_ICON.clone())
                .build(ctx),
            )
            .build(ctx);
            bind
        }))
        .add_row(Row::auto())
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .build(ctx);

        ctx.add_node(UiNode::new(StyledPropertyEditor {
            widget: self.widget_builder.with_child(grid).build(ctx),
            bind,
            inner_editor: self.inner_editor,
        }))
    }
}

pub struct StyledPropertyEditorDefinition<T>
where
    T: FieldValue,
{
    #[allow(dead_code)]
    phantom: PhantomDataSendSync<T>,
}

impl<T> StyledPropertyEditorDefinition<T>
where
    T: FieldValue,
{
    pub fn new() -> Self {
        Self {
            phantom: Default::default(),
        }
    }
}

impl<T> Debug for StyledPropertyEditorDefinition<T>
where
    T: Reflect + FieldValue,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "StyledPropertyEditorDefinition")
    }
}

impl<T> PropertyEditorDefinition for StyledPropertyEditorDefinition<T>
where
    T: Reflect + FieldValue,
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<StyledProperty<T>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<StyledProperty<T>>()?;
        if let Some(definition) = ctx
            .definition_container
            .definitions()
            .get(&TypeId::of::<T>())
        {
            let property_info = ctx.property_info;

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
                value: &**value,
            };

            let instance =
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
                    })?;

            let wrapper = StyledPropertyEditorBuilder::new(WidgetBuilder::new())
                .with_container(match instance {
                    PropertyEditorInstance::Simple { editor } => editor,
                    PropertyEditorInstance::Custom { container, .. } => container,
                })
                .with_inner_editor(match instance {
                    PropertyEditorInstance::Simple { editor } => editor,
                    PropertyEditorInstance::Custom { editor, .. } => editor,
                })
                .with_style_property_name(value.name.clone())
                .build(ctx.build_context);

            Ok(match instance {
                PropertyEditorInstance::Simple { .. } => {
                    PropertyEditorInstance::Simple { editor: wrapper }
                }
                PropertyEditorInstance::Custom { .. } => PropertyEditorInstance::Custom {
                    container: wrapper,
                    editor: wrapper,
                },
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
            let instance = ctx
                .ui
                .node(ctx.instance)
                .cast::<StyledPropertyEditor>()
                .unwrap();

            let property_info = ctx.property_info;

            let value = property_info.cast_value::<StyledProperty<T>>()?;

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
                value: &**value,
            };

            return definition
                .property_editor
                .create_message(PropertyEditorMessageContext {
                    property_info: &proxy_property_info,
                    environment: ctx.environment.clone(),
                    definition_container: ctx.definition_container.clone(),
                    sync_flag: ctx.sync_flag,
                    instance: instance.inner_editor,
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
        // Try to translate other messages using inner property editor.
        if let Some(definition) = ctx
            .definition_container
            .definitions()
            .get(&TypeId::of::<T>())
        {
            let mut property_change =
                definition
                    .property_editor
                    .translate_message(PropertyEditorTranslationContext {
                        environment: ctx.environment.clone(),
                        name: ctx.name,

                        message: ctx.message,
                        definition_container: ctx.definition_container.clone(),
                    })?;

            property_change.name += ".property";

            return Some(property_change);
        }

        None
    }
}
