mod ascii;
pub mod attribute;
mod binary;

use fyrox_resource::io::ResourceIo;

use crate::{
    core::{
        algebra::Vector3,
        pool::{Handle, Pool},
    },
    resource::fbx::{document::attribute::FbxAttribute, error::FbxError},
};
use std::{io::Cursor, path::Path};

pub struct FbxNode {
    name: String,
    attributes: Vec<FbxAttribute>,
    parent: Handle<FbxNode>,
    children: Vec<Handle<FbxNode>>,
}

impl Default for FbxNode {
    fn default() -> Self {
        Self {
            name: "".to_string(),
            attributes: Vec::new(),
            parent: Default::default(),
            children: Vec::new(),
        }
    }
}

impl FbxNode {
    pub fn get_vec3_at(&self, n: usize) -> Result<Vector3<f32>, String> {
        Ok(Vector3::new(
            self.get_attrib(n)?.as_f32()?,
            self.get_attrib(n + 1)?.as_f32()?,
            self.get_attrib(n + 2)?.as_f32()?,
        ))
    }

    pub fn get_attrib(&self, n: usize) -> Result<&FbxAttribute, String> {
        match self.attributes.get(n) {
            Some(attrib) => Ok(attrib),
            None => Err(format!(
                "Unable to get {} attribute because index out of bounds.",
                n
            )),
        }
    }

    pub fn attrib_count(&self) -> usize {
        self.attributes.len()
    }

    pub fn attributes(&self) -> &[FbxAttribute] {
        &self.attributes
    }

    pub fn children(&self) -> &[Handle<FbxNode>] {
        &self.children
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

pub struct FbxNodeContainer {
    nodes: Pool<FbxNode>,
}

impl FbxNodeContainer {
    /// Searches node by specified name and returns its handle if found
    pub fn find(&self, root: Handle<FbxNode>, name: &str) -> Result<Handle<FbxNode>, String> {
        let node = self.nodes.borrow(root);

        if node.name == name {
            return Ok(root);
        }

        for child_handle in node.children.iter() {
            if let Ok(result) = self.find(*child_handle, name) {
                return Ok(result);
            }
        }

        Err(format!("FBX DOM: Unable to find {} node", name))
    }

    /// Searches node by specified name and borrows a reference to it
    pub fn get_by_name(&self, root: Handle<FbxNode>, name: &str) -> Result<&'_ FbxNode, String> {
        let node = self.nodes.borrow(root);

        if node.name == name {
            return Ok(node);
        }

        for child_handle in node.children.iter() {
            if let Ok(result) = self.get_by_name(*child_handle, name) {
                return Ok(result);
            }
        }

        Err(format!("FBX DOM: Unable to find {} node", name))
    }

    pub fn get(&self, handle: Handle<FbxNode>) -> &FbxNode {
        self.nodes.borrow(handle)
    }
}

pub struct FbxDocument {
    root: Handle<FbxNode>,
    nodes: FbxNodeContainer,
}

fn is_binary(data: &[u8]) -> bool {
    let fbx_magic = b"Kaydara FBX Binary";
    &data[0..18] == fbx_magic
}

impl FbxDocument {
    pub async fn new<P: AsRef<Path>>(
        path: P,
        io: &dyn ResourceIo,
    ) -> Result<FbxDocument, FbxError> {
        let data = io.load_file(path.as_ref()).await?;
        let is_bin = is_binary(&data);
        let mut reader = Cursor::new(data);

        if is_bin {
            binary::read_binary(&mut reader)
        } else {
            ascii::read_ascii(&mut reader)
        }
    }

    pub fn root(&self) -> Handle<FbxNode> {
        self.root
    }

    pub fn nodes(&self) -> &FbxNodeContainer {
        &self.nodes
    }
}
