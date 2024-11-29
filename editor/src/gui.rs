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
    asset::untyped::UntypedResource,
    core::{parking_lot::Mutex, pool::Handle},
    gui::{
        border::BorderBuilder, button::ButtonBuilder, decorator::DecoratorBuilder,
        image::ImageBuilder, text::TextBuilder, utils::make_simple_tooltip, widget::WidgetBuilder,
        BuildContext, HorizontalAlignment, Thickness, UiNode, VerticalAlignment,
    },
};
use fyrox::gui::style::resource::StyleResourceExt;
use fyrox::gui::style::Style;
use std::sync::Arc;

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
        .with_corner_radius(4.0)
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
        .with_corner_radius(4.0)
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
        .with_corner_radius(4.0)
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
