//! Collider shape editing plugin.

use crate::{
    camera::PickingOptions,
    fyrox::{
        asset::untyped::ResourceKind,
        core::{
            algebra::{Vector2, Vector3},
            color::Color,
            math::plane::Plane,
            pool::Handle,
            type_traits::prelude::*,
            Uuid,
        },
        engine::Engine,
        graph::{BaseSceneGraph, SceneGraph},
        gui::{BuildContext, UiNode},
        material::{Material, MaterialResource},
        scene::{
            base::BaseBuilder, collider::Collider, collider::ColliderShape, node::Node,
            sprite::SpriteBuilder, transform::TransformBuilder, Scene,
        },
    },
    interaction::{
        gizmo::move_gizmo::MoveGizmo, make_interaction_mode_button, plane::PlaneKind,
        InteractionMode,
    },
    load_texture,
    plugin::EditorPlugin,
    scene::{controller::SceneController, GameScene, Selection},
    settings::Settings,
    Editor, Message,
};

enum ShapeGizmo {
    NonEditable,
    Cuboid {
        pos_x_handle: Handle<Node>,
        pos_y_handle: Handle<Node>,
        pos_z_handle: Handle<Node>,
        neg_x_handle: Handle<Node>,
        neg_y_handle: Handle<Node>,
        neg_z_handle: Handle<Node>,
    },
    Ball {
        radius_handle: Handle<Node>,
    },
    Capsule {
        radius_handle: Handle<Node>,
        begin_handle: Handle<Node>,
        end_handle: Handle<Node>,
    },
    Cylinder {
        radius_handle: Handle<Node>,
        half_height_handle: Handle<Node>,
    },
    Cone {
        radius_handle: Handle<Node>,
        half_height_handle: Handle<Node>,
    },
    Segment {
        begin_handle: Handle<Node>,
        end_handle: Handle<Node>,
    },
    Triangle {
        a_handle: Handle<Node>,
        b_handle: Handle<Node>,
        c_handle: Handle<Node>,
    },
}

fn make_handle(
    scene: &mut Scene,
    position: Vector3<f32>,
    root: Handle<Node>,
    visible: bool,
) -> Handle<Node> {
    let mut material = Material::standard_sprite();

    material
        .set_texture(
            &"diffuseTexture".into(),
            load_texture(include_bytes!("../../resources/circle.png")),
        )
        .unwrap();

    let handle = SpriteBuilder::new(
        BaseBuilder::new()
            .with_local_transform(
                TransformBuilder::new()
                    .with_local_position(position)
                    .build(),
            )
            .with_visibility(visible),
    )
    .with_material(MaterialResource::new_ok(ResourceKind::Embedded, material))
    .with_size(0.05)
    .with_color(Color::MAROON)
    .build(&mut scene.graph);

    scene.graph.link_nodes(handle, root);

    handle
}

