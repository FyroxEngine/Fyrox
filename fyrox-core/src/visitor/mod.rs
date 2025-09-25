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

//! Visitor is a tree-based serializer/deserializer with intermediate representation for stored data.
//! When data is serialized, it will be transformed into an intermediate representation and only then
//! will be dumped onto the disk. Deserialization is the same: the data (binary or text) is read
//! and converted into an intermediate representation (IR). End users can use this IR to save or load
//! their structures of pretty much any complexity.
//!
//! # Overview
//!
//! Visitor uses a tree to create structured data storage. Basic unit is a *node* - it is a container
//! for data fields. Each node has a name, handle to parent, set of handles to children nodes and a
//! container for data fields. Data field is a pair of a name and a value, the value can be any of
//! simple Rust types and some of the trivially copyable data structures (vectors, matrices, etc.).
//! The main criteria of what could be the field and what not is the ability to be represented as
//! a set of bytes.
//!
//! See [`Visitor`] docs for more info.

#![warn(missing_docs)]

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

use crate::{
    array_as_u8_slice_mut,
    io::{self},
    pool::{Handle, Pool},
    visitor::{
        reader::{ascii::AsciiReader, binary::BinaryReader, Reader},
        writer::{ascii::AsciiWriter, binary::BinaryWriter, Writer},
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
    fmt::{Debug, Formatter},
    fs::File,
    hash::Hash,
    io::{BufWriter, Cursor, Read, Write},
    ops::{Deref, DerefMut},
    path::Path,
    rc::Rc,
    sync::Arc,
};

/// Version of the visitor.
#[repr(u32)]
pub enum VisitorVersion {
    /// Version of the old format.
    Legacy = 0,
    /// Flattened vector structure.
    VectorFlattening,
    /// Removal of `[N:` and `{N:` counters in ascii format.
    AsciiNoCounters,

    /// ^^ Add a new version above this line ^^.
    ///
    /// There are few major rules:
    ///
    /// 1) New version name should be something like `VectorFlattening`, and clearly describe the
    /// changes introduced by the new version. Always add a doc comment that contains a clear
    /// description of what was changed.
    /// 2) Do **NOT** add explicit number value for a new version. The compiler will do that for you,
    /// and there will be no chance of mistake. `Legacy` variant is an exception.
    /// 3) Existing version entries must **NOT** be deleted or moved.
    /// 4) `Last` variant must always be the last.
    Last,
}

/// Current version number of the visitor.
pub const CURRENT_VERSION: u32 = (VisitorVersion::Last as u32).saturating_sub(1);

/// Proxy struct for plain data. It is used to serialize arrays of trivially copyable data (`Vec<u8>`)
/// directly as a large chunk of data. For example, an attempt to serialize `Vec<u8>` serialize each
/// byte as a separate node which is very inefficient.
///
/// BinaryBlob stores data very much like [`crate::visitor::pod::PodVecView`] except that BinaryBlob
/// has less type safety. In practice, it is used with `T` = `u8` for Strings and Paths. However,
/// it accepts any type T that is Copy, and it lacks the type_id system that PodVecView has for
/// checking that the data it is reading comes from the expected type.
pub struct BinaryBlob<'a, T>
where
    T: Copy,
{
    /// A reference to a vector that represents a binary blob.
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
                    _ => Err(VisitError::FieldTypeDoesNotMatch {
                        expected: stringify!(FieldKind::BinaryBlob),
                        actual: format!("{:?}", field.kind),
                    }),
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

/// The result of a [Visit::visit] or of a Visitor encoding operation such as [Visitor::save_binary_to_file].
/// It has no value unless an error occurred.
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

/// A node is a collection of [Fields](Field) that exists within a tree of nodes that allows a
/// [Visitor] to store its data. Each node has a name, and may have a parent node and child nodes.
/// A node is used when visiting complex data, that cannot be represented by a simple memory block.
#[derive(Debug)]
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

/// A RegionGuard is a [Visitor] wrapper that automatically leaves the current region when it is
/// dropped.
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
    /// Flags that can be used to influence the behavior of [Visit::visit] methods.
    #[derive(Debug)]
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

/// Visitor is a tree-based serializer/deserializer with intermediate representation for stored data.
/// When data is serialized, it will be transformed into an intermediate representation and only then
/// will be dumped onto the disk. Deserialization is the same: the data (binary or text) is read
/// and converted into an intermediate representation (IR). End users can use this IR to save or load
/// their structures of pretty much any complexity.
///
/// # Overview
///
/// Visitor uses a tree to create structured data storage. Basic unit is a *node* - it is a container
/// for data fields. Each node has a name, handle to parent, set of handles to children nodes and a
/// container for data fields. Data field is a pair of a name and a value, the value can be any of
/// simple Rust types and some of the trivially copyable data structures (vectors, matrices, etc.).
/// The main criteria of what could be the field and what not is the ability to be represented as
/// a set of bytes.
///
/// Instead of calling visitor methods to read or write the visitor's data, reading and writing
/// happen in the [Visit::visit] method of a variable that will either store the read value
/// or holds the value to be written.
///
/// For example, `x.visit("MyValue", &mut visitor)` will do one of:
///
/// 1. Take the value of `x` and store it in `visitor` under the name "MyValue", if `visitor.is_reading()` is false.
/// 2. Read a value named "MyValue" from `visitor` and store it in `x`, if `visitor.is_reading()` is true.
///
/// Whether the value of `x` gets written into `visitor` or overwritten with a value from `visitor` is determined
/// by whether [Visitor::is_reading()] returns true or false.
pub struct Visitor {
    nodes: Pool<VisitorNode>,
    unique_id_counter: u64,
    rc_map: FxHashMap<u64, Rc<dyn Any>>,
    arc_map: FxHashMap<u64, Arc<dyn Any + Send + Sync>>,
    reading: bool,
    current_node: Handle<VisitorNode>,
    root: Handle<VisitorNode>,
    version: u32,
    /// A place to store whatever objects may be needed to help with reading and writing values.
    pub blackboard: Blackboard,
    /// Flags that can activate special behavior in some Visit values, such as
    /// [crate::variable::InheritableVariable].
    pub flags: VisitorFlags,
}

impl Debug for Visitor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut output = f.debug_struct("Visitor");

        output.field("flags", &self.flags);

        for (i, node) in self.nodes.iter().enumerate() {
            output.field(&format!("node{i}"), node);
        }

        output.finish()
    }
}

