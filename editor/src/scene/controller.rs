use crate::command::Command;
use crate::{
    scene::Selection,
    settings::{keys::KeyBindings, Settings},
};
use fyrox::{
    core::{algebra::Vector2, math::Rect, pool::Handle},
    engine::Engine,
    gui::{
        message::{KeyCode, MouseButton},
        UiNode,
    },
    resource::texture::TextureResource,
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
        editor_selection: &Selection,
    );

    fn render_target(&self, engine: &Engine) -> Option<TextureResource>;

    fn save(
        &mut self,
        path: &Path,
        settings: &Settings,
        engine: &mut Engine,
    ) -> Result<String, String>;

    fn do_command(
        &mut self,
        command: Box<dyn Command>,
        selection: &mut Selection,
        engine: &mut Engine,
    );

    fn undo(&mut self, selection: &mut Selection, engine: &mut Engine);

    fn redo(&mut self, selection: &mut Selection, engine: &mut Engine);

    fn clear_command_stack(&mut self, selection: &mut Selection, engine: &mut Engine);

    fn on_before_render(&mut self, engine: &mut Engine);

    fn on_after_render(&mut self, engine: &mut Engine);

    fn update(
        &mut self,
        editor_selection: &Selection,
        engine: &mut Engine,
        dt: f32,
        path: Option<&Path>,
        settings: &mut Settings,
        screen_bounds: Rect<f32>,
    );

    fn is_interacting(&self) -> bool;

    fn on_destroy(&mut self, engine: &mut Engine);
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
