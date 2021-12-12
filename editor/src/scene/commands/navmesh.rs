use crate::command::Command;
use crate::interaction::navmesh::data_model::{
    Navmesh, NavmeshEdge, NavmeshEntity, NavmeshTriangle, NavmeshVertex,
};
use crate::interaction::navmesh::selection::NavmeshSelection;
use crate::scene::commands::SceneContext;
use crate::scene::Selection;
use rg3d::core::algebra::Vector3;
use rg3d::core::pool::{Handle, Ticket};

#[derive(Debug)]
pub struct AddNavmeshEdgeCommand {
    navmesh: Handle<Navmesh>,
    opposite_edge: NavmeshEdge,
    state: AddNavmeshEdgeCommandState,
    select: bool,
    new_selection: Selection,
}

#[derive(Debug)]
enum AddNavmeshEdgeCommandState {
    Undefined,
    NonExecuted {
        edge: (NavmeshVertex, NavmeshVertex),
    },
    Executed {
        triangles: [Handle<NavmeshTriangle>; 2],
        vertices: [Handle<NavmeshVertex>; 2],
    },
    Reverted {
        triangles: [(Ticket<NavmeshTriangle>, NavmeshTriangle); 2],
        vertices: [(Ticket<NavmeshVertex>, NavmeshVertex); 2],
    },
}

impl AddNavmeshEdgeCommand {
    pub fn new(
        navmesh: Handle<Navmesh>,
        edge: (NavmeshVertex, NavmeshVertex),
        opposite_edge: NavmeshEdge,
        select: bool,
    ) -> Self {
        Self {
            navmesh,
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
        let navmesh = &mut context.editor_scene.navmeshes[self.navmesh];
        match std::mem::replace(&mut self.state, AddNavmeshEdgeCommandState::Undefined) {
            AddNavmeshEdgeCommandState::NonExecuted { edge } => {
                let begin_handle = navmesh.vertices.spawn(edge.0);
                let end_handle = navmesh.vertices.spawn(edge.1);
                let triangle_a = navmesh.triangles.spawn(NavmeshTriangle {
                    a: self.opposite_edge.begin,
                    b: begin_handle,
                    c: self.opposite_edge.end,
                });
                let triangle_b = navmesh.triangles.spawn(NavmeshTriangle {
                    a: begin_handle,
                    b: end_handle,
                    c: self.opposite_edge.end,
                });
                self.state = AddNavmeshEdgeCommandState::Executed {
                    triangles: [triangle_a, triangle_b],
                    vertices: [begin_handle, end_handle],
                };

                let navmesh_selection = NavmeshSelection::new(
                    self.navmesh,
                    vec![NavmeshEntity::Edge(NavmeshEdge {
                        begin: begin_handle,
                        end: end_handle,
                    })],
                );

                self.new_selection = Selection::Navmesh(navmesh_selection);
            }
            AddNavmeshEdgeCommandState::Reverted {
                triangles,
                vertices,
            } => {
                let [va, vb] = vertices;
                let begin_handle = navmesh.vertices.put_back(va.0, va.1);
                let end_handle = navmesh.vertices.put_back(vb.0, vb.1);

                let [ta, tb] = triangles;
                let triangle_a = navmesh.triangles.put_back(ta.0, ta.1);
                let triangle_b = navmesh.triangles.put_back(tb.0, tb.1);

                self.state = AddNavmeshEdgeCommandState::Executed {
                    triangles: [triangle_a, triangle_b],
                    vertices: [begin_handle, end_handle],
                };
            }
            _ => unreachable!(),
        }

        if self.select {
            std::mem::swap(&mut context.editor_scene.selection, &mut self.new_selection);
        }
    }

    fn revert(&mut self, context: &mut SceneContext) {
        if self.select {
            std::mem::swap(&mut context.editor_scene.selection, &mut self.new_selection);
        }

        let navmesh = &mut context.editor_scene.navmeshes[self.navmesh];
        match std::mem::replace(&mut self.state, AddNavmeshEdgeCommandState::Undefined) {
            AddNavmeshEdgeCommandState::Executed {
                triangles,
                vertices,
            } => {
                self.state = AddNavmeshEdgeCommandState::Reverted {
                    triangles: [
                        navmesh.triangles.take_reserve(triangles[0]),
                        navmesh.triangles.take_reserve(triangles[1]),
                    ],
                    vertices: [
                        navmesh.vertices.take_reserve(vertices[0]),
                        navmesh.vertices.take_reserve(vertices[1]),
                    ],
                };
            }
            _ => unreachable!(),
        }
    }

    fn finalize(&mut self, context: &mut SceneContext) {
        if let AddNavmeshEdgeCommandState::Reverted {
            triangles,
            vertices,
        } = std::mem::replace(&mut self.state, AddNavmeshEdgeCommandState::Undefined)
        {
            if let Some(navmesh) = context.editor_scene.navmeshes.try_borrow_mut(self.navmesh) {
                // Forget tickets.
                let [va, vb] = vertices;
                navmesh.vertices.forget_ticket(va.0);
                navmesh.vertices.forget_ticket(vb.0);

                let [ta, tb] = triangles;
                navmesh.triangles.forget_ticket(ta.0);
                navmesh.triangles.forget_ticket(tb.0);
            }
        }
    }
}

#[derive(Debug)]
pub enum ConnectNavmeshEdgesCommandState {
    Undefined,
    NonExecuted {
        edges: [NavmeshEdge; 2],
    },
    Executed {
        triangles: [Handle<NavmeshTriangle>; 2],
    },
    Reverted {
        triangles: [(Ticket<NavmeshTriangle>, NavmeshTriangle); 2],
    },
}

#[derive(Debug)]
pub struct ConnectNavmeshEdgesCommand {
    navmesh: Handle<Navmesh>,
    state: ConnectNavmeshEdgesCommandState,
}

impl ConnectNavmeshEdgesCommand {
    pub fn new(navmesh: Handle<Navmesh>, edges: [NavmeshEdge; 2]) -> Self {
        Self {
            navmesh,
            state: ConnectNavmeshEdgesCommandState::NonExecuted { edges },
        }
    }
}

impl Command for ConnectNavmeshEdgesCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Connect Navmesh Edges".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        let navmesh = &mut context.editor_scene.navmeshes[self.navmesh];

