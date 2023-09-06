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
