//! Test cases for `fyrox_core::reflect::Reflect`

use std::ops::{Deref, DerefMut};

use fyrox_core::reflect::*;

#[allow(dead_code)]
#[derive(Reflect)]
pub struct Struct {
    field: usize,
    #[reflect(hidden)]
    hidden: usize,
}

#[derive(Reflect)]
pub struct Tuple(usize, usize);

#[derive(Reflect)]
pub enum Enum {
    Named { field: usize },
    Tuple(usize),
    Unit,
}

#[test]
fn property_constants() {
    assert_eq!(Struct::FIELD, "field");

    // hidden fields don't expose their keys
    // assert_eq!(SStruct::HIDDEN, "hidden");

    assert_eq!(Tuple::F_0, "0");
    assert_eq!(Tuple::F_1, "1");

    assert_eq!(Enum::NAMED_FIELD, "Named@field");
    assert_eq!(Enum::TUPLE_F_0, "Tuple@0");
}

#[test]
fn reflect_field_accessors() {
    let mut s = Struct {
        field: 10,
        hidden: 10,
    };

    assert_eq!(s.get_field::<usize>(Struct::FIELD), Some(&10));
    assert_eq!(s.get_field_mut::<usize>(Struct::FIELD), Some(&mut 10));
    *s.get_field_mut::<usize>(Struct::FIELD).unwrap() = 100;
    assert_eq!(s.get_field::<usize>(Struct::FIELD), Some(&100));

    assert!(s.get_field::<usize>("HIDDEN").is_none());

    let mut t = Tuple(0, 1);

    assert_eq!(t.get_field::<usize>(Tuple::F_0), Some(&0));
    assert_eq!(t.get_field::<usize>(Tuple::F_1), Some(&1));

    *t.get_field_mut::<usize>(Tuple::F_0).unwrap() += 10;
    *t.get_field_mut::<usize>(Tuple::F_1).unwrap() += 10;

    assert_eq!(t.get_field::<usize>(Tuple::F_0), Some(&10));
    assert_eq!(t.get_field::<usize>(Tuple::F_1), Some(&11));

    let mut e_named = Enum::Named { field: 10 };

    assert_eq!(e_named.get_field::<usize>(Enum::NAMED_FIELD), Some(&10));
    assert_eq!(e_named.get_field::<usize>(Enum::TUPLE_F_0), None);
    *e_named.get_field_mut::<usize>(Enum::NAMED_FIELD).unwrap() = 20usize;
    assert_eq!(e_named.get_field::<usize>(Enum::NAMED_FIELD), Some(&20));

    let mut e_tuple = Enum::Tuple(30);

    assert_eq!(e_tuple.get_field::<usize>(Enum::NAMED_FIELD), None);
    assert_eq!(e_tuple.get_field::<usize>(Enum::TUPLE_F_0), Some(&30));
    *e_tuple.get_field_mut::<usize>(Enum::TUPLE_F_0).unwrap() = 40usize;
    assert_eq!(e_tuple.get_field::<usize>(Enum::TUPLE_F_0), Some(&40));
}

#[test]
fn reflect_containers() {
    struct DerefContainer<T> {
        data: T,
    }

    impl<T> Deref for DerefContainer<T> {
        type Target = T;
        fn deref(&self) -> &Self::Target {
            &self.data
        }
    }

    impl<T> DerefMut for DerefContainer<T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.data
        }
    }

    #[derive(Reflect)]
    struct X {
        #[reflect(deref)]
        container: DerefContainer<Struct>,
    }

    let x = X {
        container: DerefContainer {
            data: Struct {
                field: 0,
                hidden: 1,
            },
        },
    };

    assert!(x.get_resolve_path::<usize>("container.data.field").is_err());

    assert_eq!(x.get_resolve_path::<usize>("container.field"), Ok(&0));

    #[derive(Reflect)]
    #[reflect(bounds = "T: Reflect")]
    struct B<T> {
        #[reflect(deref)]
        data: Box<T>,
    }

    let b = B {
        data: Box::new(Struct {
            field: 10,
            hidden: 11,
        }),
    };

    assert_eq!(b.get_resolve_path::<usize>("data.field"), Ok(&10));
    assert!(x.get_resolve_path::<usize>("data.hidden").is_err());
}

#[test]
fn reflect_path() {
    #[derive(Reflect)]
    struct Hierarchy {
        s: Struct,
        e: Enum,
    }

    let mut hie = Hierarchy {
        s: Struct {
            field: 1,
            hidden: 2,
        },
        e: Enum::Tuple(10),
    };

    assert_eq!(hie.get_resolve_path::<usize>("s.field"), Ok(&1));
    assert_eq!(hie.get_resolve_path::<usize>("e.Tuple@0"), Ok(&10));

    assert_eq!(hie.get_resolve_path_mut::<usize>("s.field"), Ok(&mut 1));
    assert_eq!(hie.get_resolve_path_mut::<usize>("e.Tuple@0"), Ok(&mut 10));
}

#[test]
fn reflect_list() {
    let mut data = vec![10usize, 11usize];

    let data = &mut data as &mut dyn Reflect;
    let data = data.as_list_mut().unwrap();

    assert_eq!(data.get_reflect_index(0), Some(&10usize));
    assert_eq!(data.get_reflect_index::<usize>(2), None);

    data.reflect_push(Box::new(12usize)).unwrap();
    assert_eq!(data.get_reflect_index(2), Some(&12usize));
}

#[test]
fn reflect_list_path() {
    let data = vec![vec![0usize, 1], vec![2, 3, 4]];
    assert_eq!(data.get_resolve_path("[0][1]"), Ok(&1usize));

    #[derive(Reflect)]
    struct X {
        data: Vec<usize>,
    }

    #[derive(Reflect)]
    struct A {
        xs: Vec<X>,
    }

    let a = A {
        xs: vec![
            X { data: vec![0, 1] },
            X {
                data: vec![2, 3, 4],
            },
        ],
    };

    assert_eq!(a.get_resolve_path("xs[0].data[1]"), Ok(&1usize));
}

#[test]
fn reflect_custom_setter() {
    #[derive(Reflect)]
    pub struct Wrapper<T> {
        #[reflect(setter = "set_value")]
        value: T,
        is_dirty: bool,
    }

    impl<T> Wrapper<T> {
        pub fn set_value(&mut self, value: T) -> T {
            self.is_dirty = true;
            std::mem::replace(&mut self.value, value)
        }
    }

    let mut wrapper = Wrapper {
        value: 0.0f32,
        is_dirty: false,
    };

    let value = 10.0f32;
    assert!(wrapper
        .set_field(Wrapper::<()>::VALUE, Box::new(value))
        .is_ok());
    assert!(wrapper.is_dirty);
}