        match std::mem::replace(&mut self.state, ConnectNavmeshEdgesCommandState::Undefined) {
            ConnectNavmeshEdgesCommandState::NonExecuted { edges } => {
                let ta = navmesh.triangles.spawn(NavmeshTriangle {
                    a: edges[0].begin,
                    b: edges[0].end,
                    c: edges[1].begin,
                });
                let tb = navmesh.triangles.spawn(NavmeshTriangle {
                    a: edges[1].begin,
                    b: edges[1].end,
                    c: edges[0].begin,
                });

                self.state = ConnectNavmeshEdgesCommandState::Executed {
                    triangles: [ta, tb],
                };
            }
            ConnectNavmeshEdgesCommandState::Reverted { triangles } => {
                let [a, b] = triangles;
                let ta = navmesh.triangles.put_back(a.0, a.1);
                let tb = navmesh.triangles.put_back(b.0, b.1);

                self.state = ConnectNavmeshEdgesCommandState::Executed {
                    triangles: [ta, tb],
                }
            }
            _ => unreachable!(),
        }
    }

    fn revert(&mut self, context: &mut SceneContext) {
        let navmesh = &mut context.editor_scene.navmeshes[self.navmesh];

        match std::mem::replace(&mut self.state, ConnectNavmeshEdgesCommandState::Undefined) {
            ConnectNavmeshEdgesCommandState::Executed { triangles } => {
                self.state = ConnectNavmeshEdgesCommandState::Reverted {
                    triangles: [
                        navmesh.triangles.take_reserve(triangles[0]),
                        navmesh.triangles.take_reserve(triangles[1]),
                    ],
                }
            }
            _ => unreachable!(),
        }
    }

    fn finalize(&mut self, context: &mut SceneContext) {
        let navmesh = &mut context.editor_scene.navmeshes[self.navmesh];

        if let ConnectNavmeshEdgesCommandState::Reverted { triangles } =
            std::mem::replace(&mut self.state, ConnectNavmeshEdgesCommandState::Undefined)
        {
            let [a, b] = triangles;
            navmesh.triangles.forget_ticket(a.0);
            navmesh.triangles.forget_ticket(b.0);
        }
    }
}

