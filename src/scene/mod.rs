#![warn(missing_docs)]

//! Contains all structures and methods to create and manage 3D scenes.
//!
//! A `Scene` is a container for graph nodes, animations and physics.

pub mod accel;
pub mod base;
pub mod camera;
pub mod collider;
pub mod debug;
pub mod decal;
pub mod graph;
pub mod joint;
pub mod light;
pub mod mesh;
pub mod node;
pub mod particle_system;
pub mod physics;
pub mod rigidbody;
pub mod sprite;
pub mod terrain;
pub mod transform;
pub mod variable;
pub mod visibility;

use crate::{
    animation::AnimationContainer,
    core::{
        algebra::{UnitQuaternion, Vector2},
        color::Color,
        instant,
        pool::{Handle, Pool, PoolIterator, PoolIteratorMut, Ticket},
        sstorage::ImmutableString,
        visitor::{Visit, VisitError, VisitResult, Visitor},
    },
    engine::{
        resource_manager::{MaterialSearchOptions, ResourceManager},
        PhysicsBinder,
    },
    material::{shader::SamplerFallback, PropertyValue},
    physics3d::{
        desc::{ColliderShapeDesc, JointParamsDesc, RigidBodyTypeDesc},
        PhysicsPerformanceStatistics, RigidBodyHandle,
    },
    resource::texture::Texture,
    scene::{
        base::BaseBuilder,
        collider::ColliderBuilder,
        debug::SceneDrawingContext,
        graph::Graph,
        joint::JointBuilder,
        mesh::buffer::{
            VertexAttributeDataType, VertexAttributeDescriptor, VertexAttributeUsage,
            VertexWriteTrait,
        },
        node::Node,
        physics::LegacyPhysics,
        rigidbody::RigidBodyBuilder,
        transform::TransformBuilder,
    },
    sound::{context::SoundContext, engine::SoundEngine},
    utils::{lightmap::Lightmap, log::Log, log::MessageKind, navmesh::Navmesh},
};
use fxhash::FxHashMap;
use std::{
    fmt::{Display, Formatter},
    ops::{Index, IndexMut},
    path::Path,
    sync::{Arc, Mutex},
};

/// A container for navigational meshes.
#[derive(Default, Clone, Debug)]
pub struct NavMeshContainer {
    pool: Pool<Navmesh>,
}

impl NavMeshContainer {
    /// Adds new navigational mesh to the container and returns its handle.
    pub fn add(&mut self, navmesh: Navmesh) -> Handle<Navmesh> {
        self.pool.spawn(navmesh)
    }

    /// Removes navigational mesh by its handle.
    pub fn remove(&mut self, handle: Handle<Navmesh>) -> Navmesh {
        self.pool.free(handle)
    }

    /// Creates new immutable iterator.
    pub fn iter(&self) -> impl Iterator<Item = &Navmesh> {
        self.pool.iter()
    }

    /// Creates new immutable iterator.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Navmesh> {
        self.pool.iter_mut()
    }

    /// Creates a handle to navmesh from its index.
    pub fn handle_from_index(&self, i: u32) -> Handle<Navmesh> {
        self.pool.handle_from_index(i)
    }

    /// Destroys all navmeshes. All handles will become invalid.
    pub fn clear(&mut self) {
        self.pool.clear()
    }

    /// Checks if given handle is valid.
    pub fn is_valid_handle(&self, handle: Handle<Navmesh>) -> bool {
        self.pool.is_valid_handle(handle)
    }
}

impl Index<Handle<Navmesh>> for NavMeshContainer {
    type Output = Navmesh;

    fn index(&self, index: Handle<Navmesh>) -> &Self::Output {
        &self.pool[index]
    }
}

impl IndexMut<Handle<Navmesh>> for NavMeshContainer {
    fn index_mut(&mut self, index: Handle<Navmesh>) -> &mut Self::Output {
        &mut self.pool[index]
    }
}

impl Visit for NavMeshContainer {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.pool.visit("Pool", visitor)?;

        visitor.leave_region()
    }
}

