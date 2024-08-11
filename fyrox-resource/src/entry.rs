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

//! Resource manager timed entry. It holds strong reference for a resource and a simple timer
//! variable. When someone uses a resource, the timer variable is reset to default resource
//! lifetime. Timer gradually decreases its value and once it reaches zero, the entry is deleted.
//! The inner resource might still be in use (have a strong reference to it), the resource data
//! will be deleted once no one uses the resource.

use std::ops::{Deref, DerefMut};

/// Lifetime of orphaned resource in seconds (with only one strong ref which is resource manager itself)
pub const DEFAULT_RESOURCE_LIFETIME: f32 = 60.0;

/// Resource container with fixed TTL (time-to-live). Resource will be removed
/// (and unloaded) if there were no other strong references to it in given time
/// span.
pub struct TimedEntry<T> {
    /// Payload of entry.
    pub value: T,
    /// Time to live in seconds.
    pub time_to_live: f32,
}

impl<T> Deref for TimedEntry<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for TimedEntry<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T> Default for TimedEntry<T>
where
    T: Default,
{
    fn default() -> Self {
        Self {
            value: Default::default(),
            time_to_live: DEFAULT_RESOURCE_LIFETIME,
        }
    }
}

impl<T> Clone for TimedEntry<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            time_to_live: self.time_to_live,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn timed_entry_default() {
        let t: TimedEntry<_> = TimedEntry::<u32>::default();

        assert_eq!(t.value, 0);
        assert_eq!(t.time_to_live, DEFAULT_RESOURCE_LIFETIME);
    }

    #[test]
    fn timed_entry_deref() {
        let t = TimedEntry {
            value: 42,
            ..Default::default()
        };

        assert_eq!(t.deref(), &42);
    }

    #[test]
    fn timed_entry_deref_mut() {
        let mut t = TimedEntry {
            value: 42,
            ..Default::default()
        };

        assert_eq!(t.deref_mut(), &mut 42);
    }

    #[test]
    fn timed_entry_clone() {
        let t = TimedEntry {
            value: 42,
            time_to_live: 15.0,
        };
        let t2 = t.clone();

        assert_eq!(t.value, t2.value);
        assert_eq!(t.time_to_live, t2.time_to_live);
    }
}