impl ShapeGizmo {
    fn create(
        shape: ColliderShape,
        center: Vector3<f32>,
        side: Vector3<f32>,
        up: Vector3<f32>,
        look: Vector3<f32>,
        scene: &mut Scene,
        root: Handle<Node>,
        visible: bool,
    ) -> Self {
        match shape {
            ColliderShape::Ball(ball) => Self::Ball {
                radius_handle: make_handle(scene, center + side.scale(ball.radius), root, visible),
            },
            ColliderShape::Cylinder(cylinder) => Self::Cylinder {
                radius_handle: make_handle(
                    scene,
                    center + side.scale(cylinder.radius),
                    root,
                    visible,
                ),
                half_height_handle: make_handle(
                    scene,
                    center + up.scale(cylinder.half_height),
                    root,
                    visible,
                ),
            },
            ColliderShape::Cone(cone) => Self::Cone {
                radius_handle: make_handle(scene, center + side.scale(cone.radius), root, visible),
                half_height_handle: make_handle(
                    scene,
                    center + up.scale(cone.half_height),
                    root,
                    visible,
                ),
            },
            ColliderShape::Cuboid(cuboid) => Self::Cuboid {
                pos_x_handle: make_handle(
                    scene,
                    center + side.scale(cuboid.half_extents.x),
                    root,
                    visible,
                ),
                pos_y_handle: make_handle(
                    scene,
                    center + up.scale(cuboid.half_extents.y),
                    root,
                    visible,
                ),
                pos_z_handle: make_handle(
                    scene,
                    center + look.scale(cuboid.half_extents.z),
                    root,
                    visible,
                ),
                neg_x_handle: make_handle(
                    scene,
                    center - side.scale(cuboid.half_extents.x),
                    root,
                    visible,
                ),
                neg_y_handle: make_handle(
                    scene,
                    center - up.scale(cuboid.half_extents.y),
                    root,
                    visible,
                ),
                neg_z_handle: make_handle(
                    scene,
                    center - look.scale(cuboid.half_extents.z),
                    root,
                    visible,
                ),
            },
            ColliderShape::Capsule(capsule) => Self::Capsule {
                radius_handle: make_handle(
                    scene,
                    center + side.scale(capsule.radius),
                    root,
                    visible,
                ),
                begin_handle: make_handle(scene, center + capsule.begin, root, visible),
                end_handle: make_handle(scene, center + capsule.end, root, visible),
            },
            ColliderShape::Segment(segment) => Self::Segment {
                begin_handle: make_handle(scene, center + segment.begin, root, visible),
                end_handle: make_handle(scene, center + segment.end, root, visible),
            },
            ColliderShape::Triangle(triangle) => Self::Triangle {
                a_handle: make_handle(scene, center + triangle.a, root, visible),
                b_handle: make_handle(scene, center + triangle.b, root, visible),
                c_handle: make_handle(scene, center + triangle.c, root, visible),
            },
            ColliderShape::Polyhedron(_)
            | ColliderShape::Heightfield(_)
            | ColliderShape::Trimesh(_) => Self::NonEditable,
        }
    }

    fn for_each_handle<F: FnMut(Handle<Node>)>(&self, mut func: F) {
        match self {
            Self::NonEditable => {}
            Self::Cuboid {
                pos_x_handle,
                pos_y_handle,
                pos_z_handle,
                neg_x_handle,
                neg_y_handle,
                neg_z_handle,
            } => {
                for handle in [
                    pos_x_handle,
                    pos_y_handle,
                    pos_z_handle,
                    neg_x_handle,
                    neg_y_handle,
                    neg_z_handle,
                ] {
                    func(*handle)
                }
            }
            Self::Ball { radius_handle } => func(*radius_handle),
            Self::Capsule {
                radius_handle,
                begin_handle,
                end_handle,
            } => {
                for handle in [radius_handle, begin_handle, end_handle] {
                    func(*handle)
                }
            }
            Self::Cylinder {
                radius_handle,
                half_height_handle,
            }
            | Self::Cone {
                radius_handle,
                half_height_handle,
            } => {
                for handle in [radius_handle, half_height_handle] {
                    func(*handle)
                }
            }
            Self::Segment {
                begin_handle,
                end_handle,
            } => {
                for handle in [begin_handle, end_handle] {
                    func(*handle)
                }
            }
            Self::Triangle {
                a_handle,
                b_handle,
                c_handle,
            } => {
                for handle in [a_handle, b_handle, c_handle] {
                    func(*handle)
                }
            }
        }
    }

