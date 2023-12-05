use crate::{
    command::Command,
    interaction::navmesh::selection::{NavmeshEntity, NavmeshSelection},
    scene::{commands::SceneContext, Selection},
};
use fyrox::{
    core::{
        algebra::Vector3,
        math::{TriangleDefinition, TriangleEdge},
        pool::Handle,
    },
    scene::node::Node,
    utils::navmesh::Navmesh,
};

#[derive(Debug)]
pub struct AddNavmeshEdgeCommand {
    navmesh_node: Handle<Node>,
    opposite_edge: TriangleEdge,
    state: AddNavmeshEdgeCommandState,
    select: bool,
    new_selection: Selection,
}

fn fetch_navmesh<'a>(ctx: &'a mut SceneContext, node: Handle<Node>) -> &'a mut Navmesh {
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

impl Command for AddNavmeshEdgeCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Add Navmesh Edge".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        let navmesh = fetch_navmesh(context, self.navmesh_node);

        match std::mem::replace(&mut self.state, AddNavmeshEdgeCommandState::Undefined) {
            AddNavmeshEdgeCommandState::NonExecuted { edge }
            | AddNavmeshEdgeCommandState::Reverted { edge } => {
                let begin = navmesh.add_vertex(edge.0);
                let end = navmesh.add_vertex(edge.1);
                navmesh.add_triangle(TriangleDefinition([
                    self.opposite_edge.a,
                    begin,
                    self.opposite_edge.b,
                ]));
                navmesh.add_triangle(TriangleDefinition([begin, end, self.opposite_edge.b]));
                self.state = AddNavmeshEdgeCommandState::Executed;
                let navmesh_selection = NavmeshSelection::new(
                    self.navmesh_node,
                    vec![NavmeshEntity::Edge(TriangleEdge { a: begin, b: end })],
                );

                self.new_selection = Selection::Navmesh(navmesh_selection);
            }
            _ => unreachable!(),
        }

        if self.select {
            std::mem::swap(context.selection, &mut self.new_selection);
        }
    }

    fn revert(&mut self, context: &mut SceneContext) {
        if self.select {
            std::mem::swap(context.selection, &mut self.new_selection);
        }

        let navmesh = fetch_navmesh(context, self.navmesh_node);

        match std::mem::replace(&mut self.state, AddNavmeshEdgeCommandState::Undefined) {
            AddNavmeshEdgeCommandState::Executed => {
                navmesh.pop_triangle();
                navmesh.pop_triangle();
                let va = navmesh.pop_vertex().unwrap();
                let vb = navmesh.pop_vertex().unwrap();
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

impl Command for ConnectNavmeshEdgesCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Connect Navmesh Edges".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        let navmesh = fetch_navmesh(context, self.navmesh_node);

        match std::mem::replace(&mut self.state, ConnectNavmeshEdgesCommandState::Undefined) {
            ConnectNavmeshEdgesCommandState::NonExecuted { edges } => {
                navmesh.add_triangle(TriangleDefinition([edges[0].a, edges[0].b, edges[1].a]));
                navmesh.add_triangle(TriangleDefinition([edges[1].a, edges[1].b, edges[0].a]));

                self.state = ConnectNavmeshEdgesCommandState::Executed;
            }
            ConnectNavmeshEdgesCommandState::Reverted { triangles } => {
                let [a, b] = triangles;
                navmesh.add_triangle(a);
                navmesh.add_triangle(b);
                self.state = ConnectNavmeshEdgesCommandState::Executed;
            }
            _ => unreachable!(),
        }
    }

    fn revert(&mut self, context: &mut SceneContext) {
        let navmesh = fetch_navmesh(context, self.navmesh_node);

        match std::mem::replace(&mut self.state, ConnectNavmeshEdgesCommandState::Undefined) {
            ConnectNavmeshEdgesCommandState::Executed => {
                self.state = ConnectNavmeshEdgesCommandState::Reverted {
                    triangles: [
                        navmesh.pop_triangle().unwrap(),
                        navmesh.pop_triangle().unwrap(),
                    ],
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

impl Command for DeleteNavmeshVertexCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Delete Navmesh Vertex".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        let navmesh = fetch_navmesh(context, self.navmesh_node);

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
                    vertex: navmesh.remove_vertex(vertex),
                    triangles,
                    vertex_index: vertex,
                };
            }
            _ => unreachable!(),
        }
    }

    fn revert(&mut self, context: &mut SceneContext) {
        let navmesh = fetch_navmesh(context, self.navmesh_node);

        match std::mem::replace(&mut self.state, DeleteNavmeshVertexCommandState::Undefined) {
            DeleteNavmeshVertexCommandState::Executed {
                vertex,
                vertex_index,
                triangles,
            } => {
                navmesh.insert_vertex(vertex_index as u32, vertex);

                for triangle in triangles {
                    navmesh.add_triangle(triangle);
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

    fn set_position(&self, navmesh: &mut Navmesh, position: Vector3<f32>) {
        navmesh.vertices_mut()[self.vertex] = position;
    }
}

impl Command for MoveNavmeshVertexCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Move Navmesh Vertex".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        let position = self.swap();
        self.set_position(fetch_navmesh(context, self.navmesh_node), position);
    }

    fn revert(&mut self, context: &mut SceneContext) {
        let position = self.swap();
        self.set_position(fetch_navmesh(context, self.navmesh_node), position);
    }
}
