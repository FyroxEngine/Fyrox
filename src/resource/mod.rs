pub mod texture;
pub mod fbx;
pub mod model;

use std::path::*;
use crate::resource::texture::*;
use crate::resource::model::{Model};

pub enum ResourceKind {
    Base,
    Texture(Texture),
    Model(Model),
}

pub struct Resource {
    path: PathBuf,
    kind: ResourceKind
}

impl Resource {
    pub fn new(path: &Path, kind: ResourceKind) -> Resource {
        Resource {
            path: path.to_path_buf(),
            kind,
        }
    }

    #[inline]
    pub fn get_path(&self) -> &Path {
        self.path.as_path()
    }

    #[inline]
    pub fn borrow_kind(&self) -> &ResourceKind {
        &self.kind
    }

    #[inline]
    pub fn borrow_kind_mut(&mut self) -> &mut ResourceKind {
        &mut self.kind
    }
}

pub trait ResourceBehavior {
    fn load(&self);
}