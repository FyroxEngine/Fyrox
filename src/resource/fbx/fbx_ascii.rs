use std::{
    io::{Seek, Read, SeekFrom},
    collections::HashMap,
};
use byteorder::ReadBytesExt;
use crate::{
    resource::{
        fbx::{FbxNode, Fbx},
        fbx::attribute::FbxAttribute,
        fbx::error::FbxError,
    },
};
use crate::core::{
    pool::{Pool, Handle},
};

pub fn read_ascii<R>(reader: &mut R, buf_len: u64) -> Result<Fbx, FbxError>
    where R: Read + Seek {
    let mut nodes: Pool<FbxNode> = Pool::new();
    let root_handle = nodes.spawn(FbxNode {
        name: String::from("__ROOT__"),
        children: Vec::new(),
        parent: Handle::NONE,
        attribs: Vec::new(),
    });
    let mut parent_handle: Handle<FbxNode> = root_handle;
    let mut node_handle: Handle<FbxNode> = Handle::NONE;
    let mut buffer: Vec<u8> = Vec::new();
    let mut name: Vec<u8> = Vec::new();
    let mut value: Vec<u8> = Vec::new();

    // Read line by line
    while reader.seek(SeekFrom::Current(0))? < buf_len {
        // Read line, trim spaces (but leave spaces in quotes)
        buffer.clear();

        let mut read_all = false;
        while reader.seek(SeekFrom::Current(0))? < buf_len {
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
        for i in 0..buffer.len() {
            let symbol = unsafe { *buffer.get_unchecked(i as usize) };
            if i == 0 && (symbol == b'-' || symbol.is_ascii_digit()) {
                read_value = true;
            }
            if symbol == b':' && !read_value {
                read_value = true;
                let name_copy = String::from_utf8(name.clone())?;
                let node = FbxNode {
                    name: name_copy,
                    attribs: Vec::new(),
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
                    node.attribs.push(attrib);
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
                node.attribs.push(attrib);
                value.clear();
            } else if !read_value {
                name.push(symbol);
            } else {
                value.push(symbol);
            }
        }
    }

    Ok(Fbx {
        nodes,
        root: root_handle,
        component_pool: Pool::new(),
        components: Vec::new(),
        index_to_component: HashMap::new(),
    })
}