mod kek {
    use crate::visitor::prelude::*;

    struct MyType {
        field_a: u32,
        field_b: String,
    }

    impl Visit for MyType {
        fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
            let mut region = visitor.enter_region(name)?;

            self.field_a.visit("FieldA", &mut region)?;
            self.field_b.visit("FieldB", &mut region)?;

            Ok(())
        }
    }
}

/// Trait of types that can be read from a [Visitor] or written to a Visitor.
///
/// ## Code Generation
///
/// Procedural macro could be used to generate trivial implementations for this trait, which covers
/// 99% of the cases. Consider the following example:
///
/// ```rust
/// use fyrox_core::visitor::prelude::*;
///
/// #[derive(Visit, Default)]
/// struct MyType {
///     field_a: u32,
///     field_b: String
/// }
/// ```
///
/// The generated code will be something like this:
///
/// ```rust
/// use crate::visitor::prelude::*;
///
/// struct MyType {
///     field_a: u32,
///     field_b: String
/// }
///
/// impl Visit for MyType {
///     fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
///         let mut region = visitor.enter_region(name)?;
///
///         self.field_a.visit("FieldA", &mut region)?;
///         self.field_b.visit("FieldB", &mut region)?;
///
///         Ok(())
///     }
/// }
/// ```
///
/// ### Type Attributes
///
/// - `#[visit(optional)]` - marks all the fields of the type as optional and suppresses any errors
/// on serialization and deserialization. In the generated code, all the fields will be visited like
/// this `let _ = self.field_a.visit("FieldA", &mut region);`
/// - `#[visit(pre_visit_method = "function_name")]` - name of a function, that will be called
/// before the generated body.
/// - `#[visit(post_visit_method = "function_name")]` - name of a function, that will be called
/// after the generated body.
///
/// ### Field Attributes
///
/// - `#[visit(skip)]` - disables serialization and deserialization of the field.
/// - `#[visit(rename = "new_name")]` - overrides the name of the field with `new_name`. In the
/// generated code, all the fields will be visited like this `self.field_a.visit("new_name", &mut region)?;`
/// - `#[visit(optional)]` - marks the field as optional and suppresses any errors on serialization
/// and deserialization. In the generated code, all the fields will be visited like this
/// `let _ = self.field_a.visit("FieldA", &mut region);`
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

