use crate::fyrox::graph::BaseSceneGraph;
use crate::fyrox::{
    core::{
        algebra::{Matrix4, UnitQuaternion, Vector2, Vector3},
        color::Color,
        math::{plane::Plane, Matrix4Ext},
        pool::Handle,
    },
    scene::{
        base::BaseBuilder,
        graph::Graph,
        mesh::{
            surface::{SurfaceBuilder, SurfaceData, SurfaceResource},
            MeshBuilder, RenderPath,
        },
        node::Node,
        transform::TransformBuilder,
    },
};
use crate::{
    make_color_material, scene::GameScene, set_mesh_diffuse_color,
    world::graph::selection::GraphSelection, Engine,
};
use fyrox::asset::untyped::ResourceKind;

pub enum ScaleGizmoMode {
    None,
    X,
    Y,
    Z,
    Uniform,
}

pub struct ScaleGizmo {
    mode: ScaleGizmoMode,
    pub origin: Handle<Node>,
    x_arrow: Handle<Node>,
    y_arrow: Handle<Node>,
    z_arrow: Handle<Node>,
    x_axis: Handle<Node>,
    y_axis: Handle<Node>,
    z_axis: Handle<Node>,
}

fn make_scale_axis(
    graph: &mut Graph,
    rotation: UnitQuaternion<f32>,
    color: Color,
    name_prefix: &str,
) -> (Handle<Node>, Handle<Node>) {
    let arrow;
    let axis = MeshBuilder::new(
        BaseBuilder::new()
            .with_cast_shadows(false)
            .with_children(&[{
                arrow = MeshBuilder::new(
                    BaseBuilder::new()
                        .with_cast_shadows(false)
                        .with_name(name_prefix.to_owned() + "Arrow")
                        .with_local_transform(
                            TransformBuilder::new()
                                .with_local_position(Vector3::new(0.0, 1.0, 0.0))
                                .build(),
                        ),
                )
                .with_render_path(RenderPath::Forward)
                .with_surfaces(vec![SurfaceBuilder::new(SurfaceResource::new_ok(
                    ResourceKind::Embedded,
                    SurfaceData::make_cube(Matrix4::new_nonuniform_scaling(&Vector3::new(
                        0.1, 0.1, 0.1,
                    ))),
                ))
                .with_material(make_color_material(color))
                .build()])
                .build(graph);
                arrow
            }])
            .with_name(name_prefix.to_owned() + "Axis")
            .with_local_transform(
                TransformBuilder::new()
                    .with_local_rotation(rotation)
                    .build(),
            ),
    )
    .with_render_path(RenderPath::Forward)
    .with_surfaces(vec![SurfaceBuilder::new(SurfaceResource::new_ok(
        ResourceKind::Embedded,
        SurfaceData::make_cylinder(10, 0.015, 1.0, true, &Matrix4::identity()),
    ))
    .with_material(make_color_material(color))
    .build()])
    .build(graph);

    (axis, arrow)
}

