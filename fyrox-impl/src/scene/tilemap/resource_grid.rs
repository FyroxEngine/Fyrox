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

use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use crate::asset::untyped::UntypedResource;
use crate::core::{algebra::Vector2, reflect::prelude::*, visitor::prelude::*};
use fxhash::FxHashMap;

/// A map from 2D i32 coordinates to values that keeps a list of the resource references
/// stored in its values.
#[derive(Default, Clone, Debug)]
pub struct ResourceGrid<V> {
    content: FxHashMap<Vector2<i32>, V>,
    resources: ResourceList,
}

#[derive(Default, Clone, Debug)]
struct ResourceList(Vec<UntypedResource>);

/// A value that can be stored in a [`ResourceGrid`] and provide access to its
/// internal resource references so that `ResourceGrid` can maintain its list of resources.
pub trait ResourceGridElement {
    /// Pass each of the resource references stored in this object to the given function.
    fn find_resources<F>(&self, func: F)
    where
        F: FnMut(UntypedResource);
}

impl<V: Visit + Default + ResourceGridElement> Visit for ResourceGrid<V> {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.content.visit(name, visitor)?;
        if visitor.is_reading() {
            self.build_cache();
        }
        Ok(())
    }
}

impl<V> Deref for ResourceGrid<V> {
    type Target = FxHashMap<Vector2<i32>, V>;
    fn deref(&self) -> &Self::Target {
        &self.content
    }
}
impl<V> DerefMut for ResourceGrid<V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.content
    }
}

impl Deref for ResourceList {
    type Target = Vec<UntypedResource>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for ResourceList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<V: ResourceGridElement> ResourceGrid<V> {
    /// Update the list of resources based on the values stored in the grid.
    pub fn build_cache(&mut self) {
        self.resources.clear();
        for element in self.content.values() {
            element.find_resources(|x| {
                if !self.resources.contains(&x) {
                    self.resources.push(x);
                }
            });
        }
    }
}

impl<V: Debug + 'static> Reflect for ResourceGrid<V> {
    fn source_path() -> &'static str
    where
        Self: Sized,
    {
        file!()
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn doc(&self) -> &'static str {
        "A 2D grid that contains tile data."
    }

    fn fields_info(&self, func: &mut dyn FnMut(&[FieldInfo])) {
        func(&[])
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }

    fn as_any(&self, func: &mut dyn FnMut(&dyn std::any::Any)) {
        func(self)
    }

    fn as_any_mut(&mut self, func: &mut dyn FnMut(&mut dyn std::any::Any)) {
        func(self)
    }

    fn as_reflect(&self, func: &mut dyn FnMut(&dyn Reflect)) {
        func(self as &dyn Reflect)
    }

    fn as_reflect_mut(&mut self, func: &mut dyn FnMut(&mut dyn Reflect)) {
        func(self as &mut dyn Reflect)
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
        let value = match value.take() {
            Ok(x) => x,
            Err(err) => return Err(err),
        };
        let this = std::mem::replace(self, value);
        Ok(Box::new(this))
    }

    fn assembly_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }

    fn type_assembly_name() -> &'static str
    where
        Self: Sized,
    {
        env!("CARGO_PKG_NAME")
    }

    fn as_list(&self, func: &mut dyn FnMut(Option<&dyn ReflectList>)) {
        func(Some(&self.resources))
    }

    fn as_list_mut(&mut self, func: &mut dyn FnMut(Option<&mut dyn ReflectList>)) {
        func(None)
    }
}

impl Reflect for ResourceList {
    fn source_path() -> &'static str
    where
        Self: Sized,
    {
        file!()
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn doc(&self) -> &'static str {
        "A 2D grid that contains tile data."
    }

    fn fields_info(&self, func: &mut dyn FnMut(&[FieldInfo])) {
        func(&[])
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }

    fn as_any(&self, func: &mut dyn FnMut(&dyn std::any::Any)) {
        func(self)
    }

    fn as_any_mut(&mut self, func: &mut dyn FnMut(&mut dyn std::any::Any)) {
        func(self)
    }

    fn as_reflect(&self, func: &mut dyn FnMut(&dyn Reflect)) {
        func(self as &dyn Reflect)
    }

    fn as_reflect_mut(&mut self, func: &mut dyn FnMut(&mut dyn Reflect)) {
        func(self as &mut dyn Reflect)
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
        let value = match value.take() {
            Ok(x) => x,
            Err(err) => return Err(err),
        };
        let this = std::mem::replace(self, value);
        Ok(Box::new(this))
    }

    fn assembly_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }

    fn type_assembly_name() -> &'static str
    where
        Self: Sized,
    {
        env!("CARGO_PKG_NAME")
    }

    fn as_list(&self, func: &mut dyn FnMut(Option<&dyn ReflectList>)) {
        func(Some(self))
    }

    fn as_list_mut(&mut self, func: &mut dyn FnMut(Option<&mut dyn ReflectList>)) {
        func(None)
    }
}

impl ReflectArray for ResourceList {
    fn reflect_index(&self, index: usize) -> Option<&dyn Reflect> {
        self.0.get(index).map(|x| x as &dyn Reflect)
    }

    fn reflect_index_mut(&mut self, _index: usize) -> Option<&mut dyn Reflect> {
        None
    }

    fn reflect_len(&self) -> usize {
        self.0.len()
    }
}

impl ReflectList for ResourceList {
    fn reflect_push(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        Err(value)
    }

    fn reflect_pop(&mut self) -> Option<Box<dyn Reflect>> {
        None
    }

    fn reflect_remove(&mut self, _index: usize) -> Option<Box<dyn Reflect>> {
        None
    }

    fn reflect_insert(
        &mut self,
        _index: usize,
        value: Box<dyn Reflect>,
    ) -> Result<(), Box<dyn Reflect>> {
        Err(value)
    }
}