/// See module docs.
#[derive(Debug)]
pub struct Scene {
    /// Graph is main container for all scene nodes. It calculates global transforms for nodes,
    /// updates them and performs all other important work. See `graph` module docs for more
    /// info.
    pub graph: Graph,

    /// Animations container controls all animation on scene. Each animation can have tracks which
    /// has handles to graph nodes. See `animation` module docs for more info.
    pub animations: AnimationContainer,

    /// Texture to draw scene to. If empty, scene will be drawn on screen directly.
    /// It is useful to "embed" some scene into other by drawing a quad with this
    /// texture. This can be used to make in-game video conference - you can make
    /// separate scene with your characters and draw scene into texture, then in
    /// main scene you can attach this texture to some quad which will be used as
    /// monitor. Other usage could be previewer of models, like pictogram of character
    /// in real-time strategies, in other words there are plenty of possible uses.
    pub render_target: Option<Texture>,

    /// Drawing context for simple graphics.
    pub drawing_context: SceneDrawingContext,

    /// A sound context that holds all sound sources, effects, etc. belonging to the scene.
    pub sound_context: SoundContext,

    /// A container for navigational meshes.
    pub navmeshes: NavMeshContainer,

    /// Current lightmap.
    lightmap: Option<Lightmap>,

    /// Performance statistics from last `update` call.
    pub performance_statistics: PerformanceStatistics,

    /// Color of ambient lighting.
    pub ambient_lighting_color: Color,

    /// Whether the scene will be updated and rendered or not. Default is true.
    /// This flag allowing you to build a scene manager for your game. For example,
    /// you may have a scene for menu and one per level. Menu's scene is persistent,
    /// however you don't want it to be updated and renderer while you have a level
    /// loaded and playing a game. When you're start playing, just set `enabled` flag
    /// to false for menu's scene and when you need to open a menu - set it to true and
    /// set `enabled` flag to false for level's scene.
    pub enabled: bool,

    // Legacy physics world.
    legacy_physics: LegacyPhysics,

    // Legacy physics binder.
    legacy_physics_binder: PhysicsBinder<Node, RigidBodyHandle>,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            graph: Default::default(),
            animations: Default::default(),
            legacy_physics: Default::default(),
            legacy_physics_binder: Default::default(),
            render_target: None,
            lightmap: None,
            drawing_context: Default::default(),
            sound_context: Default::default(),
            navmeshes: Default::default(),
            performance_statistics: Default::default(),
            ambient_lighting_color: Color::opaque(100, 100, 100),
            enabled: true,
        }
    }
}

fn map_texture(tex: Option<Texture>, rm: ResourceManager) -> Option<Texture> {
    if let Some(shallow_texture) = tex {
        let shallow_texture = shallow_texture.state();
        Some(rm.request_texture(shallow_texture.path(), None))
    } else {
        None
    }
}

/// A structure that holds times that specific update step took.
#[derive(Clone, Default, Debug)]
pub struct PerformanceStatistics {
    /// Physics performance statistics.
    pub physics: PhysicsPerformanceStatistics,

    /// A time (in seconds) which was required to update graph.
    pub graph_update_time: f32,

    /// A time (in seconds) which was required to update animations.
    pub animations_update_time: f32,

    /// A time (in seconds) which was required to render sounds.
    pub sound_update_time: f32,
}

impl Display for PerformanceStatistics {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\nGraph: {} ms\nAnimations: {} ms\nSounds: {} ms",
            self.physics,
            self.graph_update_time * 1000.0,
            self.animations_update_time * 1000.0,
            self.sound_update_time * 1000.0
        )
    }
}

impl Scene {
    /// Creates new scene with single root node.
    ///
    /// # Notes
    ///
    /// This method differs from Default trait implementation! Scene::default() creates
    /// empty graph with no nodes.
    #[inline]
    pub fn new() -> Self {
        Self {
            // Graph must be created with `new` method because it differs from `default`
            graph: Graph::new(),
            legacy_physics: Default::default(),
            animations: Default::default(),
            legacy_physics_binder: Default::default(),
            render_target: None,
            lightmap: None,
            drawing_context: Default::default(),
            sound_context: SoundContext::new(),
            navmeshes: Default::default(),
            performance_statistics: Default::default(),
            ambient_lighting_color: Color::opaque(100, 100, 100),
            enabled: true,
        }
    }

