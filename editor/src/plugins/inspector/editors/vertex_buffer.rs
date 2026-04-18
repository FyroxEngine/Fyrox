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

use crate::fyrox::{
    gui::{
        inspector::{
            editors::{
                PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
                PropertyEditorMessageContext, PropertyEditorTranslationContext,
            },
            InspectorError, PropertyChanged,
        },
        message::UiMessage,
        style::resource::StyleResourceExt,
        text::{TextBuilder, TextMessage},
        widget::WidgetBuilder,
        Thickness, VerticalAlignment,
    },
    scene::mesh::buffer::VertexBuffer,
};
use crate::Editor;
use std::any::TypeId;

#[derive(Debug)]
pub struct VertexBufferPropertyEditorDefinition;

fn vertex_buffer_description(vertex_buffer: &VertexBuffer) -> String {
    format!(
        "Vertices: {}\nVertex Size: {}\nAttributes: {}",
        vertex_buffer.vertex_count(),
        vertex_buffer.vertex_size(),
        vertex_buffer.layout().len()
    )
}

impl PropertyEditorDefinition for VertexBufferPropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<VertexBuffer>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<VertexBuffer>()?;
        Ok(PropertyEditorInstance::simple(
            TextBuilder::new(
                WidgetBuilder::new()
                    .with_margin(Thickness::top_bottom(1.0))
                    .with_vertical_alignment(VerticalAlignment::Center),
            )
            .with_text(vertex_buffer_description(value))
            .with_font_size(ctx.build_context.style.property(Editor::UI_FONT_SIZE))
            .build(ctx.build_context),
        ))
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<VertexBuffer>()?;
        Ok(Some(UiMessage::for_widget(
            ctx.instance,
            TextMessage::Text(vertex_buffer_description(value)),
        )))
    }

    fn translate_message(&self, _ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        None
    }
}