    fn handle_major_axis(&self, handle: Handle<Node>) -> Option<Vector3<f32>> {
        match self {
            ShapeGizmo::NonEditable => (),
            ShapeGizmo::Cuboid {
                pos_x_handle,
                pos_y_handle,
                pos_z_handle,
                neg_x_handle,
                neg_y_handle,
                neg_z_handle,
            } => {
                if handle == *pos_x_handle {
                    return Some(Vector3::x());
                } else if handle == *pos_y_handle {
                    return Some(Vector3::y());
                } else if handle == *pos_z_handle {
                    return Some(Vector3::z());
                } else if handle == *neg_x_handle {
                    return Some(-Vector3::x());
                } else if handle == *neg_y_handle {
                    return Some(-Vector3::y());
                } else if handle == *neg_z_handle {
                    return Some(-Vector3::z());
                }
            }
            ShapeGizmo::Ball { radius_handle } => {
                if handle == *radius_handle {
                    return Some(Vector3::x());
                }
            }
            ShapeGizmo::Capsule { radius_handle, .. } => {
                if handle == *radius_handle {
                    return Some(Vector3::x());
                }
            }
            ShapeGizmo::Cylinder {
                radius_handle,
                half_height_handle,
            }
            | ShapeGizmo::Cone {
                radius_handle,
                half_height_handle,
            } => {
                if handle == *radius_handle {
                    return Some(Vector3::x());
                } else if handle == *half_height_handle {
                    return Some(Vector3::y());
                }
            }
            ShapeGizmo::Segment { .. } | ShapeGizmo::Triangle { .. } => {
                // No sensible axis, because the value is a vector.
            }
        }

        None
    }

    fn try_sync_to_shape(
        &self,
        shape: ColliderShape,
        center: Vector3<f32>,
        side: Vector3<f32>,
        up: Vector3<f32>,
        look: Vector3<f32>,
        scene: &mut Scene,
    ) -> bool {
        let mut set_position = |handle: Handle<Node>, position: Vector3<f32>| {
            scene.graph[handle]
                .local_transform_mut()
                .set_position(position);
        };

        match (self, shape) {
            (Self::Ball { radius_handle }, ColliderShape::Ball(ball_shape)) => {
                set_position(*radius_handle, center + side.scale(ball_shape.radius));
                true
            }
            (
                Self::Cylinder {
                    radius_handle,
                    half_height_handle,
                },
                ColliderShape::Cylinder(cylinder),
            ) => {
                set_position(*radius_handle, center + side.scale(cylinder.radius));
                set_position(*half_height_handle, center + up.scale(cylinder.half_height));
                true
            }
            (
                Self::Cone {
                    radius_handle,
                    half_height_handle,
                },
                ColliderShape::Cone(cone),
            ) => {
                set_position(*radius_handle, center + side.scale(cone.radius));
                set_position(*half_height_handle, center + up.scale(cone.half_height));
                true
            }
            (
                Self::Cuboid {
                    pos_x_handle,
                    pos_y_handle,
                    pos_z_handle,
                    neg_x_handle,
                    neg_y_handle,
                    neg_z_handle,
                },
                ColliderShape::Cuboid(cuboid),
            ) => {
                set_position(*pos_x_handle, center + side.scale(cuboid.half_extents.x));
                set_position(*pos_y_handle, center + up.scale(cuboid.half_extents.y));
                set_position(*pos_z_handle, center + look.scale(cuboid.half_extents.z));
                set_position(*neg_x_handle, center - side.scale(cuboid.half_extents.x));
                set_position(*neg_y_handle, center - up.scale(cuboid.half_extents.y));
                set_position(*neg_z_handle, center - look.scale(cuboid.half_extents.z));
                true
            }
            (
                Self::Capsule {
                    radius_handle,
                    begin_handle,
                    end_handle,
                },
                ColliderShape::Capsule(capsule_shape),
            ) => {
                set_position(*radius_handle, center + side.scale(capsule_shape.radius));
                set_position(*begin_handle, center + capsule_shape.begin);
                set_position(*end_handle, center + capsule_shape.end);
                true
            }
            (
                Self::Segment {
                    begin_handle,
                    end_handle,
                },
                ColliderShape::Segment(segment),
            ) => {
                set_position(*begin_handle, center + segment.begin);
                set_position(*end_handle, center + segment.end);
                true
            }
            (
                Self::Triangle {
                    a_handle,
                    b_handle,
                    c_handle,
                },
                ColliderShape::Triangle(triangle),
            ) => {
                set_position(*a_handle, center + triangle.a);
                set_position(*b_handle, center + triangle.b);
                set_position(*c_handle, center + triangle.c);
                true
            }
            _ => false,
        }
    }

