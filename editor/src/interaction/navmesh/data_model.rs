use crate::interaction::navmesh::selection::NavmeshSelection;
use crate::settings::navmesh::NavmeshSettings;
use fyrox::{
    core::{
        algebra::Vector3,
        color::Color,
        pool::{Handle, Pool},
    },
    scene::debug::SceneDrawingContext,
};
use std::ops::{Deref, DerefMut};

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

    pub fn draw(
        &self,
        drawing_context: &mut SceneDrawingContext,
        selection: Option<&NavmeshSelection>,
        vertex_radius: f32,
    ) {
        for (handle, vertex) in self.vertices.pair_iter() {
            drawing_context.draw_sphere(
                vertex.position,
                10,
                10,
                vertex_radius,
                selection.map_or(Color::GREEN, |s| {
                    if s.unique_vertices().contains(&handle) {
                        Color::RED
                    } else {
                        Color::GREEN
                    }
                }),
            );
        }

        for triangle in self.triangles.iter() {
            for edge in &triangle.edges() {
                drawing_context.add_line(fyrox::scene::debug::Line {
                    begin: self.vertices[edge.begin].position,
                    end: self.vertices[edge.end].position,
                    color: selection.map_or(Color::GREEN, |s| {
                        if s.contains_edge(*edge) {
                            Color::RED
                        } else {
                            Color::GREEN
                        }
                    }),
                });
            }
        }
    }
}

pub struct NavmeshContainer {
    pub pool: Pool<Navmesh>,
}

impl Default for NavmeshContainer {
    fn default() -> Self {
        Self {
            pool: Default::default(),
        }
    }
}

impl Deref for NavmeshContainer {
    type Target = Pool<Navmesh>;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}

impl DerefMut for NavmeshContainer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.pool
    }
}

impl NavmeshContainer {
    pub fn draw(
        &self,
        drawing_context: &mut SceneDrawingContext,
        selection: Option<&NavmeshSelection>,
        settings: &NavmeshSettings,
    ) {
        if settings.draw_all {
            for navmesh in self.pool.iter() {
                navmesh.draw(drawing_context, selection, settings.vertex_radius)
            }
        } else if let Some(selection) = selection {
            if let Some(nav_mesh) = self.pool.try_borrow(selection.navmesh()) {
                nav_mesh.draw(drawing_context, Some(selection), settings.vertex_radius);
            }
        }
    }
}
