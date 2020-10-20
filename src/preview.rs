use crate::{
    gui::{UiMessage, UiNode},
    GameEngine,
};
use rg3d::utils::into_gui_texture;
use rg3d::{
    core::{
        math::{aabb::AxisAlignedBoundingBox, quat::Quat, vec2::Vec2, vec3::Vec3},
        pool::Handle,
    },
    gui::{
        button::ButtonBuilder,
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        message::{
            ButtonMessage, CursorIcon, MessageDirection, MouseButton, UiMessageData, WidgetMessage,
        },
        widget::WidgetBuilder,
        Thickness,
    },
    resource::texture::Texture,
    scene::{
        base::BaseBuilder, camera::CameraBuilder, node::Node, transform::TransformBuilder, Scene,
    },
};
use std::path::Path;

#[derive(Eq, PartialEq, Copy, Clone)]
enum Mode {
    None,
    Move,
    Rotate,
}

pub struct PreviewPanel {
    scene: Handle<Scene>,
    pub root: Handle<UiNode>,
    frame: Handle<UiNode>,
    camera_pivot: Handle<Node>,
    fit: Handle<UiNode>,
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

        let render_target = Texture::new_render_target();
        scene.render_target = Some(render_target.clone());

        let scene = engine.scenes.add(scene);

        let frame;
        let fit;
        let root = GridBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(2.0))
                .with_child({
                    frame = ImageBuilder::new(
                        WidgetBuilder::new()
                            .on_row(1)
                            .with_cursor(Some(CursorIcon::Grab)),
                    )
                    .with_texture(into_gui_texture(render_target))
                    .build(&mut engine.user_interface.build_ctx());
                    frame
                })
                .with_child({
                    fit = ButtonBuilder::new(WidgetBuilder::new().with_height(22.0).on_row(0))
                        .with_text("Fit")
                        .build(&mut engine.user_interface.build_ctx());
                    fit
                }),
        )
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(&mut engine.user_interface.build_ctx());

        Self {
            fit,
            root,
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

    pub fn fit_to_model(&mut self, scene: &mut Scene) {
        let mut bounding_box = AxisAlignedBoundingBox::default();
        for node in scene.graph.linear_iter() {
            if let Node::Mesh(mesh) = node {
                bounding_box.add_box(mesh.full_world_bounding_box(&scene.graph))
            }
        }

        self.yaw = 0.0;
        self.pitch = 45.0;

        let fov = scene.graph[self.camera].as_camera().fov();
        self.xz_position = bounding_box.center().xz();
        self.distance = (bounding_box.max - bounding_box.min).len() * (fov * 0.5).tan();
    }

    pub fn handle_message(&mut self, message: &UiMessage, engine: &mut GameEngine) {
        let scene = &mut engine.scenes[self.scene];

        match message.data() {
            UiMessageData::Button(msg) if message.destination() == self.fit => {
                if let ButtonMessage::Click = msg {
                    self.fit_to_model(scene);
                }
            }
            _ => (),
        }

        if message.destination() == self.frame
            && message.direction() == MessageDirection::FromWidget
        {
            if let UiMessageData::Widget(msg) = message.data() {
                match *msg {
                    WidgetMessage::MouseMove { pos, .. } => {
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
                    WidgetMessage::MouseDown { button, pos } => {
                        self.prev_mouse_pos = pos;
                        engine.user_interface.capture_mouse(self.frame);
                        if button == MouseButton::Left {
                            self.mode = Mode::Rotate;
                        } else if button == MouseButton::Middle {
                            self.mode = Mode::Move;
                        }
                    }
                    WidgetMessage::MouseUp { button, .. } => {
                        if (button == MouseButton::Left || button == MouseButton::Middle)
                            && self.mode != Mode::None
                        {
                            engine.user_interface.release_mouse_capture();
                            self.mode = Mode::None;
                        }
                    }
                    WidgetMessage::MouseWheel { amount, .. } => {
                        self.distance = (self.distance + amount).max(0.0);
                    }
                    _ => {}
                }
            }
        }

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

    pub fn clear(&mut self, engine: &mut GameEngine) {
        if self.model.is_some() {
            let scene = &mut engine.scenes[self.scene];
            scene.remove_node(self.model);
            self.model = Handle::NONE;
        }
    }

    pub async fn set_model(&mut self, model: &Path, engine: &mut GameEngine) {
        self.clear(engine);
        if let Ok(model) = engine.resource_manager.request_model(model).await {
            let scene = &mut engine.scenes[self.scene];
            self.model = model.instantiate_geometry(scene).await.unwrap();
            self.fit_to_model(scene);
        }
    }
}
