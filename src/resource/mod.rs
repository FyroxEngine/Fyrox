pub mod texture;
use crate::resource::texture::*;

pub enum ResourceKind {
    Base,
    Texture(Texture)
}

pub struct Resource {
    kind: ResourceKind
}

impl Resource {
    pub fn new(kind: ResourceKind) -> Resource {
        Resource {
            kind: kind,
        }
    }

    pub fn borrow_kind(&self) -> &ResourceKind {
        &self.kind
    }

    pub fn borrow_kind_mut(&mut self) -> &mut ResourceKind {
        &mut self.kind
    }
}