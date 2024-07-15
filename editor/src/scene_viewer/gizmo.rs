use crate::camera::CameraController;
use crate::fyrox::{
    core::{
        algebra::{Matrix4, UnitQuaternion, Vector2, Vector3},
        color::Color,
        pool::Handle,
        sstorage::ImmutableString,
    },
    engine::Engine,
    material::{Material, MaterialResource, PropertyValue},
    resource::texture::{TextureResource, TextureResourceExtension},
    scene::{
        base::BaseBuilder,
        camera::{CameraBuilder, SkyBoxKind},
        graph::Graph,
        light::{directional::DirectionalLightBuilder, BaseLightBuilder},
        mesh::{
            surface::{SurfaceBuilder, SurfaceData, SurfaceResource},
            MeshBuilder,
        },
        node::Node,
        pivot::PivotBuilder,
        transform::TransformBuilder,
        Scene,
    },
};
use crate::scene::GameScene;
use fyrox::asset::untyped::ResourceKind;

pub struct CameraRotation {
    pub yaw: f32,
    pub pitch: f32,
}

pub enum SceneGizmoAction {
    Rotate(CameraRotation),
    SwitchProjection,
}

pub struct DragContext {
    pub initial_click_pos: Vector2<f32>,
    pub initial_rotation: CameraRotation,
}

pub struct SceneGizmo {
    pub scene: Handle<Scene>,
    pub render_target: TextureResource,
    pub camera_pivot: Handle<Node>,
    pub camera_hinge: Handle<Node>,
    pub camera: Handle<Node>,
    pub pos_x: Handle<Node>,
    pub neg_x: Handle<Node>,
    pub pos_y: Handle<Node>,
    pub neg_y: Handle<Node>,
    pub pos_z: Handle<Node>,
    pub neg_z: Handle<Node>,
    pub center: Handle<Node>,
    pub drag_context: Option<DragContext>,
}

fn make_cone(transform: Matrix4<f32>, color: Color, graph: &mut Graph) -> Handle<Node> {
    let mut material = Material::standard();

    material
        .set_property(
            &ImmutableString::new("diffuseColor"),
            PropertyValue::Color(color),
        )
        .unwrap();

    MeshBuilder::new(BaseBuilder::new().with_cast_shadows(false))
        .with_surfaces(vec![SurfaceBuilder::new(SurfaceResource::new_ok(
            ResourceKind::Embedded,
            SurfaceData::make_cone(16, 0.3, 1.0, &transform),
        ))
        .with_material(MaterialResource::new_ok(Default::default(), material))
        .build()])
        .build(graph)
}

impl SceneGizmo {
    pub fn new(engine: &mut Engine) -> Self {
        let mut scene = Scene::new();

        let render_target = TextureResource::new_render_target(85, 85);
        scene.rendering_options.render_target = Some(render_target.clone());
        scene.rendering_options.clear_color = Some(Color::TRANSPARENT);

        DirectionalLightBuilder::new(BaseLightBuilder::new(BaseBuilder::new()))
            .build(&mut scene.graph);

        let pos_x;
        let neg_x;
        let pos_y;
        let neg_y;
        let pos_z;
        let neg_z;
        let center =
            MeshBuilder::new(BaseBuilder::new().with_cast_shadows(false).with_children(&[
                {
                    neg_y = make_cone(
                        Matrix4::new_translation(&Vector3::new(0.0, -1.50, 0.0)),
                        Color::WHITE,
                        &mut scene.graph,
                    );
                    neg_y
                },
                {
                    pos_y = make_cone(
                        Matrix4::new_translation(&Vector3::new(0.0, 1.50, 0.0))
                            * UnitQuaternion::from_axis_angle(
                                &Vector3::x_axis(),
                                180.0f32.to_radians(),
                            )
                            .to_homogeneous(),
                        Color::GREEN,
                        &mut scene.graph,
                    );
                    pos_y
                },
                {
                    pos_x = make_cone(
                        Matrix4::new_translation(&Vector3::new(1.50, 0.0, 0.0))
                            * UnitQuaternion::from_axis_angle(
                                &Vector3::z_axis(),
                                90.0f32.to_radians(),
                            )
                            .to_homogeneous(),
                        Color::RED,
                        &mut scene.graph,
                    );
                    pos_x
                },
                {
                    neg_x = make_cone(
                        Matrix4::new_translation(&Vector3::new(-1.50, 0.0, 0.0))
                            * UnitQuaternion::from_axis_angle(
                                &Vector3::z_axis(),
                                -90.0f32.to_radians(),
                            )
                            .to_homogeneous(),
                        Color::WHITE,
                        &mut scene.graph,
                    );
                    neg_x
                },
                {
                    pos_z = make_cone(
                        Matrix4::new_translation(&Vector3::new(0.0, 0.0, 1.50))
                            * UnitQuaternion::from_axis_angle(
                                &Vector3::x_axis(),
                                -90.0f32.to_radians(),
                            )
                            .to_homogeneous(),
                        Color::BLUE,
                        &mut scene.graph,
                    );
                    pos_z
                },
                {
                    neg_z = make_cone(
                        Matrix4::new_translation(&Vector3::new(0.0, 0.0, -1.50))
                            * UnitQuaternion::from_axis_angle(
                                &Vector3::x_axis(),
                                90.0f32.to_radians(),
                            )
                            .to_homogeneous(),
                        Color::WHITE,
                        &mut scene.graph,
                    );
                    neg_z
                },
            ]))
            .with_surfaces(vec![SurfaceBuilder::new(SurfaceResource::new_ok(
                ResourceKind::Embedded,
                SurfaceData::make_cube(Matrix4::identity()),
            ))
            .build()])
            .build(&mut scene.graph);

        let camera_hinge;
        let camera;
        let camera_pivot = PivotBuilder::new(BaseBuilder::new().with_children(&[{
            camera_hinge = PivotBuilder::new(BaseBuilder::new().with_children(&[{
                camera = CameraBuilder::new(
                    BaseBuilder::new().with_local_transform(
                        TransformBuilder::new()
                            .with_local_position(Vector3::new(0.0, 0.0, -3.0))
                            .build(),
                    ),
                )
                .with_specific_skybox(SkyBoxKind::None)
                .build(&mut scene.graph);
                camera
            }]))
            .build(&mut scene.graph);
            camera_hinge
        }]))
        .build(&mut scene.graph);

        scene.graph.update_hierarchical_data();

        Self {
            scene: engine.scenes.add(scene),
            render_target,
            camera_pivot,
            camera_hinge,
            camera,
            pos_x,
            neg_x,
            pos_y,
            neg_y,
            pos_z,
            neg_z,
            center,
            drag_context: None,
        }
    }

