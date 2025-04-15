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

//! Blackboard is a container for arbitrary, shared data that is available during serialization and
//! deserialization. See [`Blackboard`] docs for more info.

use fxhash::FxHashMap;
use std::{
    any::{Any, TypeId},
    sync::Arc,
};

/// Blackboard is a container for arbitrary, shared data that is available during serialization and
/// deserialization. The main use of the blackboard is to pass some "global" data to the serializer.
/// For example, to deserialize a trait object, some sort of container with constructors is needed
/// that will create an object instance by its type uuid. Such a container can be passed to the
/// serializer using the blackboard.
#[derive(Default)]
pub struct Blackboard {
    items: FxHashMap<TypeId, Arc<dyn Any>>,
}

impl Blackboard {
    /// Creates a new empty blackboard.
    pub fn new() -> Self {
        Self {
            items: Default::default(),
        }
    }

    /// Registers a shared object in the blackboard. There could be only one object of the given
    /// type at the same time.
    pub fn register<T: Any>(&mut self, value: Arc<T>) {
        self.items.insert(TypeId::of::<T>(), value);
    }

    /// Tries to find an object of the given type in the blackboard.
    pub fn get<T: Any>(&self) -> Option<&T> {
        self.items
            .get(&TypeId::of::<T>())
            .and_then(|v| (**v).downcast_ref::<T>())
    }

    /// Returns inner hash map.
    pub fn inner(&self) -> &FxHashMap<TypeId, Arc<dyn Any>> {
        &self.items
    }

    /// Returns inner hash map.
    pub fn inner_mut(&mut self) -> &mut FxHashMap<TypeId, Arc<dyn Any>> {
        &mut self.items
    }
}
