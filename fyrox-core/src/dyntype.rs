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

use crate::reflect::Reflect;
use crate::visitor::{Visit, VisitResult, Visitor};
use crate::{SafeLock, TypeUuidProvider};
use fxhash::FxHashMap;
use parking_lot::Mutex;
use std::fmt::{Debug, Display, Formatter};
use uuid::Uuid;

pub enum DynTypeError {
    NoConstructorContainerProvided,
    NoConstructorForTypeUuid(Uuid),
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
        }
    }
}

impl Debug for DynTypeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl std::error::Error for DynTypeError {}

pub trait DynType: Reflect + Visit {
    fn type_uuid(&self) -> Uuid;
}

#[derive(Default)]
pub struct DynTypeContainer(Option<Box<dyn DynType>>);

impl Visit for DynTypeContainer {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        let mut uuid = self.0.as_ref().map(|ty| ty.type_uuid()).unwrap_or_default();
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

                self.0 = Some(data);
            }
        } else if let Some(ty) = self.0.as_mut() {
            ty.visit("DynTypeData", &mut region)?;
        }

        Ok(())
    }
}

pub type DynTypeConstructor = Box<dyn Fn() -> Box<dyn DynType>>;

pub struct DynTypeConstructorContainer {
    map: Mutex<FxHashMap<Uuid, DynTypeConstructor>>,
}

impl DynTypeConstructorContainer {
    pub fn try_create(&self, type_uuid: &Uuid) -> Option<Box<dyn DynType>> {
        self.map.safe_lock().get(type_uuid).map(|c| (*c)())
    }

    pub fn add<T>(&self) -> &Self
    where
        T: TypeUuidProvider + Default + DynType,
    {
        let old = self.map.safe_lock().insert(
            <T as TypeUuidProvider>::type_uuid(),
            Box::new(|| Box::new(T::default())),
        );

        assert!(old.is_none());

        self
    }

    pub fn add_custom(
        &self,
        type_uuid: Uuid,
        constructor: DynTypeConstructor,
    ) -> Result<(), String> {
        let mut map = self.map.safe_lock();
        let old = map.insert(type_uuid, constructor);

        assert!(old.is_none());

        Ok(())
    }
}