/// Format of the data that be used by a [`Visitor`] instance for reading or writing from/to an
/// external storage.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
#[must_use]
pub enum Format {
    /// The format is unknown and unsupported.
    Unknown,
    /// Binary format. The fastest and smallest, but changes cannot be merged by a version control
    /// system, thus not suitable for collaborative work. It should be used primarily for production
    /// builds.
    Binary,
    /// Slow and "fat" format, but changes can be merged by a version control system. It makes this
    /// format ideal for collaborative work.
    Ascii,
}

impl Visitor {
    /// Old header marker for binary version.
    pub const MAGIC_BINARY_OLD: &'static str = "RG3D";

    /// Sequence of bytes that is automatically written at the start when a visitor is encoded into
    /// bytes. It is written by [Visitor::save_binary_to_file], [Visitor::save_binary_to_memory],
    /// and [Visitor::save_binary_to_vec].
    ///
    /// [Visitor::load_binary_from_file] will return an error if this sequence of bytes is not present
    /// at the beginning of the file, and [Visitor::load_binary_from_memory] will return an error of
    /// these bytes are not at the beginning of the given slice.
    pub const MAGIC_BINARY_CURRENT: &'static str = "FBAF";

    /// Old header marker for ASCII version.
    pub const MAGIC_ASCII_OLD: &'static str = "FTAF";

    /// Sequence of bytes that is automatically written at the start when a visitor is encoded into
    /// ascii form. It is written by [Visitor::save_ascii_to_file], [Visitor::save_ascii_to_memory],
    /// and [Visitor::save_ascii_to_string].
    ///
    /// [Visitor::load_ascii_from_file] will return an error if this sequence of bytes is not present
    /// at the beginning of the file, and [Visitor::load_ascii_from_memory] will return an error of
    /// these bytes are not at the beginning of the given slice.
    pub const MAGIC_ASCII_CURRENT: &'static str = "FTAX";

    /// Checks whether the given reader points to a supported file format or not.
    #[must_use]
    pub fn is_supported(src: &mut dyn Read) -> bool {
        Self::detect_format(src) != Format::Unknown
    }

    /// Tries to extract the information about the file format in the given reader.
    pub fn detect_format(src: &mut dyn Read) -> Format {
        let mut magic: [u8; 4] = Default::default();
        if src.read_exact(&mut magic).is_ok() {
            if magic.eq(Visitor::MAGIC_BINARY_OLD.as_bytes())
                || magic.eq(Visitor::MAGIC_BINARY_CURRENT.as_bytes())
            {
                return Format::Binary;
            } else if magic.eq(Visitor::MAGIC_ASCII_OLD.as_bytes())
                || magic.eq(Visitor::MAGIC_ASCII_CURRENT.as_bytes())
            {
                return Format::Ascii;
            }
        }
        Format::Unknown
    }

    /// Tries to extract the information about the file format in the given slice.
    pub fn detect_format_from_slice(data: &[u8]) -> Format {
        let mut src = Cursor::new(data);
        Self::detect_format(&mut src)
    }

    /// Creates a Visitor containing only a single node called "`__ROOT__`" which will be the
    /// current region of the visitor.
    pub fn new() -> Self {
        let mut nodes = Pool::new();
        let root = nodes.spawn(VisitorNode::new("__ROOT__", Handle::NONE));
        Self {
            nodes,
            unique_id_counter: 1,
            rc_map: FxHashMap::default(),
            arc_map: FxHashMap::default(),
            reading: false,
            current_node: root,
            root,
            version: CURRENT_VERSION,
            blackboard: Blackboard::new(),
            flags: VisitorFlags::NONE,
        }
    }

    fn gen_unique_id(&mut self) -> u64 {
        let id = self.unique_id_counter;
        self.unique_id_counter += 1;
        id
    }

    fn rc_id<T>(&mut self, rc: &Rc<T>) -> (u64, bool)
    where
        T: Any,
    {
        if let Some(id) = self.rc_map.iter().find_map(|(id, ptr)| {
            if Rc::as_ptr(ptr) as *const T == Rc::as_ptr(rc) {
                Some(*id)
            } else {
                None
            }
        }) {
            (id, false)
        } else {
            let id = self.gen_unique_id();
            self.rc_map.insert(id, rc.clone());
            (id, true)
        }
    }

