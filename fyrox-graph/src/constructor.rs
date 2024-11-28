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

use fxhash::FxHashMap;
use fyrox_core::pool::Handle;
use fyrox_core::{
    parking_lot::{Mutex, MutexGuard},
    reflect::prelude::*,
    TypeUuidProvider, Uuid,
};
use std::sync::Arc;

pub trait ConstructorProvider<T, Ctx>: TypeUuidProvider + Default + Reflect {
    fn constructor() -> GraphNodeConstructor<T, Ctx>;
}

pub enum VariantResult<Node> {
    Owned(Node),
    Handle(Handle<Node>),
}

impl<Node> From<Node> for VariantResult<Node> {
    fn from(node: Node) -> Self {
        VariantResult::Owned(node)
    }
}

impl<Node> From<Handle<Node>> for VariantResult<Node> {
    fn from(value: Handle<Node>) -> Self {
        VariantResult::Handle(value)
    }
}

/// Shared closure that creates a node of some type.
pub type Constructor<Node> = Arc<dyn Fn() -> Node + Send + Sync>;

pub type VariantConstructor<Node, Ctx> = Arc<dyn Fn(&mut Ctx) -> VariantResult<Node> + Send + Sync>;

/// Constructor variant.
pub struct Variant<Node, Ctx> {
    /// Name of the variant.
    pub name: String,
    /// Boxed type constructor.
    pub constructor: VariantConstructor<Node, Ctx>,
}

/// Node constructor creates scene nodes in various states.
pub struct GraphNodeConstructor<Node, Ctx> {
    /// A boxed type constructor that returns a node in default state. This constructor is used at
    /// deserialization stage.
    pub default: Constructor<Node>,

    /// A set of node constructors that returns specific variants of the same node type. Could be
    /// used to pre-define specific variations of nodes, for example a `Mesh` node could have
    /// different surfaces (sphere, cube, cone, etc.). It is used by the editor, this collection must
    /// have at least one item to be shown in the editor.
    pub variants: Vec<Variant<Node, Ctx>>,

    /// Name of the group the type belongs to.
    pub group: &'static str,

    /// A name of the assembly this node constructor is from.
    pub assembly_name: &'static str,
}

impl<Node, Ctx> GraphNodeConstructor<Node, Ctx> {
    /// Creates a new node constructor with default values for the given scene node type. This method
    /// automatically creates default constructor, but leaves potential variants empty (nothing will
    /// be shown in the editor, use [`Self::with_variant`] to add potential variant of the constructor).
    pub fn new<Inner>() -> Self
    where
        Node: From<Inner>,
        Inner: ConstructorProvider<Node, Ctx>,
    {
        Self {
            default: Arc::new(|| Node::from(Inner::default())),
            variants: vec![],
            group: "",
            assembly_name: Inner::type_assembly_name(),
        }
    }

    /// Sets a desired group for the constructor.
    pub fn with_group(mut self, group: &'static str) -> Self {
        self.group = group;
        self
    }

    /// Adds a new constructor variant.
    pub fn with_variant<F>(mut self, name: impl AsRef<str>, variant: F) -> Self
    where
        F: Fn(&mut Ctx) -> VariantResult<Node> + Send + Sync + 'static,
    {
        self.variants.push(Variant {
            name: name.as_ref().to_string(),
            constructor: Arc::new(variant),
        });
        self
    }
}

/// A special container that is able to create nodes by their type UUID.
pub struct GraphNodeConstructorContainer<Node, Ctx> {
    map: Mutex<FxHashMap<Uuid, GraphNodeConstructor<Node, Ctx>>>,
}

impl<Node, Ctx> Default for GraphNodeConstructorContainer<Node, Ctx> {
    fn default() -> Self {
        Self {
            map: Default::default(),
        }
    }
}

impl<Node, Ctx> GraphNodeConstructorContainer<Node, Ctx> {
    /// Adds new type constructor for a given type and return previous constructor for the type
    /// (if any).
    pub fn add<Inner>(&self)
    where
        Inner: ConstructorProvider<Node, Ctx>,
    {
        let previous = self
            .map
            .lock()
            .insert(Inner::type_uuid(), Inner::constructor());
        assert!(previous.is_none());
    }

    /// Adds custom type constructor.
    pub fn add_custom(&self, type_uuid: Uuid, constructor: GraphNodeConstructor<Node, Ctx>) {
        self.map.lock().insert(type_uuid, constructor);
    }

    /// Unregisters type constructor.
    pub fn remove(&self, type_uuid: Uuid) {
        self.map.lock().remove(&type_uuid);
    }

    /// Makes an attempt to create a node using provided type UUID. It may fail if there is no
    /// node constructor for specified type UUID.
    pub fn try_create(&self, type_uuid: &Uuid) -> Option<Node> {
        self.map.lock().get_mut(type_uuid).map(|c| (c.default)())
    }

    /// Returns total amount of constructors.
    pub fn len(&self) -> usize {
        self.map.lock().len()
    }

    /// Returns true if the container is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the inner map of the node constructors.
    pub fn map(&self) -> MutexGuard<'_, FxHashMap<Uuid, GraphNodeConstructor<Node, Ctx>>> {
        self.map.lock()
    }
}