    /// Tries to load scene from given file. File can contain any scene in native engine format.
    /// Such scenes can be made in rusty editor.
    ///
    /// # Important notes
    ///
    /// `material_search_options` in most cases should be `MaterialSearchOptions::UsePathDirectly` to be
    /// able to load materials correctly, any other option will force engine to search materials
    /// in different locations!
    pub async fn from_file<P: AsRef<Path>>(
        path: P,
        resource_manager: ResourceManager,
        material_search_options: &MaterialSearchOptions,
    ) -> Result<Self, VisitError> {
        let mut scene = Scene::default();
        {
            let mut visitor = Visitor::load_binary(path.as_ref()).await?;
            scene.visit("Scene", &mut visitor)?;
        }

        // Collect all used resources and wait for them.
        let mut resources = Vec::new();
        for node in scene.graph.linear_iter_mut() {
            if let Some(shallow_resource) = node.resource.clone() {
                let search_options =
                    if material_search_options == &MaterialSearchOptions::UsePathDirectly {
                        shallow_resource
                            .data_ref()
                            .material_search_options()
                            .clone()
                    } else {
                        material_search_options.clone()
                    };
                let resource = resource_manager
                    .clone()
                    .request_model(&shallow_resource.state().path(), search_options);
                node.resource = Some(resource.clone());
                resources.push(resource);
            }
        }

        let _ = crate::core::futures::future::join_all(resources).await;

        // Restore pointers to resources. Scene saves only paths to resources, here we must
        // find real resources instead.

        for node in scene.graph.linear_iter_mut() {
            match node {
                Node::Mesh(mesh) => {
                    for surface in mesh.surfaces_mut() {
                        surface.material().lock().resolve(resource_manager.clone());
                    }
                }
                Node::Sprite(sprite) => {
                    sprite.set_texture(map_texture(sprite.texture(), resource_manager.clone()));
                }
                Node::ParticleSystem(particle_system) => {
                    particle_system.set_texture(map_texture(
                        particle_system.texture(),
                        resource_manager.clone(),
                    ));
                }
                Node::Camera(camera) => {
                    camera.set_environment(map_texture(
                        camera.environment_map(),
                        resource_manager.clone(),
                    ));

                    if let Some(skybox) = camera.skybox_mut() {
                        skybox.bottom =
                            map_texture(skybox.bottom.clone(), resource_manager.clone());
                        skybox.top = map_texture(skybox.top.clone(), resource_manager.clone());
                        skybox.left = map_texture(skybox.left.clone(), resource_manager.clone());
                        skybox.right = map_texture(skybox.right.clone(), resource_manager.clone());
                        skybox.front = map_texture(skybox.front.clone(), resource_manager.clone());
                        skybox.back = map_texture(skybox.back.clone(), resource_manager.clone());
                    }
                }
                Node::Terrain(terrain) => {
                    for layer in terrain.layers() {
                        layer.material.lock().resolve(resource_manager.clone());
                    }
                }
                Node::Decal(decal) => {
                    decal.set_diffuse_texture(map_texture(
                        decal.diffuse_texture_value(),
                        resource_manager.clone(),
                    ));
                    decal.set_normal_texture(map_texture(
                        decal.normal_texture_value(),
                        resource_manager.clone(),
                    ));
                }
                _ => (),
            }
        }

        if let Some(lightmap) = scene.lightmap.as_mut() {
            for entries in lightmap.map.values_mut() {
                for entry in entries.iter_mut() {
                    entry.texture = map_texture(entry.texture.clone(), resource_manager.clone());
                }
            }
        }

        // We have to wait until skybox textures are all loaded, because we need to read their data
        // to re-create cube map.
        let mut skybox_textures = Vec::new();
        for node in scene.graph.linear_iter() {
            if let Node::Camera(camera) = node {
                if let Some(skybox) = camera.skybox_ref() {
                    skybox_textures.extend(skybox.textures().iter().filter_map(|t| t.clone()));
                }
            }
        }
        crate::core::futures::future::join_all(skybox_textures).await;

        // And do resolve to extract correct graphical data and so on.
        scene.resolve();

        Ok(scene)
    }

