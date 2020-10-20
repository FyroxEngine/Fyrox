#![warn(missing_docs)]

//! Contains all structures and methods to create and manage scenes.
//!
//! Scene is container for graph nodes, animations and physics.

pub mod base;
pub mod camera;
pub mod graph;
pub mod light;
pub mod mesh;
pub mod node;
pub mod particle_system;
pub mod sprite;
pub mod transform;

use crate::{
    animation::AnimationContainer,
    core::{
        color::Color,
        math::{
            aabb::AxisAlignedBoundingBox, frustum::Frustum, mat4::Mat4, vec2::Vec2, vec3::Vec3,
        },
        pool::{Handle, Pool, PoolIterator, PoolIteratorMut},
        visitor::{Visit, VisitError, VisitResult, Visitor},
    },
    engine::resource_manager::ResourceManager,
    physics::{rigid_body::RigidBody, static_geometry::StaticGeometry, Physics},
    resource::texture::Texture,
    scene::{graph::Graph, node::Node},
    utils::{self, lightmap::Lightmap, log::Log},
};
use std::{
    collections::HashMap,
    ops::{Index, IndexMut},
    path::Path,
};

/// Physics binder is used to link graph nodes with rigid bodies. Scene will
/// sync transform of node with its associated rigid body.
#[derive(Clone, Debug)]
pub struct PhysicsBinder {
    node_rigid_body_map: HashMap<Handle<Node>, Handle<RigidBody>>,
}

impl Default for PhysicsBinder {
    fn default() -> Self {
        Self {
            node_rigid_body_map: Default::default(),
        }
    }
}

impl PhysicsBinder {
    /// Links given graph node with specified rigid body.
    pub fn bind(
        &mut self,
        node: Handle<Node>,
        rigid_body: Handle<RigidBody>,
    ) -> Option<Handle<RigidBody>> {
        self.node_rigid_body_map.insert(node, rigid_body)
    }

    /// Unlinks given graph node from its associated rigid body (if any).
    pub fn unbind(&mut self, node: Handle<Node>) -> Option<Handle<RigidBody>> {
        self.node_rigid_body_map.remove(&node)
    }

    /// Unlinks given body from a node that is linked with the body.
    ///
    /// # Performance
    ///
    /// This method is slow because of two reasons:
    ///
    /// 1) Search is linear
    /// 2) Additional memory is allocated
    ///
    /// So it is not advised to call it in performance critical places.
    pub fn unbind_by_body(&mut self, body: Handle<RigidBody>) -> Handle<Node> {
        let mut node = Handle::NONE;
        self.node_rigid_body_map = self
            .node_rigid_body_map
            .clone()
            .into_iter()
            .filter(|&(n, b)| {
                if b == body {
                    node = n;
                    false
                } else {
                    true
                }
            })
            .collect();
        node
    }

    /// Returns handle of rigid body associated with given node. It will return
    /// Handle::NONE if given node isn't linked to a rigid body.
    pub fn body_of(&self, node: Handle<Node>) -> Handle<RigidBody> {
        self.node_rigid_body_map
            .get(&node)
            .copied()
            .unwrap_or_default()
    }
}

impl Visit for PhysicsBinder {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.node_rigid_body_map.visit("Map", visitor)?;

        visitor.leave_region()
    }
}

/// Colored line between two points.
#[derive(Clone, Debug)]
pub struct Line {
    /// Beginning of the line.
    pub begin: Vec3,
    /// End of the line.    
    pub end: Vec3,
    /// Color of the line.
    pub color: Color,
}

/// Drawing context for simple graphics, it allows you to draw simple figures using
/// set of lines. Most common use is to draw some debug geometry in your game, draw
/// physics info (contacts, meshes, shapes, etc.), draw temporary geometry in editor
/// and so on.
#[derive(Default, Clone, Debug)]
pub struct SceneDrawingContext {
    /// List of lines to draw.
    pub lines: Vec<Line>,
}