    fn value_by_handle(
        &self,
        handle: Handle<Node>,
        collider: &Collider,
    ) -> Option<ShapeHandleValue> {
        match (self, collider.shape()) {
            (Self::Ball { radius_handle }, ColliderShape::Ball(ball_shape)) => {
                if handle == *radius_handle {
                    return Some(ShapeHandleValue::Scalar(ball_shape.radius));
                }
            }
            (
                Self::Cylinder {
                    radius_handle,
                    half_height_handle,
                },
                ColliderShape::Cylinder(cylinder),
            ) => {
                if handle == *radius_handle {
                    return Some(ShapeHandleValue::Scalar(cylinder.radius));
                } else if handle == *half_height_handle {
                    return Some(ShapeHandleValue::Scalar(cylinder.half_height));
                }
            }
            (
                Self::Cone {
                    radius_handle,
                    half_height_handle,
                },
                ColliderShape::Cone(cone),
            ) => {
                if handle == *radius_handle {
                    return Some(ShapeHandleValue::Scalar(cone.radius));
                } else if handle == *half_height_handle {
                    return Some(ShapeHandleValue::Scalar(cone.half_height));
                }
            }
            (
                Self::Cuboid {
                    pos_x_handle,
                    pos_y_handle,
                    pos_z_handle,
                    neg_x_handle,
                    neg_y_handle,
                    neg_z_handle,
                },
                ColliderShape::Cuboid(cuboid),
            ) => {
                if handle == *pos_x_handle {
                    return Some(ShapeHandleValue::Scalar(cuboid.half_extents.x));
                } else if handle == *pos_y_handle {
                    return Some(ShapeHandleValue::Scalar(cuboid.half_extents.y));
                } else if handle == *pos_z_handle {
                    return Some(ShapeHandleValue::Scalar(cuboid.half_extents.z));
                } else if handle == *neg_x_handle {
                    return Some(ShapeHandleValue::Scalar(cuboid.half_extents.x));
                } else if handle == *neg_y_handle {
                    return Some(ShapeHandleValue::Scalar(cuboid.half_extents.y));
                } else if handle == *neg_z_handle {
                    return Some(ShapeHandleValue::Scalar(cuboid.half_extents.z));
                }
            }
            (
                Self::Capsule {
                    radius_handle,
                    begin_handle,
                    end_handle,
                },
                ColliderShape::Capsule(capsule_shape),
            ) => {
                if handle == *radius_handle {
                    return Some(ShapeHandleValue::Scalar(capsule_shape.radius));
                } else if handle == *begin_handle {
                    return Some(ShapeHandleValue::Vector(capsule_shape.begin));
                } else if handle == *end_handle {
                    return Some(ShapeHandleValue::Vector(capsule_shape.end));
                }
            }
            (
                Self::Segment {
                    begin_handle,
                    end_handle,
                },
                ColliderShape::Segment(segment),
            ) => {
                if handle == *begin_handle {
                    return Some(ShapeHandleValue::Vector(segment.begin));
                } else if handle == *end_handle {
                    return Some(ShapeHandleValue::Vector(segment.end));
                }
            }
            (
                Self::Triangle {
                    a_handle,
                    b_handle,
                    c_handle,
                },
                ColliderShape::Triangle(triangle),
            ) => {
                if handle == *a_handle {
                    return Some(ShapeHandleValue::Vector(triangle.a));
                } else if handle == *b_handle {
                    return Some(ShapeHandleValue::Vector(triangle.b));
                } else if handle == *c_handle {
                    return Some(ShapeHandleValue::Vector(triangle.c));
                }
            }
            _ => (),
        }

        None
    }

