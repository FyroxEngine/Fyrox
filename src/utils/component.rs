//! Component provider provides dynamic access to inner components of an object by their type id.

use std::any::{Any, TypeId};

/// Component provider provides dynamic access to inner components of an object by their type id.
pub trait ComponentProvider {
    /// Allows an object to provide access to inner components.
    fn query_component_ref(&self, type_id: TypeId) -> Option<&dyn Any>;

    /// Allows an object to provide access to inner components.
    fn query_component_mut(&mut self, type_id: TypeId) -> Option<&mut dyn Any>;
}

/// Implements [`ComponentProvider::query_component_ref`] and [`ComponentProvider::query_component_mut`] in a much
/// shorter way.
#[macro_export]
macro_rules! impl_component_provider {
     ($dest_type:ty) => {
        impl $crate::utils::component::ComponentProvider for $dest_type {
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

    ($dest_type:ty, $($comp_field:ident: $comp_type:ty),*) => {
        impl $crate::utils::component::ComponentProvider for $dest_type {
            fn query_component_ref(&self, type_id: std::any::TypeId) -> Option<&dyn std::any::Any> {
                if type_id == std::any::TypeId::of::<Self>() {
                    return Some(self);
                }

                $(
                    if type_id == std::any::TypeId::of::<$comp_type>() {
                        return Some(&self.$comp_field)
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
                        return Some(&mut self.$comp_field)
                    }
                )*

                None
            }
        }
    };
}
