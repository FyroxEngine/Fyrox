//! Collider shape editing plugin.

mod ball;
mod ball2d;
mod capsule;
mod capsule2d;
mod cone;
mod cuboid;
mod cuboid2d;
mod cylinder;
mod dummy;
mod segment;
mod segment2d;
mod triangle;
mod triangle2d;

use crate::{
    camera::PickingOptions,
    command::SetPropertyCommand,
    fyrox::{
        asset::untyped::ResourceKind,
        core::{
            algebra::{UnitQuaternion, Vector2, Vector3},
            color::Color,
            math::{plane::Plane, Matrix4Ext},
            pool::Handle,
            reflect::Reflect,
            type_traits::prelude::*,
            Uuid,
        },
        engine::Engine,
        graph::{BaseSceneGraph, SceneGraph, SceneGraphNode},
        gui::{BuildContext, UiNode},
        material::{
            shader::{ShaderResource, ShaderResourceExtension},
            Material, MaterialResource,
        },
        scene::{
            base::BaseBuilder, collider::Collider, collider::ColliderShape, dim2, node::Node,
            sprite::SpriteBuilder, Scene,
        },
    },
    interaction::{
        calculate_gizmo_distance_scaling, gizmo::move_gizmo::MoveGizmo,
        make_interaction_mode_button, plane::PlaneKind, InteractionMode,
    },
    load_texture,
    message::MessageSender,
    plugin::EditorPlugin,
    plugins::collider::{
        ball::BallShapeGizmo, ball2d::Ball2DShapeGizmo, capsule::CapsuleShapeGizmo,
        capsule2d::Capsule2DShapeGizmo, cone::ConeShapeGizmo, cuboid::CuboidShapeGizmo,
        cuboid2d::Cuboid2DShapeGizmo, cylinder::CylinderShapeGizmo, dummy::DummyShapeGizmo,
        segment::SegmentShapeGizmo, segment2d::Segment2DShapeGizmo, triangle::TriangleShapeGizmo,
        triangle2d::Triangle2DShapeGizmo,
    },
    scene::{commands::GameSceneContext, controller::SceneController, GameScene, Selection},
    settings::Settings,
    Editor, Message,
};
use fyrox::scene::sprite::Sprite;

fn try_get_collider_shape(collider: Handle<Node>, scene: &Scene) -> Option<ColliderShape> {
    scene
        .graph
        .try_get_of_type::<Collider>(collider)
        .map(|c| c.shape().clone())
}

fn try_get_collider_shape_mut(
    collider: Handle<Node>,
    scene: &mut Scene,
) -> Option<&mut ColliderShape> {
    scene
        .graph
        .try_get_mut_of_type::<Collider>(collider)
        .map(|c| c.shape_mut())
}

fn try_get_collider_shape_2d(
    collider: Handle<Node>,
    scene: &Scene,
) -> Option<dim2::collider::ColliderShape> {
    scene
        .graph
        .try_get_of_type::<dim2::collider::Collider>(collider)
        .map(|c| c.shape().clone())
}

fn try_get_collider_shape_mut_2d(
    collider: Handle<Node>,
    scene: &mut Scene,
) -> Option<&mut dim2::collider::ColliderShape> {
    scene
        .graph
        .try_get_mut_of_type::<dim2::collider::Collider>(collider)
        .map(|c| c.shape_mut())
}

trait ShapeGizmoTrait {
    fn for_each_handle(&self, func: &mut dyn FnMut(Handle<Node>));

    fn handle_local_position(
        &self,
        handle: Handle<Node>,
        collider: Handle<Node>,
        scene: &Scene,
    ) -> Option<Vector3<f32>>;

    fn handle_major_axis(
        &self,
        _handle: Handle<Node>,
        _collider: Handle<Node>,
        _scene: &Scene,
    ) -> Option<Vector3<f32>> {
        None
    }

    fn value_by_handle(
        &self,
        handle: Handle<Node>,
        collider: Handle<Node>,
        scene: &Scene,
    ) -> Option<ShapeHandleValue>;

