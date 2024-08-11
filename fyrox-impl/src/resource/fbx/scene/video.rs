// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

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