    fn arc_id<T>(&mut self, arc: &Arc<T>) -> (u64, bool)
    where
        T: Any + Send + Sync,
    {
        if let Some(id) = self.arc_map.iter().find_map(|(id, ptr)| {
            if Arc::as_ptr(ptr) as *const T == Arc::as_ptr(arc) {
                Some(*id)
            } else {
                None
            }
        }) {
            (id, false)
        } else {
            let id = self.gen_unique_id();
            self.arc_map.insert(id, arc.clone());
            (id, true)
        }
    }

    /// Tries to find a field by its name.
    pub fn find_field(&mut self, name: &str) -> Option<&mut Field> {
        self.nodes
            .borrow_mut(self.current_node)
            .fields
            .iter_mut()
            .find(|field| field.name == name)
    }

    /// Tries to find a node by its name.
    pub fn find_node(&self, name: &str) -> Option<&VisitorNode> {
        self.nodes.iter().find(|n| n.name == name)
    }

    /// True if this Visitor is changing the values that it visits. In other words,
    /// `x.visit("MyValue", &mut visitor)` will result in `x` being mutated to match whatever value
    /// is stored in `visitor`.
    ///
    /// False if this visitor is copying and storing the values that it visits. In other words,
    /// `x.visit("MyValue", &mut visitor)` will result in `x` being unchanged, but `visitor` will
    /// be mutated to store the value of `x` under the name "MyValue".
    pub fn is_reading(&self) -> bool {
        self.reading
    }

    fn current_node(&mut self) -> &mut VisitorNode {
        self.nodes.borrow_mut(self.current_node)
    }

    /// Returns version number of the visitor.
    pub fn version(&self) -> u32 {
        self.version
    }

