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
    command::{Command, CommandStack},
    fyrox::{
        core::{algebra::Vector2, math::Rect, pool::Handle, reflect::Reflect},
        engine::Engine,
        gui::{
            inspector::PropertyChanged,
            message::{KeyCode, MouseButton},
            UiNode,
        },
        resource::texture::TextureResource,
        scene::SceneContainer,
    },
    scene::Selection,
    settings::{keys::KeyBindings, Settings},
    Message,
};
use fyrox::core::define_as_any_trait;
use std::path::Path;

define_as_any_trait!(SceneControllerAsAny => SceneController);

pub trait SceneController: SceneControllerAsAny {
    fn on_key_up(&mut self, key: KeyCode, engine: &mut Engine, key_bindings: &KeyBindings) -> bool;

    fn on_key_down(
        &mut self,
        key: KeyCode,
        engine: &mut Engine,
        key_bindings: &KeyBindings,
    ) -> bool;

    fn on_mouse_move(
        &mut self,
        pos: Vector2<f32>,
        offset: Vector2<f32>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    );

    fn on_mouse_up(
        &mut self,
        button: MouseButton,
        pos: Vector2<f32>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    );

    fn on_mouse_down(
        &mut self,
        button: MouseButton,
        pos: Vector2<f32>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    );

    fn on_mouse_wheel(&mut self, amount: f32, engine: &mut Engine, settings: &Settings);

    fn on_mouse_leave(&mut self, engine: &mut Engine, settings: &Settings);

    fn on_drag_over(
        &mut self,
        handle: Handle<UiNode>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    );

    fn on_drop(
        &mut self,
        handle: Handle<UiNode>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    );

    fn render_target(&self, engine: &Engine) -> Option<TextureResource>;

    fn extension(&self) -> &str;

    fn save(
        &mut self,
        path: &Path,
        settings: &Settings,
        engine: &mut Engine,
    ) -> Result<String, String>;

    fn do_command(
        &mut self,
        command_stack: &mut CommandStack,
        command: Command,
        selection: &mut Selection,
        engine: &mut Engine,
    );

    fn undo(
        &mut self,
        command_stack: &mut CommandStack,
        selection: &mut Selection,
        engine: &mut Engine,
    );

    fn redo(
        &mut self,
        command_stack: &mut CommandStack,
        selection: &mut Selection,
        engine: &mut Engine,
    );

    fn clear_command_stack(
        &mut self,
        command_stack: &mut CommandStack,
        selection: &mut Selection,
        scenes: &mut SceneContainer,
    );

    fn on_before_render(&mut self, editor_selection: &Selection, engine: &mut Engine);

    fn on_after_render(&mut self, engine: &mut Engine);

    fn update(
        &mut self,
        editor_selection: &Selection,
        engine: &mut Engine,
        dt: f32,
        path: Option<&Path>,
        settings: &mut Settings,
        screen_bounds: Rect<f32>,
    ) -> Option<TextureResource>;

    fn is_interacting(&self) -> bool;

    fn on_destroy(
        &mut self,
        command_stack: &mut CommandStack,
        engine: &mut Engine,
        selection: &mut Selection,
    );

    fn on_message(&mut self, message: &Message, selection: &Selection, engine: &mut Engine)
        -> bool;

    fn command_names(
        &mut self,
        command_stack: &mut CommandStack,
        selection: &mut Selection,
        engine: &mut Engine,
    ) -> Vec<String>;

    fn first_selected_entity(
        &self,
        selection: &Selection,
        scenes: &SceneContainer,
        callback: &mut dyn FnMut(&dyn Reflect),
    );

    fn on_property_changed(
        &mut self,
        args: &PropertyChanged,
        selection: &Selection,
        engine: &mut Engine,
    );

    fn provide_docs(&self, selection: &Selection, engine: &Engine) -> Option<String>;
}

impl dyn SceneController {
    pub fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: SceneController,
    {
        self.as_any().downcast_ref::<T>()
    }

    pub fn downcast_mut<T>(&mut self) -> Option<&mut T>
    where
        T: SceneController,
    {
        self.as_any_mut().downcast_mut::<T>()
    }
}
