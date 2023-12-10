use crate::scene::controller::SceneController;
use crate::scene::Selection;
use crate::{load_image, settings::Settings, Engine};
use fyrox::fxhash::FxHashMap;
use fyrox::{
    core::{
        algebra::{Vector2, Vector3},
        color::Color,
        pool::Handle,
        uuid::Uuid,
    },
    gui::{
        border::BorderBuilder,
        brush::Brush,
        button::ButtonBuilder,
        decorator::DecoratorBuilder,
        image::ImageBuilder,
        key::HotKey,
        message::{KeyCode, UiMessage},
        utils::make_simple_tooltip,
        widget::WidgetBuilder,
        BuildContext, Thickness, UiNode, BRUSH_BRIGHT_BLUE, BRUSH_DARKER, BRUSH_LIGHT,
        BRUSH_LIGHTER, BRUSH_LIGHTEST,
    },
    scene::{camera::Projection, graph::Graph, node::Node},
};
use std::any::Any;

pub mod gizmo;
pub mod move_mode;
pub mod navmesh;
pub mod plane;
pub mod rotate_mode;
pub mod scale_mode;
pub mod select_mode;
pub mod terrain;

pub trait BaseInteractionMode: 'static {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T> BaseInteractionMode for T
where
    T: 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub trait InteractionMode: BaseInteractionMode {
    fn on_left_mouse_button_down(
        &mut self,
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
        settings: &Settings,
    );

    fn on_left_mouse_button_up(
        &mut self,
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
        settings: &Settings,
    );

    fn on_mouse_move(
        &mut self,
        mouse_offset: Vector2<f32>,
        mouse_position: Vector2<f32>,
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        frame_size: Vector2<f32>,
        settings: &Settings,
    );

    fn update(
        &mut self,
        #[allow(unused_variables)] editor_selection: &Selection,
        #[allow(unused_variables)] controller: &mut dyn SceneController,
        #[allow(unused_variables)] engine: &mut Engine,
        #[allow(unused_variables)] settings: &Settings,
    ) {
    }

    fn activate(
        &mut self,
        #[allow(unused_variables)] controller: &dyn SceneController,
        #[allow(unused_variables)] engine: &mut Engine,
    ) {
    }

    fn deactivate(&mut self, controller: &dyn SceneController, engine: &mut Engine);

    /// Should return `true` if the `key` was handled in any way, otherwise you may mess up
    /// keyboard message routing. Return `false` if the `key` is unhandled.
    fn on_key_down(
        &mut self,
        #[allow(unused_variables)] key: KeyCode,
        #[allow(unused_variables)] editor_selection: &Selection,
        #[allow(unused_variables)] controller: &mut dyn SceneController,
        #[allow(unused_variables)] engine: &mut Engine,
    ) -> bool {
        false
    }

    /// Should return `true` if the `key` was handled in any way, otherwise you may mess up
    /// keyboard message routing. Return `false` if the `key` is unhandled.
    fn on_key_up(
        &mut self,
        #[allow(unused_variables)] key: KeyCode,
        #[allow(unused_variables)] controller: &mut dyn SceneController,
        #[allow(unused_variables)] engine: &mut Engine,
    ) -> bool {
        false
    }

    fn handle_ui_message(
        &mut self,
        #[allow(unused_variables)] message: &UiMessage,
        #[allow(unused_variables)] editor_selection: &Selection,
        #[allow(unused_variables)] controller: &mut dyn SceneController,
        #[allow(unused_variables)] engine: &mut Engine,
    ) {
    }

    fn on_drop(&mut self, _engine: &mut Engine) {}

    fn on_hot_key(
        &mut self,
        #[allow(unused_variables)] hotkey: &HotKey,
        #[allow(unused_variables)] controller: &mut dyn SceneController,
        #[allow(unused_variables)] engine: &mut Engine,
        #[allow(unused_variables)] settings: &Settings,
    ) -> bool {
        false
    }

    fn make_button(&mut self, ctx: &mut BuildContext, selected: bool) -> Handle<UiNode>;

    fn uuid(&self) -> Uuid;
}

pub fn make_interaction_mode_button(
    ctx: &mut BuildContext,
    image: &[u8],
    tooltip: &str,
    selected: bool,
) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .with_tooltip(make_simple_tooltip(ctx, tooltip))
            .with_margin(Thickness {
                left: 1.0,
                top: 0.0,
                right: 1.0,
                bottom: 1.0,
            }),
    )
    .with_back(
        DecoratorBuilder::new(
            BorderBuilder::new(WidgetBuilder::new().with_foreground(BRUSH_DARKER))
                .with_stroke_thickness(Thickness::uniform(1.0)),
        )
        .with_normal_brush(BRUSH_LIGHT)
        .with_hover_brush(BRUSH_LIGHTER)
        .with_pressed_brush(BRUSH_LIGHTEST)
        .with_selected_brush(BRUSH_BRIGHT_BLUE)
        .with_selected(selected)
        .build(ctx),
    )
    .with_content(
        ImageBuilder::new(
            WidgetBuilder::new()
                .with_background(Brush::Solid(Color::opaque(220, 220, 220)))
                .with_margin(Thickness::uniform(2.0))
                .with_width(23.0)
                .with_height(23.0),
        )
        .with_opt_texture(load_image(image))
        .build(ctx),
    )
    .build(ctx)
}

pub fn calculate_gizmo_distance_scaling(
    graph: &Graph,
    camera: Handle<Node>,
    gizmo_origin: Handle<Node>,
) -> Vector3<f32> {
    let s = match graph[camera].as_camera().projection() {
        Projection::Perspective(proj) => {
            distance_scale_factor(proj.fov)
                * graph[gizmo_origin]
                    .global_position()
                    .metric_distance(&graph[camera].global_position())
        }
        Projection::Orthographic(ortho) => 0.4 * ortho.vertical_size,
    };

    Vector3::new(s, s, s)
}

fn distance_scale_factor(fov: f32) -> f32 {
    fov.tan() * 0.1
}

#[derive(Default)]
pub struct InteractionModeContainer {
    pub map: FxHashMap<Uuid, Box<dyn InteractionMode>>,
}

impl InteractionModeContainer {
    pub fn add<T: InteractionMode>(&mut self, mode: T) {
        self.map.insert(mode.uuid(), Box::new(mode));
    }
}