    /// If [Visitor::is_reading], find a node with the given name that is a child of the current
    /// node, and return a Visitor for the found node. Return an error if no node with that name exists.
    ///
    /// If not reading, create a node with the given name as a chld of the current node, and return
    /// a visitor for the new node. Return an error if a node with  that name already exists.
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
                Err(VisitError::RegionDoesNotExist(self.build_breadcrumb(" > ")))
            }
        } else {
            // Make sure that node does not exist already.
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

    fn build_breadcrumb(&self, separator: &str) -> String {
        let mut rev = String::new();
        let mut handle = self.current_node;
        loop {
            let node = self.nodes.try_borrow(handle);
            let Some(node) = node else {
                break;
            };
            if !rev.is_empty() {
                rev.extend(separator.chars().rev());
            }
            rev.extend(node.name.chars().rev());
            handle = node.parent;
        }
        rev.chars().rev().collect()
    }

    /// The name of the current region. This should never be None if the Visitor is operating normally,
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

    /// Create a string containing all the data of this Visitor in ascii form. The string is
    /// formatted to be human-readable with each node on its own line and tabs to indent child nodes.
    pub fn save_ascii_to_string(&self) -> String {
        let mut cursor = Cursor::<Vec<u8>>::default();
        self.save_ascii_to_memory(&mut cursor).unwrap();
        String::from_utf8(cursor.into_inner()).unwrap()
    }

    /// Create a string containing all the data of this Visitor in ascii form and saves it to the given
    /// path. The string is formatted to be human-readable with each node on its own line and tabs
    /// to indent child nodes.
    pub fn save_ascii_to_file(&self, path: impl AsRef<Path>) -> VisitResult {
        let mut writer = BufWriter::new(File::create(path)?);
        let text = self.save_ascii_to_string();
        writer.write_all(text.as_bytes())?;
        Ok(())
    }

    /// Create a string containing all the data of this Visitor in ascii form and writes it to the
    /// given writer. The string is formatted to be human-readable with each node on its own line
    /// and tabs to indent child nodes.
    pub fn save_ascii_to_memory(&self, mut dest: impl Write) -> VisitResult {
        let writer = AsciiWriter::default();
        writer.write(self, &mut dest)
    }

    /// Tries to create a visitor from the given data. The returned instance can then be used to
    /// deserialize some data.
    pub fn load_ascii_from_memory(data: &[u8]) -> Result<Self, VisitError> {
        let mut src = Cursor::new(data);
        let mut reader = AsciiReader::new(&mut src);
        reader.read()
    }

    /// Tries to create a visitor from the given file. The returned instance can then be used to
    /// deserialize some data.
    pub async fn load_ascii_from_file(path: impl AsRef<Path>) -> Result<Self, VisitError> {
        Self::load_ascii_from_memory(&io::load_file(path).await?)
    }

    /// Write the data of this Visitor to the given writer. Begin by writing [Visitor::MAGIC_BINARY_CURRENT].
    pub fn save_binary_to_memory(&self, mut dest: impl Write) -> VisitResult {
        let writer = BinaryWriter::default();
        writer.write(self, &mut dest)
    }

    /// Encode the data of this visitor into bytes and push the bytes into the given `Vec<u8>`.
    /// Begin by writing [Visitor::MAGIC_BINARY_CURRENT].
    pub fn save_binary_to_vec(&self) -> Result<Vec<u8>, VisitError> {
        let mut writer = Cursor::new(Vec::new());
        self.save_binary_to_memory(&mut writer)?;
        Ok(writer.into_inner())
    }

    /// Create a file at the given path and write the data of this visitor into that file in a
    /// non-human-readable binary format so that the data can be reconstructed using [Visitor::load_binary_from_file].
    /// Begin by writing [Visitor::MAGIC_BINARY_CURRENT].
    pub fn save_binary_to_file(&self, path: impl AsRef<Path>) -> VisitResult {
        let writer = BufWriter::new(File::create(path)?);
        self.save_binary_to_memory(writer)
    }

    /// Create a visitor by reading data from the file at the given path, assuming that the file was
    /// created using [Visitor::save_binary_to_file]. Return a [VisitError::NotSupportedFormat] if
    /// [Visitor::MAGIC_BINARY_CURRENT] is not the first bytes read from the file.
    pub async fn load_binary_from_file(path: impl AsRef<Path>) -> Result<Self, VisitError> {
        Self::load_binary_from_memory(&io::load_file(path).await?)
    }

    /// Create a visitor by decoding data from the given byte slice, assuming that the bytes are in
    /// the format that would be produced by [Visitor::save_binary_to_vec]. Return a
    /// [VisitError::NotSupportedFormat] if [Visitor::MAGIC_BINARY_CURRENT] is not the first bytes read from
    /// the slice.
    pub fn load_binary_from_memory(data: &[u8]) -> Result<Self, VisitError> {
        let mut src = Cursor::new(data);
        let mut reader = BinaryReader::new(&mut src);
        reader.read()
    }

    /// Tries to load a visitor from the given file. This method automatically detects a format of
    /// the incoming data (binary or ASCII) and tries to load it.
    pub async fn load_from_file(path: impl AsRef<Path>) -> Result<Self, VisitError> {
        Self::load_from_memory(&io::load_file(path).await?)
    }

    /// Tries to load a visitor from the given data. This method automatically detects a format of
    /// the incoming data (binary or ASCII) and tries to load it.
    pub fn load_from_memory(data: &[u8]) -> Result<Self, VisitError> {
        match Self::detect_format_from_slice(data) {
            Format::Unknown => Err(VisitError::NotSupportedFormat),
            Format::Binary => Self::load_binary_from_memory(data),
            Format::Ascii => Self::load_ascii_from_memory(data),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::visitor::{BinaryBlob, Visit, VisitResult, Visitor};
    use nalgebra::{
        Matrix2, Matrix3, Matrix4, UnitComplex, UnitQuaternion, Vector2, Vector3, Vector4,
    };
    use std::sync::Arc;
    use std::{fs::File, io::Write, path::Path, rc, rc::Rc, sync};
    use uuid::{uuid, Uuid};

    #[derive(Visit, Default, PartialEq, Debug)]
    pub struct Model {
        data: u64,
    }

    #[derive(Default, PartialEq, Debug)]
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
    #[derive(Visit, PartialEq, Debug)]
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

    #[derive(Visit, PartialEq, Debug)]
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

    #[derive(Default, Visit, Debug)]
    struct Weaks {
        weak_resource_arc: Option<sync::Weak<Resource>>,
        weak_resource_rc: Option<rc::Weak<Resource>>,
    }

    impl PartialEq for Weaks {
        fn eq(&self, other: &Self) -> bool {
            self.weak_resource_arc.as_ref().and_then(|r| r.upgrade())
                == other.weak_resource_arc.as_ref().and_then(|r| r.upgrade())
                && self.weak_resource_rc.as_ref().and_then(|r| r.upgrade())
                    == other.weak_resource_rc.as_ref().and_then(|r| r.upgrade())
        }
    }

    #[derive(Default, Visit, Debug, PartialEq)]
    struct Foo {
        boolean: bool,
        num_u8: u8,
        num_i8: i8,
        num_u16: u16,
        num_i16: i16,
        num_u32: u32,
        num_i32: i32,
        num_u64: u64,
        num_i64: i64,
        num_f32: f32,
        num_f64: f64,
        quat: UnitQuaternion<f32>,
        mat4: Matrix4<f32>,
        array: Vec<u8>,
        mat3: Matrix3<f32>,
        uuid: Uuid,
        complex: UnitComplex<f32>,
        mat2: Matrix2<f32>,

        vec2_u8: Vector2<u8>,
        vec2_i8: Vector2<i8>,
        vec2_u16: Vector2<u16>,
        vec2_i16: Vector2<i16>,
        vec2_u32: Vector2<u32>,
        vec2_i32: Vector2<i32>,
        vec2_u64: Vector2<u64>,
        vec2_i64: Vector2<i64>,

        vec3_u8: Vector3<u8>,
        vec3_i8: Vector3<i8>,
        vec3_u16: Vector3<u16>,
        vec3_i16: Vector3<i16>,
        vec3_u32: Vector3<u32>,
        vec3_i32: Vector3<i32>,
        vec3_u64: Vector3<u64>,
        vec3_i64: Vector3<i64>,

        vec4_u8: Vector4<u8>,
        vec4_i8: Vector4<i8>,
        vec4_u16: Vector4<u16>,
        vec4_i16: Vector4<i16>,
        vec4_u32: Vector4<u32>,
        vec4_i32: Vector4<i32>,
        vec4_u64: Vector4<u64>,
        vec4_i64: Vector4<i64>,

        string: String,

        vec2_f32: Vector2<f32>,
        vec2_f64: Vector2<f64>,
        vec3_f32: Vector3<f32>,
        vec3_f64: Vector3<f64>,
        vec4_f32: Vector4<f32>,
        vec4_f64: Vector4<f64>,

        shared_resource: Option<Rc<Resource>>,
        shared_resource_arc: Option<Arc<Resource>>,
        weaks: Weaks,
    }

    impl Foo {
        fn new(resource: Rc<Resource>, arc_resource: Arc<Resource>) -> Self {
            Self {
                boolean: true,
                num_u8: 123,
                num_i8: -123,
                num_u16: 123,
                num_i16: -123,
                num_u32: 123,
                num_i32: -123,
                num_u64: 123,
                num_i64: -123,
                num_f32: 123.321,
                num_f64: 123.321,
                quat: UnitQuaternion::from_euler_angles(1.0, 2.0, 3.0),
                mat4: Matrix4::new_scaling(3.0),
                array: vec![1, 2, 3, 4],
                mat3: Matrix3::new_scaling(3.0),
                uuid: uuid!("51a582c0-30d7-4dbc-b5a0-da8ea186edce"),
                complex: UnitComplex::new(0.0),
                mat2: Matrix2::new_scaling(2.0),
                vec2_u8: Vector2::new(1, 2),
                vec2_i8: Vector2::new(-1, -2),
                vec2_u16: Vector2::new(1, 2),
                vec2_i16: Vector2::new(-1, -2),
                vec2_u32: Vector2::new(1, 2),
                vec2_i32: Vector2::new(-1, -2),
                vec2_u64: Vector2::new(1, 2),
                vec2_i64: Vector2::new(-1, -2),
                vec3_u8: Vector3::new(1, 2, 3),
                vec3_i8: Vector3::new(-1, -2, -3),
                vec3_u16: Vector3::new(1, 2, 3),
                vec3_i16: Vector3::new(-1, -2, -3),
                vec3_u32: Vector3::new(1, 2, 3),
                vec3_i32: Vector3::new(-1, -2, -3),
                vec3_u64: Vector3::new(1, 2, 3),
                vec3_i64: Vector3::new(-1, -2, -3),
                vec4_u8: Vector4::new(1, 2, 3, 4),
                vec4_i8: Vector4::new(-1, -2, -3, -4),
                vec4_u16: Vector4::new(1, 2, 3, 4),
                vec4_i16: Vector4::new(-1, -2, -3, -4),
                vec4_u32: Vector4::new(1, 2, 3, 4),
                vec4_i32: Vector4::new(-1, -2, -3, -4),
                vec4_u64: Vector4::new(1, 2, 3, 4),
                vec4_i64: Vector4::new(-1, -2, -3, -4),
                vec2_f32: Vector2::new(123.321, 234.432),
                vec2_f64: Vector2::new(123.321, 234.432),
                vec3_f32: Vector3::new(123.321, 234.432, 567.765),
                vec3_f64: Vector3::new(123.321, 234.432, 567.765),
                vec4_f32: Vector4::new(123.321, 234.432, 567.765, 890.098),
                vec4_f64: Vector4::new(123.321, 234.432, 567.765, 890.098),
                weaks: Weaks {
                    weak_resource_arc: Some(Arc::downgrade(&arc_resource)),
                    weak_resource_rc: Some(Rc::downgrade(&resource)),
                },
                shared_resource: Some(resource),
                shared_resource_arc: Some(arc_resource),
                string: "This Is A String With Reserved Characters <>:;{}[\\\\\\\\\\] \
                and \"quotes\" many \"\"\"quotes\"\"\"\" and line\nbreak\ttabs\t\t\t\t"
                    .to_string(),
            }
        }
    }

    fn resource() -> Rc<Resource> {
        Rc::new(Resource::new(ResourceKind::Model(Model { data: 555 })))
    }

    fn resource_arc() -> Arc<Resource> {
        Arc::new(Resource::new(ResourceKind::Model(Model { data: 555 })))
    }

    fn objects(resource: Rc<Resource>, arc_resource: Arc<Resource>) -> Vec<Foo> {
        vec![
            Foo::new(resource.clone(), arc_resource.clone()),
            Foo::new(resource, arc_resource),
        ]
    }

    fn serialize() -> Visitor {
        let mut resource = resource();
        let mut resource_arc = resource_arc();
        let mut objects = objects(resource.clone(), resource_arc.clone());

        let mut visitor = Visitor::new();
        resource.visit("SharedResource", &mut visitor).unwrap();
        resource_arc
            .visit("SharedResourceArc", &mut visitor)
            .unwrap();
        objects.visit("Objects", &mut visitor).unwrap();
        visitor
    }

    #[test]
    fn visitor_test_binary() {
        let path = Path::new("test.bin");

        // Save
        {
            let visitor = serialize();

            visitor.save_binary_to_file(path).unwrap();
            if let Ok(mut file) = File::create(Path::new("test.txt")) {
                file.write_all(visitor.save_ascii_to_string().as_bytes())
                    .unwrap();
            }
        }

        // Load
        {
            let expected_resource = resource();
            let expected_resource_arc = resource_arc();
            let expected_objects =
                objects(expected_resource.clone(), expected_resource_arc.clone());

            let mut visitor = futures::executor::block_on(Visitor::load_from_file(path)).unwrap();
            let mut resource: Rc<Resource> = Rc::new(Default::default());
            resource.visit("SharedResource", &mut visitor).unwrap();
            assert_eq!(resource, expected_resource);

            let mut resource_arc: Arc<Resource> = Arc::new(Default::default());
            resource_arc
                .visit("SharedResourceArc", &mut visitor)
                .unwrap();
            assert_eq!(resource_arc, expected_resource_arc);

            let mut objects: Vec<Foo> = Vec::new();
            objects.visit("Objects", &mut visitor).unwrap();
            assert_eq!(objects, expected_objects);
        }
    }

    #[test]
    fn visitor_test_ascii() {
        let path = Path::new("test_ascii.txt");

        // Save
        {
            let visitor = serialize();
            visitor.save_ascii_to_file(path).unwrap();
        }

        // Load
        {
            let expected_resource = resource();
            let expected_resource_arc = resource_arc();
            let expected_objects =
                objects(expected_resource.clone(), expected_resource_arc.clone());

            let mut visitor =
                futures::executor::block_on(Visitor::load_ascii_from_file(path)).unwrap();
            let mut resource: Rc<Resource> = Rc::new(Default::default());
            resource.visit("SharedResource", &mut visitor).unwrap();
            assert_eq!(resource, expected_resource);

            let mut resource_arc: Arc<Resource> = Arc::new(Default::default());
            resource_arc
                .visit("SharedResourceArc", &mut visitor)
                .unwrap();
            assert_eq!(resource_arc, expected_resource_arc);

            let mut objects: Vec<Foo> = Vec::new();
            objects.visit("Objects", &mut visitor).unwrap();
            assert_eq!(objects, expected_objects);
        }
    }
}
