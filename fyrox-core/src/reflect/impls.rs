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

//! `Reflect` implementations

use crate::{
    reflect::{blank_reflect, prelude::*},
    sstorage::ImmutableString,
    uuid::Uuid,
    SafeLock,
};
use fyrox_core_derive::impl_reflect;
use nalgebra::*;
use std::any::type_name;
use std::{
    cell::{Cell, RefCell},
    fmt::Debug,
    ops::{Deref, DerefMut, Range},
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

impl_reflect! {
    pub struct Matrix<T: Copy + 'static, R: Dim + 'static, C: Dim + 'static, S: Copy + Debug + Reflect + 'static> {
        pub data: S,
        // _phantoms: PhantomData<(T, R, C)>,
    }
}

impl_reflect! {
    pub struct ArrayStorage<T: Copy + Debug + Reflect, const R: usize, const C: usize>(pub [[T; R]; C]);
}

impl_reflect! {
    pub struct Unit<T: Copy + Debug + 'static> {
        // pub(crate) value: T,
    }
}

impl_reflect! {
    pub struct Quaternion<T: Copy + Debug + Reflect> {
        pub coords: Vector4<T>,
    }
}

macro_rules! impl_blank_reflect {
    ( $( $ty:ty ),* $(,)? ) => {
        $(
            impl Reflect for $ty {
                blank_reflect!();
            }
        )*
    }
}

impl_blank_reflect! {
    f32, f64,
    usize, u8, u16, u32, u64,
    isize, i8, i16, i32, i64,
    bool, char,
    String,
    std::path::PathBuf,
    Duration, Instant,
    ImmutableString
}

macro_rules! impl_reflect_tuple {
    (
        $(
            ( $($t:ident,)* );
        )*
    ) => {
        $(
            impl< $($t: Clone + Reflect),* > Reflect for ( $($t,)* ) {
                blank_reflect!();
            }
        )*
    }
}

impl_reflect_tuple! {
    (T0,);
    (T0, T1, );
    (T0, T1, T2, );
    (T0, T1, T2, T3,);
    (T0, T1, T2, T3, T4,);
}

impl Reflect for () {
    blank_reflect!();
}

impl_reflect! { pub struct Uuid; }

impl_reflect! {
    pub struct Cell<T: Debug + Copy>;
}

impl_reflect! {
    pub enum Option<T: Clone + Debug + Reflect> {
        Some(T),
        None
    }
}

impl_reflect! {
    pub struct Range<Idx: Clone + Debug + Reflect> {
        pub start: Idx,
        pub end: Idx,
    }
}

impl<T: Reflect + Clone> Reflect for Box<T> {
    fn type_info() -> TypeInfo {
        TypeInfo {
            source_path: file!(),
            type_name: type_name::<Self>(),
            assembly_name: env!("CARGO_PKG_NAME"),
            doc_comment: "",
            derived_types: &[],
        }
    }

    fn type_info_ref(&self) -> TypeInfo {
        Self::type_info()
    }

    fn fields_ref(&self, func: &mut dyn FnMut(&[FieldRef])) {
        func(&[{
            FieldRef {
                metadata: &CONTENT_METADATA,
                value: self.deref(),
            }
        }])
    }

    fn fields_mut(&mut self, func: &mut dyn FnMut(&mut [FieldMut])) {
        func(&mut [{
            FieldMut {
                metadata: &CONTENT_METADATA,
                value: self.deref_mut(),
            }
        }])
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
        let this = std::mem::replace(self, value.take()?);
        Ok(Box::new(this))
    }

    fn try_clone_box(&self) -> Option<Box<dyn Reflect>> {
        Some(Box::new(self.clone()))
    }

    fn field_direct_ref(&self, index: usize) -> Option<FieldRef> {
        if index == 0 {
            Some(FieldRef {
                metadata: &CONTENT_METADATA,
                value: self.deref(),
            })
        } else {
            None
        }
    }

    fn field_direct_mut(&mut self, index: usize) -> Option<FieldMut> {
        if index == 0 {
            Some(FieldMut {
                metadata: &CONTENT_METADATA,
                value: self.deref_mut(),
            })
        } else {
            None
        }
    }
}

