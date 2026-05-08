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

//! Dynamic type (dyntype for short) is a user-defined data structure that supports additional
//! features which makes it available from the editor and serializable to the standard asset format.

#![warn(missing_docs)]

use crate::{reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*, SafeLock};
use fxhash::FxHashMap;
use parking_lot::{Mutex, MutexGuard};
use std::{
    any::{type_name, Any, TypeId},
    fmt::{Debug, Display, Formatter},
};
use uuid::Uuid;

/// A set of errors that may occur while working with dyntypes.
pub enum DynTypeError {
    /// The inner container was empty. This error indicates that either the dyntype was in default
    /// state, or its value was taken.
    Empty,
    /// Unable to deserialize a dynamic type, because there's no DynTypeConstructorContainer provided!
    NoConstructorContainerProvided,
    /// Unable to deserialize a dynamic type, because there's no constructor provided for the type!
    NoConstructorForTypeUuid(Uuid),
    /// Unable to perform downcasting.
    TypeCast {
        /// The actual name of the type.
        actual_type_name: &'static str,
        /// The name of the requested type.
        requested_type_name: &'static str,
    },
}

impl DynTypeError {
    fn into_boxed(self) -> Box<dyn std::error::Error + Send + Sync> {
        Box::new(self)
    }
}

impl Display for DynTypeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DynTypeError::NoConstructorContainerProvided => {
                write!(
                    f,
                    "Unable to deserialize a dynamic type, because \
                    there's no DynTypeConstructorContainer provided!"
                )
            }
            DynTypeError::NoConstructorForTypeUuid(uuid) => {
                write!(
                    f,
                    "Unable to deserialize a dynamic type, because \
                    there's no constructor provided for the {uuid} type!"
                )
            }
            DynTypeError::Empty => {
                write!(f, "The container is empty")
            }
            DynTypeError::TypeCast {
                actual_type_name,
                requested_type_name,
            } => {
                write!(
                    f,
                    "The actual type ({actual_type_name}) of the \
                dynamic type is different ({requested_type_name})."
                )
            }
        }
    }
}

impl Debug for DynTypeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl std::error::Error for DynTypeError {}

/// Dynamic type (dyntype for short) is a user-defined data structure that supports additional
/// features which makes it available from the editor and serializable to the standard asset format.
pub trait DynType: Reflect + Visit + Debug + FieldValue + Send {
    /// Returns the type uuid of the value.
    fn type_uuid(&self) -> Uuid;
    /// Creates a boxed copy of the value.
    fn clone_box(&self) -> Box<dyn DynType>;
}

impl<T> DynType for T
where
    T: TypeUuidProvider + Clone + Visit + Reflect + FieldValue + Send,
{
    fn type_uuid(&self) -> Uuid {
        <T as TypeUuidProvider>::type_uuid()
    }

    fn clone_box(&self) -> Box<dyn DynType> {
        Box::new(self.clone())
    }
}

impl dyn DynType {
    /// Tries to downcast the boxed version of the dyntype to the specified type.
    pub fn downcast<T: DynType>(self: Box<dyn DynType>) -> Result<Box<T>, Box<dyn DynType>> {
        if self.is::<T>() {
            Ok((self as Box<dyn Any>).downcast().unwrap())
        } else {
            Err(self)
        }
    }

    /// Tries to downcast the boxed version of the dyntype to the specified type, unbox it and
    /// return to the caller.
    pub fn take<T: DynType>(self: Box<dyn DynType>) -> Result<T, Box<dyn DynType>> {
        self.downcast::<T>().map(|value| *value)
    }

    /// Checks whether the inner type is the same as the specified one.
    #[inline]
    pub fn is<T: DynType>(&self) -> bool {
        self.type_id() == TypeId::of::<T>()
    }
}

/// A container for a boxed dyntype.
#[derive(Debug, TypeUuidProvider)]
#[type_uuid(id = "87d9ef74-09a9-4228-a2d1-df270b50fddb")]
pub struct DynTypeWrapper(pub Box<dyn DynType>);

impl Clone for DynTypeWrapper {
    fn clone(&self) -> Self {
        Self(self.0.clone_box())
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

impl Reflect for DynTypeWrapper {
    fn source_path() -> &'static str {
        file!()
    }

    fn derived_types() -> &'static [TypeId] {
        &[]
    }

