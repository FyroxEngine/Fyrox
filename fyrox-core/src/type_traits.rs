pub use fyrox_core_derive::ComponentProvider;
pub use fyrox_core_derive::TypeUuidProvider;
use std::any::{Any, TypeId};
use std::path::PathBuf;
use uuid::Uuid;

pub mod prelude {
    pub use super::{combine_uuids, ComponentProvider, TypeUuidProvider};
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
uuid_provider!(PathBuf = "3b104074-9d39-4a2b-b974-da8cc1759666");
uuid_provider!(String = "3b104074-9d39-4a2b-b974-da8cc1759999");

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

/// Component provider provides dynamic access to inner components of an object by their type id.
pub trait ComponentProvider {
    /// Allows an object to provide access to inner components.
    fn query_component_ref(&self, type_id: TypeId) -> Option<&dyn Any>;

    /// Allows an object to provide access to inner components.
    fn query_component_mut(&mut self, type_id: TypeId) -> Option<&mut dyn Any>;
}

impl dyn ComponentProvider {
    /// Tries to borrow a component of given type.
    #[inline]
    pub fn component_ref<T: Any>(&self) -> Option<&T> {
        ComponentProvider::query_component_ref(self, TypeId::of::<T>())
            .and_then(|c| c.downcast_ref())
    }

    /// Tries to borrow a component of given type.
    #[inline]
    pub fn component_mut<T: Any>(&mut self) -> Option<&mut T> {
        ComponentProvider::query_component_mut(self, TypeId::of::<T>())
            .and_then(|c| c.downcast_mut())
    }
}

/// Implements [`ComponentProvider::query_component_ref`] and [`ComponentProvider::query_component_mut`] in a much
/// shorter way.
#[macro_export]
macro_rules! impl_component_provider {
     ($dest_type:ty) => {
        impl $crate::type_traits::ComponentProvider for $dest_type {
            fn query_component_ref(&self, type_id: std::any::TypeId) -> Option<&dyn std::any::Any> {
                if type_id == std::any::TypeId::of::<Self>() {
                    return Some(self);
                }
                None
            }

            fn query_component_mut(
                &mut self,
                type_id: std::any::TypeId,
            ) -> Option<&mut dyn std::any::Any> {
                if type_id == std::any::TypeId::of::<Self>() {
                    return Some(self);
                }
                None
            }
        }
    };

    ($dest_type:ty, $($($comp_field:ident).*: $comp_type:ty),*) => {
        impl $crate::type_traits::ComponentProvider for $dest_type {
            fn query_component_ref(&self, type_id: std::any::TypeId) -> Option<&dyn std::any::Any> {
                if type_id == std::any::TypeId::of::<Self>() {
                    return Some(self);
                }

                $(
                    if type_id == std::any::TypeId::of::<$comp_type>() {
                        return Some(&self.$($comp_field).*)
                    }
                )*

                None
            }

            fn query_component_mut(
                &mut self,
                type_id: std::any::TypeId,
            ) -> Option<&mut dyn std::any::Any> {
                if type_id == std::any::TypeId::of::<Self>() {
                    return Some(self);
                }

                $(
                    if type_id == std::any::TypeId::of::<$comp_type>() {
                        return Some(&mut self.$($comp_field).*)
                    }
                )*

                None
            }
        }
    };
}
