use crate::{
    renderer::surface::Surface,
    scene::base::{Base, AsBase},
    core::{
        visitor::{
            Visit,
            Visitor,
            VisitResult,
        },
        math::aabb::AxisAlignedBoundingBox,
    }
};
use std::cell::Cell;

#[derive(Clone)]
pub struct Mesh {
    base: Base,
    surfaces: Vec<Surface>,
    bounding_box: Cell<AxisAlignedBoundingBox>,
    dirty: Cell<bool>,
}

impl Default for Mesh {
    fn default() -> Mesh {
        Mesh {
            base: Default::default(),
            surfaces: Default::default(),
            bounding_box: Default::default(),
            dirty: Cell::new(true),
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
    pub fn get_surfaces_mut(&mut self) -> &mut [Surface] {
        &mut self.surfaces
    }

    #[inline]
    pub fn clear_surfaces(&mut self) {
        self.surfaces.clear()
    }

    #[inline]
    pub fn add_surface(&mut self, surface: Surface) {
        self.surfaces.push(surface);
    }

    /// Performs lazy bounding box evaluation.
    /// Bounding box presented in *local coordinates*
    pub fn get_bounding_box(&self) -> AxisAlignedBoundingBox {
        if self.dirty.get() {
            let mut bounding_box = AxisAlignedBoundingBox::default();
            for surface in self.surfaces.iter() {
                let data = surface.get_data();
                let data = data.lock().unwrap();
                for vertex in data.get_vertices() {
                    bounding_box.add_point(vertex.position);
                }
            }
            self.bounding_box.set(bounding_box);
        }
        self.bounding_box.get()
    }

    /// Calculate bounding box in *world coordinates*.
    /// This method is very heavy and not intended to use every frame!
    pub fn calculate_world_bounding_box(&self) -> AxisAlignedBoundingBox {
        let mut bounding_box = AxisAlignedBoundingBox::default();
        for surface in self.surfaces.iter() {
            let data = surface.get_data();
            let data = data.lock().unwrap();
            for vertex in data.get_vertices() {
                bounding_box.add_point(self.base.get_global_transform().transform_vector(vertex.position));
            }
        }
        bounding_box
    }
}