impl SceneDrawingContext {
    /// Draws frustum with given color. Drawing is not immediate, it only pushes
    /// lines for frustum into internal buffer. It will be drawn later on in separate
    /// render pass.
    pub fn draw_frustum(&mut self, frustum: &Frustum, color: Color) {
        let left_top_front = frustum.left_top_front_corner();
        let left_bottom_front = frustum.left_bottom_front_corner();
        let right_bottom_front = frustum.right_bottom_front_corner();
        let right_top_front = frustum.right_top_front_corner();

        let left_top_back = frustum.left_top_back_corner();
        let left_bottom_back = frustum.left_bottom_back_corner();
        let right_bottom_back = frustum.right_bottom_back_corner();
        let right_top_back = frustum.right_top_back_corner();

        // Front face
        self.add_line(Line {
            begin: left_top_front,
            end: right_top_front,
            color,
        });
        self.add_line(Line {
            begin: right_top_front,
            end: right_bottom_front,
            color,
        });
        self.add_line(Line {
            begin: right_bottom_front,
            end: left_bottom_front,
            color,
        });
        self.add_line(Line {
            begin: left_bottom_front,
            end: left_top_front,
            color,
        });

        // Back face
        self.add_line(Line {
            begin: left_top_back,
            end: right_top_back,
            color,
        });
        self.add_line(Line {
            begin: right_top_back,
            end: right_bottom_back,
            color,
        });
        self.add_line(Line {
            begin: right_bottom_back,
            end: left_bottom_back,
            color,
        });
        self.add_line(Line {
            begin: left_bottom_back,
            end: left_top_back,
            color,
        });

        // Edges
        self.add_line(Line {
            begin: left_top_front,
            end: left_top_back,
            color,
        });
        self.add_line(Line {
            begin: right_top_front,
            end: right_top_back,
            color,
        });
        self.add_line(Line {
            begin: right_bottom_front,
            end: right_bottom_back,
            color,
        });
        self.add_line(Line {
            begin: left_bottom_front,
            end: left_bottom_back,
            color,
        });
    }

    /// Draws axis-aligned bounding box with given color. Drawing is not immediate,
    /// it only pushes lines for bounding box into internal buffer. It will be drawn
    /// later on in separate render pass.
    pub fn draw_aabb(&mut self, aabb: &AxisAlignedBoundingBox, color: Color) {
        let left_bottom_front = Vec3::new(aabb.min.x, aabb.min.y, aabb.max.z);
        let left_top_front = Vec3::new(aabb.min.x, aabb.max.y, aabb.max.z);
        let right_top_front = Vec3::new(aabb.max.x, aabb.max.y, aabb.max.z);
        let right_bottom_front = Vec3::new(aabb.max.x, aabb.min.y, aabb.max.z);

        let left_bottom_back = Vec3::new(aabb.min.x, aabb.min.y, aabb.min.z);
        let left_top_back = Vec3::new(aabb.min.x, aabb.max.y, aabb.min.z);
        let right_top_back = Vec3::new(aabb.max.x, aabb.max.y, aabb.min.z);
        let right_bottom_back = Vec3::new(aabb.max.x, aabb.min.y, aabb.min.z);

        // Front face
        self.add_line(Line {
            begin: left_top_front,
            end: right_top_front,
            color,
        });
        self.add_line(Line {
            begin: right_top_front,
            end: right_bottom_front,
            color,
        });
        self.add_line(Line {
            begin: right_bottom_front,
            end: left_bottom_front,
            color,
        });
        self.add_line(Line {
            begin: left_bottom_front,
            end: left_top_front,
            color,
        });

        // Back face
        self.add_line(Line {
            begin: left_top_back,
            end: right_top_back,
            color,
        });
        self.add_line(Line {
            begin: right_top_back,
            end: right_bottom_back,
            color,
        });
        self.add_line(Line {
            begin: right_bottom_back,
            end: left_bottom_back,
            color,
        });
        self.add_line(Line {
            begin: left_bottom_back,
            end: left_top_back,
            color,
        });

        // Edges
        self.add_line(Line {
            begin: left_top_front,
            end: left_top_back,
            color,
        });
        self.add_line(Line {
            begin: right_top_front,
            end: right_top_back,
            color,
        });
        self.add_line(Line {
            begin: right_bottom_front,
            end: right_bottom_back,
            color,
        });
        self.add_line(Line {
            begin: left_bottom_front,
            end: left_bottom_back,
            color,
        });
    }

