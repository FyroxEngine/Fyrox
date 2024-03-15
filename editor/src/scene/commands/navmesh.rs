use crate::command::CommandContext;
use crate::fyrox::core::parking_lot::RwLockWriteGuard;
use crate::fyrox::{
    core::{
        algebra::Vector3,
        math::{TriangleDefinition, TriangleEdge},
        pool::Handle,
    },
    scene::node::Node,
    utils::navmesh::Navmesh,
};
use crate::{
    command::CommandTrait,
    interaction::navmesh::selection::{NavmeshEntity, NavmeshSelection},
    scene::{commands::GameSceneContext, Selection},
};

#[derive(Debug)]
pub struct AddNavmeshEdgeCommand {
    navmesh_node: Handle<Node>,
    opposite_edge: TriangleEdge,
    state: AddNavmeshEdgeCommandState,
    select: bool,
    new_selection: Selection,
}

fn fetch_navmesh(ctx: &mut GameSceneContext, node: Handle<Node>) -> RwLockWriteGuard<Navmesh> {
    ctx.scene.graph[node]
        .as_navigational_mesh_mut()
        .navmesh_mut()
}

#[derive(Debug)]
enum AddNavmeshEdgeCommandState {
    Undefined,
    NonExecuted { edge: (Vector3<f32>, Vector3<f32>) },
    Executed,
    Reverted { edge: (Vector3<f32>, Vector3<f32>) },
}

impl AddNavmeshEdgeCommand {
    pub fn new(
        navmesh_node: Handle<Node>,
        edge: (Vector3<f32>, Vector3<f32>),
        opposite_edge: TriangleEdge,
        select: bool,
    ) -> Self {
        Self {
            navmesh_node,
            opposite_edge,
            state: AddNavmeshEdgeCommandState::NonExecuted { edge },
            select,
            new_selection: Default::default(),
        }
    }
}