#[derive(Debug)]
pub struct AddNavmeshCommand {
    ticket: Option<Ticket<Navmesh>>,
    handle: Handle<Navmesh>,
    navmesh: Option<Navmesh>,
}

impl AddNavmeshCommand {
    pub fn new(navmesh: Navmesh) -> Self {
        Self {
            ticket: None,
            handle: Default::default(),
            navmesh: Some(navmesh),
        }
    }
}

impl Command for AddNavmeshCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Add Navmesh".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        match self.ticket.take() {
            None => {
                self.handle = context
                    .editor_scene
                    .navmeshes
                    .spawn(self.navmesh.take().unwrap());
            }
            Some(ticket) => {
                let handle = context
                    .editor_scene
                    .navmeshes
                    .put_back(ticket, self.navmesh.take().unwrap());
                assert_eq!(handle, self.handle);
            }
        }
    }

    fn revert(&mut self, context: &mut SceneContext) {
        let (ticket, node) = context.editor_scene.navmeshes.take_reserve(self.handle);
        self.ticket = Some(ticket);
        self.navmesh = Some(node);
    }

    fn finalize(&mut self, context: &mut SceneContext) {
        if let Some(ticket) = self.ticket.take() {
            context.editor_scene.navmeshes.forget_ticket(ticket)
        }
    }
}

macro_rules! define_pool_command {
    ($name:ident, $inner_ty:ty, $human_readable_name:expr, $ctx:ident, $self:ident, $get_pool:block, $($field:ident:$type:ty),*) => {
        #[derive(Debug)]
        pub struct $name {
            pub ticket: Option<Ticket<$inner_ty>>,
            pub handle: Handle<$inner_ty>,
            pub value: Option<$inner_ty>,
            $(pub $field: $type,)*
        }

        impl Command for $name {


            fn name(&mut self, _context: &SceneContext) -> String {
                $human_readable_name.to_owned()
            }

            fn execute(&mut $self, $ctx: &mut SceneContext) {
               let pool = $get_pool;
               match $self.ticket.take() {
                    None => {
                        $self.handle = pool.spawn($self.value.take().unwrap());
                    }
                    Some(ticket) => {
                        let handle = pool.put_back(ticket, $self.value.take().unwrap());
                        assert_eq!(handle, $self.handle);
                    }
                }
            }

            fn revert(&mut $self, $ctx: &mut SceneContext) {
                let pool = $get_pool;

                let (ticket, node) = pool.take_reserve($self.handle);
                $self.ticket = Some(ticket);
                $self.value = Some(node);
            }

            fn finalize(&mut $self, $ctx: &mut SceneContext) {
                let pool = $get_pool;

                if let Some(ticket) = $self.ticket.take() {
                    pool.forget_ticket(ticket)
                }
            }
        }
    };
}

define_pool_command!(
    AddNavmeshVertexCommand,
    NavmeshVertex,
    "Add Navmesh Vertex",
    ctx,
    self,
    { &mut ctx.editor_scene.navmeshes[self.navmesh].vertices },
    navmesh: Handle<Navmesh>
);

define_pool_command!(
    AddNavmeshTriangleCommand,
    NavmeshTriangle,
    "Add Navmesh Triangle",
    ctx,
    self,
    { &mut ctx.editor_scene.navmeshes[self.navmesh].triangles },
    navmesh: Handle<Navmesh>
);

#[derive(Debug)]
pub struct DeleteNavmeshCommand {
    handle: Handle<Navmesh>,
    ticket: Option<Ticket<Navmesh>>,
    node: Option<Navmesh>,
}

impl DeleteNavmeshCommand {
    pub fn new(handle: Handle<Navmesh>) -> Self {
        Self {
            handle,
            ticket: None,
            node: None,
        }
    }
}

impl Command for DeleteNavmeshCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Delete Navmesh".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        let (ticket, node) = context.editor_scene.navmeshes.take_reserve(self.handle);
        self.node = Some(node);
        self.ticket = Some(ticket);
    }

    fn revert(&mut self, context: &mut SceneContext) {
        self.handle = context
            .editor_scene
            .navmeshes
            .put_back(self.ticket.take().unwrap(), self.node.take().unwrap());
    }

    fn finalize(&mut self, context: &mut SceneContext) {
        if let Some(ticket) = self.ticket.take() {
            context.editor_scene.navmeshes.forget_ticket(ticket)
        }
    }
}

