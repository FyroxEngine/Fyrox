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

use crate::plugins::inspector::EditorEnvironment;
use crate::{
    asset::item::AssetItem,
    fyrox::{
        asset::manager::ResourceManager,
        core::{
            futures::executor::block_on, make_relative_path, pool::Handle, reflect::prelude::*,
            type_traits::prelude::*, visitor::prelude::*,
        },
        graph::BaseSceneGraph,
        gui::{
            button::{ButtonBuilder, ButtonMessage},
            define_constructor, define_widget_deref,
            grid::{Column, GridBuilder, Row},
            inspector::{
                editors::{
                    PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
                    PropertyEditorMessageContext, PropertyEditorTranslationContext,
                },
                FieldKind, InspectorError, PropertyChanged,
            },
            message::{MessageDirection, UiMessage},
            text::{TextBuilder, TextMessage},
            widget::{Widget, WidgetBuilder, WidgetMessage},
            BuildContext, Control, Thickness, UiNode, UserInterface,
        },
        scene::mesh::surface::{SurfaceData, SurfaceResource},
    },
    message::MessageSender,
    Message,
};

use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
};

#[derive(Debug, PartialEq, Clone)]
pub enum SurfaceDataPropertyEditorMessage {
    Value(SurfaceResource),
}

impl SurfaceDataPropertyEditorMessage {
    define_constructor!(SurfaceDataPropertyEditorMessage:Value => fn value(SurfaceResource), layout: false);
}

#[derive(Clone, Visit, Reflect, Debug, ComponentProvider, TypeUuidProvider)]
#[type_uuid(id = "8461a183-4fd4-4f74-a4f4-7fd8e84bf423")]
#[reflect(derived_type = "UiNode")]
#[allow(dead_code)]
pub struct SurfaceDataPropertyEditor {
    widget: Widget,
    view: Handle<UiNode>,
    data: SurfaceResource,
    text: Handle<UiNode>,
    #[visit(skip)]
    #[reflect(hidden)]
    sender: Option<MessageSender>,
    #[visit(skip)]
    #[reflect(hidden)]
    resource_manager: ResourceManager,
}

define_widget_deref!(SurfaceDataPropertyEditor);

impl Control for SurfaceDataPropertyEditor {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination == self.view {
                if let Some(sender) = self.sender.as_ref() {
                    sender.send(Message::ViewSurfaceData(self.data.clone()));
                }
            }
        } else if let Some(WidgetMessage::Drop(dropped)) = message.data() {
            if message.destination() == self.handle() {
                if let Some(item) = ui.node(*dropped).cast::<AssetItem>() {
                    let path = if self
                        .resource_manager
                        .state()
                        .built_in_resources
                        .contains_key(&item.path)
                    {
                        Ok(item.path.clone())
                    } else {
                        make_relative_path(&item.path)
                    };

                    if let Ok(path) = path {
                        if let Ok(value) =
                            block_on(self.resource_manager.request::<SurfaceData>(path))
                        {
                            ui.send_message(SurfaceDataPropertyEditorMessage::value(
                                self.handle(),
                                MessageDirection::ToWidget,
                                value,
                            ));
                        }
                    }
                }
            }
        } else if let Some(SurfaceDataPropertyEditorMessage::Value(value)) = message.data() {
            if message.destination() == self.handle
                && message.direction() == MessageDirection::ToWidget
                && &self.data != value
            {
                self.data = value.clone();
                ui.send_message(message.reverse());

                ui.send_message(TextMessage::text(
                    self.text,
                    MessageDirection::ToWidget,
                    surface_data_info(&self.resource_manager, value),
                ));
            }
        }
    }
}

fn surface_data_info(resource_manager: &ResourceManager, data: &SurfaceResource) -> String {
    let use_count = data.use_count();
    let kind = resource_manager
        .resource_path(data.as_ref())
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "External".to_string());
    let guard = data.data_ref();
    format!(
        "{}\nVertices: {}\nTriangles: {}\nUse Count: {}",
        kind,
        guard.vertex_buffer.vertex_count(),
        guard.geometry_buffer.len(),
        use_count
    )
}

impl SurfaceDataPropertyEditor {
    pub fn build(
        ctx: &mut BuildContext,
        data: SurfaceResource,
        sender: MessageSender,
        resource_manager: ResourceManager,
    ) -> Handle<UiNode> {
        let view = ButtonBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .on_row(0)
                .on_column(1)
                .with_width(45.0)
                .with_height(22.0),
        )
        .with_text("View...")
        .build(ctx);

        let text = TextBuilder::new(
            WidgetBuilder::new()
                .on_row(0)
                .on_column(0)
                .with_margin(Thickness::uniform(1.0)),
        )
        .with_text(surface_data_info(&resource_manager, &data))
        .build(ctx);

        let widget = WidgetBuilder::new()
            .with_child(
                GridBuilder::new(WidgetBuilder::new().with_child(text).with_child(view))
                    .add_column(Column::stretch())
                    .add_column(Column::auto())
                    .add_row(Row::auto())
                    .build(ctx),
            )
            .with_allow_drop(true)
            .build(ctx);

        let editor = Self {
            widget,
            data,
            view,
            sender: Some(sender),
            resource_manager,
            text,
        };

        ctx.add_node(UiNode::new(editor))
    }
}

#[derive(Debug)]
pub struct SurfaceDataPropertyEditorDefinition {
    pub sender: MessageSender,
}

impl PropertyEditorDefinition for SurfaceDataPropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<SurfaceResource>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<SurfaceResource>()?;
        let environment = EditorEnvironment::try_get_from(&ctx.environment)?;

        Ok(PropertyEditorInstance::Simple {
            editor: SurfaceDataPropertyEditor::build(
                ctx.build_context,
                value.clone(),
                self.sender.clone(),
                environment.resource_manager.clone(),
            ),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<SurfaceResource>()?;

        Ok(Some(SurfaceDataPropertyEditorMessage::value(
            ctx.instance,
            MessageDirection::ToWidget,
            value.clone(),
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(SurfaceDataPropertyEditorMessage::Value(value)) = ctx.message.data() {
                return Some(PropertyChanged {
                    name: ctx.name.to_string(),
                    value: FieldKind::object(value.clone()),
                });
            }
        }
        None
    }
}
