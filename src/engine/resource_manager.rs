use std::{
    cell::RefCell,
    rc::Rc,
    path::{PathBuf, Path},
};
use crate::{
    resource::{
        texture::Texture,
        Resource,
        ResourceKind,
    },
};
use rg3d_core::{
    visitor::{Visitor, VisitResult, Visit}
};

pub struct ResourceManager {
    resources: Vec<Rc<RefCell<Resource>>>,
    /// Path to textures, extensively used for resource files
    /// which stores path in weird format (either relative or absolute) which
    /// is obviously not good for engine.
    textures_path: PathBuf,
}

impl ResourceManager {
    pub(in crate::engine) fn new() -> ResourceManager {
        Self {
            resources: Vec::new(),
            textures_path: PathBuf::from("data/textures/"),
        }
    }

    #[inline]
    pub fn for_each_texture_mut<Func>(&self, mut func: Func) where Func: FnMut(&mut Texture) {
        for resource in self.resources.iter() {
            if let ResourceKind::Texture(texture) = resource.borrow_mut().borrow_kind_mut() {
                func(texture);
            }
        }
    }

    #[inline]
    pub fn add_resource(&mut self, resource: Rc<RefCell<Resource>>) {
        self.resources.push(resource)
    }

    /// Searches for a resource of specified path, if found - returns handle to resource
    /// and increases reference count of resource.
    #[inline]
    pub fn find_resource(&mut self, path: &Path) -> Option<Rc<RefCell<Resource>>> {
        for resource in self.resources.iter() {
            if resource.borrow().get_path() == path {
                return Some(resource.clone());
            }
        }
        None
    }

    pub fn get_resources(&self) -> &[Rc<RefCell<Resource>>] {
        &self.resources
    }

    #[inline]
    pub fn get_textures_path(&self) -> &Path {
        self.textures_path.as_path()
    }

    pub fn update(&mut self) {
        self.resources.retain(|resource| {
            Rc::strong_count(resource) > 1
        })
    }
}

impl Visit for ResourceManager {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.resources.visit("Resources", visitor)?;

        visitor.leave_region()
    }
}