use crate::{utils::built_in_skybox, GameEngine};
use fyrox::{
    core::{
        algebra::{UnitQuaternion, Vector2, Vector3},
        color::Color,
        math::aabb::AxisAlignedBoundingBox,
        pool::Handle,
        scope_profile,
    },
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        grid::{Column, GridBuilder, Row},
        image::{Image, ImageBuilder, ImageMessage},
        message::{CursorIcon, MessageDirection, MouseButton, UiMessage},
        stack_panel::StackPanelBuilder,
        widget::{WidgetBuilder, WidgetMessage},
        Orientation, Thickness, UiNode,
    },
    resource::texture::{Texture, TextureKind},
    scene::{
        base::BaseBuilder,
        camera::{CameraBuilder, Projection},
        debug::Line,
        light::{directional::DirectionalLightBuilder, BaseLightBuilder},
        mesh::Mesh,
        node::Node,
        pivot::PivotBuilder,
        transform::TransformBuilder,
        Scene,
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
    position: Vector3<f32>,
    model: Handle<Node>,
    pub tools_panel: Handle<UiNode>,
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
        let camera_pivot = PivotBuilder::new(BaseBuilder::new().with_children(&[{
            hinge = PivotBuilder::new(BaseBuilder::new().with_children(&[{
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
                .with_skybox(built_in_skybox())
                .build(&mut scene.graph);
                camera
            }]))
            .build(&mut scene.graph);
            hinge
        }]))
        .build(&mut scene.graph);

        scene.graph.link_nodes(hinge, camera_pivot);

        DirectionalLightBuilder::new(
            BaseLightBuilder::new(
                BaseBuilder::new().with_local_transform(
                    TransformBuilder::new()
                        .with_local_rotation(UnitQuaternion::from_axis_angle(
                            &Vector3::y_axis(),
                            45.0f32.to_radians(),
                        ))
                        .build(),
                ),
            )
            .cast_shadows(false),
        )
        .build(&mut scene.graph);

        scene.ambient_lighting_color = Color::opaque(80, 80, 80);

        let render_target = Texture::new_render_target(width, height);
        scene.render_target = Some(render_target.clone());

        let scene = engine.scenes.add(scene);

        let ctx = &mut engine.user_interface.build_ctx();
        let frame;
        let fit;
        let tools_panel;
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
                    .build(ctx);
                    frame
                })
                .with_child({
                    tools_panel = StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .with_height(22.0)
                            .on_row(0)
                            .with_child({
                                fit = ButtonBuilder::new(
                                    WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                                )
                                .with_text("Fit")
                                .build(ctx);
                                fit
                            }),
                    )
                    .with_orientation(Orientation::Horizontal)
                    .build(ctx);
                    tools_panel
                }),
        )
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(ctx);

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
            position: Default::default(),
            model: Default::default(),
            tools_panel,
        }
    }

    pub fn fit_to_model(&mut self, scene: &mut Scene) {
        let mut bounding_box = AxisAlignedBoundingBox::default();
        for node in scene.graph.linear_iter() {
            if let Some(mesh) = node.cast::<Mesh>() {
                bounding_box.add_box(mesh.accurate_world_bounding_box(&scene.graph))
            }
        }

        self.yaw = 0.0;
        self.pitch = -45.0;

        if let Projection::Perspective(proj) = scene.graph[self.camera].as_camera().projection() {
            let fov = proj.fov;
            self.position = bounding_box.center();
            self.distance = (bounding_box.max - bounding_box.min).norm() * (fov * 0.5).tan();
        }
    }

    pub fn handle_message(&mut self, message: &UiMessage, engine: &mut GameEngine) {
        scope_profile!();

        let scene = &mut engine.scenes[self.scene];

        if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == self.fit {
                self.fit_to_model(scene);
            }
        }

        if message.destination() == self.frame
            && message.direction() == MessageDirection::FromWidget
        {
            if let Some(msg) = message.data::<WidgetMessage>() {
                match *msg {
                    WidgetMessage::MouseMove { pos, .. } => {
                        let delta = pos - self.prev_mouse_pos;
                        match self.mode {
                            Mode::None => {}
                            Mode::Move => {
                                let pivot = &scene.graph[self.camera_pivot];

                                let side_vector = pivot.side_vector().normalize();
                                let up_vector = pivot.up_vector().normalize();

                                self.position +=
                                    side_vector.scale(-delta.x) + up_vector.scale(delta.y);
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
                        let step = 0.1;
                        let k = 1.0 - amount.signum() * step;

                        self.distance = (self.distance * k).max(0.0);
                    }
                    _ => {}
                }
            }
        }

        scene.graph[self.camera_pivot]
            .local_transform_mut()
            .set_position(self.position)
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

    pub async fn load_model(&mut self, model: &Path, engine: &mut GameEngine) -> bool {
        self.clear(engine);
        if let Ok(model) = engine.resource_manager.request_model(model).await {
            let scene = &mut engine.scenes[self.scene];
            self.model = model.instantiate_geometry(scene);
            self.fit_to_model(scene);
            true
        } else {
            false
        }
    }

    pub fn update(&mut self, engine: &mut GameEngine) {
        let scene = &mut engine.scenes[self.scene];

        // Create new render target if preview frame has changed its size.
        let (rt_width, rt_height) = if let TextureKind::Rectangle { width, height } =
            scene.render_target.clone().unwrap().data_ref().kind()
        {
            (width, height)
        } else {
            unreachable!();
        };
        if let Some(frame) = engine.user_interface.node(self.frame).cast::<Image>() {
            let frame_size = frame.actual_size();
            if rt_width != frame_size.x as u32 || rt_height != frame_size.y as u32 {
                let rt = Texture::new_render_target(frame_size.x as u32, frame_size.y as u32);
                scene.render_target = Some(rt.clone());
                engine.user_interface.send_message(ImageMessage::texture(
                    self.frame,
                    MessageDirection::ToWidget,
                    Some(into_gui_texture(rt)),
                ));
            }
        }
    }

    pub fn set_model(&mut self, model: Handle<Node>, engine: &mut GameEngine) {
        self.clear(engine);
        self.model = model;
        self.fit_to_model(&mut engine.scenes[self.scene])
    }

    pub fn scene(&self) -> Handle<Scene> {
        self.scene
    }

    pub fn model(&self) -> Handle<Node> {
        self.model
    }
}
