#![allow(missing_docs)] // TODO

use std::ops::{Deref, DerefMut};

pub mod geometry;
pub mod shader;
pub mod texture;

pub struct CacheEntry<T> {
    pub value: T,
    pub value_hash: u64,
    pub time_to_live: f32,
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
