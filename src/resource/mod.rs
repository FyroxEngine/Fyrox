pub mod texture;
pub mod fbx;
use std::path::*;
use crate::resource::texture::*;

pub enum ResourceKind {
    Base,
    Texture(Texture)
}

pub struct Resource {
    pub(crate) path: PathBuf,
    kind: ResourceKind
}

impl Resource {
    pub fn new(path: &Path, kind: ResourceKind) -> Resource {
        Resource {
            path: path.to_path_buf(),
            kind,
        }
    }

    pub fn borrow_kind(&self) -> &ResourceKind {
        &self.kind
    }

    pub fn borrow_kind_mut(&mut self) -> &mut ResourceKind {
        &mut self.kind
    }
}