    /// Draws object-oriented bounding box with given color. Drawing is not immediate,
    /// it only pushes lines for object-oriented bounding box into internal buffer. It
    /// will be drawn later on in separate render pass.
    pub fn draw_oob(&mut self, aabb: &AxisAlignedBoundingBox, transform: Mat4, color: Color) {
        let left_bottom_front =
            transform.transform_vector(Vec3::new(aabb.min.x, aabb.min.y, aabb.max.z));
        let left_top_front =
            transform.transform_vector(Vec3::new(aabb.min.x, aabb.max.y, aabb.max.z));
        let right_top_front =
            transform.transform_vector(Vec3::new(aabb.max.x, aabb.max.y, aabb.max.z));
        let right_bottom_front =
            transform.transform_vector(Vec3::new(aabb.max.x, aabb.min.y, aabb.max.z));

        let left_bottom_back =
            transform.transform_vector(Vec3::new(aabb.min.x, aabb.min.y, aabb.min.z));
        let left_top_back =
            transform.transform_vector(Vec3::new(aabb.min.x, aabb.max.y, aabb.min.z));
        let right_top_back =
            transform.transform_vector(Vec3::new(aabb.max.x, aabb.max.y, aabb.min.z));
        let right_bottom_back =
            transform.transform_vector(Vec3::new(aabb.max.x, aabb.min.y, aabb.min.z));

        // Front face
        self.add_line(Line {
            begin: left_top_front,
            end: right_top_front,
            color,
        });
        self.add_line(Line {
            begin: right_top_front,
            end: right_bottom_front,
            color,
        });
        self.add_line(Line {
            begin: right_bottom_front,
            end: left_bottom_front,
            color,
        });
        self.add_line(Line {
            begin: left_bottom_front,
            end: left_top_front,
            color,
        });

        // Back face
        self.add_line(Line {
            begin: left_top_back,
            end: right_top_back,
            color,
        });
        self.add_line(Line {
            begin: right_top_back,
            end: right_bottom_back,
            color,
        });
        self.add_line(Line {
            begin: right_bottom_back,
            end: left_bottom_back,
            color,
        });
        self.add_line(Line {
            begin: left_bottom_back,
            end: left_top_back,
            color,
        });

        // Edges
        self.add_line(Line {
            begin: left_top_front,
            end: left_top_back,
            color,
        });
        self.add_line(Line {
            begin: right_top_front,
            end: right_top_back,
            color,
        });
        self.add_line(Line {
            begin: right_bottom_front,
            end: right_bottom_back,
            color,
        });
        self.add_line(Line {
            begin: left_bottom_front,
            end: left_bottom_back,
            color,
        });
    }

    /// Adds single line into internal buffer.
    pub fn add_line(&mut self, line: Line) {
        self.lines.push(line);
    }

    /// Removes all lines from internal buffer. For dynamic drawing you should call it
    /// every update tick of your application.
    pub fn clear_lines(&mut self) {
        self.lines.clear()
    }
}

/// Allows you to bind a mesh and a static geometry together, it is used on deserialization
/// stage to fill a static geometry by geometry from a mesh. This is useful in situations
/// when you need to use a model from your map as static geometry for physics. In this case
/// there is no need to serialize static geometry in a file. This is somewhat similar to mesh
/// geometry resolving at deserialization - it takes geometry from resource, not from data in
/// a file.
#[derive(Default, Clone, Debug)]
pub struct StaticGeometryBinder {
    map: HashMap<Handle<StaticGeometry>, Handle<Node>>,
}

