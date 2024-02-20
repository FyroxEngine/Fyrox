use crate::{
    core::pool::Handle,
    resource::fbx::document::{attribute::FbxAttribute, FbxNode, FbxNodeContainer},
};
use base64::Engine;

pub struct FbxVideo {
    pub content: Vec<u8>,
}

impl FbxVideo {
    pub(in crate::resource::fbx) fn read(
        video_node_handle: Handle<FbxNode>,
        nodes: &FbxNodeContainer,
    ) -> Result<Self, String> {
        if let Ok(content_node) = nodes.get_by_name(video_node_handle, "Content") {
            let attrib = content_node.get_attrib(0)?;

            let content = match attrib {
                FbxAttribute::String(base64) => base64::engine::general_purpose::STANDARD
                    .decode(base64)
                    .expect("FBX cannot contain invalid base64-encoded data."),
                FbxAttribute::RawData(raw) => raw.clone(),
                _ => Default::default(),
            };

            Ok(Self { content })
        } else {
            Ok(Self {
                content: Default::default(),
            })
        }
    }
}
