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

use crate::core::byteorder::ReadBytesExt;
use crate::{
    core::pool::{Handle, Pool},
    resource::fbx::{
        document::{attribute::FbxAttribute, FbxDocument, FbxNode, FbxNodeContainer},
        error::FbxError,
    },
};
use std::io::{Read, Seek, SeekFrom};

pub fn read_ascii<R>(reader: &mut R) -> Result<FbxDocument, FbxError>
where
    R: Read + Seek,
{
    let mut nodes: Pool<FbxNode> = Pool::new();
    let root_handle = nodes.spawn(FbxNode {
        name: String::from("__ROOT__"),
        children: Vec::new(),
        parent: Handle::NONE,
        attributes: Vec::new(),
    });
    let mut parent_handle: Handle<FbxNode> = root_handle;
    let mut node_handle: Handle<FbxNode> = Handle::NONE;
    let mut buffer: Vec<u8> = Vec::new();
    let mut name: Vec<u8> = Vec::new();
    let mut value: Vec<u8> = Vec::new();

    let buf_len = reader.seek(SeekFrom::End(0))?;
    reader.rewind()?;

    // Read line by line
    while reader.stream_position()? < buf_len {
        // Read line, trim spaces (but leave spaces in quotes)
        buffer.clear();

        let mut read_all = false;
        while reader.stream_position()? < buf_len {
            let symbol = reader.read_u8()?;
            if symbol == b'\n' {
                break;
            } else if symbol == b'"' {
                read_all = !read_all;
            } else if read_all || !symbol.is_ascii_whitespace() {
                buffer.push(symbol);
            }
        }

        // Ignore comments and empty lines
        if buffer.is_empty() || buffer[0] == b';' {
            continue;
        }

        // Parse string
        let mut read_value = false;
        name.clear();
        for (i, symbol) in buffer.iter().enumerate() {
            let symbol = *symbol;
            if i == 0 && (symbol == b'-' || symbol.is_ascii_digit()) {
                read_value = true;
            }
            if symbol == b':' && !read_value {
                read_value = true;
                let name_copy = String::from_utf8(name.clone())?;
                let node = FbxNode {
                    name: name_copy,
                    attributes: Vec::new(),
                    parent: parent_handle,
                    children: Vec::new(),
                };
                node_handle = nodes.spawn(node);
                name.clear();
                let parent = nodes.borrow_mut(parent_handle);
                parent.children.push(node_handle);
            } else if symbol == b'{' {
                // Enter child scope
                parent_handle = node_handle;
                // Commit attribute if we have one
                if !value.is_empty() {
                    let node = nodes.borrow_mut(node_handle);
                    let string_value = String::from_utf8(value.clone())?;
                    let attrib = FbxAttribute::String(string_value);
                    node.attributes.push(attrib);
                    value.clear();
                }
            } else if symbol == b'}' {
                // Exit child scope
                let parent = nodes.borrow_mut(parent_handle);
                parent_handle = parent.parent;
            } else if symbol == b',' || (i == buffer.len() - 1) {
                // Commit attribute
                if symbol != b',' {
                    value.push(symbol);
                }
                let node = nodes.borrow_mut(node_handle);
                let string_value = String::from_utf8(value.clone())?;
                let attrib = FbxAttribute::String(string_value);
                node.attributes.push(attrib);
                value.clear();
            } else if !read_value {
                name.push(symbol);
            } else {
                value.push(symbol);
            }
        }
    }

    Ok(FbxDocument {
        nodes: FbxNodeContainer { nodes },
        root: root_handle,
    })
}
