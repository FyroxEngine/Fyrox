use rg3d_core::{
    math::vec3::Vec3,
    pool::Handle,
    visitor::{Visit, VisitResult, Visitor}
};
use crate::{
    rigid_body::RigidBody,
    static_geometry::StaticGeometry
};

#[derive(Debug, Clone)]
pub struct Contact {
    pub body: Handle<RigidBody>,
    pub position: Vec3,
    pub normal: Vec3,
    pub triangle_index: u32,
    pub static_geom: Handle<StaticGeometry>
}

impl Default for Contact {
    fn default() -> Self {
        Self {
            body: Handle::NONE,
            position: Vec3::ZERO,
            normal: Vec3::new(0.0, 1.0, 0.0),
            triangle_index: 0,
            static_geom: Handle::NONE,
        }
    }
}

impl Visit for Contact {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.body.visit("Body", visitor)?;
        self.position.visit("Position", visitor)?;
        self.normal.visit("Normal", visitor)?;
        self.triangle_index.visit("TriangleIndex", visitor)?;

        visitor.leave_region()
    }
}