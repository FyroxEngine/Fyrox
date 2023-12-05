#![allow(unused_variables)] // TODO

use crate::{
    command::Command,
    scene::controller::SceneController,
    scene::Selection,
    settings::{keys::KeyBindings, Settings},
};
use fyrox::resource::texture::TextureKind;
use fyrox::{
    core::{
        algebra::Vector2,
        log::Log,
        math::Rect,
        pool::Handle,
        visitor::{Visit, Visitor},
    },
    engine::Engine,
    gui::{
        button::ButtonBuilder,
        message::{KeyCode, MouseButton},
        widget::WidgetBuilder,
        UiNode, UserInterface,
    },
    resource::texture::{TextureResource, TextureResourceExtension},
};
use std::{any::Any, path::Path};

pub struct UiScene {
    ui: UserInterface,
    render_target: TextureResource,
}

impl UiScene {
    pub fn new() -> Self {
        let mut ui = UserInterface::new(Vector2::new(200.0, 200.0));

        ButtonBuilder::new(WidgetBuilder::new().with_width(160.0).with_height(32.0))
            .with_text("Click Me!")
            .build(&mut ui.build_ctx());

        Self {
            ui,
            render_target: TextureResource::new_render_target(200, 200),
        }
    }
}

impl SceneController for UiScene {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

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
        editor_selection: &Selection,
    ) {
    }

    fn render_target(&self, engine: &Engine) -> Option<TextureResource> {
        Some(self.render_target.clone())
    }

    fn save(
        &mut self,
        path: &Path,
        settings: &Settings,
        engine: &mut Engine,
    ) -> Result<String, String> {
        let mut visitor = Visitor::new();
        self.ui.visit("Ui", &mut visitor).unwrap();
        visitor.save_binary(path).unwrap();

        Ok("".to_string())
    }

    fn do_command(
        &mut self,
        command: Box<dyn Command>,
        selection: &mut Selection,
        engine: &mut Engine,
    ) {
    }

    fn undo(&mut self, selection: &mut Selection, engine: &mut Engine) {}

    fn redo(&mut self, selection: &mut Selection, engine: &mut Engine) {}

    fn clear_command_stack(&mut self, selection: &mut Selection, engine: &mut Engine) {}

    fn on_before_render(&mut self, engine: &mut Engine) {
        Log::verify(
            engine
                .graphics_context
                .as_initialized_mut()
                .renderer
                .render_ui_to_texture(self.render_target.clone(), &mut self.ui),
        );
    }

    fn on_after_render(&mut self, engine: &mut Engine) {}

    fn update(
        &mut self,
        editor_selection: &Selection,
        engine: &mut Engine,
        dt: f32,
        path: Option<&Path>,
        settings: &mut Settings,
        screen_bounds: Rect<f32>,
    ) -> Option<TextureResource> {
        self.ui.update(screen_bounds.size, dt);

        // Create new render target if preview frame has changed its size.
        let mut new_render_target = None;
        if let TextureKind::Rectangle { width, height } =
            self.render_target.clone().data_ref().kind()
        {
            let frame_size = screen_bounds.size;
            if width != frame_size.x as u32 || height != frame_size.y as u32 {
                self.render_target =
                    TextureResource::new_render_target(frame_size.x as u32, frame_size.y as u32);
                new_render_target = Some(self.render_target.clone());
            }
        }

        while let Some(message) = self.ui.poll_message() {}

        new_render_target
    }

    fn is_interacting(&self) -> bool {
        false
    }

    fn on_destroy(&mut self, engine: &mut Engine) {}
}
