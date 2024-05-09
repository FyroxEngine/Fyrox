use crate::fyrox::graph::BaseSceneGraph;
use crate::fyrox::{
    core::{algebra::Vector2, pool::ErasedHandle, pool::Handle, visitor::Visitor},
    gui::{
        file_browser::{FileBrowserMode, FileSelectorBuilder, Filter},
        message::MessageDirection,
        widget::{WidgetBuilder, WidgetMessage},
        window::{Window, WindowBuilder},
        BuildContext, UiNode, UserInterface,
    },
};
use std::{fs::File, io::Read, path::Path};

pub mod doc;
pub mod path_fixer;
pub mod ragdoll;

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

pub fn window_content(window: Handle<UiNode>, ui: &UserInterface) -> Handle<UiNode> {
    ui.node(window)
        .cast::<Window>()
        .map(|w| w.content)
        .unwrap_or_default()
}

pub fn enable_widget(handle: Handle<UiNode>, state: bool, ui: &UserInterface) {
    ui.send_message(WidgetMessage::enabled(
        handle,
        MessageDirection::ToWidget,
        state,
    ));
}

pub fn create_file_selector(
    ctx: &mut BuildContext,
    extension: &'static str,
    mode: FileBrowserMode,
) -> Handle<UiNode> {
    FileSelectorBuilder::new(
        WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0)).open(false),
    )
    .with_filter(Filter::new(move |path| {
        path.is_dir()
            || path
                .extension()
                .map_or(false, |ext| ext.to_string_lossy().as_ref() == extension)
    }))
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
    ui.try_get(handle)
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

            ui.send_message(WidgetMessage::visibility(
                node,
                MessageDirection::ToWidget,
                is_any_match,
            ));
        }

        is_any_match
    }

    apply_filter_recursive(root, ui, &filter);
}

pub fn is_native_scene(path: &Path) -> bool {
    if let Ok(mut file) = File::open(path) {
        let mut magic: [u8; 4] = Default::default();
        if file.read_exact(&mut magic).is_ok() {
            return magic.eq(Visitor::MAGIC.as_bytes());
        }
    }
    false
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