    fn set_value_by_handle(
        &self,
        handle: Handle<Node>,
        value: ShapeHandleValue,
        collider: Handle<Node>,
        scene: &mut Scene,
        initial_collider_local_position: Vector3<f32>,
    );

    fn is_vector_handle(&self, _handle: Handle<Node>) -> bool {
        false
    }

    fn reset_handles(&self, scene: &mut Scene) {
        self.for_each_handle(&mut |handle| {
            scene.graph[handle].as_sprite_mut().set_color(Color::MAROON);
        });
    }

    fn destroy(self: Box<Self>, scene: &mut Scene) {
        self.for_each_handle(&mut |handle| scene.graph.remove_node(handle));
    }

    fn has_handle(&self, handle: Handle<Node>) -> bool {
        let mut has_handle = false;
        self.for_each_handle(&mut |other_handle| {
            if other_handle == handle {
                has_handle = true
            }
        });
        has_handle
    }

    fn set_visibility(&self, scene: &mut Scene, visibility: bool) {
        self.for_each_handle(&mut |handle| {
            scene.graph[handle].set_visibility(visibility);
        })
    }

    fn try_sync_to_collider(
        &self,
        collider: Handle<Node>,
        camera: Handle<Node>,
        scene: &mut Scene,
    ) -> bool {
        let mut is_ok = true;
        let transform = scene.graph[collider].global_transform();
        self.for_each_handle(&mut |handle| {
            if let Some(local_position) = self.handle_local_position(handle, collider, scene) {
                let scale = calculate_gizmo_distance_scaling(&scene.graph, camera, handle);

                let node = &mut scene.graph[handle];
                node.local_transform_mut()
                    .set_position(transform.transform_point(&local_position.into()).coords)
                    .set_scale(scale)
                    .set_rotation(UnitQuaternion::from_matrix_eps(
                        &transform.basis(),
                        f32::EPSILON,
                        16,
                        Default::default(),
                    ));
                if let Some(sprite) = node.component_mut::<Sprite>() {
                    sprite.set_size(0.05 * scale.x);
                }
            } else {
                is_ok = false;
            }
        });
        is_ok
    }
}

fn make_shape_gizmo(
    collider: Handle<Node>,
    scene: &mut Scene,
    root: Handle<Node>,
    visible: bool,
) -> Box<dyn ShapeGizmoTrait> {
    if let Some(collider) = scene.graph.try_get_of_type::<Collider>(collider) {
        let shape = collider.shape().clone();
        use fyrox::scene::collider::ColliderShape;
        match shape {
            ColliderShape::Ball(_) => Box::new(BallShapeGizmo::new(root, visible, scene)),
            ColliderShape::Cylinder(_) => Box::new(CylinderShapeGizmo::new(visible, root, scene)),
            ColliderShape::Cone(_) => Box::new(ConeShapeGizmo::new(visible, root, scene)),
            ColliderShape::Cuboid(_) => Box::new(CuboidShapeGizmo::new(visible, root, scene)),
            ColliderShape::Capsule(_) => Box::new(CapsuleShapeGizmo::new(visible, root, scene)),
            ColliderShape::Segment(_) => Box::new(SegmentShapeGizmo::new(root, visible, scene)),
            ColliderShape::Triangle(_) => Box::new(TriangleShapeGizmo::new(root, visible, scene)),
            ColliderShape::Trimesh(_)
            | ColliderShape::Heightfield(_)
            | ColliderShape::Polyhedron(_) => Box::new(DummyShapeGizmo),
        }
    } else if let Some(collider) = scene
        .graph
        .try_get_of_type::<dim2::collider::Collider>(collider)
    {
        let shape = collider.shape().clone();
        use dim2::collider::ColliderShape;
        match shape {
            ColliderShape::Ball(_) => Box::new(Ball2DShapeGizmo::new(root, visible, scene)),
            ColliderShape::Cuboid(_) => Box::new(Cuboid2DShapeGizmo::new(visible, root, scene)),
            ColliderShape::Capsule(_) => Box::new(Capsule2DShapeGizmo::new(visible, root, scene)),
            ColliderShape::Segment(_) => Box::new(Segment2DShapeGizmo::new(root, visible, scene)),
            ColliderShape::Triangle(_) => Box::new(Triangle2DShapeGizmo::new(root, visible, scene)),
            ColliderShape::Trimesh(_)
            | ColliderShape::Heightfield(_)
            | ColliderShape::TileMap(_) => Box::new(DummyShapeGizmo),
        }
    } else {
        Box::new(DummyShapeGizmo)
    }
}

