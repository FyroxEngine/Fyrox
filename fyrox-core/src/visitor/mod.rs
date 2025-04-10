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

//! Visitor is a tree-based serializer/deserializer.
//!
//! # Overview
//!
//! Visitor uses tree to create structured storage of data. Basic unit is a *node* - it is a container
//! for data fields. Each node has name, handle to parent, set of handles to children nodes and some
//! container for data fields. Data field is tuple of name and value, value can be any of simple Rust
//! types and some of basic structures of the crate. Main criteria of what could be the field and what
//! not is the ability to be represented as set of bytes without any aliasing issues.

pub mod blackboard;
pub mod error;
pub mod field;
mod impls;
pub mod pod;
mod reader;
mod writer;

pub use fyrox_core_derive::Visit;

pub mod prelude {
    //! Types to use `#[derive(Visit)]`
    pub use super::{Visit, VisitResult, Visitor};
    pub use crate::visitor::error::VisitError;
}

use crate::visitor::writer::AsciiWriter;
use crate::{
    array_as_u8_slice_mut,
    io::{self},
    pool::{Handle, Pool},
    visitor::{
        reader::{BinaryReader, Reader},
        writer::{BinaryWriter, Writer},
    },
};
use bitflags::bitflags;
use blackboard::Blackboard;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use error::VisitError;
use field::{Field, FieldKind};
use fxhash::FxHashMap;
use std::{
    any::Any,
    fs::File,
    hash::Hash,
    io::{BufWriter, Cursor, Read, Write},
    ops::{Deref, DerefMut},
    path::Path,
    rc::Rc,
    sync::Arc,
};

/// Proxy struct for plain data, we can't use `Vec<u8>` directly,
/// because it will serialize each byte as separate node.
/// BinaryBlob stores data very much like [PodVecView] except that BinaryBlob
/// has less type safety. In practice it is used with T = u8 for Strings and Paths,
/// but it accepts any type T that is Copy, and it lacks the type_id system that
/// PodVecView has for checking that the data it is reading comes from the expected type.
pub struct BinaryBlob<'a, T>
where
    T: Copy,
{
    pub vec: &'a mut Vec<T>,
}

impl<T> Visit for BinaryBlob<'_, T>
where
    T: Copy + bytemuck::Pod,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        if visitor.reading {
            if let Some(field) = visitor.find_field(name) {
                match &field.kind {
                    FieldKind::BinaryBlob(data) => {
                        let len = data.len() / size_of::<T>();
                        let mut vec = Vec::<T>::with_capacity(len);

                        unsafe {
                            std::ptr::copy_nonoverlapping(
                                data.as_ptr(),
                                array_as_u8_slice_mut(&mut vec).as_mut_ptr(),
                                data.len(),
                            );

                            vec.set_len(len);
                        }

                        *self.vec = vec;

                        Ok(())
                    }
                    _ => Err(VisitError::FieldTypeDoesNotMatch),
                }
            } else {
                Err(VisitError::FieldDoesNotExist(name.to_owned()))
            }
        } else if visitor.find_field(name).is_some() {
            Err(VisitError::FieldAlreadyExists(name.to_owned()))
        } else {
            let node = visitor.current_node();

            let len_bytes = self.vec.len() * std::mem::size_of::<T>();
            let mut bytes = Vec::<u8>::with_capacity(len_bytes);
            bytes.extend_from_slice(unsafe {
                std::slice::from_raw_parts(self.vec.as_ptr() as *const u8, len_bytes)
            });

            node.fields
                .push(Field::new(name, FieldKind::BinaryBlob(bytes)));

            Ok(())
        }
    }
}

/// The result of a [Visit::visit] or of a Visitor encoding operation
/// such as [Visitor::save_binary]. It has no value unless an error occurred.
pub type VisitResult = Result<(), VisitError>;

trait VisitableElementaryField {
    fn write(&self, file: &mut dyn Write) -> VisitResult;
    fn read(&mut self, file: &mut dyn Read) -> VisitResult;
}

