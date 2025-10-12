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

//! This is an extension for Mutex-like objects that creates a `safe_lock` method
//! to replace `lock` and put a time limit on locking, preventing a game from
//! permanently freezing due to a deadlock.

use parking_lot::{Mutex, MutexGuard};
#[allow(unused_imports)]
use std::sync::TryLockError;
use std::time::Duration;

#[allow(dead_code)]
const PANIC_MESSAGE: &str = "lock timeout";

/// Trait for lockable objects that can panic if they take too long
/// to lock. Panicking is preferable over freezing.
pub trait SafeLock {
    const TIMEOUT: Duration = Duration::from_secs(10);
    type Output<'a>
    where
        Self: 'a;
    /// Attempt to lock the object, with a limit on how long locking may block for,
    /// panicking if the time limit is exceeded. It panics instead of freezing.
    fn safe_lock(&self) -> Self::Output<'_>;
}

impl<T: ?Sized> SafeLock for Mutex<T> {
    type Output<'a>
        = MutexGuard<'a, T>
    where
        T: 'a;
    #[cfg(target_arch = "wasm32")]
    fn safe_lock(&self) -> Self::Output<'_> {
        self.lock()
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn safe_lock(&self) -> Self::Output<'_> {
        self.try_lock_for(Self::TIMEOUT).expect(PANIC_MESSAGE)
    }
}

impl<T: ?Sized> SafeLock for std::sync::Mutex<T> {
    type Output<'a>
        = std::sync::LockResult<std::sync::MutexGuard<'a, T>>
    where
        T: 'a;

    #[cfg(target_arch = "wasm32")]
    fn safe_lock(&self) -> Self::Output<'_> {
        self.lock()
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn safe_lock(&self) -> Self::Output<'_> {
        let start = std::time::Instant::now();
        loop {
            match self.try_lock() {
                Ok(guard) => return Ok(guard),
                Err(TryLockError::WouldBlock) => (),
                Err(TryLockError::Poisoned(err)) => return Err(err),
            }
            std::thread::yield_now();
            if start.elapsed() > Self::TIMEOUT {
                std::panic::panic_any(PANIC_MESSAGE);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::{
        any::Any,
        panic::{catch_unwind, AssertUnwindSafe},
    };
    fn panic_to_string(message: Box<dyn Any>) -> Option<String> {
        match message.downcast_ref::<&str>() {
            Some(&str) => Some(str.into()),
            None => message.downcast::<String>().ok().map(|s| *s),
        }
    }
    #[test]
    fn successful_lock_parking_lot() {
        let mutex = Mutex::new(());
        drop(mutex.safe_lock());
    }
    #[test]
    fn successful_lock_std() {
        let mutex = std::sync::Mutex::new(());
        drop(mutex.safe_lock());
    }
    #[test]
    fn failed_lock_parking_lot() {
        let mutex = Mutex::new(());
        let _guard = mutex.safe_lock();
        let panic_message = catch_unwind(AssertUnwindSafe(|| mutex.safe_lock()))
            .expect_err("safe_lock did not panic");
        let Some(message) = panic_to_string(panic_message) else {
            panic!("safe_lock panicked with wrong type");
        };
        assert_eq!(message, PANIC_MESSAGE);
    }
    #[test]
    fn failed_lock_std() {
        let mutex = std::sync::Mutex::new(());
        let _guard = mutex.safe_lock();
        let panic_message =
            catch_unwind(|| mutex.safe_lock()).expect_err("safe_lock did not panic");
        let Some(message) = panic_to_string(panic_message) else {
            panic!("safe_lock panicked with wrong type");
        };
        assert_eq!(message, PANIC_MESSAGE);
    }
}
