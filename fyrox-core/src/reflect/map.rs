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

use crate::reflect::blank_reflect;
use crate::reflect::Reflect;
use std::collections::HashMap;
use std::hash::{BuildHasher, Hash};

pub trait ReflectHashMap: Reflect {
    fn reflect_insert(
        &mut self,
        key: Box<dyn Reflect>,
        value: Box<dyn Reflect>,
    ) -> Option<Box<dyn Reflect>>;
    fn reflect_len(&self) -> usize;
    fn reflect_get(&self, key: &dyn Reflect, func: &mut dyn FnMut(Option<&dyn Reflect>));
    fn reflect_get_mut(
        &mut self,
        key: &dyn Reflect,
        func: &mut dyn FnMut(Option<&mut dyn Reflect>),
    );
    fn reflect_get_nth_value_ref(&self, index: usize) -> Option<&dyn Reflect>;
    fn reflect_get_nth_value_mut(&mut self, index: usize) -> Option<&mut dyn Reflect>;
    fn reflect_get_at(&self, index: usize) -> Option<(&dyn Reflect, &dyn Reflect)>;
    fn reflect_get_at_mut(&mut self, index: usize) -> Option<(&dyn Reflect, &mut dyn Reflect)>;
    fn reflect_remove(&mut self, key: &dyn Reflect, func: &mut dyn FnMut(Option<Box<dyn Reflect>>));
}

impl<K, V, S> Reflect for HashMap<K, V, S>
where
    K: Reflect + Eq + Hash + Clone + 'static,
    V: Reflect + Clone,
    S: BuildHasher + Clone + 'static,
{
    blank_reflect!();

    fn as_hash_map(&self, func: &mut dyn FnMut(Option<&dyn ReflectHashMap>)) {
        func(Some(self))
    }

    fn as_hash_map_mut(&mut self, func: &mut dyn FnMut(Option<&mut dyn ReflectHashMap>)) {
        func(Some(self))
    }
}

impl<K, V, S> ReflectHashMap for HashMap<K, V, S>
where
    K: Reflect + Eq + Hash + Clone + 'static,
    V: Reflect + Clone,
    S: BuildHasher + Clone + 'static,
{
    fn reflect_insert(
        &mut self,
        key: Box<dyn Reflect>,
        value: Box<dyn Reflect>,
    ) -> Option<Box<dyn Reflect>> {
        if let Ok(key) = key.downcast::<K>() {
            if let Ok(value) = value.downcast::<V>() {
                if let Some(previous) = self.insert(*key, *value) {
                    return Some(Box::new(previous));
                }
            }
        }

        None
    }

    fn reflect_len(&self) -> usize {
        self.len()
    }

    fn reflect_get(&self, key: &dyn Reflect, func: &mut dyn FnMut(Option<&dyn Reflect>)) {
        match key.downcast_ref::<K>() {
            Some(key) => match self.get(key) {
                Some(value) => func(Some(value as &dyn Reflect)),
                None => func(None),
            },
            None => func(None),
        }
    }

    fn reflect_get_mut(
        &mut self,
        key: &dyn Reflect,
        func: &mut dyn FnMut(Option<&mut dyn Reflect>),
    ) {
        match key.downcast_ref::<K>() {
            Some(key) => match self.get_mut(key) {
                Some(value) => func(Some(value as &mut dyn Reflect)),
                None => func(None),
            },
            None => func(None),
        }
    }

    fn reflect_get_nth_value_ref(&self, index: usize) -> Option<&dyn Reflect> {
        self.values().nth(index).map(|v| v as &dyn Reflect)
    }

    fn reflect_get_nth_value_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        self.values_mut().nth(index).map(|v| v as &mut dyn Reflect)
    }

    fn reflect_get_at(&self, index: usize) -> Option<(&dyn Reflect, &dyn Reflect)> {
        self.iter()
            .nth(index)
            .map(|(k, v)| (k as &dyn Reflect, v as &dyn Reflect))
    }

    fn reflect_get_at_mut(&mut self, index: usize) -> Option<(&dyn Reflect, &mut dyn Reflect)> {
        self.iter_mut()
            .nth(index)
            .map(|(k, v)| (k as &dyn Reflect, v as &mut dyn Reflect))
    }

    fn reflect_remove(
        &mut self,
        key: &dyn Reflect,
        func: &mut dyn FnMut(Option<Box<dyn Reflect>>),
    ) {
        match key.downcast_ref::<K>() {
            Some(key) => func(
                self.remove(key)
                    .map(|value| Box::new(value) as Box<dyn Reflect>),
            ),
            None => func(None),
        }
    }
}
