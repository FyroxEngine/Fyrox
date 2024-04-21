use crate::fyrox::{
    asset::untyped::UntypedResource,
    core::{color::Color, parking_lot::Mutex, pool::Handle},
    gui::{
        border::BorderBuilder, brush::Brush, button::ButtonBuilder, decorator::DecoratorBuilder,
        image::ImageBuilder, text::TextBuilder, utils::make_simple_tooltip, widget::WidgetBuilder,
        BuildContext, HorizontalAlignment, Thickness, UiNode, VerticalAlignment,
    },
};
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
                .with_background(Brush::Solid(Color::opaque(180, 180, 180)))
                .with_margin(Thickness::uniform(2.0))
                .with_width(width)
                .with_height(height),
        )
        .with_opt_texture(image)
        .build(ctx),
    )
    .build(ctx)
}
