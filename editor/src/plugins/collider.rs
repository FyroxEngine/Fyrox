//! Collider shape editing plugin.

use crate::{
    camera::PickingOptions,
    fyrox::{
        asset::untyped::ResourceKind,
        core::{
            algebra::{Vector2, Vector3},
            color::Color,
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
    interaction::{make_interaction_mode_button, InteractionMode},
    load_texture,
    plugin::EditorPlugin,
    scene::{controller::SceneController, GameScene, Selection},
    settings::Settings,
    Editor, Message,
};

enum ShapeGizmo {
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
}

fn make_handle(scene: &mut Scene, position: Vector3<f32>, root: Handle<Node>) -> Handle<Node> {
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
            .with_visibility(false),
    )
    .with_material(MaterialResource::new_ok(ResourceKind::Embedded, material))
    .with_size(0.05)
    .with_color(Color::MAROON)
    .build(&mut scene.graph);

    scene.graph.link_nodes(handle, root);

    handle
}

impl ShapeGizmo {
    fn try_create(
        shape: ColliderShape,
        center: Vector3<f32>,
        side: Vector3<f32>,
        up: Vector3<f32>,
        look: Vector3<f32>,
        scene: &mut Scene,
        root: Handle<Node>,
    ) -> Option<Self> {
        match shape {
            ColliderShape::Ball(ball_shape) => Some(Self::Ball {
                radius_handle: make_handle(scene, center + side.scale(ball_shape.radius), root),
            }),
            ColliderShape::Cylinder(cylinder_shape) => Some(Self::Cylinder {
                radius_handle: make_handle(scene, center + side.scale(cylinder_shape.radius), root),
                half_height_handle: make_handle(
                    scene,
                    center + up.scale(cylinder_shape.half_height),
                    root,
                ),
            }),
            ColliderShape::Cone(cone_shape) => Some(Self::Cone {
                radius_handle: make_handle(scene, center + side.scale(cone_shape.radius), root),
                half_height_handle: make_handle(
                    scene,
                    center + up.scale(cone_shape.half_height),
                    root,
                ),
            }),
            ColliderShape::Cuboid(cuboid_shape) => Some(Self::Cuboid {
                pos_x_handle: make_handle(
                    scene,
                    center + side.scale(cuboid_shape.half_extents.x),
                    root,
                ),
                pos_y_handle: make_handle(
                    scene,
                    center + up.scale(cuboid_shape.half_extents.y),
                    root,
                ),
                pos_z_handle: make_handle(
                    scene,
                    center + look.scale(cuboid_shape.half_extents.z),
                    root,
                ),
                neg_x_handle: make_handle(
                    scene,
                    center - side.scale(cuboid_shape.half_extents.x),
                    root,
                ),
                neg_y_handle: make_handle(
                    scene,
                    center - up.scale(cuboid_shape.half_extents.y),
                    root,
                ),
                neg_z_handle: make_handle(
                    scene,
                    center - look.scale(cuboid_shape.half_extents.z),
                    root,
                ),
            }),
            ColliderShape::Capsule(capsule_shape) => Some(Self::Capsule {
                radius_handle: make_handle(scene, center + side.scale(capsule_shape.radius), root),
                begin_handle: make_handle(scene, center + capsule_shape.begin, root),
                end_handle: make_handle(scene, center + capsule_shape.end, root),
            }),
            _ => None,
        }
    }

    fn for_each_handle<F: FnMut(Handle<Node>)>(&self, mut func: F) {
        match self {
            ShapeGizmo::Cuboid {
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
            ShapeGizmo::Ball { radius_handle } => func(*radius_handle),
            ShapeGizmo::Capsule {
                radius_handle,
                begin_handle,
                end_handle,
            } => {
                for handle in [radius_handle, begin_handle, end_handle] {
                    func(*handle)
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
                for handle in [radius_handle, half_height_handle] {
                    func(*handle)
                }
            }
        }
    }

    fn sync_to_shape(
        &self,
        shape: ColliderShape,
        center: Vector3<f32>,
        side: Vector3<f32>,
        up: Vector3<f32>,
        look: Vector3<f32>,
        scene: &mut Scene,
    ) {
        let mut set_position = |handle: Handle<Node>, position: Vector3<f32>| {
            scene.graph[handle]
                .local_transform_mut()
                .set_position(position);
        };

        match (self, shape) {
            (Self::Ball { radius_handle }, ColliderShape::Ball(ball_shape)) => {
                set_position(*radius_handle, center + side.scale(ball_shape.radius));
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
            }
            _ => (),
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

#[derive(TypeUuidProvider)]
#[type_uuid(id = "a012dd4c-ce6d-4e7e-8879-fd8eddaa9677")]
pub struct ColliderShapeInteractionMode {
    active_handle: Handle<Node>,
    collider: Handle<Node>,
    gizmo: ShapeGizmo,
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

        self.gizmo.set_visibility(scene, visibility);
    }
}

impl InteractionMode for ColliderShapeInteractionMode {
    fn on_left_mouse_button_down(
        &mut self,
        _editor_selection: &Selection,
        _controller: &mut dyn SceneController,
        _engine: &mut Engine,
        _mouse_pos: Vector2<f32>,
        _frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
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
    }

    fn on_mouse_move(
        &mut self,
        _mouse_offset: Vector2<f32>,
        mouse_position: Vector2<f32>,
        _editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        _frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        let Some(game_scene) = controller.downcast_mut::<GameScene>() else {
            return;
        };

        let scene = &mut engine.scenes[game_scene.scene];

        self.gizmo.reset_handles(scene);
        self.active_handle = Handle::NONE;

        if let Some(result) = game_scene.camera_controller.pick(
            &scene.graph,
            PickingOptions {
                cursor_pos: mouse_position,
                editor_only: true,
                ..Default::default()
            },
        ) {
            if self.gizmo.has_handle(result.node) {
                scene.graph[result.node]
                    .as_sprite_mut()
                    .set_color(Color::RED);

                self.active_handle = result.node;
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

        self.gizmo
            .sync_to_shape(collider.shape().clone(), center, side, up, look, scene);
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
                mode.gizmo.destroy(scene);
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

                    if let Some(gizmo) = ShapeGizmo::try_create(
                        collider.shape().clone(),
                        center,
                        side,
                        up,
                        look,
                        scene,
                        game_scene.editor_objects_root,
                    ) {
                        entry.interaction_modes.add(ColliderShapeInteractionMode {
                            active_handle: Default::default(),
                            collider: *node_handle,
                            gizmo,
                        })
                    }

                    break;
                }
            }
        }
    }
}
