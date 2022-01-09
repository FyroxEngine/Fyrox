use fyrox::core::algebra::Vector3;
use fyrox::core::pool::{Handle, Pool};

#[derive(Debug, Clone)]
pub struct NavmeshVertex {
    pub position: Vector3<f32>,
}

#[derive(Debug, Clone)]
pub struct NavmeshTriangle {
    pub a: Handle<NavmeshVertex>,
    pub b: Handle<NavmeshVertex>,
    pub c: Handle<NavmeshVertex>,
}

#[derive(PartialEq, Copy, Clone, Debug, Eq)]
pub struct NavmeshEdge {
    pub begin: Handle<NavmeshVertex>,
    pub end: Handle<NavmeshVertex>,
}

impl NavmeshTriangle {
    pub fn vertices(&self) -> [Handle<NavmeshVertex>; 3] {
        [self.a, self.b, self.c]
    }

    pub fn edges(&self) -> [NavmeshEdge; 3] {
        [
            NavmeshEdge {
                begin: self.a,
                end: self.b,
            },
            NavmeshEdge {
                begin: self.b,
                end: self.c,
            },
            NavmeshEdge {
                begin: self.c,
                end: self.a,
            },
        ]
    }
}

#[derive(PartialEq, Copy, Clone, Debug, Eq)]
pub enum NavmeshEntity {
    Vertex(Handle<NavmeshVertex>),
    Edge(NavmeshEdge),
}

#[derive(Debug, Default)]
pub struct Navmesh {
    pub vertices: Pool<NavmeshVertex>,
    pub triangles: Pool<NavmeshTriangle>,
}

impl Navmesh {
    pub fn new() -> Self {
        let mut vertices = Pool::new();

        let a = vertices.spawn(NavmeshVertex {
            position: Vector3::new(-1.0, 0.0, -1.0),
        });
        let b = vertices.spawn(NavmeshVertex {
            position: Vector3::new(1.0, 0.0, -1.0),
        });
        let c = vertices.spawn(NavmeshVertex {
            position: Vector3::new(1.0, 0.0, 1.0),
        });
        let d = vertices.spawn(NavmeshVertex {
            position: Vector3::new(-1.0, 0.0, 1.0),
        });

        let mut triangles = Pool::new();

        let _ = triangles.spawn(NavmeshTriangle { a, b, c });
        let _ = triangles.spawn(NavmeshTriangle { a, b: c, c: d });

        Self {
            vertices,
            triangles,
        }
    }
}