    fn set_value_by_handle(
        &self,
        handle: Handle<Node>,
        value: ShapeHandleValue,
        collider: &mut Collider,
        initial_collider_local_position: Vector3<f32>,
    ) -> Option<ShapeHandleValue> {
        match (self, collider.shape_mut()) {
            (Self::Ball { radius_handle }, ColliderShape::Ball(ball_shape)) => {
                if handle == *radius_handle {
                    ball_shape.radius = value.into_scalar().max(0.0);
                }
            }
            (
                Self::Cylinder {
                    radius_handle,
                    half_height_handle,
                },
                ColliderShape::Cylinder(cylinder),
            ) => {
                if handle == *radius_handle {
                    cylinder.radius = value.into_scalar().max(0.0);
                } else if handle == *half_height_handle {
                    cylinder.half_height = value.into_scalar().max(0.0);
                }
            }
            (
                Self::Cone {
                    radius_handle,
                    half_height_handle,
                },
                ColliderShape::Cone(cone),
            ) => {
                if handle == *radius_handle {
                    cone.radius = value.into_scalar().max(0.0);
                } else if handle == *half_height_handle {
                    cone.half_height = value.into_scalar().max(0.0);
                }
            }
            (
                Self::Cuboid {
                    pos_x_handle,
                    pos_y_handle,
                    pos_z_handle,
                    neg_x_handle,
                    neg_y_handle,
                    neg_z_handle,
                },
                ColliderShape::Cuboid(cuboid),
            ) => {
                if handle == *pos_x_handle {
                    cuboid.half_extents.x = value.into_scalar().max(0.0);
                } else if handle == *pos_y_handle {
                    cuboid.half_extents.y = value.into_scalar().max(0.0);
                } else if handle == *pos_z_handle {
                    cuboid.half_extents.z = value.into_scalar().max(0.0);
                } else if handle == *neg_x_handle {
                    cuboid.half_extents.x = value.into_scalar().max(0.0);
                    let transform = collider.local_transform_mut();
                    transform.set_position(Vector3::new(
                        initial_collider_local_position.x - value.into_scalar() / 2.0,
                        initial_collider_local_position.y,
                        initial_collider_local_position.z,
                    ));
                } else if handle == *neg_y_handle {
                    cuboid.half_extents.y = value.into_scalar().max(0.0);
                    let transform = collider.local_transform_mut();
                    transform.set_position(Vector3::new(
                        initial_collider_local_position.x,
                        initial_collider_local_position.y - value.into_scalar() / 2.0,
                        initial_collider_local_position.z,
                    ));
                } else if handle == *neg_z_handle {
                    cuboid.half_extents.z = value.into_scalar().max(0.0);
                    let transform = collider.local_transform_mut();
                    transform.set_position(Vector3::new(
                        initial_collider_local_position.x,
                        initial_collider_local_position.y,
                        initial_collider_local_position.z - value.into_scalar() / 2.0,
                    ));
                }
            }
            (
                Self::Capsule {
                    radius_handle,
                    begin_handle,
                    end_handle,
                },
                ColliderShape::Capsule(capsule),
            ) => {
                if handle == *radius_handle {
                    capsule.radius = value.into_scalar().max(0.0);
                } else if handle == *begin_handle {
                    capsule.begin = value.into_vector();
                } else if handle == *end_handle {
                    capsule.end = value.into_vector();
                }
            }
            (
                Self::Segment {
                    begin_handle,
                    end_handle,
                },
                ColliderShape::Segment(segment),
            ) => {
                if handle == *begin_handle {
                    segment.begin = value.into_vector();
                } else if handle == *end_handle {
                    segment.end = value.into_vector();
                }
            }
            (
                Self::Triangle {
                    a_handle,
                    b_handle,
                    c_handle,
                },
                ColliderShape::Triangle(triangle),
            ) => {
                if handle == *a_handle {
                    triangle.a = value.into_vector();
                } else if handle == *b_handle {
                    triangle.b = value.into_vector();
                } else if handle == *c_handle {
                    triangle.c = value.into_vector();
                }
            }
            _ => (),
        }

        None
    }

    fn is_vector_handle(&self, handle: Handle<Node>) -> bool {
        match self {
            ShapeGizmo::NonEditable
            | ShapeGizmo::Cuboid { .. }
            | ShapeGizmo::Ball { .. }
            | ShapeGizmo::Cylinder { .. }
            | ShapeGizmo::Cone { .. } => false,
            ShapeGizmo::Capsule {
                begin_handle,
                end_handle,
                ..
            } => handle == *begin_handle || handle == *end_handle,
            ShapeGizmo::Segment {
                begin_handle,
                end_handle,
            } => handle == *begin_handle || handle == *end_handle,
            ShapeGizmo::Triangle {
                a_handle,
                b_handle,
                c_handle,
            } => handle == *a_handle || handle == *b_handle || handle == *c_handle,
        }
    }

