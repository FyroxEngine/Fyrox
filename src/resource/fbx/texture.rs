use std::path::{
    PathBuf,
    Path,
};
use crate::{
    utils::{
        pool::Handle,
        pool::Pool,
    },
    resource::fbx::{
        FbxNode,
    },
};
use crate::resource::fbx::find_and_borrow_node;

pub struct FbxTexture {
    filename: PathBuf,
}

impl FbxTexture {
    pub(in crate::resource::fbx) fn read(texture_node_hanle: Handle<FbxNode>, nodes: &Pool<FbxNode>) -> Result<Self, String> {
        let mut texture = FbxTexture {
            filename: PathBuf::new()
        };
        if let Ok(relative_file_name_node) = find_and_borrow_node(nodes, texture_node_hanle, "RelativeFilename") {
            let relative_filename = relative_file_name_node.get_attrib(0)?.as_string();
            let path = Path::new(relative_filename.as_str());
            if let Some(filename) = path.file_name() {
                texture.filename = PathBuf::from(filename);
            }
        }
        Ok(texture)
    }

    pub(in crate::resource::fbx) fn get_file_path(&self) -> &PathBuf {
        &self.filename
    }
}