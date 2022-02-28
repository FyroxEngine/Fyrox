use fyrox::{
    core::pool::Handle,
    gui::{window::Window, UiNode, UserInterface},
};

pub mod path_fixer;

pub fn is_slice_equal_permutation<T: PartialEq>(a: &[T], b: &[T]) -> bool {
    if a.is_empty() && !b.is_empty() {
        false
    } else {
        // TODO: Find a way to do this faster.
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
}

pub fn window_content(window: Handle<UiNode>, ui: &UserInterface) -> Handle<UiNode> {
    ui.node(window)
        .cast::<Window>()
        .map(|w| w.content())
        .unwrap_or_default()
}