impl ScaleGizmo {
    pub fn new(game_scene: &GameScene, engine: &mut Engine) -> Self {
        let scene = &mut engine.scenes[game_scene.scene];
        let graph = &mut scene.graph;

        let origin = MeshBuilder::new(
            BaseBuilder::new()
                .with_cast_shadows(false)
                .with_name("Origin")
                .with_visibility(false),
        )
        .with_render_path(RenderPath::Forward)
        .with_surfaces(vec![SurfaceBuilder::new(SurfaceResource::new_ok(
            ResourceKind::Embedded,
            SurfaceData::make_cube(Matrix4::new_nonuniform_scaling(&Vector3::new(
                0.1, 0.1, 0.1,
            ))),
        ))
        .with_material(make_color_material(Color::opaque(0, 255, 255)))
        .build()])
        .build(graph);

        graph.link_nodes(origin, game_scene.editor_objects_root);

        let (x_axis, x_arrow) = make_scale_axis(
            graph,
            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), -90.0f32.to_radians()),
            Color::RED,
            "X",
        );
        graph.link_nodes(x_axis, origin);
        let (y_axis, y_arrow) = make_scale_axis(
            graph,
            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.0f32.to_radians()),
            Color::GREEN,
            "Y",
        );
        graph.link_nodes(y_axis, origin);
        let (z_axis, z_arrow) = make_scale_axis(
            graph,
            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), 90.0f32.to_radians()),
            Color::BLUE,
            "Z",
        );
        graph.link_nodes(z_axis, origin);

        Self {
            mode: ScaleGizmoMode::None,
            origin,
            x_arrow,
            y_arrow,
            z_arrow,
            x_axis,
            y_axis,
            z_axis,
        }
    }

    pub fn reset_state(&self, graph: &mut Graph) {
        set_mesh_diffuse_color(graph[self.origin].as_mesh_mut(), Color::opaque(0, 255, 255));
        set_mesh_diffuse_color(graph[self.x_axis].as_mesh_mut(), Color::RED);
        set_mesh_diffuse_color(graph[self.x_arrow].as_mesh_mut(), Color::RED);
        set_mesh_diffuse_color(graph[self.y_axis].as_mesh_mut(), Color::GREEN);
        set_mesh_diffuse_color(graph[self.y_arrow].as_mesh_mut(), Color::GREEN);
        set_mesh_diffuse_color(graph[self.z_axis].as_mesh_mut(), Color::BLUE);
        set_mesh_diffuse_color(graph[self.z_arrow].as_mesh_mut(), Color::BLUE);
    }

    pub fn set_mode(&mut self, mode: ScaleGizmoMode, graph: &mut Graph) {
        self.mode = mode;

        // Restore initial colors first.
        self.reset_state(graph);

        let yellow = Color::opaque(255, 255, 0);
        match self.mode {
            ScaleGizmoMode::None => (),
            ScaleGizmoMode::X => {
                set_mesh_diffuse_color(graph[self.x_axis].as_mesh_mut(), yellow);
                set_mesh_diffuse_color(graph[self.x_arrow].as_mesh_mut(), yellow);
            }
            ScaleGizmoMode::Y => {
                set_mesh_diffuse_color(graph[self.y_axis].as_mesh_mut(), yellow);
                set_mesh_diffuse_color(graph[self.y_arrow].as_mesh_mut(), yellow);
            }
            ScaleGizmoMode::Z => {
                set_mesh_diffuse_color(graph[self.z_axis].as_mesh_mut(), yellow);
                set_mesh_diffuse_color(graph[self.z_arrow].as_mesh_mut(), yellow);
            }
            ScaleGizmoMode::Uniform => {
                set_mesh_diffuse_color(graph[self.origin].as_mesh_mut(), yellow);
            }
        }
    }

    pub fn handle_pick(&mut self, picked: Handle<Node>, graph: &mut Graph) -> bool {
        if picked == self.x_axis || picked == self.x_arrow {
            self.set_mode(ScaleGizmoMode::X, graph);
            true
        } else if picked == self.y_axis || picked == self.y_arrow {
            self.set_mode(ScaleGizmoMode::Y, graph);
            true
        } else if picked == self.z_axis || picked == self.z_arrow {
            self.set_mode(ScaleGizmoMode::Z, graph);
            true
        } else if picked == self.origin {
            self.set_mode(ScaleGizmoMode::Uniform, graph);
            true
        } else {
            self.set_mode(ScaleGizmoMode::None, graph);
            false
        }
    }

    pub fn calculate_scale_delta(
        &self,
        camera: Handle<Node>,
        mouse_offset: Vector2<f32>,
        mouse_position: Vector2<f32>,
        graph: &Graph,
        frame_size: Vector2<f32>,
    ) -> Vector3<f32> {
        let node_global_transform = graph[self.origin].global_transform();

        let camera = &graph[camera].as_camera();
        let inv_node_transform = node_global_transform.try_inverse().unwrap_or_default();

        // Create two rays in object space.
        let initial_ray = camera
            .make_ray(mouse_position, frame_size)
            .transform(inv_node_transform);
        let offset_ray = camera
            .make_ray(mouse_position + mouse_offset, frame_size)
            .transform(inv_node_transform);

        let dlook = inv_node_transform
            .transform_vector(&(node_global_transform.position() - camera.global_position()));

        // Select plane by current active mode.
        let plane = match self.mode {
            ScaleGizmoMode::None => return Vector3::default(),
            ScaleGizmoMode::X => Plane::from_normal_and_point(
                &Vector3::new(0.0, dlook.y, dlook.z),
                &Vector3::default(),
            ),
            ScaleGizmoMode::Y => Plane::from_normal_and_point(
                &Vector3::new(dlook.x, 0.0, dlook.z),
                &Vector3::default(),
            ),
            ScaleGizmoMode::Z => Plane::from_normal_and_point(
                &Vector3::new(dlook.x, dlook.y, 0.0),
                &Vector3::default(),
            ),
            ScaleGizmoMode::Uniform => Plane::from_normal_and_point(&dlook, &Vector3::default()),
        }
        .unwrap_or_default();

        // Get two intersection points with plane and use delta between them to calculate scale delta.
        if let Some(initial_point) = initial_ray.plane_intersection_point(&plane) {
            if let Some(next_point) = offset_ray.plane_intersection_point(&plane) {
                let delta = next_point - initial_point;
                return match self.mode {
                    ScaleGizmoMode::None => unreachable!(),
                    ScaleGizmoMode::X => Vector3::new(delta.x, 0.0, 0.0),
                    ScaleGizmoMode::Y => Vector3::new(0.0, delta.y, 0.0),
                    ScaleGizmoMode::Z => Vector3::new(0.0, 0.0, delta.z),
                    ScaleGizmoMode::Uniform => {
                        // TODO: Still may behave weird.
                        let amount = delta.norm() * (delta.y + delta.x + delta.z).signum();
                        Vector3::new(amount, amount, amount)
                    }
                };
            }
        }

        Vector3::default()
    }

    pub fn sync_transform(
        &self,
        graph: &mut Graph,
        selection: &GraphSelection,
        scale: Vector3<f32>,
    ) {
        if let Some((rotation, position)) = selection.global_rotation_position(graph) {
            let node = &mut graph[self.origin];
            node.set_visibility(true);
            node.local_transform_mut()
                .set_rotation(rotation)
                .set_position(position)
                .set_scale(scale);
        }
    }

    pub fn set_visible(&self, graph: &mut Graph, visible: bool) {
        graph[self.origin].set_visibility(visible);
    }
}