    /// Removes node from scene with all associated entities, like animations etc. This method
    /// should be used all times instead of [Graph::remove_node](crate::scene::graph::Graph::remove_node).
    ///
    /// # Panics
    ///
    /// Panics if handle is invalid.
    pub fn remove_node(&mut self, handle: Handle<Node>) {
        for descendant in self.graph.traverse_handle_iter(handle) {
            // Remove all associated animations.
            self.animations.retain(|animation| {
                for track in animation.get_tracks() {
                    if track.get_node() == descendant {
                        return false;
                    }
                }
                true
            });
        }

        self.graph.remove_node(handle)
    }

    fn convert_legacy_physics(&mut self) {
        // Convert rigid bodies and colliders.
        let mut body_map = FxHashMap::default();
        for (node, body_handle) in self.legacy_physics_binder.forward_map() {
            let body_ref = if let Some(body_ref) = self.legacy_physics.bodies.get(body_handle) {
                body_ref
            } else {
                continue;
            };

            let [x_rotation_locked, y_rotation_locked, z_rotation_locked] =
                body_ref.is_rotation_locked();

            let body_node_handle = RigidBodyBuilder::new(
                BaseBuilder::new()
                    .with_name("Rigid Body")
                    .with_local_transform(
                        TransformBuilder::new()
                            .with_local_position(body_ref.position().translation.vector)
                            .with_local_rotation(body_ref.position().rotation)
                            .build(),
                    ),
            )
            .with_body_type(RigidBodyTypeDesc::from(body_ref.body_type()))
            .with_mass(body_ref.mass())
            .with_ang_vel(*body_ref.angvel())
            .with_lin_vel(*body_ref.linvel())
            .with_lin_damping(body_ref.linear_damping())
            .with_ang_damping(body_ref.angular_damping())
            .with_x_rotation_locked(x_rotation_locked)
            .with_y_rotation_locked(y_rotation_locked)
            .with_z_rotation_locked(z_rotation_locked)
            .with_translation_locked(body_ref.is_translation_locked())
            .build(&mut self.graph);

            body_map.insert(body_handle.clone(), body_node_handle);

            for c in body_ref.colliders() {
                let collider_ref =
                    if let Some(collider_ref) = self.legacy_physics.colliders.native_ref(*c) {
                        collider_ref
                    } else {
                        continue;
                    };

                let shape = ColliderShapeDesc::from_collider_shape(collider_ref.shape());

                let name = match shape {
                    ColliderShapeDesc::Ball(_) => "Ball Collider",
                    ColliderShapeDesc::Cylinder(_) => "Cylinder Collider",
                    ColliderShapeDesc::RoundCylinder(_) => "Round Cylinder Collider",
                    ColliderShapeDesc::Cone(_) => "Cone Collider",
                    ColliderShapeDesc::Cuboid(_) => "Cuboid Collider",
                    ColliderShapeDesc::Capsule(_) => "Capsule Collider",
                    ColliderShapeDesc::Segment(_) => "Segment Collider",
                    ColliderShapeDesc::Triangle(_) => "Triangle Collider",
                    ColliderShapeDesc::Trimesh(_) => "Trimesh Collider",
                    ColliderShapeDesc::Heightfield(_) => "Heightfield Collider",
                };

                let collider_handle = ColliderBuilder::new(
                    BaseBuilder::new()
                        .with_name(name.to_owned())
                        .with_local_transform(
                            TransformBuilder::new()
                                .with_local_position(
                                    collider_ref
                                        .position_wrt_parent()
                                        .map(|p| p.translation.vector)
                                        .unwrap_or_default(),
                                )
                                .with_local_rotation(
                                    collider_ref
                                        .position_wrt_parent()
                                        .map(|p| p.rotation)
                                        .unwrap_or_default(),
                                )
                                .build(),
                        ),
                )
                .with_shape(shape)
                .with_sensor(collider_ref.is_sensor())
                .with_restitution(collider_ref.restitution())
                .with_density(collider_ref.density())
                .with_collision_groups(collider_ref.collision_groups().into())
                .with_solver_groups(collider_ref.solver_groups().into())
                .with_friction(collider_ref.friction())
                .build(&mut self.graph);

                self.graph.link_nodes(collider_handle, body_node_handle);
            }

            let node_ref = &mut self.graph[*node];
            node_ref
                .local_transform_mut()
                .set_position(Default::default())
                .set_rotation(UnitQuaternion::default());
            let parent = node_ref.parent();

            self.graph.link_nodes(*node, body_node_handle);
            self.graph.link_nodes(body_node_handle, parent);
        }

        // Convert joints.
        for joint in self.legacy_physics.joints.iter() {
            let body1 = if let Some(body1) = self
                .legacy_physics
                .bodies
                .handle_map()
                .key_of(&joint.body1)
                .and_then(|h| body_map.get(h))
            {
                *body1
            } else {
                continue;
            };

            let body2 = if let Some(body2) = self
                .legacy_physics
                .bodies
                .handle_map()
                .key_of(&joint.body2)
                .and_then(|h| body_map.get(h))
            {
                *body2
            } else {
                continue;
            };

            let joint_handle = JointBuilder::new(BaseBuilder::new())
                .with_params(JointParamsDesc::from_params(&joint.params))
                .with_body1(body1)
                .with_body2(body2)
                .build(&mut self.graph);

            self.graph.link_nodes(joint_handle, body1);
        }
    }