#[derive(Debug)]
pub struct DeleteNavmeshVertexCommand {
    navmesh: Handle<Navmesh>,
    state: DeleteNavmeshVertexCommandState,
}

#[derive(Debug)]
pub enum DeleteNavmeshVertexCommandState {
    Undefined,
    NonExecuted {
        vertex: Handle<NavmeshVertex>,
    },
    Executed {
        vertex: (Ticket<NavmeshVertex>, NavmeshVertex),
        triangles: Vec<(Ticket<NavmeshTriangle>, NavmeshTriangle)>,
    },
    Reverted {
        vertex: Handle<NavmeshVertex>,
    },
}

impl DeleteNavmeshVertexCommand {
    pub fn new(navmesh: Handle<Navmesh>, vertex: Handle<NavmeshVertex>) -> Self {
        Self {
            navmesh,
            state: DeleteNavmeshVertexCommandState::NonExecuted { vertex },
        }
    }
}

impl Command for DeleteNavmeshVertexCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Delete Navmesh Vertex".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        let navmesh = &mut context.editor_scene.navmeshes[self.navmesh];

        match std::mem::replace(&mut self.state, DeleteNavmeshVertexCommandState::Undefined) {
            DeleteNavmeshVertexCommandState::NonExecuted { vertex }
            | DeleteNavmeshVertexCommandState::Reverted { vertex } => {
                // Find each triangle that shares the same vertex and move them out of pool.
                let mut triangles = Vec::new();
                for (handle, triangle) in navmesh.triangles.pair_iter() {
                    if triangle.vertices().contains(&vertex) {
                        triangles.push(handle);
                    }
                }

                self.state = DeleteNavmeshVertexCommandState::Executed {
                    vertex: navmesh.vertices.take_reserve(vertex),
                    triangles: triangles
                        .iter()
                        .map(|&t| navmesh.triangles.take_reserve(t))
                        .collect(),
                };
            }
            _ => unreachable!(),
        }
    }

    fn revert(&mut self, context: &mut SceneContext) {
        let navmesh = &mut context.editor_scene.navmeshes[self.navmesh];

        match std::mem::replace(&mut self.state, DeleteNavmeshVertexCommandState::Undefined) {
            DeleteNavmeshVertexCommandState::Executed { vertex, triangles } => {
                let vertex = navmesh.vertices.put_back(vertex.0, vertex.1);
                for (ticket, triangle) in triangles {
                    navmesh.triangles.put_back(ticket, triangle);
                }

                self.state = DeleteNavmeshVertexCommandState::Reverted { vertex };
            }
            _ => unreachable!(),
        }
    }

    fn finalize(&mut self, context: &mut SceneContext) {
        if let DeleteNavmeshVertexCommandState::Executed { vertex, triangles } =
            std::mem::replace(&mut self.state, DeleteNavmeshVertexCommandState::Undefined)
        {
            if let Some(navmesh) = context.editor_scene.navmeshes.try_borrow_mut(self.navmesh) {
                navmesh.vertices.forget_ticket(vertex.0);
                for (ticket, _) in triangles {
                    navmesh.triangles.forget_ticket(ticket);
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct MoveNavmeshVertexCommand {
    navmesh: Handle<Navmesh>,
    vertex: Handle<NavmeshVertex>,
    old_position: Vector3<f32>,
    new_position: Vector3<f32>,
}

impl MoveNavmeshVertexCommand {
    pub fn new(
        navmesh: Handle<Navmesh>,
        vertex: Handle<NavmeshVertex>,
        old_position: Vector3<f32>,
        new_position: Vector3<f32>,
    ) -> Self {
        Self {
            navmesh,
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
        navmesh.vertices[self.vertex].position = position;
    }
}

impl Command for MoveNavmeshVertexCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Move Navmesh Vertex".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        let position = self.swap();
        self.set_position(&mut context.editor_scene.navmeshes[self.navmesh], position);
    }

    fn revert(&mut self, context: &mut SceneContext) {
        let position = self.swap();
        self.set_position(&mut context.editor_scene.navmeshes[self.navmesh], position);
    }
}