macro_rules! impl_visitable_elementary_field {
    ($ty:ty, $write:ident, $read:ident $(, $endian:ident)*) => {
        impl VisitableElementaryField for $ty {
            fn write(&self, file: &mut dyn Write) -> VisitResult {
                file.$write::<$($endian)*>(*self)?;
                Ok(())
            }

            fn read(&mut self, file: &mut dyn Read) -> VisitResult {
                *self = file.$read::<$($endian)*>()?;
                Ok(())
            }
        }
    };
}
impl_visitable_elementary_field!(f64, write_f64, read_f64, LittleEndian);
impl_visitable_elementary_field!(f32, write_f32, read_f32, LittleEndian);
impl_visitable_elementary_field!(u8, write_u8, read_u8);
impl_visitable_elementary_field!(i8, write_i8, read_i8);
impl_visitable_elementary_field!(u16, write_u16, read_u16, LittleEndian);
impl_visitable_elementary_field!(i16, write_i16, read_i16, LittleEndian);
impl_visitable_elementary_field!(u32, write_u32, read_u32, LittleEndian);
impl_visitable_elementary_field!(i32, write_i32, read_i32, LittleEndian);
impl_visitable_elementary_field!(u64, write_u64, read_u64, LittleEndian);
impl_visitable_elementary_field!(i64, write_i64, read_i64, LittleEndian);

/// A node is a collection of [Fields](Field) that exists within a tree of nodes
/// that allows a [Visitor] to store its data.
/// Each node has a name, and may have a parent node and child nodes.
pub struct VisitorNode {
    name: String,
    fields: Vec<Field>,
    parent: Handle<VisitorNode>,
    children: Vec<Handle<VisitorNode>>,
}

impl VisitorNode {
    fn new(name: &str, parent: Handle<VisitorNode>) -> Self {
        Self {
            name: name.to_owned(),
            fields: Vec::new(),
            parent,
            children: Vec::new(),
        }
    }
}

impl Default for VisitorNode {
    fn default() -> Self {
        Self {
            name: String::new(),
            fields: Vec::new(),
            parent: Handle::NONE,
            children: Vec::new(),
        }
    }
}

/// A RegionGuard is a [Visitor] that automatically leaves the current region
/// when it is dropped.
#[must_use = "the guard must be used"]
pub struct RegionGuard<'a>(&'a mut Visitor);

impl Deref for RegionGuard<'_> {
    type Target = Visitor;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl DerefMut for RegionGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl Drop for RegionGuard<'_> {
    fn drop(&mut self) {
        // If we acquired RegionGuard instance, then it is safe to assert that
        // `leave_region` was successful.
        self.0.leave_region().unwrap();
    }
}

bitflags! {
    /// Flags that can be used to influence the behaviour of [Visit::visit] methods.
    pub struct VisitorFlags: u32 {
        /// No flags set, do nothing special.
        const NONE = 0;
        /// Tell [crate::variable::InheritableVariable::visit] to assume that it's
        /// [VariableFlags::MODIFIED](create::variable::VariableFlags::MODIFIED) is set,
        /// and therefore write its data. Otherwise, InheritableVariable has the special
        /// property of *not writing itself* when the `MODIFIED` flag is not set.
        const SERIALIZE_EVERYTHING = 1 << 1;
    }
}

/// A collection of nodes that stores data that can be read or write values of types with the [Visit] trait.
///
/// Instead of calling methods of the visitor in order to read or write the visitor's data, reading
/// and writing happens in the [Visit::visit] method of a variable that will either store the read value
/// or holds the value to be written.
///
/// For example, `x.visit("MyValue", &mut visitor)` will do one of:
///
/// 1. Take the value of `x` and store it in `visitor` under the name "MyValue", if `visitor.is_reading()` is false.
/// 2. Read a value named "MyValue" from `visitor` and store it in `x`, if `visitor.is_reading()` is true.
///
/// Whether the value of `x` gets written into `visitor` or overwitten with a value from `visitor` is determined
/// by whether [Visitor::is_reading()] returns true or false.
pub struct Visitor {
    nodes: Pool<VisitorNode>,
    rc_map: FxHashMap<u64, Rc<dyn Any>>,
    arc_map: FxHashMap<u64, Arc<dyn Any + Send + Sync>>,
    reading: bool,
    current_node: Handle<VisitorNode>,
    root: Handle<VisitorNode>,
    /// A place to store whatever objects may be needed to assist with reading and writing values.
    pub blackboard: Blackboard,
    /// Flags that can activate special behaviour in some Visit values, such as
    /// [crate::variable::InheritableVariable].
    pub flags: VisitorFlags,
}

