pub mod astar;

use std::marker::PhantomData;
use crate::{
    scene::mesh::Mesh,
    scene::base::AsBase
};
use crate::physics::static_geometry::{StaticGeometry, StaticTriangle};

pub struct UnsafeCollectionView<T> {
    items: *const T,
    len: usize,
}

impl<T> UnsafeCollectionView<T> {
    pub fn empty() -> UnsafeCollectionView<T> {
        UnsafeCollectionView {
            items: std::ptr::null(),
            len: 0,
        }
    }

    pub fn from_slice(vec: &[T]) -> UnsafeCollectionView<T> {
        UnsafeCollectionView {
            items: vec.as_ptr(),
            len: vec.len(),
        }
    }

    pub fn iter(&self) -> UnsafeCollectionViewIterator<T> {
        unsafe {
            UnsafeCollectionViewIterator {
                current: self.items,
                end: self.items.add(self.len),
                marker: PhantomData,
            }
        }
    }
}

pub struct UnsafeCollectionViewIterator<'a, T> {
    current: *const T,
    end: *const T,
    marker: PhantomData<&'a T>,
}

impl<'a, T> Iterator for UnsafeCollectionViewIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        unsafe {
            if self.current != self.end {
                let value = self.current;
                self.current = self.current.offset(1);
                Some(&*value)
            } else {
                None
            }
        }
    }
}

/// Small helper that creates static physics geometry from given mesh.
///
/// # Notes
///
/// This method *bakes* global transform of given mesh into static geometry
/// data. So if given mesh was at some position with any rotation and scale
/// resulting static geometry will have vertices that exactly matches given
/// mesh.
pub fn mesh_to_static_geometry(mesh: &Mesh) -> StaticGeometry {
    let mut triangles = Vec::new();
    let global_transform = mesh.base().get_global_transform();
    for surface in mesh.get_surfaces() {
        let data_rc = surface.get_data();
        let shared_data = data_rc.lock().unwrap();

        let vertices = shared_data.get_vertices();
        let indices = shared_data.get_indices();

        let last = indices.len() - indices.len() % 3;
        let mut i: usize = 0;
        while i < last {
            let a = global_transform.transform_vector(vertices[indices[i] as usize].position);
            let b = global_transform.transform_vector(vertices[indices[i + 1] as usize].position);
            let c = global_transform.transform_vector(vertices[indices[i + 2] as usize].position);

            if let Some(triangle) = StaticTriangle::from_points(&a, &b, &c) {
                triangles.push(triangle);
            } else {
                println!("degenerated triangle!");
            }

            i += 3;
        }
    }
    StaticGeometry::new(triangles)
}