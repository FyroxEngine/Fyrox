#![allow(unused_variables)] // TODO

use crate::{
    command::Command,
    scene::{controller::SceneController, Selection},
    settings::{keys::KeyBindings, Settings},
    world::WorldViewerDataProvider,
    Message,
};
use fyrox::{
    core::{
        algebra::Vector2,
        color::Color,
        log::Log,
        math::Rect,
        pool::{ErasedHandle, Handle},
        visitor::{Visit, Visitor},
    },
    engine::Engine,
    gui::{
        button::ButtonBuilder,
        draw::SharedTexture,
        message::{KeyCode, MouseButton},
        text::TextBuilder,
        widget::WidgetBuilder,
        UiNode, UserInterface,
    },
    resource::texture::{TextureKind, TextureResource, TextureResourceExtension},
};
use std::{any::Any, path::Path};

pub struct UiScene {
    pub ui: UserInterface,
    pub render_target: TextureResource,
}

impl Default for UiScene {
    fn default() -> Self {
        Self::new()
    }
}

impl UiScene {
    pub fn new() -> Self {
        let mut ui = UserInterface::new(Vector2::new(200.0, 200.0));

        // Create test content.
        ButtonBuilder::new(
            WidgetBuilder::new()
                .with_width(160.0)
                .with_height(32.0)
                .with_desired_position(Vector2::new(20.0, 20.0)),
        )
        .with_text("Click Me!")
        .build(&mut ui.build_ctx());

        TextBuilder::new(WidgetBuilder::new().with_desired_position(Vector2::new(300.0, 300.0)))
            .with_text("This is some text.")
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
                .render_ui_to_texture(self.render_target.clone(), &mut self.ui, Color::DARK_GRAY),
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

    fn on_message(
        &mut self,
        message: &Message,
        selection: &Selection,
        engine: &mut Engine,
    ) -> bool {
        false
    }
}

pub struct UiSceneWrapper<'a> {
    pub ui: &'a UserInterface,
    pub path: Option<&'a Path>,
}

impl<'a> WorldViewerDataProvider for UiSceneWrapper<'a> {
    fn root_node(&self) -> ErasedHandle {
        self.ui.root().into()
    }

    fn path(&self) -> Option<&Path> {
        self.path
    }

    fn children_of(&self, node: ErasedHandle) -> Vec<ErasedHandle> {
        self.ui
            .try_get_node(node.into())
            .map(|n| n.children.iter().map(|c| (*c).into()).collect::<Vec<_>>())
            .unwrap_or_default()
    }

    fn child_count_of(&self, node: ErasedHandle) -> usize {
        self.ui
            .try_get_node(node.into())
            .map(|n| n.children.len())
            .unwrap_or_default()
    }

    fn is_node_has_child(&self, node: ErasedHandle, child: ErasedHandle) -> bool {
        self.ui
            .try_get_node(node.into())
            .map_or(false, |n| n.children().iter().any(|c| *c == child.into()))
    }

    fn parent_of(&self, node: ErasedHandle) -> ErasedHandle {
        self.ui
            .try_get_node(node.into())
            .map(|n| n.parent().into())
            .unwrap_or_default()
    }

    fn name_of(&self, node: ErasedHandle) -> Option<&str> {
        self.ui.try_get_node(node.into()).map(|n| n.name())
    }

    fn is_valid_handle(&self, node: ErasedHandle) -> bool {
        self.ui.try_get_node(node.into()).is_some()
    }

    fn icon_of(&self, node: ErasedHandle) -> Option<SharedTexture> {
        // TODO
        None
    }

    fn is_instance(&self, node: ErasedHandle) -> bool {
        false
    }

    fn selection(&self) -> Vec<ErasedHandle> {
        // TODO
        Default::default()
    }

    fn on_drop(&self, child: ErasedHandle, parent: ErasedHandle) {}

    fn validate(&self) -> Vec<(ErasedHandle, Result<(), String>)> {
        Default::default()
    }

    fn on_selection_changed(&self, _new_selection: &[ErasedHandle]) {}
}