impl StaticGeometryBinder {
    /// Links given static geometry with specified mesh.
    pub fn bind(
        &mut self,
        geom: Handle<StaticGeometry>,
        node: Handle<Node>,
    ) -> Option<Handle<Node>> {
        self.map.insert(geom, node)
    }

    /// Unlinks given static geometry from its associated mesh (if any).
    pub fn unbind(&mut self, geom: Handle<StaticGeometry>) -> Option<Handle<Node>> {
        self.map.remove(&geom)
    }

    /// Returns amount of entries in the binder.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Tries to find associated static geometry of a node.
    ///
    /// # Performance
    ///
    /// This method performs linear search so it could be slow for some applications.
    pub fn static_geometry_of_node(&self, node: Handle<Node>) -> Option<Handle<StaticGeometry>> {
        for (&geom, &node_handle) in self.map.iter() {
            if node_handle == node {
                return Some(geom);
            }
        }
        None
    }

    /// Unlinks given static geometry from a node.
    ///
    /// # Performance
    ///
    /// This method is slow because of two reasons:
    ///
    /// 1) Search is linear
    /// 2) Additional memory is allocated
    ///
    /// So it is not advised to call it in performance critical places.
    pub fn unbind_by_node(&mut self, node: Handle<Node>) -> Handle<StaticGeometry> {
        let mut geom_handle = Handle::NONE;
        self.map = self
            .map
            .clone()
            .into_iter()
            .filter(|&(g, n)| {
                if n == node {
                    geom_handle = g;
                    false
                } else {
                    true
                }
            })
            .collect();
        geom_handle
    }
}

impl Visit for StaticGeometryBinder {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.map.visit("Map", visitor)?;

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

    /// Physics world. Allows you create various physics objects such as static geometries and
    /// rigid bodies. Rigid bodies then should be linked with graph nodes using binder.
    pub physics: Physics,

    /// Physics binder is a bridge between physics world and scene graph. If a rigid body is linked
    /// to a graph node, then rigid body will control local transform of node.
    pub physics_binder: PhysicsBinder,

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

    /// Links static geometry with a mesh to be able to get geometry data at deserialization
    /// stage.
    pub static_geometry_binder: StaticGeometryBinder,

