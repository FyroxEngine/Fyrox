use crate::fyrox::graph::BaseSceneGraph;
use crate::fyrox::{
    core::{
        algebra::{Matrix4, UnitQuaternion, Vector2, Vector3},
        color::Color,
        math::Matrix4Ext,
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
        pivot::PivotBuilder,
        transform::{Transform, TransformBuilder},
        Scene,
    },
};
use crate::{
    interaction::plane::PlaneKind,
    make_color_material,
    scene::{GameScene, Selection},
    set_mesh_diffuse_color, Engine,
};
use fyrox::asset::untyped::ResourceKind;

pub struct MoveGizmo {
    pub origin: Handle<Node>,
    smart_dot: Handle<Node>,
    x_arrow: Handle<Node>,
    y_arrow: Handle<Node>,
    z_arrow: Handle<Node>,
    x_axis: Handle<Node>,
    y_axis: Handle<Node>,
    z_axis: Handle<Node>,
    xy_plane: Handle<Node>,
    yz_plane: Handle<Node>,
    zx_plane: Handle<Node>,
}

fn make_smart_dot(graph: &mut Graph) -> Handle<Node> {
    let scale = 0.075;
    MeshBuilder::new(
        BaseBuilder::new()
            .with_cast_shadows(false)
            .with_name("smart_dot"),
    )
    .with_render_path(RenderPath::Forward)
    .with_surfaces(vec![{
        SurfaceBuilder::new(SurfaceResource::new_ok(
            ResourceKind::Embedded,
            SurfaceData::make_sphere(8, 8, scale, &Matrix4::identity()),
        ))
        .with_material(make_color_material(Color::WHITE))
        .build()
    }])
    .build(graph)
}

