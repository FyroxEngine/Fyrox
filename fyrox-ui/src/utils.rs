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

use crate::button::ButtonBuilder;
use crate::decorator::DecoratorBuilder;
use crate::image::ImageBuilder;
use crate::style::resource::StyleResourceExt;
use crate::style::Style;
use crate::{
    border::BorderBuilder,
    core::{algebra::Vector2, color::Color, pool::Handle},
    formatted_text::WrapMode,
    text::TextBuilder,
    vector_image::{Primitive, VectorImageBuilder},
    widget::WidgetBuilder,
    Brush, BuildContext, HorizontalAlignment, RcUiNodeHandle, Thickness, UiNode, VerticalAlignment,
};
use fyrox_core::parking_lot::Mutex;
use fyrox_resource::untyped::UntypedResource;
use std::sync::Arc;

pub enum ArrowDirection {
    Top,
    Bottom,
    Left,
    Right,
}

pub fn make_arrow_primitives_non_uniform_size(
    orientation: ArrowDirection,
    width: f32,
    height: f32,
) -> Vec<Primitive> {
    vec![match orientation {
        ArrowDirection::Top => Primitive::Triangle {
            points: [
                Vector2::new(width * 0.5, 0.0),
                Vector2::new(width, height),
                Vector2::new(0.0, height),
            ],
        },
        ArrowDirection::Bottom => Primitive::Triangle {
            points: [
                Vector2::new(0.0, 0.0),
                Vector2::new(width, 0.0),
                Vector2::new(width * 0.5, height),
            ],
        },
        ArrowDirection::Right => Primitive::Triangle {
            points: [
                Vector2::new(0.0, 0.0),
                Vector2::new(width, height * 0.5),
                Vector2::new(0.0, height),
            ],
        },
        ArrowDirection::Left => Primitive::Triangle {
            points: [
                Vector2::new(0.0, height * 0.5),
                Vector2::new(width, 0.0),
                Vector2::new(width, height),
            ],
        },
    }]
}

pub fn make_arrow_primitives(orientation: ArrowDirection, size: f32) -> Vec<Primitive> {
    make_arrow_primitives_non_uniform_size(orientation, size, size)
}

pub fn make_arrow_non_uniform_size(
    ctx: &mut BuildContext,
    orientation: ArrowDirection,
    width: f32,
    height: f32,
) -> Handle<UiNode> {
    VectorImageBuilder::new(
        WidgetBuilder::new()
            .with_foreground(ctx.style.property(Style::BRUSH_BRIGHT))
            .with_width(width)
            .with_height(height)
            .with_horizontal_alignment(HorizontalAlignment::Center)
            .with_vertical_alignment(VerticalAlignment::Center),
    )
    .with_primitives(make_arrow_primitives_non_uniform_size(
        orientation,
        width,
        height,
    ))
    .build(ctx)
}

pub fn make_arrow(
    ctx: &mut BuildContext,
    orientation: ArrowDirection,
    size: f32,
) -> Handle<UiNode> {
    make_arrow_non_uniform_size(ctx, orientation, size, size)
}

pub fn make_cross_primitive(size: f32, thickness: f32) -> Vec<Primitive> {
    vec![
        Primitive::Line {
            begin: Vector2::new(0.0, 0.0),
            end: Vector2::new(size, size),
            thickness,
        },
        Primitive::Line {
            begin: Vector2::new(size, 0.0),
            end: Vector2::new(0.0, size),
            thickness,
        },
    ]
}

pub fn make_cross(ctx: &mut BuildContext, size: f32, thickness: f32) -> Handle<UiNode> {
    VectorImageBuilder::new(
        WidgetBuilder::new()
            .with_horizontal_alignment(HorizontalAlignment::Center)
            .with_vertical_alignment(VerticalAlignment::Center)
            .with_width(size)
            .with_height(size)
            .with_foreground(ctx.style.property(Style::BRUSH_BRIGHT)),
    )
    .with_primitives(make_cross_primitive(size, thickness))
    .build(ctx)
}

pub fn make_simple_tooltip(ctx: &mut BuildContext, text: &str) -> RcUiNodeHandle {
    let handle = BorderBuilder::new(
        WidgetBuilder::new()
            .with_visibility(false)
            .with_hit_test_visibility(false)
            .with_foreground(ctx.style.property(Style::BRUSH_DARKEST))
            .with_background(Brush::Solid(Color::opaque(230, 230, 230)).into())
            .with_max_size(Vector2::new(300.0, f32::INFINITY))
            .with_child(
                TextBuilder::new(
                    WidgetBuilder::new()
                        .with_margin(Thickness::uniform(2.0))
                        .with_foreground(ctx.style.property(Style::BRUSH_DARKER)),
                )
                .with_wrap(WrapMode::Word)
                .with_text(text)
                .build(ctx),
            ),
    )
    .build(ctx);
    RcUiNodeHandle::new(handle, ctx.sender())
}

pub fn make_dropdown_list_option_universal<T: Send + 'static>(
    ctx: &mut BuildContext,
    name: &str,
    height: f32,
    user_data: T,
) -> Handle<UiNode> {
    DecoratorBuilder::new(
        BorderBuilder::new(
            WidgetBuilder::new()
                .with_height(height)
                .with_user_data(Arc::new(Mutex::new(user_data)))
                .with_child(
                    TextBuilder::new(WidgetBuilder::new())
                        .with_vertical_text_alignment(VerticalAlignment::Center)
                        .with_horizontal_text_alignment(HorizontalAlignment::Center)
                        .with_text(name)
                        .build(ctx),
                ),
        )
        .with_corner_radius(4.0f32.into())
        .with_pad_by_corner_radius(false),
    )
    .build(ctx)
}

pub fn make_dropdown_list_option(ctx: &mut BuildContext, name: &str) -> Handle<UiNode> {
    DecoratorBuilder::new(
        BorderBuilder::new(
            WidgetBuilder::new().with_child(
                TextBuilder::new(WidgetBuilder::new())
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .with_horizontal_text_alignment(HorizontalAlignment::Center)
                    .with_text(name)
                    .build(ctx),
            ),
        )
        .with_corner_radius(4.0f32.into())
        .with_pad_by_corner_radius(false),
    )
    .build(ctx)
}

pub fn make_dropdown_list_option_with_height(
    ctx: &mut BuildContext,
    name: &str,
    height: f32,
) -> Handle<UiNode> {
    DecoratorBuilder::new(
        BorderBuilder::new(
            WidgetBuilder::new().with_height(height).with_child(
                TextBuilder::new(WidgetBuilder::new())
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .with_horizontal_text_alignment(HorizontalAlignment::Center)
                    .with_text(name)
                    .build(ctx),
            ),
        )
        .with_corner_radius(4.0f32.into())
        .with_pad_by_corner_radius(false),
    )
    .build(ctx)
}

pub fn make_image_button_with_tooltip(
    ctx: &mut BuildContext,
    width: f32,
    height: f32,
    image: Option<UntypedResource>,
    tooltip: &str,
    tab_index: Option<usize>,
) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .with_tab_index(tab_index)
            .with_tooltip(make_simple_tooltip(ctx, tooltip))
            .with_margin(Thickness::uniform(1.0)),
    )
    .with_content(
        ImageBuilder::new(
            WidgetBuilder::new()
                .with_background(ctx.style.property(Style::BRUSH_BRIGHTEST))
                .with_margin(Thickness::uniform(2.0))
                .with_width(width)
                .with_height(height),
        )
        .with_opt_texture(image)
        .build(ctx),
    )
    .build(ctx)
}
