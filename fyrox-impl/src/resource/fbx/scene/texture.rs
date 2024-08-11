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
