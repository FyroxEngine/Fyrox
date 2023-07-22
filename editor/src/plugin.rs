use crate::{Editor, Message};
use fyrox::gui::message::UiMessage;

/// Editor plugin allows you to extend editor functionality with custom tools. It provides a standard way of interaction
/// between your plugin and built-in editor's functionality.
///
/// ## Development Patterns
///
/// There are multiple development patterns that **should** (and strongly advised) be used. Following them will help you to
/// write your tools _the right way_.
///
/// ### MVC
///
/// The editor uses classic [MVC](https://en.wikipedia.org/wiki/Model%E2%80%93view%E2%80%93controller) (model-view-controller)
/// pattern. This means that the editor always "renders" the actual state of your data model and its UI is used only to show
/// the data - it does not store anything. Any user change forces the editor to sync the UI with the new data.
///
/// ### Commands
///
/// The editor usually operates on scenes (there could be multiple opened scenes, but only one active) and any modification of
/// their content **must** be done via _commands_. [Command](https://en.wikipedia.org/wiki/Command_pattern) is a standard
/// pattern that encapsulates an action. Command pattern is used for undo/redo functionality.
pub trait EditorPlugin {
    /// This method is called right after the editor was fully initialized. It is guaranteed to be called only once.
    fn on_start(&mut self, #[allow(unused_variables)] editor: &mut Editor) {}

    /// This method is called when the editor is about to close. It is guaranteed to be called only once.
    fn on_exit(&mut self, #[allow(unused_variables)] editor: &mut Editor) {}

    /// This method is called either when there was some action via command, or a syncing request is performed. It should
    /// be used to synchronize the state of your widgets with the actual data model.  
    fn on_sync_to_model(&mut self, #[allow(unused_variables)] editor: &mut Editor) {}

    /// This method is called when the editor switches to another mode. For example, if a user clicks the "Play" button,
    /// the mode will be changed from [`crate::Mode::Edit`] to [`crate::Mode::Build`], and if the build was successful,
    /// it will then be changed to [`crate::Mode::Play`]. When the game was closed, the mode will be changed back to
    /// [`crate::Mode::Edit`].
    fn on_mode_changed(&mut self, #[allow(unused_variables)] editor: &mut Editor) {}

    /// This method is called when a UI message was extracted from the message queue. It should be used to react to user
    /// changes, for example a user could click a button, then a [`fyrox::gui::button::ButtonMessage::Click`] will be
    /// passed to this method. It then can be used to perform some other action.
    fn on_ui_message(
        &mut self,
        #[allow(unused_variables)] message: &mut UiMessage,
        #[allow(unused_variables)] editor: &mut Editor,
    ) {
    }

    /// This method is called every frame at stable update rate of 60 FPS. It could be used to perform any contiguous
    /// actions.
    fn on_update(&mut self, #[allow(unused_variables)] editor: &mut Editor) {}

    /// This method is called when the editor receives a control message. It could be used to catch and react to specific
    /// actions in the editor (such as: scene loading, command execution, undo, redo, etc.).
    fn on_message(
        &mut self,
        #[allow(unused_variables)] message: &Message,
        #[allow(unused_variables)] editor: &mut Editor,
    ) {
    }
}

#[macro_export]
macro_rules! for_each_plugin {
    ($container:expr => $func:ident($($param:expr),*)) => {{
        let mut i = 0;
        while i < $container.len() {
            if let Some(mut plugin) = $container.get_mut(i).and_then(|p| p.take()) {
                plugin.$func($($param),*);

                if let Some(entry) = $container.get_mut(i) {
                    *entry = Some(plugin);
                }
            }

            i += 1;
        }
    }};
}
