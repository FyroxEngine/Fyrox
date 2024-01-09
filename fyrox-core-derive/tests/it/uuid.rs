use fyrox_core::type_traits::prelude::*;
use std::marker::PhantomData;

#[derive(TypeUuidProvider)]
#[type_uuid(id = "5fb10a22-4ea9-4a13-a58c-82f2734aefd8")]
struct Foo {}

#[derive(TypeUuidProvider)]
#[type_uuid(id = "5fb10a22-4ea9-4a13-a58c-82f2734aefd9")]
struct Bar<T> {
    phantom: PhantomData<T>,
}