    lightmap: Option<Lightmap>,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            graph: Default::default(),
            animations: Default::default(),
            physics: Default::default(),
            physics_binder: Default::default(),
            render_target: None,
            lightmap: None,
            drawing_context: Default::default(),
            static_geometry_binder: Default::default(),
        }
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
            physics: Default::default(),
            animations: Default::default(),
            physics_binder: Default::default(),
            render_target: None,
            lightmap: None,
            drawing_context: Default::default(),
            static_geometry_binder: Default::default(),
        }
    }

    /// Tries to load scene from given file. File can contain any scene in native engine format.
    /// Such scenes can be made in rusty editor.
    pub fn from_file<P: AsRef<Path>>(
        path: P,
        resource_manager: ResourceManager,
    ) -> Result<Self, VisitError> {
        let mut scene = Scene::default();
        let mut visitor = Visitor::load_binary(path.as_ref())?;
        scene.visit("Scene", &mut visitor)?;

        // Restore pointers to resources. Scene saves only paths to resources, here we must
        // find real resources instead.
        for node in scene.graph.linear_iter_mut() {
            if let Some(shallow_resource) = node.resource.clone() {
                node.resource = Some(
                    resource_manager
                        .clone()
                        .request_model(&shallow_resource.state().path()),
                );
            }

            fn map_texture(tex: Option<Texture>, rm: ResourceManager) -> Option<Texture> {
                if let Some(shallow_texture) = tex {
                    let shallow_texture = shallow_texture.state();
                    Some(rm.request_texture(shallow_texture.path()))
                } else {
                    None
                }
            };

            match node {
                Node::Mesh(mesh) => {
                    for surface in mesh.surfaces_mut() {
                        surface.set_diffuse_texture(map_texture(
                            surface.diffuse_texture(),
                            resource_manager.clone(),
                        ));

                        surface.set_normal_texture(map_texture(
                            surface.normal_texture(),
                            resource_manager.clone(),
                        ));

                        surface.set_lightmap_texture(map_texture(
                            surface.lightmap_texture(),
                            resource_manager.clone(),
                        ));
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
                _ => (),
            }
        }

        // And do resolve to extract correct graphical data and so on.
        scene.resolve();

        Ok(scene)
    }

    fn update_physics(&mut self, dt: f32) {
        self.physics.step(dt);

        // Keep pair when node and body are both alive.
        let graph = &self.graph;
        let physics = &self.physics;
        self.physics_binder
            .node_rigid_body_map
            .retain(|node, body| {
                graph.is_valid_handle(*node) && physics.is_valid_body_handle(*body)
            });

        // Sync node positions with assigned physics bodies
        for (node, body) in self.physics_binder.node_rigid_body_map.iter() {
            let body = physics.borrow_body(*body);
            self.graph[*node]
                .local_transform_mut()
                .set_position(body.get_position());
        }
    }

    /// Removes node from scene with all associated entities, like animations etc.
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

    fn restore_static_geometries(&mut self) {
        Log::writeln(
            "Trying to restore data for static geometries from associated nodes...".to_owned(),
        );
        // Obtain geometry for static geometries from linked meshes.
        for (&geom_handle, &node_handle) in self.static_geometry_binder.map.iter() {
            if self.graph.is_valid_handle(node_handle) {
                *self.physics.borrow_static_geometry_mut(geom_handle) =
                    utils::mesh_to_static_geometry(self.graph[node_handle].as_mesh(), false);
            } else {
                Log::writeln(format!("Unable to get geometry for static geometry, node at handle {:?} does not exists!", node_handle))
            }
        }
    }

    pub(in crate) fn resolve(&mut self) {
        Log::writeln("Starting resolve...".to_owned());
        // TODO: Remove blocking here.
        futures::executor::block_on(self.graph.resolve());
        self.animations.resolve(&self.graph);
        self.restore_static_geometries();

        Log::writeln("Resolve succeeded!".to_owned());
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
                    surface.set_lightmap_texture(Some(texture))
                }
            }
        }
        Ok(std::mem::replace(&mut self.lightmap, Some(lightmap)))
    }

    /// Performs single update tick with given delta time from last frame. Internally
    /// it updates physics, animations, and each graph node. In most cases there is
    /// no need to call it directly, engine automatically updates all available scenes.
    pub fn update(&mut self, frame_size: Vec2, dt: f32) {
        self.update_physics(dt);
        self.animations.update_animations(dt);
        self.graph.update_nodes(frame_size, dt);
    }

    /// Creates deep copy of a scene, filter predicate allows you to filter out nodes
    /// by your criteria.
    pub fn clone<F>(&self, filter: &mut F) -> Self
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
        let physics = self.physics.clone();
        let mut physics_binder = PhysicsBinder::default();
        for (node, &body) in self.physics_binder.node_rigid_body_map.iter() {
            // Make sure we bind existing node with new physical body.
            if let Some(&new_node) = old_new_map.get(node) {
                // Re-use of body handle is fine here because physics copy bodies
                // directly and handles from previous pool is still suitable for copy.
                physics_binder.bind(new_node, body);
            }
        }
        let mut static_geometry_binder = StaticGeometryBinder::default();
        for (&geom, node) in self.static_geometry_binder.map.iter() {
            // Make sure we bind existing node with new physical body.
            if let Some(&new_node) = old_new_map.get(node) {
                // Re-use of body handle is fine here because physics copy bodies
                // directly and handles from previous pool is still suitable for copy.
                static_geometry_binder.bind(geom, new_node);
            }
        }
        Self {
            graph,
            animations,
            physics,
            physics_binder,
            static_geometry_binder,
            // Render target is intentionally not copied, because it does not makes sense - a copy
            // will redraw frame completely.
            render_target: Default::default(),
            lightmap: self.lightmap.clone(),
            drawing_context: self.drawing_context.clone(),
        }
    }
}

