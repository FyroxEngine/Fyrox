use crate::{
    core::pool::{Handle, Pool},
    resource::fbx::{
        document::{FbxNode, FbxNodeContainer},
        scene::FbxComponent,
    },
};
use std::path::PathBuf;

#[derive(Debug)]
pub struct FbxTexture {
    pub filename: PathBuf,
    pub content: Vec<u8>,
    pub ancestor: Handle<FbxComponent>,
}

impl FbxTexture {
    pub(in crate::resource::fbx) fn read(
        texture_node_handle: Handle<FbxNode>,
        nodes: &FbxNodeContainer,
    ) -> Result<Self, String> {
        let mut texture = FbxTexture {
            filename: PathBuf::new(),
            content: Default::default(),
            ancestor: Default::default(),
        };
        if let Ok(relative_file_name_node) =
            nodes.get_by_name(texture_node_handle, "RelativeFilename")
        {
            // Since most of FBX files were made on Windows in 3ds MAX or Maya, it contains
            // paths with double back slashes, we must fix this so this path can be used
            // on linux.
            if let Ok(attrib) = relative_file_name_node.get_attrib(0) {
                let str_path = attrib.as_string().replace('\\', "/");
                texture.filename = PathBuf::from(str_path);
            }
        }
        Ok(texture)
    }

    /// Tries to resolve the entire chain of material nodes and find texture path.
    pub(in crate::resource::fbx) fn get_root_file_path(
        &self,
        components: &Pool<FbxComponent>,
    ) -> PathBuf {
        if self.filename == PathBuf::default() {
            components
                .try_borrow(self.ancestor)
                .and_then(|parent| parent.as_texture().ok())
                .map(|texture| texture.get_root_file_path(components))
                .unwrap_or_default()
        } else {
            self.filename.clone()
        }
    }
}
