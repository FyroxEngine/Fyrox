use fyrox::{
    asset::untyped::UntypedResource,
    core::pool::Handle,
    gui::{
        border::BorderBuilder, button::ButtonBuilder, decorator::DecoratorBuilder,
        text::TextBuilder, widget::WidgetBuilder, BuildContext, HorizontalAlignment, Thickness,
        UiNode, VerticalAlignment,
    },
    resource::texture::{
        CompressionOptions, TextureImportOptions, TextureMinificationFilter, TextureResource,
        TextureResourceExtension,
    },
};
use std::process::Command;

pub fn is_tool_installed(name: &str) -> bool {
    let Ok(output) = Command::new(name).output() else {
        return false;
    };

    output.status.success()
}

pub fn is_production_ready() -> bool {
    is_tool_installed("rustc") && is_tool_installed("cargo")
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

pub fn make_button(
    text: &str,
    width: f32,
    height: f32,
    tab_index: usize,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .with_width(width)
            .with_height(height)
            .with_tab_index(Some(tab_index))
            .with_margin(Thickness::uniform(1.0)),
    )
    .with_content(
        TextBuilder::new(WidgetBuilder::new())
            .with_text(text)
            .with_font_size(16.0)
            .with_vertical_text_alignment(VerticalAlignment::Center)
            .with_horizontal_text_alignment(HorizontalAlignment::Center)
            .build(ctx),
    )
    .build(ctx)
}

pub fn load_image(data: &[u8]) -> Option<UntypedResource> {
    Some(
        TextureResource::load_from_memory(
            Default::default(),
            data,
            TextureImportOptions::default()
                .with_compression(CompressionOptions::NoCompression)
                .with_minification_filter(TextureMinificationFilter::Linear),
        )
        .ok()?
        .into(),
    )
}
