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
use std::{
    any::{Any, TypeId},
    sync::Arc,
};

/// A Blackboard is a mapping from TypeId to value that allows a [`crate::visitor::Visitor`] to store
/// a particular value for each registered type.
#[derive(Default)]
pub struct Blackboard {
    items: FxHashMap<TypeId, Arc<dyn Any>>,
}

impl Blackboard {
    pub fn new() -> Self {
        Self {
            items: Default::default(),
        }
    }

    pub fn register<T: Any>(&mut self, value: Arc<T>) {
        self.items.insert(TypeId::of::<T>(), value);
    }

    pub fn get<T: Any>(&self) -> Option<&T> {
        self.items
            .get(&TypeId::of::<T>())
            .and_then(|v| (**v).downcast_ref::<T>())
    }

    pub fn inner(&self) -> &FxHashMap<TypeId, Arc<dyn Any>> {
        &self.items
    }

    pub fn inner_mut(&mut self) -> &mut FxHashMap<TypeId, Arc<dyn Any>> {
        &mut self.items
    }
}