    pub(in crate) fn resolve(&mut self) {
        Log::writeln(MessageKind::Information, "Starting resolve...".to_owned());

        self.graph.resolve();
        self.animations.resolve(&self.graph);

        self.graph.update_hierarchical_data();
        self.legacy_physics
            .resolve(&self.legacy_physics_binder, &self.graph, None);

        self.convert_legacy_physics();

        // Re-apply lightmap if any. This has to be done after resolve because we must patch surface
        // data at this stage, but if we'd do this before we wouldn't be able to do this because
        // meshes contains invalid surface data.
        if let Some(lightmap) = self.lightmap.as_mut() {
            // Patch surface data first. To do this we gather all surface data instances and
            // look in patch data if we have patch for data.
            let mut unique_data_set = FxHashMap::default();
            for &handle in lightmap.map.keys() {
                if let Node::Mesh(mesh) = &mut self.graph[handle] {
                    for surface in mesh.surfaces() {
                        let data = surface.data();
                        let key = &*data as *const _ as u64;
                        unique_data_set.entry(key).or_insert(data);
                    }
                }
            }

            for (_, data) in unique_data_set.into_iter() {
                let mut data = data.lock();

                if let Some(patch) = lightmap.patches.get(&data.content_hash()) {
                    if !data
                        .vertex_buffer
                        .has_attribute(VertexAttributeUsage::TexCoord1)
                    {
                        data.vertex_buffer
                            .modify()
                            .add_attribute(
                                VertexAttributeDescriptor {
                                    usage: VertexAttributeUsage::TexCoord1,
                                    data_type: VertexAttributeDataType::F32,
                                    size: 2,
                                    divisor: 0,
                                    shader_location: 6, // HACK: GBuffer renderer expects it to be at 6
                                },
                                Vector2::<f32>::default(),
                            )
                            .unwrap();
                    }

                    data.geometry_buffer.set_triangles(patch.triangles.clone());

                    let mut vertex_buffer_mut = data.vertex_buffer.modify();
                    for &v in patch.additional_vertices.iter() {
                        vertex_buffer_mut.duplicate(v as usize);
                    }

                    assert_eq!(
                        vertex_buffer_mut.vertex_count() as usize,
                        patch.second_tex_coords.len()
                    );
                    for (mut view, &tex_coord) in vertex_buffer_mut
                        .iter_mut()
                        .zip(patch.second_tex_coords.iter())
                    {
                        view.write_2_f32(VertexAttributeUsage::TexCoord1, tex_coord)
                            .unwrap();
                    }
                } else {
                    Log::writeln(
                        MessageKind::Warning,
                        "Failed to get surface data patch while resolving lightmap!\
                    This means that surface has changed and lightmap must be regenerated!"
                            .to_owned(),
                    );
                }
            }

            // Apply textures.
            for (&handle, entries) in lightmap.map.iter_mut() {
                if let Node::Mesh(mesh) = &mut self.graph[handle] {
                    for (entry, surface) in entries.iter_mut().zip(mesh.surfaces_mut()) {
                        if let Err(e) = surface.material().lock().set_property(
                            &ImmutableString::new("lightmapTexture"),
                            PropertyValue::Sampler {
                                value: entry.texture.clone(),
                                fallback: SamplerFallback::Black,
                            },
                        ) {
                            Log::writeln(
                                MessageKind::Error,
                                format!(
                                    "Failed to apply light map texture to material. Reason {:?}",
                                    e
                                ),
                            )
                        }
                    }
                }
            }
        }

        Log::writeln(MessageKind::Information, "Resolve succeeded!".to_owned());
    }

