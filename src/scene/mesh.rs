use crate::{
    renderer::surface::Surface,
    scene::base::{Base, AsBase}
};
use rg3d_core::visitor::{
    Visit,
    Visitor,
    VisitResult
};

#[derive(Clone)]
pub struct Mesh {
    base: Base,
    surfaces: Vec<Surface>,
}

impl Default for Mesh {
    fn default() -> Mesh {
        Mesh {
            base: Default::default(),
            surfaces: Vec::new()
        }
    }
}

impl AsBase for Mesh {
    fn base(&self) -> &Base {
        &self.base
    }

    fn base_mut(&mut self) -> &mut Base {
        &mut self.base
    }
}

impl Visit for Mesh {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.base.visit("Common", visitor)?;

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