    fn query_derived_types(&self) -> &'static [TypeId] {
        Self::derived_types()
    }

    fn type_name(&self) -> &'static str {
        type_name::<Self>()
    }

    fn doc(&self) -> &'static str {
        ""
    }

    fn assembly_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }

    fn type_assembly_name() -> &'static str {
        env!("CARGO_PKG_NAME")
    }

    fn fields_ref(&self, func: &mut dyn FnMut(&[FieldRef])) {
        func(&[{
            FieldRef {
                metadata: &CONTENT_METADATA,
                value: &*self.0,
            }
        }])
    }

    fn fields_mut(&mut self, func: &mut dyn FnMut(&mut [FieldMut])) {
        func(&mut [{
            FieldMut {
                metadata: &CONTENT_METADATA,
                value: &mut *self.0,
            }
        }])
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn as_any(&self, func: &mut dyn FnMut(&dyn Any)) {
        func(self)
    }

    fn as_any_mut(&mut self, func: &mut dyn FnMut(&mut dyn Any)) {
        func(self)
    }

    fn as_reflect(&self, func: &mut dyn FnMut(&dyn Reflect)) {
        func(self)
    }

    fn as_reflect_mut(&mut self, func: &mut dyn FnMut(&mut dyn Reflect)) {
        func(self)
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
        let this = std::mem::replace(self, value.take()?);
        Ok(Box::new(this))
    }

    fn try_clone_box(&self) -> Option<Box<dyn Reflect>> {
        Some(Box::new(self.clone()))
    }

    fn get_field_direct_ref(&self, index: usize) -> Option<FieldRef> {
        if index == 0 {
            Some(FieldRef {
                metadata: &CONTENT_METADATA,
                value: &*self.0,
            })
        } else {
            None
        }
    }

    fn get_field_direct_mut(&mut self, index: usize) -> Option<FieldMut> {
        if index == 0 {
            Some(FieldMut {
                metadata: &CONTENT_METADATA,
                value: &mut *self.0,
            })
        } else {
            None
        }
    }
}

/// "Nullable" container for a dyntype. This container is essentially a wrapper for [`Option`] that
/// supports additional functionality and handles serialization for you.
#[derive(Default, Reflect, Clone, Debug)]
pub struct DynTypeContainer(pub Option<DynTypeWrapper>);

impl DynTypeContainer {
    /// Tries to take and downcast the actual value in the container to the specified type and return it
    /// to the caller.
    pub fn try_take<T: DynType>(&mut self) -> Result<T, DynTypeError> {
        match self.0.take() {
            None => Err(DynTypeError::Empty),
            Some(wrapper) => match wrapper.0.take::<T>() {
                Ok(casted) => Ok(casted),
                Err(value) => {
                    let actual_type_name = Reflect::type_name(&*value);
                    self.0.replace(DynTypeWrapper(value));
                    Err(DynTypeError::TypeCast {
                        actual_type_name,
                        requested_type_name: type_name::<T>(),
                    })
                }
            },
        }
    }

    /// Tries to return a reference to the inner value. Returns [`None`] if the container is empty.
    pub fn value_ref(&self) -> Option<&dyn DynType> {
        self.0.as_ref().map(|v| &*v.0)
    }

    /// Tries to return a reference to the inner value. Returns [`None`] if the container is empty.
    pub fn value_mut(&mut self) -> Option<&mut dyn DynType> {
        self.0.as_mut().map(|v| &mut *v.0)
    }

    /// Tries downcast the actual value in the container to the specified type and return it
    /// to the caller.
    pub fn data_ref<T: DynType>(&self) -> Result<&T, DynTypeError> {
        let value = self.value_ref().ok_or(DynTypeError::Empty)?;
        (value as &dyn Any)
            .downcast_ref()
            .ok_or_else(|| DynTypeError::TypeCast {
                actual_type_name: Reflect::type_name(value),
                requested_type_name: type_name::<T>(),
            })
    }