    /// Tries to set new lightmap to scene.
    pub fn set_lightmap(&mut self, lightmap: Lightmap) -> Result<Option<Lightmap>, &'static str> {
        // Assign textures to surfaces.
        for (handle, lightmaps) in lightmap.map.iter() {
            if let Node::Mesh(mesh) = &mut self.graph[*handle] {
                if mesh.surfaces().len() != lightmaps.len() {
                    return Err("failed to set lightmap, surface count mismatch");
                }

                for (surface, entry) in mesh.surfaces_mut().iter_mut().zip(lightmaps) {
                    // This unwrap() call must never panic in normal conditions, because texture wrapped in Option
                    // only to implement Default trait to be serializable.
                    let texture = entry.texture.clone().unwrap();
                    if let Err(e) = surface.material().lock().set_property(
                        &ImmutableString::new("lightmapTexture"),
                        PropertyValue::Sampler {
                            value: Some(texture),
                            fallback: SamplerFallback::Black,
                        },
                    ) {
                        Log::writeln(
                            MessageKind::Error,
                            format!(
                                "Failed to apply light map texture to material. Reason {:?}",
                                e
                            ),
                        )
                    }
                }
            }
        }
        Ok(std::mem::replace(&mut self.lightmap, Some(lightmap)))
    }

    /// Performs single update tick with given delta time from last frame. Internally
    /// it updates physics, animations, and each graph node. In most cases there is
    /// no need to call it directly, engine automatically updates all available scenes.
    pub fn update(&mut self, frame_size: Vector2<f32>, dt: f32) {
        let last = instant::Instant::now();
        self.animations.update_animations(dt);
        self.performance_statistics.animations_update_time =
            (instant::Instant::now() - last).as_secs_f32();

        let last = instant::Instant::now();
        self.graph.update(frame_size, dt);
        self.performance_statistics.graph_update_time =
            (instant::Instant::now() - last).as_secs_f32();

        self.performance_statistics.sound_update_time = self
            .sound_context
            .state()
            .full_render_duration()
            .as_secs_f32();
    }

    /// Creates deep copy of a scene, filter predicate allows you to filter out nodes
    /// by your criteria.
    pub fn clone<F>(&self, filter: &mut F) -> (Self, FxHashMap<Handle<Node>, Handle<Node>>)
    where
        F: FnMut(Handle<Node>, &Node) -> bool,
    {
        let (graph, old_new_map) = self.graph.clone(filter);
        let mut animations = self.animations.clone();
        for animation in animations.iter_mut() {
            // Remove all tracks for nodes that were filtered out.
            animation.retain_tracks(|track| old_new_map.contains_key(&track.get_node()));
            // Remap track nodes.
            for track in animation.get_tracks_mut() {
                track.set_node(old_new_map[&track.get_node()]);
            }
        }
        (
            Self {
                graph,
                animations,
                legacy_physics: Default::default(),
                legacy_physics_binder: Default::default(),
                // Render target is intentionally not copied, because it does not makes sense - a copy
                // will redraw frame completely.
                render_target: Default::default(),
                lightmap: self.lightmap.clone(),
                drawing_context: self.drawing_context.clone(),
                sound_context: self.sound_context.deep_clone(),
                navmeshes: self.navmeshes.clone(),
                performance_statistics: Default::default(),
                ambient_lighting_color: self.ambient_lighting_color,
                enabled: self.enabled,
            },
            old_new_map,
        )
    }
}