fn make_move_axis(
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
                    SurfaceData::make_cone(10, 0.05, 0.1, &Matrix4::identity()),
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

fn create_quad_plane(
    graph: &mut Graph,
    transform: Matrix4<f32>,
    color: Color,
    name: &str,
) -> Handle<Node> {
    let scale = 0.2;
    MeshBuilder::new(
        BaseBuilder::new()
            .with_cast_shadows(false)
            .with_name(name)
            .with_local_transform(
                TransformBuilder::new()
                    .with_local_scale(Vector3::new(scale, scale, scale))
                    .build(),
            ),
    )
    .with_render_path(RenderPath::Forward)
    .with_surfaces(vec![{
        SurfaceBuilder::new(SurfaceResource::new_ok(
            ResourceKind::Embedded,
            SurfaceData::make_quad(
                &(transform
                    * UnitQuaternion::from_axis_angle(&Vector3::x_axis(), 90.0f32.to_radians())
                        .to_homogeneous()),
            ),
        ))
        .with_material(make_color_material(color))
        .build()
    }])
    .build(graph)
}

impl MoveGizmo {
    pub fn new(game_scene: &GameScene, engine: &mut Engine) -> Self {
        let scene = &mut engine.scenes[game_scene.scene];
        let graph = &mut scene.graph;

        let origin = PivotBuilder::new(
            BaseBuilder::new()
                .with_name("Origin")
                .with_visibility(false),
        )
        .build(graph);

        graph.link_nodes(origin, game_scene.editor_objects_root);

        let smart_dot = make_smart_dot(graph);
        graph.link_nodes(smart_dot, origin);
        let (x_axis, x_arrow) = make_move_axis(
            graph,
            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), -90.0f32.to_radians()),
            Color::RED,
            "X",
        );
        graph.link_nodes(x_axis, origin);
        let (y_axis, y_arrow) = make_move_axis(
            graph,
            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.0f32.to_radians()),
            Color::GREEN,
            "Y",
        );
        graph.link_nodes(y_axis, origin);
        let (z_axis, z_arrow) = make_move_axis(
            graph,
            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), 90.0f32.to_radians()),
            Color::BLUE,
            "Z",
        );
        graph.link_nodes(z_axis, origin);

        let xy_transform = Matrix4::new_translation(&Vector3::new(1.5, 1.5, 0.0))
            * UnitQuaternion::from_axis_angle(&Vector3::x_axis(), 90.0f32.to_radians())
                .to_homogeneous();
        let xy_plane = create_quad_plane(graph, xy_transform, Color::BLUE, "XYPlane");
        graph.link_nodes(xy_plane, origin);

        let yz_transform = Matrix4::new_translation(&Vector3::new(0.0, 1.5, 1.5))
            * UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 90.0f32.to_radians())
                .to_homogeneous();
        let yz_plane = create_quad_plane(graph, yz_transform, Color::RED, "YZPlane");
        graph.link_nodes(yz_plane, origin);

        let zx_plane = create_quad_plane(
            graph,
            Matrix4::new_translation(&Vector3::new(1.5, 0.0, 1.5)),
            Color::GREEN,
            "ZXPlane",
        );
        graph.link_nodes(zx_plane, origin);

        Self {
            origin,
            smart_dot,
            x_arrow,
            y_arrow,
            z_arrow,
            x_axis,
            y_axis,
            z_axis,
            xy_plane,
            yz_plane,
            zx_plane,
        }
    }

    pub fn reset_state(&self, graph: &mut Graph) {
        set_mesh_diffuse_color(graph[self.x_axis].as_mesh_mut(), Color::RED);
        set_mesh_diffuse_color(graph[self.x_arrow].as_mesh_mut(), Color::RED);
        set_mesh_diffuse_color(graph[self.y_axis].as_mesh_mut(), Color::GREEN);
        set_mesh_diffuse_color(graph[self.y_arrow].as_mesh_mut(), Color::GREEN);
        set_mesh_diffuse_color(graph[self.z_axis].as_mesh_mut(), Color::BLUE);
        set_mesh_diffuse_color(graph[self.z_arrow].as_mesh_mut(), Color::BLUE);
        set_mesh_diffuse_color(graph[self.zx_plane].as_mesh_mut(), Color::GREEN);
        set_mesh_diffuse_color(graph[self.yz_plane].as_mesh_mut(), Color::RED);
        set_mesh_diffuse_color(graph[self.xy_plane].as_mesh_mut(), Color::BLUE);
        set_mesh_diffuse_color(graph[self.smart_dot].as_mesh_mut(), Color::WHITE);
    }

    pub fn apply_mode(&self, mode: Option<PlaneKind>, graph: &mut Graph) {
        // Restore initial colors first.
        self.reset_state(graph);

        if let Some(mode) = mode {
            let yellow = Color::opaque(255, 255, 0);
            match mode {
                PlaneKind::SMART => {
                    set_mesh_diffuse_color(graph[self.smart_dot].as_mesh_mut(), yellow);
                }
                PlaneKind::X => {
                    set_mesh_diffuse_color(graph[self.x_axis].as_mesh_mut(), yellow);
                    set_mesh_diffuse_color(graph[self.x_arrow].as_mesh_mut(), yellow);
                }
                PlaneKind::Y => {
                    set_mesh_diffuse_color(graph[self.y_axis].as_mesh_mut(), yellow);
                    set_mesh_diffuse_color(graph[self.y_arrow].as_mesh_mut(), yellow);
                }
                PlaneKind::Z => {
                    set_mesh_diffuse_color(graph[self.z_axis].as_mesh_mut(), yellow);
                    set_mesh_diffuse_color(graph[self.z_arrow].as_mesh_mut(), yellow);
                }
                PlaneKind::XY => {
                    set_mesh_diffuse_color(graph[self.xy_plane].as_mesh_mut(), yellow);
                }
                PlaneKind::YZ => {
                    set_mesh_diffuse_color(graph[self.yz_plane].as_mesh_mut(), yellow);
                }
                PlaneKind::ZX => {
                    set_mesh_diffuse_color(graph[self.zx_plane].as_mesh_mut(), yellow);
                }
            }
        }
    }

    pub fn handle_pick(&mut self, picked: Handle<Node>, graph: &mut Graph) -> Option<PlaneKind> {
        let mode = if picked == self.x_axis || picked == self.x_arrow {
            Some(PlaneKind::X)
        } else if picked == self.y_axis || picked == self.y_arrow {
            Some(PlaneKind::Y)
        } else if picked == self.z_axis || picked == self.z_arrow {
            Some(PlaneKind::Z)
        } else if picked == self.zx_plane {
            Some(PlaneKind::ZX)
        } else if picked == self.xy_plane {
            Some(PlaneKind::XY)
        } else if picked == self.yz_plane {
            Some(PlaneKind::YZ)
        } else if picked == self.smart_dot {
            Some(PlaneKind::SMART)
        } else {
            None
        };

        self.apply_mode(mode, graph);

        mode
    }

    pub fn transform<'a>(&self, graph: &'a mut Graph) -> &'a mut Transform {
        graph[self.origin].local_transform_mut()
    }

    pub fn calculate_offset(
        &self,
        graph: &Graph,
        camera: Handle<Node>,
        mouse_offset: Vector2<f32>,
        mouse_position: Vector2<f32>,
        frame_size: Vector2<f32>,
        plane_kind: PlaneKind,
    ) -> Vector3<f32> {
        let node_global_transform = graph[self.origin].global_transform();
        let node_local_transform = graph[self.origin].local_transform().matrix();

        let camera = &graph[camera].as_camera();
        let inv_node_transform = node_global_transform
            .try_inverse()
            .unwrap_or_else(Matrix4::identity);

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
        let plane = plane_kind.make_plane_from_view(dlook);
        if let Some(plane) = plane {
            // Get two intersection points with plane and use delta between them to calculate offset.
            if let Some(initial_point) = initial_ray.plane_intersection_point(&plane) {
                if let Some(next_point) = offset_ray.plane_intersection_point(&plane) {
                    let delta = next_point - initial_point;
                    let offset = plane_kind.project_point(delta);
                    // Make sure offset will be in local coordinates.
                    return node_local_transform.transform_vector(&offset);
                }
            }
        }
        Vector3::default()
    }

    pub fn set_position(&self, scene: &mut Scene, position: Vector3<f32>) {
        let graph = &mut scene.graph;
        let node = &mut graph[self.origin];
        node.local_transform_mut().set_position(position);
    }

    pub fn sync_transform(&self, scene: &mut Scene, selection: &Selection, scale: Vector3<f32>) {
        let graph = &mut scene.graph;
        if let Some(selection) = selection.as_graph() {
            if let Some((rotation, position)) = selection.global_rotation_position(graph) {
                let node = &mut graph[self.origin];
                node.set_visibility(true);
                node.local_transform_mut()
                    .set_rotation(rotation)
                    .set_position(position)
                    .set_scale(scale);
            }
        }
    }

    pub fn set_visible(&self, graph: &mut Graph, visible: bool) {
        graph[self.origin].set_visibility(visible);
    }

    pub fn destroy(self, graph: &mut Graph) {
        graph.remove_node(self.origin)
    }
}
