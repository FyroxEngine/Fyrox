//! Collider shape editing plugin.

mod ball;
mod capsule;
mod cone;
mod cuboid;
mod cylinder;
mod dummy;
mod segment;
mod triangle;

use crate::{
    camera::PickingOptions,
    command::SetPropertyCommand,
    fyrox::{
        asset::untyped::ResourceKind,
        core::{
            algebra::{Vector2, Vector3},
            color::Color,
            math::plane::Plane,
            pool::Handle,
            reflect::Reflect,
            type_traits::prelude::*,
            Uuid,
        },
        engine::Engine,
        graph::{BaseSceneGraph, SceneGraph},
        gui::{BuildContext, UiNode},
        material::{
            shader::{ShaderResource, ShaderResourceExtension},
            Material, MaterialResource,
        },
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
    message::MessageSender,
    plugin::EditorPlugin,
    plugins::collider::{
        ball::BallShapeGizmo, capsule::CapsuleShapeGizmo, cone::ConeShapeGizmo,
        cuboid::CuboidShapeGizmo, cylinder::CylinderShapeGizmo, dummy::DummyShapeGizmo,
        segment::SegmentShapeGizmo, triangle::TriangleShapeGizmo,
    },
    scene::{commands::GameSceneContext, controller::SceneController, GameScene, Selection},
    settings::Settings,
    Editor, Message,
};

fn set_node_position(handle: Handle<Node>, position: Vector3<f32>, scene: &mut Scene) {
    scene.graph[handle]
        .local_transform_mut()
        .set_position(position);
}

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

trait ShapeGizmoTrait {
    fn for_each_handle(&self, func: &mut dyn FnMut(Handle<Node>));

    fn handle_major_axis(&self, handle: Handle<Node>) -> Option<Vector3<f32>>;

    fn try_sync_to_collider(
        &self,
        collider: Handle<Node>,
        center: Vector3<f32>,
        side: Vector3<f32>,
        up: Vector3<f32>,
        look: Vector3<f32>,
        scene: &mut Scene,
    ) -> bool;

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

    fn is_vector_handle(&self, handle: Handle<Node>) -> bool;

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
}

fn make_shape_gizmo(
    collider: Handle<Node>,
    center: Vector3<f32>,
    side: Vector3<f32>,
    up: Vector3<f32>,
    look: Vector3<f32>,
    scene: &mut Scene,
    root: Handle<Node>,
    visible: bool,
) -> Box<dyn ShapeGizmoTrait> {
    if let Some(collider) = scene.graph.try_get_of_type::<Collider>(collider) {
        let shape = collider.shape().clone();
        match shape {
            ColliderShape::Ball(ball) => Box::new(BallShapeGizmo::new(
                &ball, center, side, root, visible, scene,
            )),
            ColliderShape::Cylinder(cylinder) => Box::new(CylinderShapeGizmo::new(
                &cylinder, center, side, up, visible, root, scene,
            )),
            ColliderShape::Cone(cone) => Box::new(ConeShapeGizmo::new(
                &cone, center, side, up, visible, root, scene,
            )),
            ColliderShape::Cuboid(cuboid) => Box::new(CuboidShapeGizmo::new(
                &cuboid, center, side, up, look, visible, root, scene,
            )),
            ColliderShape::Capsule(capsule) => Box::new(CapsuleShapeGizmo::new(
                &capsule, center, side, visible, root, scene,
            )),
            ColliderShape::Segment(segment) => Box::new(SegmentShapeGizmo::new(
                &segment, center, root, visible, scene,
            )),
            ColliderShape::Triangle(triangle) => Box::new(TriangleShapeGizmo::new(
                &triangle, center, root, visible, scene,
            )),
            ColliderShape::Trimesh(_)
            | ColliderShape::Heightfield(_)
            | ColliderShape::Polyhedron(_) => Box::new(DummyShapeGizmo),
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

fn make_handle(
    scene: &mut Scene,
    position: Vector3<f32>,
    root: Handle<Node>,
    visible: bool,
) -> Handle<Node> {
    let mut material = Material::from_shader(GIZMO_SHADER.clone(), None);

    material
        .set_texture(
            &"diffuseTexture".into(),
            load_texture(include_bytes!("../../../resources/circle.png")),
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
    initial_shape: ColliderShape,
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
            let initial_shape = collider.shape().clone();

            if let Some(handle_value) =
                self.shape_gizmo
                    .value_by_handle(result.node, self.collider, scene)
            {
                self.selected_handle = result.node;

                self.drag_context = Some(DragContext {
                    handle: result.node,
                    initial_handle_position: initial_position,
                    plane,
                    handle_major_axis: self.shape_gizmo.handle_major_axis(result.node),
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

            let value = std::mem::replace(
                scene.graph[collider].as_collider_mut().shape_mut(),
                drag_context.initial_shape,
            );

            let command = SetPropertyCommand::new(
                "shape".into(),
                Box::new(value) as Box<dyn Reflect>,
                move |ctx| {
                    ctx.get_mut::<GameSceneContext>()
                        .scene
                        .graph
                        .node_mut(collider)
                },
            );
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

        if !self
            .shape_gizmo
            .try_sync_to_collider(self.collider, center, side, up, look, scene)
        {
            let new_gizmo = make_shape_gizmo(
                self.collider,
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

                    let shape_gizmo = make_shape_gizmo(
                        *node_handle,
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
                        message_sender: editor.message_sender.clone(),
                    });

                    break;
                }
            }
        }
    }
}
