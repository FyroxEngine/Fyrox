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

use crate::{
    fyrox::{
        core::{
            algebra::Vector2, color::Color, pool::ErasedHandle, pool::Handle, visitor::Visitor,
        },
        graph::SceneGraph,
        gui::{
            brush::Brush,
            button::ButtonBuilder,
            file_browser::{FileSelectorBuilder, PathFilter},
            image::ImageBuilder,
            widget::{WidgetBuilder, WidgetMessage},
            window::{Window, WindowBuilder},
            BuildContext, HorizontalAlignment, Thickness, UiNode, UserInterface, VerticalAlignment,
        },
    },
    load_image,
};
use fyrox::gui::button::Button;
use fyrox::gui::file_browser::{FileSelectorMode, FileType};
use fyrox::gui::texture::TextureResource;
use fyrox::gui::utils::make_image_button_with_tooltip;
use std::{fs::File, path::Path};

pub mod doc;

pub fn make_pick_button(column: usize, ctx: &mut BuildContext) -> Handle<Button> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .with_width(20.0)
            .with_height(20.0)
            .with_vertical_alignment(VerticalAlignment::Center)
            .with_horizontal_alignment(HorizontalAlignment::Center)
            .on_column(column)
            .with_margin(Thickness::uniform(1.0)),
    )
    .with_content(
        ImageBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .with_background(Brush::Solid(Color::opaque(0, 180, 0)).into()),
        )
        .with_opt_texture(load_image!("../../resources/pick.png"))
        .build(ctx),
    )
    .build(ctx)
}

/// True if `a` and `b` have the same length, and every element of `a` is equal to some element of `b`
/// and every element of `b` is equal to some element of `a`.
pub fn is_slice_equal_permutation<T: PartialEq>(a: &[T], b: &[T]) -> bool {
    a.len() == b.len() && is_slice_subset_permutation(a, b) && is_slice_subset_permutation(b, a)
}

/// True if every elmenet of `a` is equal to some element of `b`.
pub fn is_slice_subset_permutation<T: PartialEq>(a: &[T], b: &[T]) -> bool {
    for source in a.iter() {
        let mut found = false;
        for other in b.iter() {
            if other == source {
                found = true;
                break;
            }
        }
        if !found {
            return false;
        }
    }
    true
}

pub fn window_content(window: Handle<Window>, ui: &UserInterface) -> Handle<UiNode> {
    ui.try_get(window)
        .ok()
        .map(|w| w.content)
        .unwrap_or_default()
}

pub fn enable_widget(handle: Handle<UiNode>, state: bool, ui: &UserInterface) {
    ui.send(handle, WidgetMessage::Enabled(state));
}

pub fn create_file_selector(
    ctx: &mut BuildContext,
    file_type: FileType,
    mode: FileSelectorMode,
) -> Handle<UiNode> {
    FileSelectorBuilder::new(
        WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0)).open(false),
    )
    .with_filter(PathFilter::new().with_file_type(file_type))
    .with_mode(mode)
    .build(ctx)
}

pub fn fetch_node_center(handle: Handle<UiNode>, ctx: &BuildContext) -> Vector2<f32> {
    ctx.try_get_node(handle)
        .map(|node| node.center())
        .unwrap_or_default()
}

pub fn fetch_node_screen_center(handle: Handle<UiNode>, ctx: &BuildContext) -> Vector2<f32> {
    ctx.try_get_node(handle)
        .map(|node| node.screen_bounds().center())
        .unwrap_or_default()
}

pub fn fetch_node_screen_center_ui(handle: Handle<UiNode>, ui: &UserInterface) -> Vector2<f32> {
    ui.try_get_node(handle)
        .map(|node| node.screen_bounds().center())
        .unwrap_or_default()
}

pub fn make_node_name(name: &str, handle: ErasedHandle) -> String {
    format!("{} ({}:{})", name, handle.index(), handle.generation())
}

pub fn apply_visibility_filter<F>(root: Handle<UiNode>, ui: &UserInterface, filter: F)
where
    F: Fn(&UiNode) -> Option<bool>,
{
    fn apply_filter_recursive<F>(node: Handle<UiNode>, ui: &UserInterface, filter: &F) -> bool
    where
        F: Fn(&UiNode) -> Option<bool>,
    {
        let node_ref = ui.node(node);

        let mut is_any_match = false;
        for &child in node_ref.children() {
            is_any_match |= apply_filter_recursive(child, ui, filter)
        }

        if let Some(has_match) = filter(node_ref) {
            is_any_match |= has_match;

            ui.send(node, WidgetMessage::Visibility(is_any_match));
        }

        is_any_match
    }

    apply_filter_recursive(root, ui, &filter);
}

pub fn make_square_image_button_with_tooltip(
    ctx: &mut BuildContext,
    image: Option<TextureResource>,
    tooltip: &str,
    tab_index: Option<usize>,
) -> Handle<Button> {
    make_image_button_with_tooltip(ctx, 18.0, 18.0, image, tooltip, tab_index)
}

pub fn is_native_scene(path: &Path) -> bool {
    if let Ok(mut file) = File::open(path) {
        Visitor::is_supported(&mut file)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_subset() {
        assert!(is_slice_subset_permutation(&[], &[1, 2, 3]));
    }
    #[test]
    fn subset() {
        assert!(is_slice_subset_permutation(&[2, 3], &[1, 2, 3]));
    }
    #[test]
    fn not_subset() {
        assert!(!is_slice_subset_permutation(&[1, 2, 3], &[1, 2]));
    }
    #[test]
    fn not_empty() {
        assert!(!is_slice_subset_permutation(&[1, 2], &[]));
    }
    #[test]
    fn equal() {
        assert!(is_slice_equal_permutation(&[1, 2], &[1, 2]));
        assert!(is_slice_equal_permutation(&[1, 2], &[2, 1]));
    }
    #[test]
    fn not_equal() {
        assert!(!is_slice_equal_permutation(&[1, 2], &[1]));
        assert!(!is_slice_equal_permutation(&[1], &[2, 1]));
        assert!(!is_slice_equal_permutation(&[1, 2], &[2, 3]));
        assert!(!is_slice_equal_permutation(&[1, 1], &[1, 2]));
        assert!(!is_slice_equal_permutation(&[1, 2], &[2, 2]));
    }
}