/// Trait of types that can be read from a [Visitor] or written to a Visitor.
pub trait Visit {
    /// Read or write this value, depending on whether [Visitor::is_reading()] is true or false.
    ///
    /// # In Write Mode
    ///
    /// The given name is a key to identify where this value will be stored in the visitor.
    /// Whether this name indicates a field or a region is determined by the value being visited.
    /// No two regions can exist with the same name as children of a single node,
    /// and no two fields can exist with the same name within a single node,
    /// but a region may share the same name as a field. If a name clash occurs, then an error
    /// is returned. Otherwise the value is written into the Visitor data at the given name.
    ///
    /// # In Read Mode
    ///
    /// The given name is a key to identify where this value should be found the visitor.
    /// Whether the name indicates a field or a region is determined by the the value being visited.
    /// If the field or region is not found with the given name
    /// then an error is returned. Otherwise the value being visited will be mutated
    /// to match the data in the visitor.
    ///
    /// # Buiding a Complex Value out of Multiple Fields
    ///
    /// If representing this value requires more than one field,
    /// [Visitor::enter_region] can be used to create a new node within the
    /// visitor with the given name, and the fields of this value can then read from
    /// or write to that node using the returned Visitor without risk of name
    /// clashes.
    ///
    /// See the documentation for [the Visit derive macro](fyrox_core_derive::Visit) for examples of how to
    /// implement Visit for some simple types.
    ///
    /// # Abnormal Implementations
    ///
    /// Types with special needs may choose to read and write in unusual ways. In addition to choosing
    /// whether they will store their data in a region or a field, a value might choose to do neither.
    /// A value may also choose to attempt to read its data in multiple ways so as to remain
    /// backwards-compatible with older versions where the format of data storage may be different.
    ///
    /// See [crate::variable::InheritableVariable::visit] for an example of an abnormal implementation of Visit.
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult;
}

impl Default for Visitor {
    fn default() -> Self {
        Self::new()
    }
}

impl Visitor {
    /// Sequence of bytes that is automatically written at the start when a visitor
    /// is encoded into bytes. It is written by [Visitor::save_binary], [Visitor::save_binary_to_memory],
    /// and [Visitor::save_binary_to_vec].
    ///
    /// [Visitor::load_binary] will return an error if this sequence of bytes is not present at the beginning
    /// of the file, and [Visitor::load_from_memory] will return an error of these bytes are not at the beginning
    /// of the given slice.
    pub const MAGIC: &'static str = "RG3D";

    /// Creates a Visitor containing only a single node called "`__ROOT__`" which will be the
    /// current region of the visitor.
    pub fn new() -> Self {
        let mut nodes = Pool::new();
        let root = nodes.spawn(VisitorNode::new("__ROOT__", Handle::NONE));
        Self {
            nodes,
            rc_map: FxHashMap::default(),
            arc_map: FxHashMap::default(),
            reading: false,
            current_node: root,
            root,
            blackboard: Blackboard::new(),
            flags: VisitorFlags::NONE,
        }
    }

    fn find_field(&mut self, name: &str) -> Option<&mut Field> {
        self.nodes
            .borrow_mut(self.current_node)
            .fields
            .iter_mut()
            .find(|field| field.name == name)
    }

