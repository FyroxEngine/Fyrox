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

use crate::math::Rect;
pub use fyrox_core_derive::TypeUuidProvider;
use nalgebra::{SMatrix, Vector2, Vector3, Vector4};
use std::path::PathBuf;
use uuid::Uuid;

pub mod prelude {
    pub use super::{combine_uuids, TypeUuidProvider};
    pub use uuid::{uuid, Uuid};
}

/// A trait for an entity that has unique type identifier.
pub trait TypeUuidProvider: Sized {
    /// Return type UUID.
    fn type_uuid() -> Uuid;
}

#[macro_export]
macro_rules! uuid_provider {
    ($type:ty = $uuid:expr) => {
        impl $crate::type_traits::TypeUuidProvider for $type {
            fn type_uuid() -> $crate::uuid::Uuid {
                $crate::uuid::uuid!($uuid)
            }
        }
    };
}

#[macro_export]
macro_rules! stub_uuid_provider {
    ($type:ty) => {
        impl $crate::TypeUuidProvider for $type {
            fn type_uuid() -> $crate::uuid::Uuid {
                unimplemented!()
            }
        }
    };
}

uuid_provider!(u8 = "7a8c337c-0219-466b-92b5-81460fa9c836");
uuid_provider!(i8 = "3036f00e-5986-4ac3-8763-19e51d0889d7");
uuid_provider!(u16 = "c662169d-cc3b-453c-bdf3-e0104ac3b966");
uuid_provider!(i16 = "abce35a9-5e7b-4f7e-a729-2620a9806a6b");
uuid_provider!(u32 = "8c4d2541-76a5-4dd8-9eb1-10222d2d6912");
uuid_provider!(i32 = "7413ddd4-71ce-484d-a808-4f3479f5712d");
uuid_provider!(u64 = "d1a45bd5-5066-4b28-b103-95c59c230e77");
uuid_provider!(i64 = "35b89368-805f-486d-b3b1-fd3e86b5d645");
uuid_provider!(f32 = "479e29c6-85fd-4bb8-b311-7b98793b8bf6");
uuid_provider!(f64 = "dac09d54-d069-47f4-aa0e-aa0057cc2b52");
uuid_provider!(usize = "620e24e3-fb51-48c6-a885-91d65135c5c9");
uuid_provider!(isize = "0a06591a-1c66-4299-ba6f-2b205b795575");
uuid_provider!(bool = "3b104074-9d39-4a2b-b974-da8cc1759fe8");
uuid_provider!(PathBuf = "3b104074-9d39-4a2b-b974-da8cc1759666");
uuid_provider!(String = "3b104074-9d39-4a2b-b974-da8cc1759999");
uuid_provider!(char = "9b5050ef-b3e5-41d2-90f8-8273bcdf7bfb");

uuid_provider!(Vector2<f32> = "79d4fae7-27f7-4d28-ac3e-6a569b025a82");
uuid_provider!(Vector3<f32> = "85e32efb-1784-46f7-8ec0-8ee038661ed4");
uuid_provider!(Vector4<f32> = "c5222adb-5b68-4105-93e9-4ecaee39987f");

uuid_provider!(SMatrix<f32,2,2> = "9ff7f3d0-6c2c-4282-9b90-1822f3818559");
uuid_provider!(SMatrix<f32,3,3> = "ca636377-9078-4d60-ac6a-e275e88a7e30");
uuid_provider!(SMatrix<f32,4,4> = "b822b0c6-b396-4950-ba03-40cebad0bfc1");
uuid_provider!(SMatrix<f64,2,2> = "3d51e09c-df48-4d22-84f7-ae805a24562a");
uuid_provider!(SMatrix<f64,3,3> = "a3493898-6a81-40e1-9e36-9389587d0f1e");
uuid_provider!(SMatrix<f64,4,4> = "1277658b-9da7-40fa-98ee-cbf6050c60a4");

impl<T: TypeUuidProvider> TypeUuidProvider for Rect<T> {
    fn type_uuid() -> Uuid {
        combine_uuids(
            uuid::uuid!("0f88dcde-f145-4ba0-a5c1-cf5036fa0706"),
            T::type_uuid(),
        )
    }
}

impl<T: TypeUuidProvider> TypeUuidProvider for Option<T> {
    fn type_uuid() -> Uuid {
        combine_uuids(
            uuid::uuid!("ffe06d3b-0d07-42cd-886b-5248f6ca7f7d"),
            T::type_uuid(),
        )
    }
}

impl<T: TypeUuidProvider> TypeUuidProvider for Vec<T> {
    fn type_uuid() -> Uuid {
        combine_uuids(
            uuid::uuid!("51bc577b-5a50-4a97-9b31-eda2f3d46c9c"),
            T::type_uuid(),
        )
    }
}

#[inline]
pub fn combine_uuids(a: Uuid, b: Uuid) -> Uuid {
    let mut combined_bytes = a.into_bytes();

    for (src, dest) in b.into_bytes().into_iter().zip(combined_bytes.iter_mut()) {
        *dest ^= src;
    }

    Uuid::from_bytes(combined_bytes)
}
