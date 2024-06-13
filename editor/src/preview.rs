use crate::fyrox::graph::BaseSceneGraph;
use crate::fyrox::scene::SceneRenderingOptions;
use crate::fyrox::{
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
        HorizontalAlignment, Orientation, Thickness, UiNode, VerticalAlignment,
    },
    resource::{
        model::{Model, ModelResourceExtension},
        texture::{TextureKind, TextureResource, TextureResourceExtension},
    },
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
};
use crate::{load_image, Engine};
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
    pub fn new(engine: &mut Engine, width: u32, height: u32) -> Self {
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

        scene.rendering_options.ambient_lighting_color = Color::opaque(80, 80, 80);

        let render_target = TextureResource::new_render_target(width, height);
        scene.rendering_options = SceneRenderingOptions {
            render_target: Some(render_target.clone()),
            ..Default::default()
        }
        .into();

        let scene = engine.scenes.add(scene);

        let ctx = &mut engine.user_interfaces.first_mut().build_ctx();
        let frame;
        let fit;
        let tools_panel;
        let root = GridBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(2.0))
                .with_child({
                    frame = ImageBuilder::new(
                        WidgetBuilder::new()
                            .with_child({
                                tools_panel = StackPanelBuilder::new(
                                    WidgetBuilder::new()
                                        .with_vertical_alignment(VerticalAlignment::Top)
                                        .with_horizontal_alignment(HorizontalAlignment::Right)
                                        .with_cursor(Some(CursorIcon::Pointer))
                                        .with_opacity(Some(0.7))
                                        .on_row(0)
                                        .with_child({
                                            fit = ButtonBuilder::new(
                                                WidgetBuilder::new()
                                                    .with_margin(Thickness::uniform(1.0)),
                                            )
                                            .with_content(
                                                ImageBuilder::new(
                                                    WidgetBuilder::new()
                                                        .with_width(18.0)
                                                        .with_height(18.0)
                                                        .with_margin(Thickness::uniform(2.0)),
                                                )
                                                .with_opt_texture(load_image(include_bytes!(
                                                    "../resources/fit.png"
                                                )))
                                                .build(ctx),
                                            )
                                            .build(ctx);
                                            fit
                                        }),
                                )
                                .with_orientation(Orientation::Horizontal)
                                .build(ctx);
                                tools_panel
                            })
                            .on_row(1)
                            .with_cursor(Some(CursorIcon::Grab)),
                    )
                    .with_flip(true)
                    .with_texture(render_target.into())
                    .build(ctx);
                    frame
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

    pub fn handle_message(&mut self, message: &UiMessage, engine: &mut Engine) {
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
                                self.pitch = (self.pitch - delta.y).clamp(-90.0, 90.0);
                            }
                        }
                        self.prev_mouse_pos = pos;
                    }
                    WidgetMessage::MouseDown { button, pos } => {
                        self.prev_mouse_pos = pos;
                        engine.user_interfaces.first_mut().capture_mouse(self.frame);
                        if button == MouseButton::Left {
                            self.mode = Mode::Rotate;
                        } else if button == MouseButton::Middle {
                            self.mode = Mode::Move;
                        }
                    }
                    WidgetMessage::MouseUp { button, .. } => {
                        if (button == MouseButton::Left || button == MouseButton::Middle)
                            && self.mode != Mode::None
                            && !message.handled()
                        {
                            engine.user_interfaces.first_mut().release_mouse_capture();
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

    pub fn clear(&mut self, engine: &mut Engine) {
        let graph = &mut engine.scenes[self.scene].graph;
        if graph.is_valid_handle(self.model) {
            graph.remove_node(self.model);
            self.model = Handle::NONE;
        }
    }

    pub async fn load_model(&mut self, model: &Path, engine: &mut Engine) -> bool {
        self.clear(engine);
        if let Ok(model) = engine.resource_manager.request::<Model>(model).await {
            let scene = &mut engine.scenes[self.scene];
            self.model = model.instantiate(scene);

            self.fit_to_model(scene);
            true
        } else {
            false
        }
    }

    pub fn update(&mut self, engine: &mut Engine) {
        let scene = &mut engine.scenes[self.scene];

        // Create new render target if preview frame has changed its size.
        let (rt_width, rt_height) = if let TextureKind::Rectangle { width, height } = scene
            .rendering_options
            .render_target
            .clone()
            .unwrap()
            .data_ref()
            .kind()
        {
            (width, height)
        } else {
            unreachable!();
        };
        if let Some(frame) = engine
            .user_interfaces
            .first_mut()
            .node(self.frame)
            .cast::<Image>()
        {
            let frame_size = frame.actual_local_size();
            if rt_width != frame_size.x as u32 || rt_height != frame_size.y as u32 {
                let rt =
                    TextureResource::new_render_target(frame_size.x as u32, frame_size.y as u32);
                scene.rendering_options.render_target = Some(rt.clone());
                engine
                    .user_interfaces
                    .first_mut()
                    .send_message(ImageMessage::texture(
                        self.frame,
                        MessageDirection::ToWidget,
                        Some(rt.into()),
                    ));
            }
        }
    }

    pub fn set_model(&mut self, model: Handle<Node>, engine: &mut Engine) {
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

    pub fn destroy(self, engine: &mut Engine) {
        engine
            .user_interfaces
            .first_mut()
            .send_message(WidgetMessage::remove(self.root, MessageDirection::ToWidget));
        engine.scenes.remove(self.scene);
    }
}