    /// True if this Visitor is changing the values that it visits.
    /// In other words `x.visit("MyValue", &mut visitor)` will result in `x` being mutated to match
    /// whatever value is stored in `visitor`.
    ///
    /// False if this visitor is copying and storing the values that it visits.
    /// In other words `x.visit("MyValue", &mut visitor)` will result in `x` being unchanged,
    /// but `visitor` will be mutated to store the value of `x` under the name "MyValue".
    pub fn is_reading(&self) -> bool {
        self.reading
    }

    fn current_node(&mut self) -> &mut VisitorNode {
        self.nodes.borrow_mut(self.current_node)
    }

    /// If [Visitor::is_reading], find a node with the given name that is a child
    /// of the current node, and return a Visitor for the found node. Return an error
    /// if no node with that name exists.
    ///
    /// If not reading, create a node with the given name as a chld of the current
    /// node, and return a visitor for the new node. Return an error if a node with
    /// that name already exists.
    pub fn enter_region(&mut self, name: &str) -> Result<RegionGuard, VisitError> {
        let node = self.nodes.borrow(self.current_node);
        if self.reading {
            let mut region = Handle::NONE;
            for child_handle in node.children.iter() {
                let child = self.nodes.borrow(*child_handle);
                if child.name == name {
                    region = *child_handle;
                    break;
                }
            }
            if region.is_some() {
                self.current_node = region;
                Ok(RegionGuard(self))
            } else {
                Err(VisitError::RegionDoesNotExist(name.to_owned()))
            }
        } else {
            // Make sure that node does not exists already.
            for child_handle in node.children.iter() {
                let child = self.nodes.borrow(*child_handle);
                if child.name == name {
                    return Err(VisitError::RegionAlreadyExists(name.to_owned()));
                }
            }

            let node_handle = self.nodes.spawn(VisitorNode::new(name, self.current_node));
            self.nodes
                .borrow_mut(self.current_node)
                .children
                .push(node_handle);
            self.current_node = node_handle;

            Ok(RegionGuard(self))
        }
    }

    /// The name of the current region.
    /// This should never be None if the Visitor is operating normally,
    /// because there should be no way to leave the initial `__ROOT__` region.
    pub fn current_region(&self) -> Option<&str> {
        self.nodes
            .try_borrow(self.current_node)
            .map(|n| n.name.as_str())
    }

    fn leave_region(&mut self) -> VisitResult {
        self.current_node = self.nodes.borrow(self.current_node).parent;
        if self.current_node.is_none() {
            Err(VisitError::NoActiveNode)
        } else {
            Ok(())
        }
    }

    /// Create a String containing all the data of this Visitor.
    /// The String is formatted to be human-readable with each node on its own line
    /// and tabs to indent child nodes.
    pub fn save_text(&self) -> String {
        let mut cursor = Cursor::<Vec<u8>>::default();
        self.save_ascii(&mut cursor).unwrap();
        String::from_utf8(cursor.into_inner()).unwrap()
    }

    pub fn save_ascii<W: Write>(&self, mut dest: W) -> VisitResult {
        let writer = AsciiWriter::default();
        writer.write(self, &mut dest)
    }

    /// Write the data of this Visitor to the given writer.
    /// Begin by writing [Visitor::MAGIC].
    pub fn save_binary_to_memory<W: Write>(&self, mut dest: W) -> VisitResult {
        let writer = BinaryWriter::default();
        writer.write(self, &mut dest)
    }

    /// Encode the data of this visitor into bytes and push the bytes
    /// into the given `Vec<u8>`.
    /// Begin by writing [Visitor::MAGIC].
    pub fn save_binary_to_vec(&self) -> Result<Vec<u8>, VisitError> {
        let mut writer = Cursor::new(Vec::new());
        self.save_binary_to_memory(&mut writer)?;
        Ok(writer.into_inner())
    }

    /// Create a file at the given path and write the data of this visitor
    /// into that file in a non-human-readable binary format so that the data
    /// can be reconstructed using [Visitor::load_binary].
    /// Begin by writing [Visitor::MAGIC].
    pub fn save_binary<P: AsRef<Path>>(&self, path: P) -> VisitResult {
        let writer = BufWriter::new(File::create(path)?);
        self.save_binary_to_memory(writer)
    }