impl CommandTrait for AddNavmeshEdgeCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Add Navmesh Edge".to_owned()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let mut navmesh = fetch_navmesh(context, self.navmesh_node);

        match std::mem::replace(&mut self.state, AddNavmeshEdgeCommandState::Undefined) {
            AddNavmeshEdgeCommandState::NonExecuted { edge }
            | AddNavmeshEdgeCommandState::Reverted { edge } => {
                let mut ctx = navmesh.modify();
                let begin = ctx.add_vertex(edge.0);
                let end = ctx.add_vertex(edge.1);
                ctx.add_triangle(TriangleDefinition([
                    self.opposite_edge.a,
                    begin,
                    self.opposite_edge.b,
                ]));
                ctx.add_triangle(TriangleDefinition([begin, end, self.opposite_edge.b]));
                self.state = AddNavmeshEdgeCommandState::Executed;
                let navmesh_selection = NavmeshSelection::new(
                    self.navmesh_node,
                    vec![NavmeshEntity::Edge(TriangleEdge { a: begin, b: end })],
                );

                self.new_selection = Selection::new(navmesh_selection);
            }
            _ => unreachable!(),
        }

        drop(navmesh);

        if self.select {
            std::mem::swap(context.selection, &mut self.new_selection);
        }
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        if self.select {
            std::mem::swap(context.selection, &mut self.new_selection);
        }

        let mut navmesh = fetch_navmesh(context, self.navmesh_node);

        match std::mem::replace(&mut self.state, AddNavmeshEdgeCommandState::Undefined) {
            AddNavmeshEdgeCommandState::Executed => {
                let mut ctx = navmesh.modify();
                ctx.pop_triangle();
                ctx.pop_triangle();
                let va = ctx.pop_vertex().unwrap();
                let vb = ctx.pop_vertex().unwrap();
                self.state = AddNavmeshEdgeCommandState::Reverted { edge: (vb, va) };
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub enum ConnectNavmeshEdgesCommandState {
    Undefined,
    NonExecuted { edges: [TriangleEdge; 2] },
    Executed,
    Reverted { triangles: [TriangleDefinition; 2] },
}

#[derive(Debug)]
pub struct ConnectNavmeshEdgesCommand {
    navmesh_node: Handle<Node>,
    state: ConnectNavmeshEdgesCommandState,
}

impl ConnectNavmeshEdgesCommand {
    pub fn new(navmesh_node: Handle<Node>, edges: [TriangleEdge; 2]) -> Self {
        Self {
            navmesh_node,
            state: ConnectNavmeshEdgesCommandState::NonExecuted { edges },
        }
    }
}

impl CommandTrait for ConnectNavmeshEdgesCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Connect Navmesh Edges".to_owned()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let mut navmesh = fetch_navmesh(context, self.navmesh_node);
        let mut ctx = navmesh.modify();

        match std::mem::replace(&mut self.state, ConnectNavmeshEdgesCommandState::Undefined) {
            ConnectNavmeshEdgesCommandState::NonExecuted { edges } => {
                ctx.add_triangle(TriangleDefinition([edges[0].a, edges[0].b, edges[1].a]));
                ctx.add_triangle(TriangleDefinition([edges[1].a, edges[1].b, edges[0].a]));

                self.state = ConnectNavmeshEdgesCommandState::Executed;
            }
            ConnectNavmeshEdgesCommandState::Reverted { triangles } => {
                let [a, b] = triangles;
                ctx.add_triangle(a);
                ctx.add_triangle(b);
                self.state = ConnectNavmeshEdgesCommandState::Executed;
            }
            _ => unreachable!(),
        }
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let mut navmesh = fetch_navmesh(context, self.navmesh_node);
        let mut ctx = navmesh.modify();

        match std::mem::replace(&mut self.state, ConnectNavmeshEdgesCommandState::Undefined) {
            ConnectNavmeshEdgesCommandState::Executed => {
                self.state = ConnectNavmeshEdgesCommandState::Reverted {
                    triangles: [ctx.pop_triangle().unwrap(), ctx.pop_triangle().unwrap()],
                }
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub struct DeleteNavmeshVertexCommand {
    navmesh_node: Handle<Node>,
    state: DeleteNavmeshVertexCommandState,
}

#[derive(Debug)]
pub enum DeleteNavmeshVertexCommandState {
    Undefined,
    NonExecuted {
        vertex: usize,
    },
    Executed {
        vertex: Vector3<f32>,
        vertex_index: usize,
        triangles: Vec<TriangleDefinition>,
    },
    Reverted {
        vertex: usize,
    },
}

impl DeleteNavmeshVertexCommand {
    pub fn new(navmesh_node: Handle<Node>, vertex: usize) -> Self {
        Self {
            navmesh_node,
            state: DeleteNavmeshVertexCommandState::NonExecuted { vertex },
        }
    }
}

impl CommandTrait for DeleteNavmeshVertexCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Delete Navmesh Vertex".to_owned()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let mut navmesh = fetch_navmesh(context, self.navmesh_node);

        match std::mem::replace(&mut self.state, DeleteNavmeshVertexCommandState::Undefined) {
            DeleteNavmeshVertexCommandState::NonExecuted { vertex }
            | DeleteNavmeshVertexCommandState::Reverted { vertex } => {
                let mut triangles = Vec::new();

                for triangle in navmesh.triangles() {
                    if triangle.indices().contains(&(vertex as u32)) {
                        triangles.push(*triangle);
                    }
                }

                self.state = DeleteNavmeshVertexCommandState::Executed {
                    vertex: navmesh.modify().remove_vertex(vertex),
                    triangles,
                    vertex_index: vertex,
                };
            }
            _ => unreachable!(),
        }
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let mut navmesh = fetch_navmesh(context, self.navmesh_node);

        match std::mem::replace(&mut self.state, DeleteNavmeshVertexCommandState::Undefined) {
            DeleteNavmeshVertexCommandState::Executed {
                vertex,
                vertex_index,
                triangles,
            } => {
                let mut ctx = navmesh.modify();

                ctx.insert_vertex(vertex_index as u32, vertex);

                for triangle in triangles {
                    ctx.add_triangle(triangle);
                }

                self.state = DeleteNavmeshVertexCommandState::Reverted {
                    vertex: vertex_index,
                };
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub struct MoveNavmeshVertexCommand {
    navmesh_node: Handle<Node>,
    vertex: usize,
    old_position: Vector3<f32>,
    new_position: Vector3<f32>,
}

impl MoveNavmeshVertexCommand {
    pub fn new(
        navmesh_node: Handle<Node>,
        vertex: usize,
        old_position: Vector3<f32>,
        new_position: Vector3<f32>,
    ) -> Self {
        Self {
            navmesh_node,
            vertex,
            old_position,
            new_position,
        }
    }

    fn swap(&mut self) -> Vector3<f32> {
        let position = self.new_position;
        std::mem::swap(&mut self.new_position, &mut self.old_position);
        position
    }

    fn set_position(&self, mut navmesh: RwLockWriteGuard<Navmesh>, position: Vector3<f32>) {
        navmesh.modify().vertices_mut()[self.vertex] = position;
    }
}

impl CommandTrait for MoveNavmeshVertexCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Move Navmesh Vertex".to_owned()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let position = self.swap();
        self.set_position(fetch_navmesh(context, self.navmesh_node), position);
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let position = self.swap();
        self.set_position(fetch_navmesh(context, self.navmesh_node), position);
    }
}
