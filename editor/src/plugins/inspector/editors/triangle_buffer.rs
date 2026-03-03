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
    core::math::TriangleDefinition,
    gui::{
        inspector::{
            editors::{
                PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
                PropertyEditorMessageContext, PropertyEditorTranslationContext,
            },
            InspectorError, PropertyChanged,
        },
        message::UiMessage,
        text::{TextBuilder, TextMessage},
        widget::WidgetBuilder,
        Thickness, VerticalAlignment,
    },
    scene::mesh::buffer::TriangleBuffer,
};
use std::any::TypeId;

#[derive(Debug)]
pub struct TriangleBufferPropertyEditorDefinition;

fn triangle_buffer_description(triangle_buffer: &TriangleBuffer) -> String {
    format!(
        "Triangles: {}\nTriangle Size: {}",
        triangle_buffer.triangles_ref().len(),
        size_of::<TriangleDefinition>()
    )
}

impl PropertyEditorDefinition for TriangleBufferPropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<TriangleBuffer>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<TriangleBuffer>()?;
        Ok(PropertyEditorInstance::simple(
            TextBuilder::new(
                WidgetBuilder::new()
                    .with_margin(Thickness::top_bottom(1.0))
                    .with_vertical_alignment(VerticalAlignment::Center),
            )
            .with_text(triangle_buffer_description(value))
            .build(ctx.build_context),
        ))
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<TriangleBuffer>()?;
        Ok(Some(UiMessage::for_widget(
            ctx.instance,
            TextMessage::Text(triangle_buffer_description(value)),
        )))
    }

    fn translate_message(&self, _ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        None
    }
}
