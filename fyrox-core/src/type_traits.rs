pub use fyrox_core_derive::TypeUuidProvider;
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
    ($type:ident $(<$($generics:tt),*>)? = $uuid:expr) => {
        impl$(<$($generics),*>)? $crate::type_traits::TypeUuidProvider for $type $(<$($generics),*>)? {
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

impl<T: TypeUuidProvider> TypeUuidProvider for Option<T> {
    fn type_uuid() -> Uuid {
        combine_uuids(
            uuid::uuid!("ffe06d3b-0d07-42cd-886b-5248f6ca7f7d"),
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
