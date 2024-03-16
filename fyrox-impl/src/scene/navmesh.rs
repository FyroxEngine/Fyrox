//! Navigational mesh (navmesh for short) is a surface which can be used for path finding. See [`NavigationalMesh`] docs
//! for more info and usage examples.

use crate::{
    core::{
        color::Color,
        math::aabb::AxisAlignedBoundingBox,
        parking_lot::RwLock,
        pool::Handle,
        reflect::prelude::*,
        uuid::{uuid, Uuid},
        variable::InheritableVariable,
        visitor::prelude::*,
        TypeUuidProvider,
    },
    scene::{
        base::{Base, BaseBuilder},
        debug::{Line, SceneDrawingContext},
        graph::Graph,
        node::{Node, NodeTrait},
    },
    utils::navmesh::Navmesh,
};
use fyrox_core::parking_lot::{RwLockReadGuard, RwLockWriteGuard};
use fyrox_graph::BaseSceneGraph;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

#[derive(Clone, Default, Reflect, Debug)]
pub(crate) struct Container(Arc<RwLock<Navmesh>>);

impl PartialEq for Container {
    fn eq(&self, other: &Self) -> bool {
        *self.0.read() == *other.0.read()
    }
}

impl Visit for Container {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.0.write().visit(name, visitor)
    }
}

/// Navigational mesh (navmesh for short) is a surface which can be used for path finding. Unlike [A* Pathfinder](crate::utils::astar),
/// it can build arbitrary paths on a surface of large polygons, making a path from point A to point B linear (standard pathfinder builds
/// path only from vertex to vertex). Navmeshes should be used when you have an arbitrary "walkable" surface, for example, a game level
/// with rooms, hallways, multiple floors and so on. A* pathfinder should be used for strategies or any other types of games with uniform
/// pathfinding grid.
///
/// ## How to create
///
/// You should prefer using the navmesh editor to create navigational meshes, however if it is not possible, you can create it manually.
/// Use [`NavigationalMeshBuilder`] to create new instance and add it to your scene graph. Keep in mind, that this node is just a
/// convenient wrapper around [`Navmesh`], so you should also read its docs to get better understanding how it works.
///
/// ```rust
/// # use fyrox_impl::{
/// #     core::{algebra::Vector3, math::TriangleDefinition, pool::Handle},
/// #     scene::{base::BaseBuilder, graph::Graph, navmesh::NavigationalMeshBuilder, node::Node},
/// #     utils::navmesh::Navmesh,
/// # };
/// fn create_navmesh(graph: &mut Graph) -> Handle<Node> {
///     // A simple navmesh with four vertices and two triangles.
///     let navmesh = Navmesh::new(
///         vec![TriangleDefinition([0, 1, 2]), TriangleDefinition([0, 2, 3])],
///         vec![
///             Vector3::new(-1.0, 0.0, 1.0),
///             Vector3::new(1.0, 0.0, 1.0),
///             Vector3::new(1.0, 0.0, -1.0),
///             Vector3::new(-1.0, 0.0, -1.0),
///         ],
///     );
///     NavigationalMeshBuilder::new(BaseBuilder::new())
///         .with_navmesh(navmesh)
///         .build(graph)
/// }
/// ```
///
/// ## Agents
///
/// Navigational mesh agent helps you to build paths along the surface of a navigational mesh and follow it. Agents can be
/// used to drive the motion of your game characters. Every agent knows about its target and automatically rebuilds the path
/// if the target has moved. Navmesh agents are able to move along the path, providing you with their current position, so you
/// can use it to perform an actual motion of your game characters. Agents work together with navigational meshes, you need
/// to update their state every frame, so they can recalculate path if needed. A simple example could something like this:
///
/// ```rust
/// # use fyrox_impl::utils::navmesh::NavmeshAgent;
/// # struct Foo {
/// // Add this to your script
/// agent: NavmeshAgent
/// # }
/// ```
///
/// After that, you need to update the agent every frame to make sure it will follow the target:
///
/// ```rust
/// # use fyrox_impl::{
/// #    core::algebra::Vector3, scene::navmesh::NavigationalMesh, utils::navmesh::NavmeshAgent,
/// # };
/// fn update_agent(
///     agent: &mut NavmeshAgent,
///     target: Vector3<f32>,
///     dt: f32,
///     navmesh: &NavigationalMesh,
/// ) {
///     // Set the target to follow and the speed.
///     agent.set_target(target);
///     agent.set_speed(1.0);
///
///     // Update the agent.
///     agent.update(dt, &navmesh.navmesh_ref()).unwrap();
///
///     // Print its position - you can use this position as target point of your game character.
///     println!("{}", agent.position());
/// }
/// ```
///
/// This method should be called in `on_update` of your script. It accepts four parameters: a reference to the agent, a
/// target which it will follow, a time step (`context.dt`), and a reference to navigational mesh node. You can fetch
/// navigational mesh from the scene graph by its name:
///
/// ```rust
/// # use fyrox_impl::scene::{navmesh::NavigationalMesh, Scene};
/// # use fyrox_graph::SceneGraph;
/// fn find_navmesh<'a>(scene: &'a mut Scene, name: &str) -> &'a mut NavigationalMesh {
///     let handle = scene.graph.find_by_name_from_root(name).unwrap().0;
///     scene.graph[handle].as_navigational_mesh_mut()
/// }
/// ```
#[derive(Debug, Clone, Visit, Reflect, Default)]
pub struct NavigationalMesh {
    base: Base,
    #[reflect(read_only)]
    navmesh: InheritableVariable<Container>,
}

