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

use std::cell::UnsafeCell;

// pub trait PayloadContainer: Sized {
//     type Element: Sized;

//     fn new_empty() -> Self;

//     fn new(element: Self::Element) -> Self;

//     fn is_some(&self) -> bool;

//     fn as_ref(&self) -> Option<&Self::Element>;

//     fn as_mut(&mut self) -> Option<&mut Self::Element>;

//     fn replace(&mut self, element: Self::Element) -> Option<Self::Element>;

//     fn take(&mut self) -> Option<Self::Element>;
// }

// impl<T> PayloadContainer for Option<T> {
//     type Element = T;

//     #[inline]
//     fn new_empty() -> Self {
//         Self::None
//     }

//     #[inline]
//     fn new(element: Self::Element) -> Self {
//         Self::Some(element)
//     }

//     #[inline]
//     fn is_some(&self) -> bool {
//         Option::is_some(self)
//     }

//     #[inline]
//     fn as_ref(&self) -> Option<&Self::Element> {
//         Option::as_ref(self)
//     }

//     #[inline]
//     fn as_mut(&mut self) -> Option<&mut Self::Element> {
//         Option::as_mut(self)
//     }

//     #[inline]
//     fn replace(&mut self, element: Self::Element) -> Option<Self::Element> {
//         Option::replace(self, element)
//     }

//     #[inline]
//     fn take(&mut self) -> Option<Self::Element> {
//         Option::take(self)
//     }
// }

#[derive(Debug)]
pub struct Payload<T>(pub UnsafeCell<Option<T>>);

impl<T> Clone for Payload<T>
where
    T: Sized + Clone, // Sized: pool record needs a known size; Clone: for implementing Reflect
{
    fn clone(&self) -> Self {
        Self(UnsafeCell::new(self.get().clone()))
    }
}

impl<T> Payload<T>
where
    T: Sized, // Sized: pool record needs a known size; Clone: for implementing Reflect
{
    pub fn new(data: T) -> Self {
        Self(UnsafeCell::new(Some(data)))
    }

    pub fn new_empty() -> Self {
        Self(UnsafeCell::new(None))
    }

    pub fn get(&self) -> &Option<T> {
        unsafe { &*self.0.get() }
    }

    pub fn get_mut(&mut self) -> &mut Option<T> {
        self.0.get_mut()
    }

    pub fn is_some(&self) -> bool {
        self.get().is_some()
    }

    pub fn as_ref(&self) -> Option<&T> {
        self.get().as_ref()
    }

    pub fn as_mut(&mut self) -> Option<&mut T> {
        self.get_mut().as_mut()
    }

    pub fn replace(&mut self, element: T) -> Option<T> {
        self.get_mut().replace(element)
    }

    pub fn take(&mut self) -> Option<T> {
        self.get_mut().take()
    }
}

// SAFETY: This is safe, because Payload is never directly exposed to the call site. It is always
// accessed using a sort of read-write lock that forces borrowing rules at runtime.
unsafe impl<T> Sync for Payload<T> where
    T: Sized // Sized: pool record needs a known size; Clone: for implementing Reflect
{
}

// SAFETY: This is safe, because Payload is never directly exposed to the call site. It is always
// accessed using a sort of read-write lock that forces borrowing rules at runtime.
unsafe impl<T> Send for Payload<T> where
    T: Sized // Sized: pool record needs a known size; Clone: for implementing Reflect
{
}