    /// Tries downcast the actual value in the container to the specified type and return it
    /// to the caller.
    pub fn data_mut<T: DynType>(&mut self) -> Result<&mut T, DynTypeError> {
        let value = self.value_mut().ok_or(DynTypeError::Empty)?;
        let actual_type_name = Reflect::type_name(value);
        (value as &mut dyn Any)
            .downcast_mut()
            .ok_or_else(|| DynTypeError::TypeCast {
                actual_type_name,
                requested_type_name: type_name::<T>(),
            })
    }
}

impl Visit for DynTypeContainer {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        let mut uuid = self
            .0
            .as_ref()
            .map(|ty| ty.0.type_uuid())
            .unwrap_or_default();
        uuid.visit("TypeUuid", &mut region)?;

        if region.is_reading() {
            if uuid.is_nil() {
                // Keep the container empty.
            } else {
                let constructors = region
                    .blackboard
                    .get::<DynTypeConstructorContainer>()
                    .ok_or_else(|| DynTypeError::NoConstructorContainerProvided.into_boxed())?;

                let mut data = constructors
                    .try_create(&uuid)
                    .ok_or_else(|| DynTypeError::NoConstructorForTypeUuid(uuid).into_boxed())?;

                data.visit("DynTypeData", &mut region)?;

                self.0 = Some(DynTypeWrapper(data));
            }
        } else if let Some(ty) = self.0.as_mut() {
            ty.0.visit("DynTypeData", &mut region)?;
        }

        Ok(())
    }
}

/// A type alias for a boxed closure that can be used to create an instance of a type that
/// implements [`DynType`] trait.
pub type DynTypeConstructor = Box<dyn Fn() -> Box<dyn DynType> + Send + 'static>;

/// A set of parameters that describes a dyntype constructor. It's main purpose is to attach additional
/// information (such as name) to make it human-friendly.
pub struct DynTypeConstructorDefinition {
    /// A human-readable name of the type that will be constructed by the constructor.
    pub name: String,
    /// A boxed closure that creates instances of a dyntype.
    pub constructor: DynTypeConstructor,
    /// A name of the assembly this constructor is from.
    pub assembly_name: &'static str,
}

/// A set of constructors that allows to create a dyntype by its type uuid.
#[derive(Default)]
pub struct DynTypeConstructorContainer {
    map: Mutex<FxHashMap<Uuid, DynTypeConstructorDefinition>>,
}

impl DynTypeConstructorContainer {
    /// Tries to create a dyntype by its type uuid. Returns [`None`] if there's no constructor
    /// for the given uuid.
    pub fn try_create(&self, type_uuid: &Uuid) -> Option<Box<dyn DynType>> {
        self.map
            .safe_lock()
            .get(type_uuid)
            .map(|c| (*c.constructor)())
    }

    /// Registers a new type `T` that belongs to an entity `A` (usually a plugin) with the
    /// human-readable `name`.
    pub fn add<T, A>(&self, name: &str) -> &Self
    where
        T: TypeUuidProvider + Default + DynType,
        A: Reflect,
    {
        let old = self.map.safe_lock().insert(
            <T as TypeUuidProvider>::type_uuid(),
            DynTypeConstructorDefinition {
                name: name.to_string(),
                constructor: Box::new(|| Box::new(T::default())),
                assembly_name: A::type_assembly_name(),
            },
        );

        assert!(old.is_none());

        self
    }

    /// Adds a new constructor.
    pub fn add_custom(
        &self,
        type_uuid: Uuid,
        constructor: DynTypeConstructorDefinition,
    ) -> Result<(), String> {
        let mut map = self.map.safe_lock();
        let old = map.insert(type_uuid, constructor);

        assert!(old.is_none());

        Ok(())
    }

    /// Removes a constructor at the given uuid.
    pub fn remove(&self, type_uuid: &Uuid) -> Option<DynTypeConstructorDefinition> {
        self.map.safe_lock().remove(type_uuid)
    }

    /// Returns an immutable reference to the inner container of constructors.
    pub fn inner(&self) -> MutexGuard<FxHashMap<Uuid, DynTypeConstructorDefinition>> {
        self.map.safe_lock()
    }

    /// Removes all constructors at once.
    pub fn clear(&self) {
        self.inner().clear();
    }
}