impl Visit for Scene {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;
        self.graph.visit("Graph", visitor)?;
        self.animations.visit("Animations", visitor)?;
        self.lightmap.visit("Lightmap", visitor)?;
        self.sound_context.visit("SoundContext", visitor)?;
        self.navmeshes.visit("NavMeshes", visitor)?;
        self.ambient_lighting_color
            .visit("AmbientLightingColor", visitor)?;
        self.enabled.visit("Enabled", visitor)?;
        // Load legacy stuff for backward compatibility.
        if visitor.is_reading() {
            let _ = self.legacy_physics.visit("Physics", visitor);
            let _ = self.legacy_physics_binder.visit("PhysicsBinder", visitor);
        }
        visitor.leave_region()
    }
}

/// Container for scenes in the engine.
#[derive(Default)]
pub struct SceneContainer {
    pool: Pool<Scene>,
    sound_engine: Arc<Mutex<SoundEngine>>,
}

impl SceneContainer {
    pub(in crate) fn new(sound_engine: Arc<Mutex<SoundEngine>>) -> Self {
        Self {
            pool: Pool::new(),
            sound_engine,
        }
    }

    /// Return true if given handle is valid and "points" to "alive" scene.
    pub fn is_valid_handle(&self, handle: Handle<Scene>) -> bool {
        self.pool.is_valid_handle(handle)
    }

    /// Returns pair iterator which yields (handle, scene_ref) pairs.
    pub fn pair_iter(&self) -> impl Iterator<Item = (Handle<Scene>, &Scene)> {
        self.pool.pair_iter()
    }

    /// Creates new iterator over scenes in container.
    #[inline]
    pub fn iter(&self) -> PoolIterator<Scene> {
        self.pool.iter()
    }

    /// Creates new mutable iterator over scenes in container.
    #[inline]
    pub fn iter_mut(&mut self) -> PoolIteratorMut<Scene> {
        self.pool.iter_mut()
    }

    /// Adds new scene into container.
    #[inline]
    pub fn add(&mut self, scene: Scene) -> Handle<Scene> {
        self.sound_engine
            .lock()
            .unwrap()
            .add_context(scene.sound_context.clone());
        self.pool.spawn(scene)
    }

    /// Removes all scenes from container.
    #[inline]
    pub fn clear(&mut self) {
        self.pool.clear()
    }

    /// Removes given scene from container.
    #[inline]
    pub fn remove(&mut self, handle: Handle<Scene>) {
        self.sound_engine
            .lock()
            .unwrap()
            .remove_context(self.pool[handle].sound_context.clone());
        self.pool.free(handle);
    }

    /// Takes scene from the container and transfers ownership to caller. You must either
    /// put scene back using ticket or call `forget_ticket` to make memory used by scene
    /// vacant again.
    pub fn take_reserve(&mut self, handle: Handle<Scene>) -> (Ticket<Scene>, Scene) {
        self.pool.take_reserve(handle)
    }

    /// Puts scene back using its ticket.
    pub fn put_back(&mut self, ticket: Ticket<Scene>, scene: Scene) -> Handle<Scene> {
        self.pool.put_back(ticket, scene)
    }

    /// Forgets ticket of a scene, making place at which ticket points, vacant again.
    pub fn forget_ticket(&mut self, ticket: Ticket<Scene>) {
        self.pool.forget_ticket(ticket)
    }
}

impl Index<Handle<Scene>> for SceneContainer {
    type Output = Scene;

    #[inline]
    fn index(&self, index: Handle<Scene>) -> &Self::Output {
        &self.pool[index]
    }
}

impl IndexMut<Handle<Scene>> for SceneContainer {
    #[inline]
    fn index_mut(&mut self, index: Handle<Scene>) -> &mut Self::Output {
        &mut self.pool[index]
    }
}

impl Visit for SceneContainer {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.pool.visit("Pool", visitor)?;
        self.sound_engine.visit("SoundEngine", visitor)?;

        visitor.leave_region()
    }
}