static CONTENT_METADATA: FieldMetadata = FieldMetadata {
    name: "Content",
    display_name: "Content",
    tag: "",
    read_only: false,
    immutable_collection: false,
    min_value: None,
    max_value: None,
    step: None,
    precision: None,
    doc: "",
};

macro_rules! impl_reflect_inner_mutability {
    ($self:ident, $acquire_lock_guard:block, $clone:block) => {
        fn type_info() -> TypeInfo {
            TypeInfo {
                source_path: file!(),
                type_name: std::any::type_name::<T>(),
                assembly_name: env!("CARGO_PKG_NAME"),
                doc_comment: "",
                derived_types: T::type_info().derived_types,
            }
        }

        fn type_info_ref(&self) -> TypeInfo {
            Self::type_info()
        }

        fn try_clone_box(&$self) -> Option<Box<dyn Reflect>> {
            Some(Box::new($clone))
        }

        fn fields_ref(&$self, func: &mut dyn FnMut(&[FieldRef])) {
            let guard = $acquire_lock_guard;
            func(&[{
                FieldRef {
                    metadata: &CONTENT_METADATA,
                    value: &*guard,
                }
            }])
        }

        fn fields_mut(&mut $self, func: &mut dyn FnMut(&mut [FieldMut])) {
            let mut guard = $acquire_lock_guard;
            func(&mut [{
                FieldMut {
                    metadata: &CONTENT_METADATA,
                    value: &mut *guard,
                }
            }])
        }

        fn set(&mut $self, value: Box<dyn Reflect>) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
            let this = std::mem::replace($self, value.take()?);
            Ok(Box::new(this))
        }

        fn field_direct_ref(&self, _index: usize) -> Option<FieldRef> {
            None
        }

        fn field_direct_mut(&mut self, _index: usize) -> Option<FieldMut> {
            None
        }
    };
}

impl<T: Reflect + Clone> Reflect for parking_lot::Mutex<T> {
    impl_reflect_inner_mutability!(self, { self.safe_lock() }, {
        parking_lot::Mutex::new(self.safe_lock().clone())
    });
}

impl<T: Reflect + Clone> Reflect for parking_lot::RwLock<T> {
    impl_reflect_inner_mutability!(self, { self.write() }, {
        parking_lot::RwLock::new(self.read().clone())
    });
}

#[allow(clippy::mut_mutex_lock)]
impl<T: Reflect + Clone> Reflect for std::sync::Mutex<T> {
    impl_reflect_inner_mutability!(self, { self.safe_lock().unwrap() }, {
        std::sync::Mutex::new(self.safe_lock().unwrap().clone())
    });
}

impl<T: Reflect + Clone> Reflect for std::sync::RwLock<T> {
    impl_reflect_inner_mutability!(self, { self.write().unwrap() }, {
        std::sync::RwLock::new(self.read().unwrap().clone())
    });
}

impl<T: Reflect + Clone> Reflect for Arc<parking_lot::Mutex<T>> {
    impl_reflect_inner_mutability!(self, { self.safe_lock() }, {
        Arc::new(parking_lot::Mutex::new(self.safe_lock().clone()))
    });
}

impl<T: Reflect + Clone> Reflect for Arc<std::sync::Mutex<T>> {
    impl_reflect_inner_mutability!(self, { self.lock().unwrap() }, {
        Arc::new(std::sync::Mutex::new(self.safe_lock().unwrap().clone()))
    });
}

impl<T: Reflect + Clone> Reflect for Arc<std::sync::RwLock<T>> {
    impl_reflect_inner_mutability!(self, { self.write().unwrap() }, {
        Arc::new(std::sync::RwLock::new(self.read().unwrap().clone()))
    });
}

impl<T: Reflect + Clone> Reflect for Arc<parking_lot::RwLock<T>> {
    impl_reflect_inner_mutability!(self, { self.write() }, {
        Arc::new(parking_lot::RwLock::new(self.read().clone()))
    });
}

impl<T: Reflect + Clone> Reflect for RefCell<T> {
    impl_reflect_inner_mutability!(self, { self.borrow_mut() }, {
        RefCell::new(self.borrow().clone())
    });
}

impl<T: Reflect + Clone> Reflect for Rc<RefCell<T>> {
    impl_reflect_inner_mutability!(self, { self.borrow_mut() }, {
        Rc::new(RefCell::new(self.borrow().clone()))
    });
}
