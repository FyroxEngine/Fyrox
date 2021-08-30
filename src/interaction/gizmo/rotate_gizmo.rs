use crate::{
    make_color_material,
    scene::{EditorScene, GraphSelection},
    set_mesh_diffuse_color, GameEngine,
};
use rg3d::{
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
            surface::{SurfaceBuilder, SurfaceData},
            MeshBuilder, RenderPath,
        },
        node::Node,
        transform::TransformBuilder,
    },
};
use std::sync::{Arc, RwLock};

pub enum RotateGizmoMode {
    Pitch,
    Yaw,
    Roll,
}

pub struct RotationGizmo {
    mode: RotateGizmoMode,
    pub origin: Handle<Node>,
    x_axis: Handle<Node>,
    y_axis: Handle<Node>,
    z_axis: Handle<Node>,
}

fn make_rotation_ribbon(
    graph: &mut Graph,
    rotation: UnitQuaternion<f32>,
    color: Color,
    name: &str,
) -> Handle<Node> {
    MeshBuilder::new(
        BaseBuilder::new()
            .with_name(name)
            .with_depth_offset(0.5)
            .with_local_transform(
                TransformBuilder::new()
                    .with_local_rotation(rotation)
                    .build(),
            ),
    )
    .with_render_path(RenderPath::Forward)
    .with_cast_shadows(false)
    .with_surfaces(vec![SurfaceBuilder::new(Arc::new(RwLock::new(
        SurfaceData::make_cylinder(
            30,
            0.5,
            0.05,
            false,
            &Matrix4::new_translation(&Vector3::new(0.0, -0.05, 0.0)),
        ),
    )))
    .with_material(make_color_material(color))
    .build()])
    .build(graph)
}

impl RotationGizmo {
    pub fn new(editor_scene: &EditorScene, engine: &mut GameEngine) -> Self {
        let scene = &mut engine.scenes[editor_scene.scene];
        let graph = &mut scene.graph;

        let origin = MeshBuilder::new(
            BaseBuilder::new()
                .with_name("Origin")
                .with_depth_offset(0.5)
                .with_visibility(false),
        )
        .with_render_path(RenderPath::Forward)
        .with_cast_shadows(false)
        .with_surfaces(vec![SurfaceBuilder::new(Arc::new(RwLock::new(
            SurfaceData::make_sphere(10, 10, 0.1, &Matrix4::identity()),
        )))
        .with_material(make_color_material(Color::opaque(100, 100, 100)))
        .build()])
        .build(graph);

        graph.link_nodes(origin, editor_scene.root);

        let x_axis = make_rotation_ribbon(
            graph,
            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 90.0f32.to_radians()),
            Color::RED,
            "X",
        );
        graph.link_nodes(x_axis, origin);
        let y_axis = make_rotation_ribbon(
            graph,
            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.0f32.to_radians()),
            Color::GREEN,
            "Y",
        );
        graph.link_nodes(y_axis, origin);
        let z_axis = make_rotation_ribbon(
            graph,
            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), 90.0f32.to_radians()),
            Color::BLUE,
            "Z",
        );
        graph.link_nodes(z_axis, origin);

        Self {
            mode: RotateGizmoMode::Pitch,
            origin,
            x_axis,
            y_axis,
            z_axis,
        }
    }

    pub fn set_mode(&mut self, mode: RotateGizmoMode, graph: &mut Graph) {
        self.mode = mode;

        // Restore initial colors first.
        set_mesh_diffuse_color(graph[self.origin].as_mesh_mut(), Color::WHITE);
        set_mesh_diffuse_color(graph[self.x_axis].as_mesh_mut(), Color::RED);
        set_mesh_diffuse_color(graph[self.y_axis].as_mesh_mut(), Color::GREEN);
        set_mesh_diffuse_color(graph[self.z_axis].as_mesh_mut(), Color::BLUE);

        let yellow = Color::opaque(255, 255, 0);
        match self.mode {
            RotateGizmoMode::Pitch => {
                set_mesh_diffuse_color(graph[self.x_axis].as_mesh_mut(), yellow);
            }
            RotateGizmoMode::Yaw => {
                set_mesh_diffuse_color(graph[self.y_axis].as_mesh_mut(), yellow);
            }
            RotateGizmoMode::Roll => {
                set_mesh_diffuse_color(graph[self.z_axis].as_mesh_mut(), yellow);
            }
        }
    }

    pub fn handle_pick(
        &mut self,
        picked: Handle<Node>,
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
    ) -> bool {
        let graph = &mut engine.scenes[editor_scene.scene].graph;

        if picked == self.x_axis {
            self.set_mode(RotateGizmoMode::Pitch, graph);
            true
        } else if picked == self.y_axis {
            self.set_mode(RotateGizmoMode::Yaw, graph);
            true
        } else if picked == self.z_axis {
            self.set_mode(RotateGizmoMode::Roll, graph);
            true
        } else {
            false
        }
    }

    pub fn calculate_rotation_delta(
        &self,
        editor_scene: &EditorScene,
        camera: Handle<Node>,
        mouse_offset: Vector2<f32>,
        mouse_position: Vector2<f32>,
        engine: &GameEngine,
        frame_size: Vector2<f32>,
    ) -> UnitQuaternion<f32> {
        let graph = &engine.scenes[editor_scene.scene].graph;

        if let Node::Camera(camera) = &graph[camera] {
            let transform = graph[self.origin].global_transform();

            let initial_ray = camera.make_ray(mouse_position, frame_size);
            let offset_ray = camera.make_ray(mouse_position + mouse_offset, frame_size);

            let oriented_axis = match self.mode {
                RotateGizmoMode::Pitch => transform.side(),
                RotateGizmoMode::Yaw => transform.up(),
                RotateGizmoMode::Roll => transform.look(),
            };

            let plane = Plane::from_normal_and_point(&oriented_axis, &transform.position())
                .unwrap_or_default();

            if let Some(old_pos) = initial_ray.plane_intersection_point(&plane) {
                if let Some(new_pos) = offset_ray.plane_intersection_point(&plane) {
                    let center = transform.position();
                    let old = (old_pos - center)
                        .try_normalize(std::f32::EPSILON)
                        .unwrap_or_default();
                    let new = (new_pos - center)
                        .try_normalize(std::f32::EPSILON)
                        .unwrap_or_default();

                    let angle_delta = old.dot(&new).max(-1.0).min(1.0).acos();
                    let sign = old.cross(&new).dot(&oriented_axis).signum();

                    let static_axis = match self.mode {
                        RotateGizmoMode::Pitch => Vector3::x_axis(),
                        RotateGizmoMode::Yaw => Vector3::y_axis(),
                        RotateGizmoMode::Roll => Vector3::z_axis(),
                    };
                    return UnitQuaternion::from_axis_angle(&static_axis, sign * angle_delta);
                }
            }
        }

        UnitQuaternion::default()
    }

    pub fn sync_transform(
        &self,
        graph: &mut Graph,
        selection: &GraphSelection,
        scale: Vector3<f32>,
    ) {
        if let Some((rotation, position)) = selection.global_rotation_position(graph) {
            graph[self.origin]
                .set_visibility(true)
                .local_transform_mut()
                .set_rotation(rotation)
                .set_position(position)
                .set_scale(scale);
        }
    }

    pub fn set_visible(&self, graph: &mut Graph, visible: bool) {
        graph[self.origin].set_visibility(visible);
    }
}