    fn reset_handles(&self, scene: &mut Scene) {
        self.for_each_handle(|handle| {
            scene.graph[handle].as_sprite_mut().set_color(Color::MAROON);
        });
    }

    fn destroy(self, scene: &mut Scene) {
        self.for_each_handle(|handle| scene.graph.remove_node(handle));
    }

    fn has_handle(&self, handle: Handle<Node>) -> bool {
        let mut has_handle = false;
        self.for_each_handle(|other_handle| {
            if other_handle == handle {
                has_handle = true
            }
        });
        has_handle
    }

    fn set_visibility(&self, scene: &mut Scene, visibility: bool) {
        self.for_each_handle(|handle| {
            scene.graph[handle].set_visibility(visibility);
        })
    }
}

#[derive(Copy, Clone)]
enum ShapeHandleValue {
    Scalar(f32),
    Vector(Vector3<f32>),
}

impl ShapeHandleValue {
    fn into_scalar(self) -> f32 {
        match self {
            ShapeHandleValue::Scalar(scalar) => scalar,
            ShapeHandleValue::Vector(_) => {
                unreachable!()
            }
        }
    }

    fn into_vector(self) -> Vector3<f32> {
        match self {
            ShapeHandleValue::Scalar(_) => unreachable!(),
            ShapeHandleValue::Vector(vector) => vector,
        }
    }
}

struct DragContext {
    handle: Handle<Node>,
    initial_handle_position: Vector3<f32>,
    plane: Plane,
    initial_value: ShapeHandleValue,
    initial_collider_local_position: Vector3<f32>,
    handle_major_axis: Option<Vector3<f32>>,
    plane_kind: Option<PlaneKind>,
}

#[derive(TypeUuidProvider)]
#[type_uuid(id = "a012dd4c-ce6d-4e7e-8879-fd8eddaa9677")]
pub struct ColliderShapeInteractionMode {
    collider: Handle<Node>,
    shape_gizmo: ShapeGizmo,
    move_gizmo: MoveGizmo,
    drag_context: Option<DragContext>,
    selected_handle: Handle<Node>,
}

impl ColliderShapeInteractionMode {
    fn set_visibility(
        &mut self,
        controller: &dyn SceneController,
        engine: &mut Engine,
        visibility: bool,
    ) {
        let Some(game_scene) = controller.downcast_ref::<GameScene>() else {
            return;
        };

        let scene = &mut engine.scenes[game_scene.scene];

        self.shape_gizmo.set_visibility(scene, visibility);
    }
}

impl InteractionMode for ColliderShapeInteractionMode {
    fn on_left_mouse_button_down(
        &mut self,
        _editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        mouse_position: Vector2<f32>,
        _frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        let Some(game_scene) = controller.downcast_mut::<GameScene>() else {
            return;
        };

        let scene = &mut engine.scenes[game_scene.scene];

        if let Some(result) = game_scene.camera_controller.pick(
            &scene.graph,
            PickingOptions {
                cursor_pos: mouse_position,
                editor_only: true,
                ..Default::default()
            },
        ) {
            let initial_position = scene.graph[result.node].global_position();
            let camera_view_dir = scene.graph[game_scene.camera_controller.camera]
                .look_vector()
                .try_normalize(f32::EPSILON)
                .unwrap_or_default();
            let plane = Plane::from_normal_and_point(&-camera_view_dir, &initial_position)
                .unwrap_or_default();
            let collider = scene.graph[self.collider].as_collider();
            let initial_collider_local_position = **collider.local_transform().position();

            if let Some(handle_value) = self.shape_gizmo.value_by_handle(result.node, collider) {
                self.selected_handle = result.node;

                self.drag_context = Some(DragContext {
                    handle: result.node,
                    initial_handle_position: initial_position,
                    plane,
                    handle_major_axis: self.shape_gizmo.handle_major_axis(result.node),
                    initial_value: handle_value,
                    initial_collider_local_position,
                    plane_kind: None,
                })
            } else if let Some(plane_kind) =
                self.move_gizmo.handle_pick(result.node, &mut scene.graph)
            {
                let collider = scene.graph[self.collider].as_collider();
                if let Some(handle_value) = self
                    .shape_gizmo
                    .value_by_handle(self.selected_handle, collider)
                {
                    self.drag_context = Some(DragContext {
                        handle: self.selected_handle,
                        initial_handle_position: initial_position,
                        plane,
                        handle_major_axis: None,
                        initial_value: handle_value,
                        initial_collider_local_position,
                        plane_kind: Some(plane_kind),
                    })
                }
            }
        }
    }

