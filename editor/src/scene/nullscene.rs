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

//! The module for [`NullSceneController`] that provides a [`SceneController`] for when
//! there is no scene to control.

use crate::command::CommandContext;
use fyrox::core::ComponentProvider;
use fyrox::gui::file_browser::FileType;

use super::*;

/// The [`CommandContext`] that is used when the current scene is the
/// null scene.
#[derive(ComponentProvider)]
pub struct NullSceneContext {
    #[component(include)]
    pub selection: &'static mut Selection,
    #[component(include)]
    pub message_sender: MessageSender,
    #[component(include)]
    pub resource_manager: ResourceManager,
}

impl CommandContext for NullSceneContext {}

impl NullSceneContext {
    pub fn exec<'a, F>(
        selection: &'a mut Selection,
        message_sender: MessageSender,
        resource_manager: ResourceManager,
        func: F,
    ) where
        F: FnOnce(&mut NullSceneContext),
    {
        // SAFETY: Temporarily extend lifetime to 'static and execute external closure with it.
        // The closure accepts this extended context by reference, so there's no way it escapes to
        // outer world. The initial lifetime is still preserved by this function call.
        func(unsafe {
            &mut Self {
                selection: std::mem::transmute::<&'a mut _, &'static mut _>(selection),
                message_sender,
                resource_manager,
            }
        });
    }
}

/// A scene controller for when there is no scene to control.
/// Everything it does is as close to nothing as possible, since it
/// controls no scene, but it allows commands to be added and undone
/// with some minimal context provided by [`NullSceneContext`].
pub struct NullSceneController {
    pub sender: MessageSender,
    pub resource_manager: ResourceManager,
}

#[allow(unused_variables)]
impl SceneController for NullSceneController {
    fn on_key_up(&mut self, key: KeyCode, engine: &mut Engine, key_bindings: &KeyBindings) -> bool {
        false
    }

    fn on_key_down(
        &mut self,
        key: KeyCode,
        engine: &mut Engine,
        key_bindings: &KeyBindings,
    ) -> bool {
        false
    }

    fn on_mouse_move(
        &mut self,
        pos: Vector2<f32>,
        offset: Vector2<f32>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    ) {
    }

    fn on_mouse_up(
        &mut self,
        button: MouseButton,
        pos: Vector2<f32>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    ) {
    }

    fn on_mouse_down(
        &mut self,
        button: MouseButton,
        pos: Vector2<f32>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    ) {
    }

    fn on_mouse_wheel(&mut self, amount: f32, engine: &mut Engine, settings: &Settings) {}

    fn on_mouse_leave(&mut self, engine: &mut Engine, settings: &Settings) {}

    fn on_drag_over(
        &mut self,
        handle: Handle<UiNode>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    ) {
    }

    fn on_drop(
        &mut self,
        handle: Handle<UiNode>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    ) {
    }

    fn render_target(&self, engine: &Engine) -> Option<fyrox::gui::texture::TextureResource> {
        None
    }

    fn file_type(&self) -> FileType {
        Default::default()
    }

    fn save(
        &mut self,
        path: &std::path::Path,
        settings: &Settings,
        engine: &mut Engine,
    ) -> Result<String, String> {
        Err("Cannot save null scene".into())
    }

    fn do_command(
        &mut self,
        command_stack: &mut CommandStack,
        command: crate::command::Command,
        selection: &mut Selection,
        engine: &mut Engine,
    ) {
        NullSceneContext::exec(
            selection,
            self.sender.clone(),
            engine.resource_manager.clone(),
            |ctx| command_stack.do_command(command, ctx),
        );
    }

    fn undo(
        &mut self,
        command_stack: &mut CommandStack,
        selection: &mut Selection,
        engine: &mut Engine,
    ) {
        NullSceneContext::exec(
            selection,
            self.sender.clone(),
            engine.resource_manager.clone(),
            |ctx| command_stack.undo(ctx),
        );
    }

    fn redo(
        &mut self,
        command_stack: &mut CommandStack,
        selection: &mut Selection,
        engine: &mut Engine,
    ) {
        NullSceneContext::exec(
            selection,
            self.sender.clone(),
            engine.resource_manager.clone(),
            |ctx| command_stack.redo(ctx),
        );
    }

    fn clear_command_stack(
        &mut self,
        command_stack: &mut CommandStack,
        selection: &mut Selection,
        scenes: &mut fyrox::scene::SceneContainer,
    ) {
        NullSceneContext::exec(
            selection,
            self.sender.clone(),
            self.resource_manager.clone(),
            |ctx| command_stack.clear(ctx),
        );
    }

    fn on_before_render(&mut self, editor_selection: &Selection, engine: &mut Engine) {}

    fn on_after_render(&mut self, engine: &mut Engine) {}

    fn update(
        &mut self,
        editor_selection: &Selection,
        engine: &mut Engine,
        dt: f32,
        path: Option<&std::path::Path>,
        settings: &mut Settings,
        screen_bounds: Rect<f32>,
    ) -> Option<fyrox::gui::texture::TextureResource> {
        None
    }

    fn is_interacting(&self) -> bool {
        false
    }

    fn on_destroy(
        &mut self,
        command_stack: &mut CommandStack,
        engine: &mut Engine,
        selection: &mut Selection,
    ) {
        panic!("The NullSceneController was destroyed!")
    }

    fn on_message(
        &mut self,
        message: &crate::Message,
        selection: &Selection,
        engine: &mut Engine,
    ) -> bool {
        false
    }

    fn command_names(
        &mut self,
        command_stack: &mut CommandStack,
        selection: &mut Selection,
        engine: &mut Engine,
    ) -> Vec<String> {
        command_stack
            .commands
            .iter_mut()
            .map(|c| {
                let mut name = String::new();
                NullSceneContext::exec(
                    selection,
                    self.sender.clone(),
                    engine.resource_manager.clone(),
                    |ctx| {
                        name = c.name(ctx);
                    },
                );
                name
            })
            .collect::<Vec<_>>()
    }
}
