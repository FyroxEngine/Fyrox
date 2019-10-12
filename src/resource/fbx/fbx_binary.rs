use crate::{
    resource::{
        fbx::{FbxNode, Fbx, attribute::FbxAttribute, error::FbxError}
    },
};
use std::{
    io::{Read, Cursor, Seek, SeekFrom},
    path::Path,
    fs::File,
};
use byteorder::{LittleEndian, ReadBytesExt};
use rg3d_core::{
    pool::{Handle, Pool},
};

fn read_attrib<R>(type_code: u8, file: &mut R) -> Result<FbxAttribute, FbxError>
    where R: Read {
    match type_code {
        b'f' | b'F' => Ok(FbxAttribute::Float(file.read_f32::<LittleEndian>()?)),
        b'd' | b'D' => Ok(FbxAttribute::Double(file.read_f64::<LittleEndian>()?)),
        b'l' | b'L' => Ok(FbxAttribute::Long(file.read_i64::<LittleEndian>()?)),
        b'i' | b'I' => Ok(FbxAttribute::Integer(file.read_i32::<LittleEndian>()?)),
        b'Y' => Ok(FbxAttribute::Integer(i32::from(file.read_i16::<LittleEndian>()?))),
        b'b' | b'C' => Ok(FbxAttribute::Bool(file.read_u8()? != 0)),
        _ => Err(FbxError::UnknownAttributeType(type_code))
    }
}

fn read_array<R>(type_code: u8, file: &mut R) -> Result<Vec<FbxAttribute>, FbxError>
    where R: Read {
    let length = file.read_u32::<LittleEndian>()? as usize;
    let encoding = file.read_u32::<LittleEndian>()?;
    let compressed_length = file.read_u32::<LittleEndian>()? as usize;
    let mut array = Vec::new();

    if encoding == 0 {
        for _ in 0..length {
            array.push(read_attrib(type_code, file)?);
        }
    } else {
        let mut compressed = Vec::with_capacity(compressed_length);
        unsafe { compressed.set_len(compressed_length) };
        file.read_exact(compressed.as_mut_slice())?;
        let decompressed = inflate::inflate_bytes_zlib(&compressed)?;
        let mut cursor = Cursor::new(decompressed);
        for _ in 0..length {
            array.push(read_attrib(type_code, &mut cursor)?);
        }
    }

    Ok(array)
}

fn read_string<R>(file: &mut R) -> Result<FbxAttribute, FbxError> where R: Read {
    let length = file.read_u32::<LittleEndian>()? as usize;
    let mut raw_string = Vec::with_capacity(length);
    unsafe { raw_string.set_len(length); };
    file.read_exact(raw_string.as_mut_slice())?;
    // Find null terminator. It is required because for some reason some strings
    // have additional data after null terminator like this: Omni004\x0\x1Model, but
    // length still more than position of null terminator.
    if let Some(null_terminator_pos) = raw_string.iter().position(|c| *c == 0) {
        raw_string.truncate(null_terminator_pos);
    }
    let string = String::from_utf8(raw_string)?;
    Ok(FbxAttribute::String(string))
}

/// Read binary FBX DOM using this specification:
/// https://code.blender.org/2013/08/fbx-binary-file-format-specification/
/// In case of success returns Ok(valid_handle), in case if no more nodes
/// are present returns Ok(none_handle), in case of error returns some FbxError.
fn read_binary_node<R>(file: &mut R, pool: &mut Pool<FbxNode>) -> Result<Handle<FbxNode>, FbxError>
    where R: Read + Seek {
    let end_offset = u64::from(file.read_u32::<LittleEndian>()?);
    if end_offset == 0 {
        // Footer found. We're done.
        return Ok(Handle::NONE);
    }

    let num_attrib = file.read_u32::<LittleEndian>()? as usize;
    let _attrib_list_len = file.read_u32::<LittleEndian>()?;

    // Read name.
    let name_len = file.read_u8()? as usize;
    let mut raw_name = Vec::with_capacity(name_len);
    unsafe { raw_name.set_len(name_len) };
    file.read_exact(raw_name.as_mut_slice())?;

    let mut node = FbxNode::default();
    node.name = String::from_utf8(raw_name)?;
    let node_handle = pool.spawn(node);

    // Read attributes.
    for _ in 0..num_attrib {
        let type_code = file.read_u8()?;
        match type_code {
            b'C' | b'Y' | b'I' | b'F' | b'D' | b'L' => {
                let node = pool.borrow_mut(node_handle);
                node.attribs.push(read_attrib(type_code, file)?);
            }
            b'f' | b'd' | b'l' | b'i' | b'b' => {
                let mut a = FbxNode::default();
                a.name = String::from("a");
                a.attribs = read_array(type_code, file)?;
                a.parent = node_handle;
                let a_handle = pool.spawn(a);
                let node = pool.borrow_mut(node_handle);
                node.children.push(a_handle);
            }
            b'S' => pool.borrow_mut(node_handle).attribs.push(read_string(file)?),
            b'R' => {
                // Ignore Raw data
                let length = i64::from(file.read_u32::<LittleEndian>()?);
                file.seek(SeekFrom::Current(length))?;
            }
            _ => ()
        }
    }

    if file.seek(SeekFrom::Current(0))? < end_offset {
        let null_record_position = end_offset - 13;
        while file.seek(SeekFrom::Current(0))? < null_record_position {
            let child_handle = read_binary_node(file, pool)?;
            if child_handle.is_none() {
                return Ok(child_handle);
            }
            pool.borrow_mut(child_handle).parent = node_handle;
            pool.borrow_mut(node_handle).children.push(child_handle);
        }

        // Check if we have a null-record
        let mut null_record = [0; 13];
        file.read_exact(&mut null_record)?;
        if !null_record.iter().all(|i| *i == 0) {
            return Err(FbxError::InvalidNullRecord);
        }
    }

    Ok(node_handle)
}

pub fn is_binary(path: &Path) -> Result<bool, FbxError> {
    let mut file = File::open(path)?;
    let mut magic = [0; 18];
    file.read_exact(&mut magic)?;
    let fbx_magic = b"Kaydara FBX Binary";
    Ok(magic == *fbx_magic)
}

pub fn read_binary<R>(file: &mut R) -> Result<Fbx, FbxError>
    where R: Read + Seek {
    let total_length = file.seek(SeekFrom::End(0))?;
    file.seek(SeekFrom::Start(0))?;

    // Ignore all stuff until version.
    let mut temp = [0; 23];
    file.read_exact(&mut temp)?;

    // Verify version.
    let version = file.read_u32::<LittleEndian>()? as i32;
    if version < 7100 || version > 7400 {
        return Err(FbxError::UnsupportedVersion(version));
    }

    let mut nodes = Pool::new();
    let mut root = FbxNode::default();
    root.name = String::from("__ROOT__");
    let root_handle = nodes.spawn(root);

    // FBX document can have multiple root nodes, so we must read the file
    // until the end.
    while file.seek(SeekFrom::Current(0))? < total_length {
        let root_child = read_binary_node(file, &mut nodes)?;
        if root_child.is_none() {
            break;
        }
        nodes.borrow_mut(root_child).parent = root_handle;
        nodes.borrow_mut(root_handle).children.push(root_child);
    }

    Ok(Fbx {
        nodes,
        root: root_handle,
        index_to_component: Default::default(),
        component_pool: Pool::new(),
        components: Vec::new(),
    })
}