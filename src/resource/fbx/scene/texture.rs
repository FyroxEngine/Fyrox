use crate::{
    core::pool::Handle,
    resource::fbx::document::{FbxNode, FbxNodeContainer},
};
use std::path::PathBuf;

pub struct FbxTexture {
    filename: PathBuf,
}

impl FbxTexture {
    pub(in crate::resource::fbx) fn read(
        texture_node_handle: Handle<FbxNode>,
        nodes: &FbxNodeContainer,
    ) -> Result<Self, String> {
        let mut texture = FbxTexture {
            filename: PathBuf::new(),
        };
        if let Ok(relative_file_name_node) =
            nodes.get_by_name(texture_node_handle, "RelativeFilename")
        {
            // Since most of FBX files were made on Windows in 3ds MAX or Maya, it contains
            // paths with double back slashes, we must fix this so this path can be used
            // on linux.
            let str_path = relative_file_name_node
                .get_attrib(0)?
                .as_string()
                .replace("\\", "/");
            texture.filename = PathBuf::from(str_path);
        }
        Ok(texture)
    }

    pub(in crate::resource::fbx) fn get_file_path(&self) -> &PathBuf {
        &self.filename
    }
}
