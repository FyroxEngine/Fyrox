//! Collider shape editing plugin.

use crate::{
    camera::PickingOptions,
    fyrox::{
        asset::untyped::ResourceKind,
        core::{algebra::Vector3, color::Color, pool::Handle},
        graph::{BaseSceneGraph, SceneGraph},
        gui::{message::UiMessage, widget::WidgetMessage},
        material::{Material, MaterialResource},
        scene::{
            base::BaseBuilder, collider::Collider, collider::ColliderShape, node::Node,
            sprite::SpriteBuilder, transform::TransformBuilder, Scene,
        },
    },
    load_texture,
    plugin::EditorPlugin,
    scene::GameScene,
    Editor, Message,
};

enum ShapeHandles {
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
        BaseBuilder::new().with_local_transform(
            TransformBuilder::new()
                .with_local_position(position)
                .build(),
        ),
    )
    .with_material(MaterialResource::new_ok(ResourceKind::Embedded, material))
    .with_size(0.05)
    .with_color(Color::MAROON)
    .build(&mut scene.graph);

    scene.graph.link_nodes(handle, root);

    handle
}

impl ShapeHandles {
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
            ShapeHandles::Cuboid {
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
            ShapeHandles::Ball { radius_handle } => func(*radius_handle),
            ShapeHandles::Capsule {
                radius_handle,
                begin_handle,
                end_handle,
            } => {
                for handle in [radius_handle, begin_handle, end_handle] {
                    func(*handle)
                }
            }
            ShapeHandles::Cylinder {
                radius_handle,
                half_height_handle,
            }
            | ShapeHandles::Cone {
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
}

#[derive(Default)]
pub struct ColliderShapePlugin {
    collider: Handle<Node>,
    active_handle: Handle<Node>,
    shape_handles: Option<ShapeHandles>,
}

impl EditorPlugin for ColliderShapePlugin {
    fn on_ui_message(&mut self, message: &mut UiMessage, editor: &mut Editor) {
        if message.destination() != editor.scene_viewer.frame() {
            return;
        }

        let frame = editor
            .engine
            .user_interfaces
            .first()
            .node(editor.scene_viewer.frame());

        let origin = frame.screen_position();

        let Some(entry) = editor.scenes.current_scene_entry_mut() else {
            return;
        };

        let Some(game_scene) = entry.controller.downcast_mut::<GameScene>() else {
            return;
        };

        let scene = &mut editor.engine.scenes[game_scene.scene];

        if let Some(WidgetMessage::MouseMove { pos, .. }) = message.data() {
            let cursor_pos = *pos - origin;

            if let Some(shape) = self.shape_handles.as_ref() {
                shape.reset_handles(scene);
                self.active_handle = Handle::NONE;

                if let Some(result) = game_scene.camera_controller.pick(
                    &scene.graph,
                    PickingOptions {
                        cursor_pos,
                        editor_only: true,
                        filter: None,
                        ignore_back_faces: false,
                        use_picking_loop: false,
                        only_meshes: false,
                    },
                ) {
                    if shape.has_handle(result.node) {
                        scene.graph[result.node]
                            .as_sprite_mut()
                            .set_color(Color::RED);

                        self.active_handle = result.node;
                    }
                }
            }
        }
    }

    fn on_update(&mut self, editor: &mut Editor) {
        let Some(shape) = self.shape_handles.as_ref() else {
            return;
        };

        let Some(entry) = editor.scenes.current_scene_entry_mut() else {
            return;
        };

        let Some(game_scene) = entry.controller.downcast_mut::<GameScene>() else {
            return;
        };

        let scene = &mut editor.engine.scenes[game_scene.scene];

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

        shape.sync_to_shape(collider.shape().clone(), center, side, up, look, scene);
    }

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
            if let Some(shape_handles) = self.shape_handles.take() {
                shape_handles.destroy(scene);
            }

            for node in selection.nodes().iter() {
                if let Some(collider) = scene.graph.try_get_of_type::<Collider>(*node) {
                    self.collider = *node;

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

                    self.shape_handles = ShapeHandles::try_create(
                        collider.shape().clone(),
                        center,
                        side,
                        up,
                        look,
                        scene,
                        game_scene.editor_objects_root,
                    );

                    break;
                }
            }
        }
    }
}