    pub fn save_text_to_file<P: AsRef<Path>>(&self, path: P) -> VisitResult {
        let mut writer = BufWriter::new(File::create(path)?);
        let text = self.save_text();
        writer.write_all(text.as_bytes())?;
        Ok(())
    }

    /// Create a visitor by reading data from the file at the given path,
    /// assuming that the file was created using [Visitor::save_binary].
    /// Return a [VisitError::NotSupportedFormat] if [Visitor::MAGIC] is not the first bytes read from the file.
    pub async fn load_binary<P: AsRef<Path>>(path: P) -> Result<Self, VisitError> {
        Self::load_from_memory(&io::load_file(path).await?)
    }

    /// Create a visitor by decoding data from the given byte slice,
    /// assuming that the bytes are in the format that would be produced
    /// by [Visitor::save_binary_to_vec].
    /// Return a [VisitError::NotSupportedFormat] if [Visitor::MAGIC] is not the first bytes read from the slice.
    pub fn load_from_memory(data: &[u8]) -> Result<Self, VisitError> {
        let mut src = Cursor::new(data);
        let reader = BinaryReader::default();
        reader.read(&mut src)
    }
}

#[cfg(test)]
mod test {
    use crate::visitor::{BinaryBlob, Visit, VisitResult, Visitor};
    use std::{fs::File, io::Write, path::Path, rc::Rc};

    #[derive(Visit, Default)]
    pub struct Model {
        data: u64,
    }

    #[derive(Default)]
    pub struct Texture {
        data: Vec<u8>,
    }

    impl Visit for Texture {
        fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
            let mut region = visitor.enter_region(name)?;
            let mut proxy = BinaryBlob {
                vec: &mut self.data,
            };
            proxy.visit("Data", &mut region)?;
            Ok(())
        }
    }

    #[allow(dead_code)]
    #[derive(Visit)]
    pub enum ResourceKind {
        Unknown,
        Model(Model),
        Texture(Texture),
    }

    impl Default for ResourceKind {
        fn default() -> Self {
            Self::Unknown
        }
    }

    #[derive(Visit)]
    struct Resource {
        kind: ResourceKind,
        data: u16,
    }

    impl Resource {
        fn new(kind: ResourceKind) -> Self {
            Self { kind, data: 0 }
        }
    }

    impl Default for Resource {
        fn default() -> Self {
            Self {
                kind: ResourceKind::Unknown,
                data: 0,
            }
        }
    }

    #[derive(Default, Visit)]
    struct Foo {
        bar: u64,
        shared_resource: Option<Rc<Resource>>,
    }

    impl Foo {
        fn new(resource: Rc<Resource>) -> Self {
            Self {
                bar: 123,
                shared_resource: Some(resource),
            }
        }
    }

    #[test]
    fn visitor_test() {
        let path = Path::new("test.bin");

        // Save
        {
            let mut visitor = Visitor::new();
            let mut resource = Rc::new(Resource::new(ResourceKind::Model(Model { data: 555 })));
            resource.visit("SharedResource", &mut visitor).unwrap();

            let mut objects = vec![Foo::new(resource.clone()), Foo::new(resource)];

            objects.visit("Objects", &mut visitor).unwrap();

            visitor.save_binary(path).unwrap();
            if let Ok(mut file) = File::create(Path::new("test.txt")) {
                file.write_all(visitor.save_text().as_bytes()).unwrap();
            }
        }

        // Load
        {
            let mut visitor = futures::executor::block_on(Visitor::load_binary(path)).unwrap();
            let mut resource: Rc<Resource> = Rc::new(Default::default());
            resource.visit("SharedResource", &mut visitor).unwrap();

            let mut objects: Vec<Foo> = Vec::new();
            objects.visit("Objects", &mut visitor).unwrap();
        }
    }
}