    pub fn sync_rotations(&self, game_scene: &GameScene, engine: &mut Engine) {
        let graph = &engine.scenes[game_scene.scene].graph;
        let hinge_rotation = **graph[game_scene.camera_controller.camera_hinge]
            .local_transform()
            .rotation();
        let pivot_rotation = **graph[game_scene.camera_controller.pivot]
            .local_transform()
            .rotation();
        let gizmo_graph = &mut engine.scenes[self.scene].graph;

        gizmo_graph[self.camera_hinge]
            .local_transform_mut()
            .set_rotation(hinge_rotation);
        gizmo_graph[self.camera_pivot]
            .local_transform_mut()
            .set_rotation(pivot_rotation);
    }

    fn parts(&self) -> [(Handle<Node>, Color); 7] {
        [
            (self.center, Color::WHITE),
            (self.pos_x, Color::RED),
            (self.neg_x, Color::WHITE),
            (self.pos_y, Color::GREEN),
            (self.neg_y, Color::WHITE),
            (self.pos_z, Color::BLUE),
            (self.neg_z, Color::WHITE),
        ]
    }

    fn pick(&self, pos: Vector2<f32>, engine: &Engine) -> Handle<Node> {
        let graph = &engine.scenes[self.scene].graph;
        let ray = graph[self.camera].as_camera().make_ray(
            pos,
            self.render_target
                .data_ref()
                .kind()
                .rectangle_size()
                .unwrap()
                .map(|c| c as f32),
        );

        let mut closest = Handle::NONE;
        let mut min_toi = f32::MAX;
        for (node, _) in self.parts() {
            let node_ref = &graph[node];
            if let Some(result) = ray.aabb_intersection(
                &node_ref
                    .local_bounding_box()
                    .transform(&node_ref.global_transform()),
            ) {
                if result.min < min_toi {
                    closest = node;
                    min_toi = result.min;
                }
            }
        }
        closest
    }

    pub fn on_mouse_move(
        &mut self,
        pos: Vector2<f32>,
        engine: &mut Engine,
        camera_controller: &mut CameraController,
    ) {
        if let Some(drag_context) = self.drag_context.as_ref() {
            let delta = pos - drag_context.initial_click_pos;
            let sens: f32 = 0.09;
            camera_controller.yaw = drag_context.initial_rotation.yaw + delta.x * -sens;
            camera_controller.pitch = drag_context.initial_rotation.pitch + delta.y * sens;
        } else {
            let graph = &engine.scenes[self.scene].graph;
            let closest = self.pick(pos, engine);
            fn set_color(node: Handle<Node>, graph: &Graph, color: Color) {
                graph[node].as_mesh().surfaces()[0]
                    .material()
                    .data_ref()
                    .set_property(
                        &ImmutableString::new("diffuseColor"),
                        PropertyValue::Color(color),
                    )
                    .unwrap();
            }
            for (node, default_color) in self.parts() {
                set_color(
                    node,
                    graph,
                    if node == closest {
                        Color::opaque(255, 255, 0)
                    } else {
                        default_color
                    },
                );
            }
        }
    }

    pub fn on_click(&mut self, pos: Vector2<f32>, engine: &Engine) -> Option<SceneGizmoAction> {
        if let Some(_drag_context) = self.drag_context.as_ref() {
            return None;
        }

        let closest = self.pick(pos, engine);
        if closest == self.neg_x {
            Some(SceneGizmoAction::Rotate(CameraRotation {
                pitch: 0.0,
                yaw: 90.0f32.to_radians(),
            }))
        } else if closest == self.pos_x {
            Some(SceneGizmoAction::Rotate(CameraRotation {
                pitch: 0.0,
                yaw: -90.0f32.to_radians(),
            }))
        } else if closest == self.neg_y {
            Some(SceneGizmoAction::Rotate(CameraRotation {
                pitch: -90.0f32.to_radians(),
                yaw: 0.0,
            }))
        } else if closest == self.pos_y {
            Some(SceneGizmoAction::Rotate(CameraRotation {
                pitch: 90.0f32.to_radians(),
                yaw: 0.0,
            }))
        } else if closest == self.neg_z {
            Some(SceneGizmoAction::Rotate(CameraRotation {
                pitch: 0.0,
                yaw: 0.0f32.to_radians(),
            }))
        } else if closest == self.pos_z {
            Some(SceneGizmoAction::Rotate(CameraRotation {
                pitch: 0.0,
                yaw: -180.0f32.to_radians(),
            }))
        } else if closest == self.center {
            Some(SceneGizmoAction::SwitchProjection)
        } else {
            None
        }
    }
}
