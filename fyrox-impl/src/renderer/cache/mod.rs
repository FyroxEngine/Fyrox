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
    scene::mesh::{
        buffer::{BytesStorage, TriangleBuffer, VertexAttributeDescriptor, VertexBuffer},
        surface::{SurfaceData, SurfaceResource},
    },
};
use fxhash::FxHashMap;
use fyrox_resource::untyped::ResourceKind;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};
use uuid::Uuid;

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

/// A cache for dynamic surfaces, the content of which changes every frame. The main purpose of this
/// cache is to keep associated GPU buffers alive a long as the surfaces in the cache and thus prevent
/// redundant resource reallocation on every frame. This is very important for dynamic drawing, such
/// as 2D sprites, tile maps, etc.
#[derive(Default)]
pub struct DynamicSurfaceCache {
    cache: FxHashMap<u64, SurfaceResource>,
}

impl DynamicSurfaceCache {
    /// Creates a new empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Tries to get an existing surface from the cache using its unique id or creates a new one and
    /// returns it.
    pub fn get_or_create(
        &mut self,
        unique_id: u64,
        layout: &[VertexAttributeDescriptor],
    ) -> SurfaceResource {
        if let Some(surface) = self.cache.get(&unique_id) {
            surface.clone()
        } else {
            let default_capacity = 4096;

            // Initialize empty vertex buffer.
            let vertex_buffer = VertexBuffer::new_with_layout(
                layout,
                0,
                BytesStorage::with_capacity(default_capacity),
            )
            .unwrap();

            // Initialize empty triangle buffer.
            let triangle_buffer = TriangleBuffer::new(Vec::with_capacity(default_capacity * 3));

            let surface = SurfaceResource::new_ok(
                Uuid::new_v4(),
                ResourceKind::Embedded,
                SurfaceData::new(vertex_buffer, triangle_buffer),
            );

            self.cache.insert(unique_id, surface.clone());

            surface
        }
    }

    /// Clears the surfaces in the cache, does **not** clear the cache itself.
    pub fn clear(&mut self) {
        for surface in self.cache.values_mut() {
            let mut surface_data = surface.data_ref();
            surface_data.vertex_buffer.modify().clear();
            surface_data.geometry_buffer.modify().clear();
        }
    }
}
