use rg3d::core::algebra::Vector3;
use rg3d::core::pool::{Handle, Pool};

#[derive(Debug)]
pub struct NavmeshVertex {
    pub position: Vector3<f32>,
}

#[derive(Debug)]
pub struct Triangle {
    pub a: Handle<NavmeshVertex>,
    pub b: Handle<NavmeshVertex>,
    pub c: Handle<NavmeshVertex>,
}

#[derive(PartialEq, Copy, Clone)]
pub struct Edge {
    pub begin: Handle<NavmeshVertex>,
    pub end: Handle<NavmeshVertex>,
}

impl Triangle {
    pub fn vertices(&self) -> [Handle<NavmeshVertex>; 3] {
        [self.a, self.b, self.c]
    }

    pub fn edges(&self) -> [Edge; 3] {
        [
            Edge {
                begin: self.a,
                end: self.b,
            },
            Edge {
                begin: self.b,
                end: self.c,
            },
            Edge {
                begin: self.c,
                end: self.a,
            },
        ]
    }
}

#[derive(PartialEq, Copy, Clone)]
pub enum NavmeshEntity {
    Vertex(Handle<NavmeshVertex>),
    Edge(Edge),
}

#[derive(Debug)]
pub struct Navmesh {
    pub vertices: Pool<NavmeshVertex>,
    pub triangles: Pool<Triangle>,
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
            position: Vector3::new(0.0, 0.0, 1.0),
        });

        let mut triangles = Pool::new();

        let _ = triangles.spawn(Triangle { a, b, c });

        Self {
            vertices,
            triangles,
        }
    }
}