impl Visit for Scene {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;
        self.physics_binder.visit("PhysicsBinder", visitor)?;
        self.graph.visit("Graph", visitor)?;
        self.animations.visit("Animations", visitor)?;
        self.physics.visit("Physics", visitor)?;
        let _ = self.lightmap.visit("Lightmap", visitor);
        let _ = self
            .static_geometry_binder
            .visit("StaticGeometryBinder", visitor);
        visitor.leave_region()
    }
}

/// Container for scenes in the engine. It just a simple wrapper around Pool.
pub struct SceneContainer {
    pool: Pool<Scene>,
}

impl SceneContainer {
    pub(in crate) fn new() -> Self {
        Self { pool: Pool::new() }
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
        self.pool.free(handle);
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

impl Default for SceneContainer {
    fn default() -> Self {
        Self { pool: Pool::new() }
    }
}

impl Visit for SceneContainer {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.pool.visit("Pool", visitor)?;

        visitor.leave_region()
    }
}

/// Visibility cache stores information about objects visibility for a single frame.
/// Allows you to quickly check if object is visible or not.
#[derive(Default, Debug)]
pub struct VisibilityCache {
    map: HashMap<Handle<Node>, bool>,
}

impl From<HashMap<Handle<Node>, bool>> for VisibilityCache {
    fn from(map: HashMap<Handle<Node>, bool>) -> Self {
        Self { map }
    }
}

impl VisibilityCache {
    /// Replaces internal map with empty and returns previous value. This trick is useful
    /// to reuse hash map to prevent redundant memory allocations.
    pub fn invalidate(&mut self) -> HashMap<Handle<Node>, bool> {
        std::mem::replace(&mut self.map, Default::default())
    }

    /// Updates visibility cache - checks visibility for each node in given graph, also performs
    /// frustum culling if frustum specified.
    pub fn update(
        &mut self,
        graph: &Graph,
        view_matrix: Mat4,
        z_far: f32,
        frustum: Option<&Frustum>,
    ) {
        self.map.clear();

        let view_position = view_matrix.position();

        // Check LODs first, it has priority over other visibility settings.
        for node in graph.linear_iter() {
            if let Some(lod_group) = node.lod_group() {
                for level in lod_group.levels.iter() {
                    for &object in level.objects.iter() {
                        let normalized_distance =
                            view_position.distance(&graph[object].global_position()) / z_far;
                        let visible = normalized_distance >= level.begin()
                            && normalized_distance <= level.end();
                        self.map.insert(object, visible);
                    }
                }
            }
        }

        // Fill rest of data from global visibility flag of nodes.
        for (handle, node) in graph.pair_iter() {
            // We need to fill only unfilled entries, none of visibility flags of a node can
            // make it visible again if lod group hid it.
            self.map.entry(handle).or_insert_with(|| {
                let mut visibility = node.global_visibility();
                if visibility {
                    if let Some(frustum) = frustum {
                        if let Node::Mesh(mesh) = node {
                            visibility = mesh.is_intersect_frustum(graph, frustum);
                        }
                    }
                }
                visibility
            });
        }
    }

    /// Checks if given node is visible or not.
    pub fn is_visible(&self, node: Handle<Node>) -> bool {
        self.map.get(&node).cloned().unwrap_or(false)
    }
}