    fn on_left_mouse_button_up(
        &mut self,
        _editor_selection: &Selection,
        _controller: &mut dyn SceneController,
        _engine: &mut Engine,
        _mouse_pos: Vector2<f32>,
        _frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        if let Some(_drag_context) = self.drag_context.take() {
            // TODO: Commit changes using commands.
        }
    }

    fn on_mouse_move(
        &mut self,
        mouse_offset: Vector2<f32>,
        mouse_position: Vector2<f32>,
        _editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        let Some(game_scene) = controller.downcast_mut::<GameScene>() else {
            return;
        };

        let scene = &mut engine.scenes[game_scene.scene];

        self.shape_gizmo.reset_handles(scene);
        self.move_gizmo.reset_state(&mut scene.graph);

        if let Some(result) = game_scene.camera_controller.pick(
            &scene.graph,
            PickingOptions {
                cursor_pos: mouse_position,
                editor_only: true,
                ..Default::default()
            },
        ) {
            if self.shape_gizmo.has_handle(result.node) {
                scene.graph[result.node]
                    .as_sprite_mut()
                    .set_color(Color::RED);
            }

            self.move_gizmo.handle_pick(result.node, &mut scene.graph);
        }

        if let Some(drag_context) = self.drag_context.as_ref() {
            match drag_context.initial_value {
                ShapeHandleValue::Scalar(initial_value) => {
                    let camera = scene.graph[game_scene.camera_controller.camera].as_camera();
                    let ray = camera.make_ray(mouse_position, frame_size);
                    if let Some(intersection) = ray.plane_intersection_point(&drag_context.plane) {
                        let inv_transform = scene.graph[self.collider]
                            .global_transform()
                            .try_inverse()
                            .unwrap_or_default();
                        let local_space_drag_dir = inv_transform.transform_vector(
                            &(intersection - drag_context.initial_handle_position),
                        );
                        let sign = local_space_drag_dir
                            .dot(&drag_context.handle_major_axis.unwrap_or_default())
                            .signum();
                        let delta = sign
                            * drag_context
                                .initial_handle_position
                                .metric_distance(&intersection);

                        self.shape_gizmo.set_value_by_handle(
                            drag_context.handle,
                            ShapeHandleValue::Scalar(initial_value + delta),
                            scene.graph[self.collider].as_collider_mut(),
                            drag_context.initial_collider_local_position,
                        );
                    }
                }
                ShapeHandleValue::Vector(_) => {
                    if let Some(plane_kind) = drag_context.plane_kind {
                        let value = self
                            .shape_gizmo
                            .value_by_handle(
                                drag_context.handle,
                                scene.graph[self.collider].as_collider(),
                            )
                            .unwrap()
                            .into_vector();

                        let offset = self.move_gizmo.calculate_offset(
                            &scene.graph,
                            game_scene.camera_controller.camera,
                            mouse_offset,
                            mouse_position,
                            frame_size,
                            plane_kind,
                        );

                        self.shape_gizmo.set_value_by_handle(
                            drag_context.handle,
                            ShapeHandleValue::Vector(value + offset),
                            scene.graph[self.collider].as_collider_mut(),
                            drag_context.initial_collider_local_position,
                        );
                    }
                }
            }
        }
    }

