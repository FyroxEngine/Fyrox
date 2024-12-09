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

use cargo_metadata::Metadata;
use fyrox::gui::utils::make_simple_tooltip;
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
use std::{
    path::{Path, PathBuf},
    process::Command,
    process::Stdio,
};

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
        .with_corner_radius(4.0f32.into())
        .with_pad_by_corner_radius(false),
    )
    .build(ctx)
}

#[allow(clippy::too_many_arguments)]
pub fn make_button(
    text: &str,
    width: f32,
    height: f32,
    tab_index: usize,
    row: usize,
    column: usize,
    tooltip: Option<&str>,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    let mut widget_builder = WidgetBuilder::new()
        .on_row(row)
        .on_column(column)
        .with_width(width)
        .with_height(height)
        .with_tab_index(Some(tab_index))
        .with_margin(Thickness::uniform(1.0));

    if let Some(tooltip) = tooltip {
        widget_builder = widget_builder.with_tooltip(make_simple_tooltip(ctx, tooltip));
    }

    ButtonBuilder::new(widget_builder)
        .with_content(
            TextBuilder::new(WidgetBuilder::new())
                .with_text(text)
                .with_font_size(16.0.into())
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

pub fn folder_to_manifest_path(path: &Path) -> PathBuf {
    let manifest_path = path
        .join("Cargo.toml")
        .canonicalize()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    // Remove "\\?\" prefix on Windows, otherwise it will be impossible to compile anything,
    // because there are some quirks on Unicode path handling on Windows and any path starting
    // from two slashes will not work correctly as a working directory for a child process.
    manifest_path.replace(r"\\?\", r"").into()
}

pub fn read_crate_metadata(manifest_path: &Path) -> Result<Metadata, String> {
    match Command::new("cargo")
        .arg("metadata")
        .arg("--no-deps")
        .arg("--format-version")
        .arg("1")
        .arg("--manifest-path")
        .arg(manifest_path)
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(handle) => match handle.wait_with_output() {
            Ok(output) => match serde_json::from_slice::<Metadata>(&output.stdout) {
                Ok(metadata) => Ok(metadata),
                Err(err) => Err(format!(
                    "Unable to parse workspace metadata. Reason {err:?}"
                )),
            },
            Err(err) => Err(format!("Unable to fetch project metadata. Reason {err:?}")),
        },
        Err(err) => Err(format!("Unable to fetch project metadata. Reason {err:?}")),
    }
}

pub fn fyrox_version(metadata: &Metadata) -> Option<String> {
    for package in metadata.packages.iter() {
        for dependency in package.dependencies.iter() {
            if dependency.name == "fyrox" {
                let version = dependency.req.to_string();
                let pretty_version = version.replace('^', "");
                let pretty_version = pretty_version.replace('+', "");
                return Some(pretty_version);
            }
        }
    }
    None
}

pub fn fyrox_version_or_default(manifest_path: &Path) -> String {
    read_crate_metadata(manifest_path)
        .ok()
        .and_then(|metadata| fyrox_version(&metadata))
        .unwrap_or_default()
}

pub fn has_fyrox_in_deps(metadata: &Metadata) -> bool {
    fyrox_version(metadata).is_some()
}
