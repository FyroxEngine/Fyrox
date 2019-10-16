use crate::{
    renderer::surface::Surface,
    scene::node::CommonNodeData
};
use rg3d_core::visitor::{
    Visit,
    Visitor,
    VisitResult
};

pub struct Mesh {
    common: CommonNodeData,
    surfaces: Vec<Surface>,
}

impl Default for Mesh {
    fn default() -> Mesh {
        Mesh {
            common: Default::default(),
            surfaces: Vec::new()
        }
    }
}

impl Visit for Mesh {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.common.visit("Common", visitor)?;

        // No need to serialize surfaces, correct ones will be assigned on resolve stage.
        visitor.leave_region()
    }
}

impl Mesh {
    #[inline]
    pub fn get_surfaces(&self) -> &Vec<Surface> {
        &self.surfaces
    }

    #[inline]
    pub fn get_surfaces_mut(&mut self) -> &mut Vec<Surface> {
        &mut self.surfaces
    }

    #[inline]
    pub fn add_surface(&mut self, surface: Surface) {
        self.surfaces.push(surface);
    }
}

impl_node_trait!(Mesh);
impl_node_trait_private!(Mesh);

impl Clone for Mesh {
    fn clone(&self) -> Self {
        Self {
            common: self.common.clone(),
            surfaces: self.surfaces.iter().map(|surf| surf.make_copy()).collect()
        }
    }
}