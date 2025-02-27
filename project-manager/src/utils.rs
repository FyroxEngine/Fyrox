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

use cargo_metadata::{semver::VersionReq, Dependency, Metadata};
use fyrox::{
    core::pool::Handle,
    gui::{
        button::ButtonBuilder, text::TextBuilder, utils::make_simple_tooltip,
        widget::WidgetBuilder, BuildContext, HorizontalAlignment, Thickness, UiNode,
        VerticalAlignment,
    },
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
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

pub fn fyrox_dependency(metadata: &Metadata) -> Option<&Dependency> {
    for package in metadata.packages.iter() {
        for dependency in package.dependencies.iter() {
            if dependency.name == "fyrox" {
                return Some(dependency);
            }
        }
    }
    None
}

fn to_pretty_version(version_req: &VersionReq) -> String {
    let version = version_req.to_string();
    let pretty_version = version.replace('^', "");
    pretty_version.replace('+', "")
}

pub fn fyrox_version_string(metadata: &Metadata) -> Option<String> {
    fyrox_dependency(metadata).and_then(|dependency| {
        if let Some(source) = dependency.source.as_ref() {
            if source.contains("registry+") {
                return Some(to_pretty_version(&dependency.req));
            } else if source.contains("git+") {
                return Some("nightly".to_string());
            }
        } else if let Some(path) = dependency.path.as_ref() {
            return Some(path.to_string());
        }
        None
    })
}

pub fn fyrox_dependency_from_path(manifest_path: &Path) -> Option<Dependency> {
    read_crate_metadata(manifest_path)
        .ok()
        .and_then(|metadata| fyrox_dependency(&metadata).cloned())
}

pub fn has_fyrox_in_deps(metadata: &Metadata) -> bool {
    fyrox_dependency(metadata).is_some()
}

pub fn calculate_directory_size(path: &Path) -> u64 {
    let mut total_size = 0;

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_file() {
                if let Ok(metadata) = entry.metadata() {
                    total_size += metadata.len();
                }
            } else if entry_path.is_dir() {
                total_size += calculate_directory_size(&entry_path);
            }
        }
    }

    total_size
}

pub fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}
