use crate::{
    gui::{UiMessage, UiNode},
    GameEngine,
};
use rg3d::{
    core::{
        math::{quat::Quat, vec2::Vec2, vec3::Vec3},
        pool::Handle,
    },
    gui::{
        image::ImageBuilder,
        message::{MessageDirection, MouseButton, UiMessageData, WidgetMessage},
        widget::WidgetBuilder,
    },
    resource::texture::Texture,
    scene::{
        base::BaseBuilder, camera::CameraBuilder, node::Node, transform::TransformBuilder, Scene,
    },
};
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Eq, PartialEq, Copy, Clone)]
enum Mode {
    None,
    Move,
    Rotate,
}

pub struct PreviewPanel {
    scene: Handle<Scene>,
    pub frame: Handle<UiNode>,
    camera_pivot: Handle<Node>,
    hinge: Handle<Node>,
    camera: Handle<Node>,
    prev_mouse_pos: Vec2,
    yaw: f32,
    pitch: f32,
    distance: f32,
    mode: Mode,
    xz_position: Vec2,
    model: Handle<Node>,
}

impl PreviewPanel {
    pub fn new(engine: &mut GameEngine) -> Self {
        let mut scene = Scene::new();

        let camera_pivot = scene.graph.add_node(BaseBuilder::new().build_node());
        let hinge = scene.graph.add_node(BaseBuilder::new().build_node());
        let camera = scene.graph.add_node(
            CameraBuilder::new(
                BaseBuilder::new().with_local_transform(
                    TransformBuilder::new()
                        .with_local_rotation(Quat::from_axis_angle(Vec3::UP, 180.0f32.to_radians()))
                        .with_local_position(Vec3::new(0.0, 0.0, 3.0))
                        .build(),
                ),
            )
            .build_node(),
        );
        scene.graph.link_nodes(hinge, camera_pivot);
        scene.graph.link_nodes(camera, hinge);

        let render_target = Arc::new(Mutex::new(Texture::default()));
        scene.render_target = Some(render_target.clone());

        let scene = engine.scenes.add(scene);

        let frame = ImageBuilder::new(WidgetBuilder::new())
            .with_texture(render_target.into())
            .build(&mut engine.user_interface.build_ctx());

        Self {
            scene,
            frame,
            camera,
            camera_pivot,
            mode: Mode::None,
            prev_mouse_pos: Default::default(),
            yaw: 0.0,
            pitch: 0.0,
            distance: 3.0,
            hinge,
            xz_position: Default::default(),
            model: Default::default(),
        }
    }

    pub fn handle_message(&mut self, message: &UiMessage, engine: &mut GameEngine) {
        if message.destination() == self.frame
            && message.direction() == MessageDirection::FromWidget
        {
            if let UiMessageData::Widget(msg) = message.data() {
                match msg {
                    &WidgetMessage::MouseMove { pos, .. } => {
                        let delta = pos - self.prev_mouse_pos;
                        match self.mode {
                            Mode::None => {}
                            Mode::Move => {
                                self.xz_position += delta;
                            }
                            Mode::Rotate => {
                                self.yaw += delta.x;
                                self.pitch = (self.pitch - delta.y).max(-90.0).min(90.0);
                            }
                        }
                        self.prev_mouse_pos = pos;
                    }
                    &WidgetMessage::MouseDown { button, pos } => {
                        self.prev_mouse_pos = pos;
                        engine.user_interface.capture_mouse(self.frame);
                        if button == MouseButton::Left {
                            self.mode = Mode::Rotate;
                        } else if button == MouseButton::Middle {
                            self.mode = Mode::Move;
                        }
                    }
                    &WidgetMessage::MouseUp { button, .. } => {
                        if (button == MouseButton::Left || button == MouseButton::Middle)
                            && self.mode != Mode::None
                        {
                            engine.user_interface.release_mouse_capture();
                            self.mode = Mode::None;
                        }
                    }
                    &WidgetMessage::MouseWheel { amount, .. } => {
                        self.distance = (self.distance + amount).max(0.0);
                    }
                    _ => {}
                }
            }

            let scene = &mut engine.scenes[self.scene];
            scene.graph[self.camera_pivot]
                .local_transform_mut()
                .set_position(Vec3::new(self.xz_position.x, 0.0, self.xz_position.y))
                .set_rotation(Quat::from_axis_angle(Vec3::UP, self.yaw.to_radians()));
            scene.graph[self.hinge]
                .local_transform_mut()
                .set_rotation(Quat::from_axis_angle(Vec3::RIGHT, self.pitch.to_radians()));
            scene.graph[self.camera]
                .local_transform_mut()
                .set_position(Vec3::new(0.0, 0.0, self.distance));
        }
    }

    pub fn set_model(&mut self, model: &Path, engine: &mut GameEngine) {
        if let Some(model) = engine.resource_manager.lock().unwrap().request_model(model) {
            let scene = &mut engine.scenes[self.scene];
            if self.model.is_some() {
                scene.remove_node(self.model);
            }
            self.model = model.lock().unwrap().instantiate_geometry(scene);
        }
    }
}
