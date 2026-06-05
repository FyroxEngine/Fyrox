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
use std::{
    any::type_name,
    cell::{Cell, RefCell},
    fmt::Debug,
    ops::{Deref, DerefMut, Range},
    path::PathBuf,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};
use uuid::uuid;

impl_reflect! {
    #[reflect(type_uuid = "a1ac7e15-e67d-4df6-b3f6-ed11e1dc15a2")]
    pub struct Const<const R: usize>;
}

impl_reflect! {
    #[reflect(type_uuid = "53ca268f-7f7a-4b1c-bdee-af49f2d89945")]
    pub struct Matrix<T: Reflect + Copy, R: Dim + Reflect, C: Dim + Reflect, S: Copy + Reflect> {
        pub data: S,
        // _phantoms: PhantomData<(T, R, C)>,
    }
}

impl_reflect! {
    #[reflect(type_uuid = "469986cd-6611-4e61-b6f0-bd2d984913c4")]
    pub struct ArrayStorage<T: Copy + Reflect, const R: usize, const C: usize>(pub [[T; R]; C]);
}

impl_reflect! {
    #[reflect(type_uuid = "470e104b-992f-405f-ba6c-c2634502a668")]
    pub struct Unit<T: Copy + Reflect> {
        // pub(crate) value: T,
    }
}

impl_reflect! {
    #[reflect(type_uuid = "3b45cc6d-6db4-4fa3-9d96-edcc87d47919")]
    pub struct Quaternion<T: Copy + Debug + Reflect> {
        pub coords: Vector4<T>,
    }
}

impl_reflect! {
    #[reflect(type_uuid = "399bf5fd-e506-4874-9d2f-ca40ed11bc5d")]
    pub struct Complex<T: Copy + Debug + Reflect> {
        pub re: T,
        pub im: T,
    }
}

impl Reflect for f32 {
    blank_reflect!("479e29c6-85fd-4bb8-b311-7b98793b8bf6");
}

impl Reflect for f64 {
    blank_reflect!("dac09d54-d069-47f4-aa0e-aa0057cc2b52");
}

impl Reflect for usize {
    blank_reflect!("620e24e3-fb51-48c6-a885-91d65135c5c9");
}

impl Reflect for u8 {
    blank_reflect!("7a8c337c-0219-466b-92b5-81460fa9c836");
}

impl Reflect for i8 {
    blank_reflect!("3036f00e-5986-4ac3-8763-19e51d0889d7");
}

impl Reflect for u16 {
    blank_reflect!("c662169d-cc3b-453c-bdf3-e0104ac3b966");
}

impl Reflect for i16 {
    blank_reflect!("abce35a9-5e7b-4f7e-a729-2620a9806a6b");
}

impl Reflect for u32 {
    blank_reflect!("8c4d2541-76a5-4dd8-9eb1-10222d2d6912");
}

impl Reflect for i32 {
    blank_reflect!("7413ddd4-71ce-484d-a808-4f3479f5712d");
}

impl Reflect for u64 {
    blank_reflect!("d1a45bd5-5066-4b28-b103-95c59c230e77");
}

impl Reflect for i64 {
    blank_reflect!("35b89368-805f-486d-b3b1-fd3e86b5d645");
}

impl Reflect for isize {
    blank_reflect!("0a06591a-1c66-4299-ba6f-2b205b795575");
}

impl Reflect for bool {
    blank_reflect!("3b104074-9d39-4a2b-b974-da8cc1759fe8");
}

impl Reflect for PathBuf {
    blank_reflect!("3b104074-9d39-4a2b-b974-da8cc1759666");
}

impl Reflect for String {
    blank_reflect!("3b104074-9d39-4a2b-b974-da8cc1759999");
}

impl Reflect for char {
    blank_reflect!("9b5050ef-b3e5-41d2-90f8-8273bcdf7bfb");
}

impl Reflect for Duration {
    blank_reflect!("da291c90-a796-439e-8d0e-a3e206693813");
}

impl Reflect for Instant {
    blank_reflect!("68bb51d9-7fb6-4391-b05d-59137234577d");
}

impl Reflect for ImmutableString {
    blank_reflect!("452caac1-19f7-43d6-9e33-92c2c9163332");
}

impl<T0> Reflect for (T0,)
where
    T0: Clone + Reflect,
{
    blank_reflect!("874c2dd7-8e5c-44dd-a4f0-b6e64e07113d");
}

impl<T0, T1> Reflect for (T0, T1)
where
    T0: Clone + Reflect,
    T1: Clone + Reflect,
{
    blank_reflect!("2722316a-172e-4c95-8cb6-b75c81454233");
}

impl<T0, T1, T2> Reflect for (T0, T1, T2)
where
    T0: Clone + Reflect,
    T1: Clone + Reflect,
    T2: Clone + Reflect,
{
    blank_reflect!("99983877-3ed7-4e58-aef0-3a5951789900");
}

impl<T0, T1, T2, T3> Reflect for (T0, T1, T2, T3)
where
    T0: Clone + Reflect,
    T1: Clone + Reflect,
    T2: Clone + Reflect,
    T3: Clone + Reflect,
{
    blank_reflect!("1d9b68be-0bf4-498a-a337-d84960d08049");
}

impl<T0, T1, T2, T3, T4> Reflect for (T0, T1, T2, T3, T4)
where
    T0: Clone + Reflect,
    T1: Clone + Reflect,
    T2: Clone + Reflect,
    T3: Clone + Reflect,
    T4: Clone + Reflect,
{
    blank_reflect!("02a92848-9a7b-4bdf-8cc0-0e525a1a8021");
}

impl Reflect for () {
    blank_reflect!("fb11afff-0afc-467f-b271-7c5b94ea159d");
}

impl_reflect! {
    #[reflect(type_uuid = "6f5cc9a7-e8ce-4841-8c82-30007c9c1039")]
    pub struct Uuid;
}

impl_reflect! {
    #[reflect(type_uuid = "c0112c31-73fb-4457-a851-a15c0bae00e6")]
    pub struct Cell<T: Reflect + Debug + Copy>;
}

impl_reflect! {
    #[reflect(type_uuid = "99c30632-b72f-4460-8f3b-c7d3c47b1702")]
    pub enum Option<T: Clone + Debug + Reflect> {
        Some(T),
        None
    }
}

impl_reflect! {
    #[reflect(type_uuid = "0e37a8f6-fae7-48a7-b7b4-f824e40d6d7e")]
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
            type_uuid: combine_uuids(
                uuid!("bc74add8-4b4a-4b9f-84ee-b67b21bdad1d"),
                T::type_info().type_uuid,
            ),
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
                type_uuid: T::type_info().type_uuid
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
