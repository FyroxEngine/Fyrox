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

#![allow(missing_docs)] // TODO

use crate::{
    asset::entry::DEFAULT_RESOURCE_LIFETIME,
    core::sparse::{AtomicIndex, SparseBuffer},
};
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

pub mod geometry;
pub mod shader;
pub mod texture;
pub mod uniform;

#[derive(Copy, Clone, PartialEq)]
pub struct TimeToLive(pub f32);

impl Default for TimeToLive {
    fn default() -> Self {
        Self(DEFAULT_RESOURCE_LIFETIME)
    }
}

impl Deref for TimeToLive {
    type Target = f32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TimeToLive {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct CacheEntry<T> {
    pub value: T,
    pub time_to_live: TimeToLive,
    pub self_index: Arc<AtomicIndex>,
}

impl<T> Drop for CacheEntry<T> {
    fn drop(&mut self) {
        // Reset self index to unassigned. This is needed, because there could be the following
        // situation:
        // 1) Cache entry was removed
        // 2) Its index was stored somewhere else.
        // 3) The index can then be used to access some entry on the index, but the cache cannot
        // guarantee, that the data of the entry is the same.
        self.self_index.set(AtomicIndex::UNASSIGNED_INDEX)
    }
}

impl<T> Deref for CacheEntry<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for CacheEntry<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

pub struct TemporaryCache<T> {
    pub buffer: SparseBuffer<CacheEntry<T>>,
}

impl<T> Default for TemporaryCache<T> {
    fn default() -> Self {
        Self {
            buffer: Default::default(),
        }
    }
}

impl<T> TemporaryCache<T> {
    pub fn spawn(
        &mut self,
        value: T,
        self_index: Arc<AtomicIndex>,
        time_to_live: TimeToLive,
    ) -> AtomicIndex {
        let index = self.buffer.spawn(CacheEntry {
            value,
            time_to_live,
            self_index,
        });

        self.buffer
            .get_mut(&index)
            .unwrap()
            .self_index
            .set(index.get());

        index
    }

    pub fn get_mut(&mut self, index: &AtomicIndex) -> Option<&mut CacheEntry<T>> {
        if let Some(entry) = self.buffer.get_mut(index) {
            entry.time_to_live = TimeToLive::default();
            Some(entry)
        } else {
            None
        }
    }

    pub fn get_entry_mut_or_insert_with<F, E>(
        &mut self,
        index: &Arc<AtomicIndex>,
        time_to_live: TimeToLive,
        func: F,
    ) -> Result<&mut CacheEntry<T>, E>
    where
        F: FnOnce() -> Result<T, E>,
    {
        if let Some(entry) = self.buffer.get_mut(index) {
            entry.time_to_live = time_to_live;
            Ok(self.buffer.get_mut(index).unwrap())
        } else {
            let value = func()?;
            let index = self.buffer.spawn(CacheEntry {
                value,
                time_to_live,
                self_index: index.clone(),
            });
            let entry = self.buffer.get_mut(&index).unwrap();
            entry.self_index.set(index.get());
            Ok(entry)
        }
    }

    pub fn get_mut_or_insert_with<F, E>(
        &mut self,
        index: &Arc<AtomicIndex>,
        time_to_live: TimeToLive,
        func: F,
    ) -> Result<&mut T, E>
    where
        F: FnOnce() -> Result<T, E>,
    {
        self.get_entry_mut_or_insert_with(index, time_to_live, func)
            .map(|entry| &mut entry.value)
    }

    pub fn get_or_insert_with<F, E>(
        &mut self,
        index: &Arc<AtomicIndex>,
        time_to_live: TimeToLive,
        func: F,
    ) -> Result<&T, E>
    where
        F: FnOnce() -> Result<T, E>,
    {
        self.get_entry_mut_or_insert_with(index, time_to_live, func)
            .map(|entry| &entry.value)
    }

    pub fn update(&mut self, dt: f32) {
        for entry in self.buffer.iter_mut() {
            *entry.time_to_live -= dt;
        }

        for i in 0..self.buffer.len() {
            if let Some(entry) = self.buffer.get_raw(i) {
                if *entry.time_to_live <= 0.0 {
                    self.buffer.free_raw(i);
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    pub fn alive_count(&self) -> usize {
        self.buffer.filled()
    }

    pub fn remove(&mut self, index: &AtomicIndex) {
        self.buffer.free(index);
    }
}