impl TypeUuidProvider for NavigationalMesh {
    fn type_uuid() -> Uuid {
        uuid!("d0ce963c-b50a-4707-bd21-af6dc0d1c668")
    }
}

impl Deref for NavigationalMesh {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for NavigationalMesh {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl NodeTrait for NavigationalMesh {
    crate::impl_query_component!();

    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.base.local_bounding_box()
    }

    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.base.world_bounding_box()
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn debug_draw(&self, ctx: &mut SceneDrawingContext) {
        let navmesh = self.navmesh.0.read();

        for vertex in navmesh.vertices().iter() {
            ctx.draw_sphere(*vertex, 6, 6, 0.1, Color::GREEN);
        }

        for triangle in navmesh.triangles().iter() {
            for edge in &triangle.edges() {
                ctx.add_line(Line {
                    begin: navmesh.vertices()[edge.a as usize],
                    end: navmesh.vertices()[edge.b as usize],
                    color: Color::GREEN,
                });
            }
        }
    }
}

impl NavigationalMesh {
    /// Returns a reference to the inner navigational mesh.
    pub fn navmesh_ref(&self) -> RwLockReadGuard<Navmesh> {
        self.navmesh.0.read()
    }

    /// Returns a reference to the inner navigational mesh.
    pub fn navmesh_mut(&mut self) -> RwLockWriteGuard<Navmesh> {
        self.navmesh.0.write()
    }

    /// Returns a shared reference to the inner navigational mesh. It could be used to perform
    /// off-thread path calculations.
    pub fn navmesh(&self) -> Arc<RwLock<Navmesh>> {
        self.navmesh.0.clone()
    }
}

/// Creates navigational meshes and adds them to a scene graph.
pub struct NavigationalMeshBuilder {
    base_builder: BaseBuilder,
    navmesh: Navmesh,
}

impl NavigationalMeshBuilder {
    /// Creates new navigational mesh builder.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            navmesh: Default::default(),
        }
    }

    /// Sets the actual navigational mesh.
    pub fn with_navmesh(mut self, navmesh: Navmesh) -> Self {
        self.navmesh = navmesh;
        self
    }

    fn build_navigational_mesh(self) -> NavigationalMesh {
        NavigationalMesh {
            base: self.base_builder.build_base(),
            navmesh: InheritableVariable::new_modified(Container(Arc::new(RwLock::new(
                self.navmesh,
            )))),
        }
    }

    /// Creates new navigational mesh instance.
    pub fn build_node(self) -> Node {
        Node::new(self.build_navigational_mesh())
    }

    /// Creates new navigational mesh instance and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
