use crate::{
    utils::visitor::{Visit, Visitor, VisitResult},
    renderer::surface::Surface
};

pub struct Mesh {
    surfaces: Vec<Surface>,
}

impl Default for Mesh {
    fn default() -> Mesh {
        Mesh {
            surfaces: Vec::new()
        }
    }
}

impl Visit for Mesh {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;
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

impl Clone for Mesh {
    fn clone(&self) -> Self {
        Self {
            surfaces: self.surfaces.iter().map(|surf| surf.make_copy()).collect()
        }
    }
}