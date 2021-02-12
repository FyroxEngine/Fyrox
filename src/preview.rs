use crate::{
    gui::{UiMessage, UiNode},
    GameEngine,
};
use rg3d::core::color::Color;
use rg3d::core::scope_profile;
use rg3d::scene::Line;
use rg3d::{
    core::{
        algebra::{UnitQuaternion, Vector2, Vector3},
        math::aabb::AxisAlignedBoundingBox,
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
    utils::into_gui_texture,
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
    prev_mouse_pos: Vector2<f32>,
    yaw: f32,
    pitch: f32,
    distance: f32,
    mode: Mode,
    xz_position: Vector2<f32>,
    model: Handle<Node>,
}

impl PreviewPanel {
    pub fn new(engine: &mut GameEngine, width: u32, height: u32) -> Self {
        let mut scene = Scene::new();

        let size = 10;

        for x in -size..=size {
            if x == 0 {
                // Z Axis
                scene.drawing_context.add_line(Line {
                    begin: Vector3::new(x as f32, 0.0, -size as f32),
                    end: Vector3::new(x as f32, 0.0, 0.0),
                    color: Color::BLACK,
                });
                scene.drawing_context.add_line(Line {
                    begin: Vector3::new(x as f32, 0.0, 0.0),
                    end: Vector3::new(x as f32, 0.0, size as f32),
                    color: Color::BLUE,
                });
            } else {
                scene.drawing_context.add_line(Line {
                    begin: Vector3::new(x as f32, 0.0, -size as f32),
                    end: Vector3::new(x as f32, 0.0, size as f32),
                    color: Color::BLACK,
                });
            }
        }

        for z in -size..=size {
            if z == 0 {
                // X Axis
                scene.drawing_context.add_line(Line {
                    begin: Vector3::new(-size as f32, 0.0, z as f32),
                    end: Vector3::new(0.0, 0.0, z as f32),
                    color: Color::BLACK,
                });
                scene.drawing_context.add_line(Line {
                    begin: Vector3::new(0.0, 0.0, z as f32),
                    end: Vector3::new(size as f32, 0.0, z as f32),
                    color: Color::RED,
                });
            } else {
                scene.drawing_context.add_line(Line {
                    begin: Vector3::new(-size as f32, 0.0, z as f32),
                    end: Vector3::new(size as f32, 0.0, z as f32),
                    color: Color::BLACK,
                });
            }
        }

        // Y Axis
        scene.drawing_context.add_line(Line {
            begin: Vector3::new(0.0, 0.0, 0.0),
            end: Vector3::new(0.0, 2.0, 0.0),
            color: Color::GREEN,
        });

        let camera;
        let hinge;
        let camera_pivot = BaseBuilder::new()
            .with_children(&[{
                hinge = BaseBuilder::new()
                    .with_children(&[{
                        camera = CameraBuilder::new(
                            BaseBuilder::new().with_local_transform(
                                TransformBuilder::new()
                                    .with_local_rotation(UnitQuaternion::from_axis_angle(
                                        &Vector3::y_axis(),
                                        180.0f32.to_radians(),
                                    ))
                                    .with_local_position(Vector3::new(0.0, 0.0, 3.0))
                                    .build(),
                            ),
                        )
                        .build(&mut scene.graph);
                        camera
                    }])
                    .build(&mut scene.graph);
                hinge
            }])
            .build(&mut scene.graph);

        scene.graph.link_nodes(hinge, camera_pivot);

        let render_target = Texture::new_render_target(width, height);
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
                    .with_flip(true)
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
            pitch: -45.0,
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
        self.pitch = -45.0;

        let fov = scene.graph[self.camera].as_camera().fov();
        self.xz_position = bounding_box.center().xz();
        self.distance = (bounding_box.max - bounding_box.min).norm() * (fov * 0.5).tan();
    }

    pub fn handle_message(&mut self, message: &UiMessage, engine: &mut GameEngine) {
        scope_profile!();

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
                                self.yaw -= delta.x;
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
                        self.distance = (self.distance - amount).max(0.0);
                    }
                    _ => {}
                }
            }
        }

        scene.graph[self.camera_pivot]
            .local_transform_mut()
            .set_position(Vector3::new(self.xz_position.x, 0.0, self.xz_position.y))
            .set_rotation(UnitQuaternion::from_axis_angle(
                &Vector3::y_axis(),
                self.yaw.to_radians(),
            ));
        scene.graph[self.hinge].local_transform_mut().set_rotation(
            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), self.pitch.to_radians()),
        );
        scene.graph[self.camera]
            .local_transform_mut()
            .set_position(Vector3::new(0.0, 0.0, self.distance));
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
            self.model = model.instantiate_geometry(scene);
            self.fit_to_model(scene);
        }
    }
}
