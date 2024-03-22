use crate::command::{Command, CommandStack};
use crate::fyrox::{
    core::{algebra::Vector2, math::Rect, pool::Handle, reflect::Reflect},
    engine::Engine,
    gui::{
        inspector::PropertyChanged,
        message::{KeyCode, MouseButton},
        UiNode,
    },
    resource::texture::TextureResource,
    scene::SceneContainer,
};
use crate::{
    scene::Selection,
    settings::{keys::KeyBindings, Settings},
    Message,
};
use std::{any::Any, path::Path};

pub trait SceneController: 'static {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

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
