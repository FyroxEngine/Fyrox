pub mod texture;
pub mod fbx;
pub mod model;
pub mod ttf;

use std::{
    path::*,
    any::{
        TypeId,
        Any,
    },
};
use crate::{
    resource::{
        texture::*,
        model::Model,
    },
    utils::visitor::{
        Visit,
        Visitor,
        VisitResult,
    },
};

pub enum ResourceKind {
    Unknown,
    Texture(Texture),
    Model(Model),
}

pub struct Resource {
    path: PathBuf,
    kind: ResourceKind,
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

    pub fn get_kind_id(&self) -> TypeId {
        match &self.kind {
            ResourceKind::Unknown => panic!("must not get here"),
            ResourceKind::Model(model) => model.type_id(),
            ResourceKind::Texture(texture) => texture.type_id()
        }
    }
}

impl Default for Resource {
    fn default() -> Self {
        Self {
            kind: ResourceKind::Unknown,
            path: PathBuf::new(),
        }
    }
}

impl Visit for Resource {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut kind: u8 = if visitor.is_reading() {
            0
        } else {
            match &mut self.kind {
                ResourceKind::Unknown => panic!("must not get here"),
                ResourceKind::Model(_) => 0,
                ResourceKind::Texture(_) => 1
            }
        };

        kind.visit("Kind", visitor)?;

        if visitor.is_reading() {
            self.kind = match kind {
                0 => ResourceKind::Model(Default::default()),
                1 => ResourceKind::Texture(Default::default()),
                _ => panic!("must not get here"),
            };
        }

        self.path.visit("Path", visitor)?;

        visitor.leave_region()
    }
}

impl Drop for Resource {
    fn drop(&mut self) {
        println!("Resource {:?} was destroyed!", self.path);
    }
}