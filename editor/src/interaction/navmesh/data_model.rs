use fyrox::core::math::TriangleEdge;

#[derive(PartialEq, Clone, Debug, Eq)]
pub enum NavmeshEntity {
    Vertex(usize),
    Edge(TriangleEdge),
}