    fn update(
        &mut self,
        _editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        _settings: &Settings,
    ) {
        let Some(game_scene) = controller.downcast_mut::<GameScene>() else {
            return;
        };

        let scene = &mut engine.scenes[game_scene.scene];

        let Some(collider) = scene.graph.try_get_of_type::<Collider>(self.collider) else {
            return;
        };

        let center = collider.global_position();
        let side = collider
            .side_vector()
            .try_normalize(f32::EPSILON)
            .unwrap_or_default();
        let up = collider
            .up_vector()
            .try_normalize(f32::EPSILON)
            .unwrap_or_default();
        let look = collider
            .look_vector()
            .try_normalize(f32::EPSILON)
            .unwrap_or_default();

        let shape = collider.shape().clone();
        if !self
            .shape_gizmo
            .try_sync_to_shape(shape.clone(), center, side, up, look, scene)
        {
            let new_gizmo = ShapeGizmo::create(
                shape,
                center,
                side,
                up,
                look,
                scene,
                game_scene.editor_objects_root,
                true,
            );

            let old_gizmo = std::mem::replace(&mut self.shape_gizmo, new_gizmo);

            old_gizmo.destroy(scene);
        }

        self.move_gizmo.set_visible(
            &mut scene.graph,
            self.shape_gizmo.is_vector_handle(self.selected_handle),
        );
        if let Some(selected_handle) = scene.graph.try_get(self.selected_handle) {
            let position = selected_handle.global_position();
            self.move_gizmo.set_position(scene, position)
        }
    }

    fn activate(&mut self, controller: &dyn SceneController, engine: &mut Engine) {
        self.set_visibility(controller, engine, true)
    }

    fn deactivate(&mut self, controller: &dyn SceneController, engine: &mut Engine) {
        self.set_visibility(controller, engine, false)
    }

    fn make_button(&mut self, ctx: &mut BuildContext, selected: bool) -> Handle<UiNode> {
        make_interaction_mode_button(
            ctx,
            include_bytes!("../../resources/triangle.png"),
            "Edit Collider Shape",
            selected,
        )
    }

    fn uuid(&self) -> Uuid {
        Self::type_uuid()
    }
}

#[derive(Default)]
pub struct ColliderShapePlugin {}

impl EditorPlugin for ColliderShapePlugin {
    fn on_message(&mut self, message: &Message, editor: &mut Editor) {
        let Some(entry) = editor.scenes.current_scene_entry_mut() else {
            return;
        };

        let Some(selection) = entry.selection.as_graph() else {
            return;
        };

        let Some(game_scene) = entry.controller.downcast_mut::<GameScene>() else {
            return;
        };

        let scene = &mut editor.engine.scenes[game_scene.scene];

        if let Message::SelectionChanged { .. } = message {
            if let Some(mode) = entry
                .interaction_modes
                .remove_typed::<ColliderShapeInteractionMode>()
            {
                mode.shape_gizmo.destroy(scene);
            }

            for node_handle in selection.nodes().iter() {
                if let Some(collider) = scene.graph.try_get_of_type::<Collider>(*node_handle) {
                    let center = collider.global_position();
                    let side = collider
                        .side_vector()
                        .try_normalize(f32::EPSILON)
                        .unwrap_or_default();
                    let up = collider
                        .up_vector()
                        .try_normalize(f32::EPSILON)
                        .unwrap_or_default();
                    let look = collider
                        .look_vector()
                        .try_normalize(f32::EPSILON)
                        .unwrap_or_default();

                    let shape_gizmo = ShapeGizmo::create(
                        collider.shape().clone(),
                        center,
                        side,
                        up,
                        look,
                        scene,
                        game_scene.editor_objects_root,
                        false,
                    );

                    let move_gizmo = MoveGizmo::new(game_scene, &mut editor.engine);

                    entry.interaction_modes.add(ColliderShapeInteractionMode {
                        collider: *node_handle,
                        shape_gizmo,
                        move_gizmo,
                        drag_context: None,
                        selected_handle: Default::default(),
                    });

                    break;
                }
            }
        }
    }
}