lazy_static! {
    static ref GIZMO_SHADER: ShaderResource = {
        ShaderResource::from_str(
            include_str!("../../../resources/shaders/sprite_gizmo.shader",),
            Default::default(),
        )
        .unwrap()
    };
}

fn make_handle(scene: &mut Scene, root: Handle<Node>, visible: bool) -> Handle<Node> {
    let mut material = Material::from_shader(GIZMO_SHADER.clone(), None);

    material
        .set_texture(
            &"diffuseTexture".into(),
            load_texture(include_bytes!("../../../resources/circle.png")),
        )
        .unwrap();

    let handle = SpriteBuilder::new(BaseBuilder::new().with_visibility(visible))
        .with_material(MaterialResource::new_ok(ResourceKind::Embedded, material))
        .with_size(0.05)
        .with_color(Color::MAROON)
        .build(&mut scene.graph);

    scene.graph.link_nodes(handle, root);

    handle
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

#[derive(Clone)]
enum ColliderInitialShape {
    TwoD(dim2::collider::ColliderShape),
    ThreeD(ColliderShape),
}

struct DragContext {
    handle: Handle<Node>,
    initial_handle_position: Vector3<f32>,
    plane: Plane,
    initial_value: ShapeHandleValue,
    initial_collider_local_position: Vector3<f32>,
    handle_major_axis: Option<Vector3<f32>>,
    plane_kind: Option<PlaneKind>,
    initial_shape: ColliderInitialShape,
}

#[derive(TypeUuidProvider)]
#[type_uuid(id = "a012dd4c-ce6d-4e7e-8879-fd8eddaa9677")]
pub struct ColliderShapeInteractionMode {
    collider: Handle<Node>,
    shape_gizmo: Box<dyn ShapeGizmoTrait>,
    move_gizmo: MoveGizmo,
    drag_context: Option<DragContext>,
    selected_handle: Handle<Node>,
    message_sender: MessageSender,
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
                filter: Some(&mut |handle, _| handle != self.move_gizmo.origin),
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
            let collider_node = &scene.graph[self.collider];
            let initial_collider_local_position = **collider_node.local_transform().position();

            let initial_shape = if let Some(collider) = collider_node.component_ref::<Collider>() {
                ColliderInitialShape::ThreeD(collider.shape().clone())
            } else if let Some(collider_2d) =
                collider_node.component_ref::<dim2::collider::Collider>()
            {
                ColliderInitialShape::TwoD(collider_2d.shape().clone())
            } else {
                unreachable!();
            };

            if let Some(handle_value) =
                self.shape_gizmo
                    .value_by_handle(result.node, self.collider, scene)
            {
                self.selected_handle = result.node;

                self.drag_context = Some(DragContext {
                    handle: result.node,
                    initial_handle_position: initial_position,
                    plane,
                    handle_major_axis: self.shape_gizmo.handle_major_axis(
                        result.node,
                        self.collider,
                        scene,
                    ),
                    initial_value: handle_value,
                    initial_collider_local_position,
                    plane_kind: None,
                    initial_shape,
                })
            } else if let Some(plane_kind) =
                self.move_gizmo.handle_pick(result.node, &mut scene.graph)
            {
                if let Some(handle_value) =
                    self.shape_gizmo
                        .value_by_handle(self.selected_handle, self.collider, scene)
                {
                    self.drag_context = Some(DragContext {
                        handle: self.selected_handle,
                        initial_handle_position: initial_position,
                        plane,
                        handle_major_axis: None,
                        initial_value: handle_value,
                        initial_collider_local_position,
                        plane_kind: Some(plane_kind),
                        initial_shape,
                    })
                }
            }
        }
    }

    fn on_left_mouse_button_up(
        &mut self,
        _editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        _mouse_pos: Vector2<f32>,
        _frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        let Some(game_scene) = controller.downcast_mut::<GameScene>() else {
            return;
        };

        let scene = &mut engine.scenes[game_scene.scene];

        if let Some(drag_context) = self.drag_context.take() {
            let collider = self.collider;

            let value = if let (Some(collider), ColliderInitialShape::ThreeD(shape)) = (
                scene.graph.try_get_mut_of_type::<Collider>(collider),
                drag_context.initial_shape.clone(),
            ) {
                Box::new(std::mem::replace(collider.shape_mut(), shape)) as Box<dyn Reflect>
            } else if let (Some(collider), ColliderInitialShape::TwoD(shape)) = (
                scene
                    .graph
                    .try_get_mut_of_type::<dim2::collider::Collider>(collider),
                drag_context.initial_shape.clone(),
            ) {
                Box::new(std::mem::replace(collider.shape_mut(), shape)) as Box<dyn Reflect>
            } else {
                unreachable!();
            };

            let command = SetPropertyCommand::new("shape".into(), value, move |ctx| {
                ctx.get_mut::<GameSceneContext>()
                    .scene
                    .graph
                    .node_mut(collider)
            });
            self.message_sender.do_command(command);
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
                filter: Some(&mut |handle, _| handle != self.move_gizmo.origin),
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
                            self.collider,
                            scene,
                            drag_context.initial_collider_local_position,
                        );
                    }
                }
                ShapeHandleValue::Vector(_) => {
                    if let Some(plane_kind) = drag_context.plane_kind {
                        let value = self
                            .shape_gizmo
                            .value_by_handle(drag_context.handle, self.collider, scene)
                            .unwrap()
                            .into_vector();

                        let global_offset = self.move_gizmo.calculate_offset(
                            &scene.graph,
                            game_scene.camera_controller.camera,
                            mouse_offset,
                            mouse_position,
                            frame_size,
                            plane_kind,
                        );

                        let local_offset = scene.graph[self.collider]
                            .global_transform()
                            .try_inverse()
                            .unwrap_or_default()
                            .transform_vector(&global_offset);

                        self.shape_gizmo.set_value_by_handle(
                            drag_context.handle,
                            ShapeHandleValue::Vector(value + local_offset),
                            self.collider,
                            scene,
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

        if !self.shape_gizmo.try_sync_to_collider(
            self.collider,
            game_scene.camera_controller.camera,
            scene,
        ) {
            let new_gizmo =
                make_shape_gizmo(self.collider, scene, game_scene.editor_objects_root, true);

            let old_gizmo = std::mem::replace(&mut self.shape_gizmo, new_gizmo);

            old_gizmo.destroy(scene);
        }

        self.move_gizmo.set_visible(
            &mut scene.graph,
            self.shape_gizmo.is_vector_handle(self.selected_handle),
        );
        let scale = calculate_gizmo_distance_scaling(
            &scene.graph,
            game_scene.camera_controller.camera,
            self.move_gizmo.origin,
        );
        if let Some(handle_local_position) =
            self.shape_gizmo
                .handle_local_position(self.selected_handle, self.collider, scene)
        {
            let transform = scene.graph[self.collider].global_transform();
            let position = transform
                .transform_point(&handle_local_position.into())
                .coords;
            self.move_gizmo
                .transform(&mut scene.graph)
                .set_position(position)
                .set_scale(scale);
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
            include_bytes!("../../../resources/triangle.png"),
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
                if let Some(collider) = scene.graph.try_get(*node_handle) {
                    if collider.component_ref::<Collider>().is_none()
                        && collider
                            .component_ref::<dim2::collider::Collider>()
                            .is_none()
                    {
                        continue;
                    }

                    let shape_gizmo = make_shape_gizmo(
                        *node_handle,
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
                        message_sender: editor.message_sender.clone(),
                    });

                    break;
                }
            }
        }
    }
}
