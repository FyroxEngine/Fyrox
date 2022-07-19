//! Test cases for `fyrox_core::reflect::Reflect`

use fyrox_core::reflect::*;

#[test]
fn property_values() {
    #[allow(dead_code)]
    #[derive(Reflect)]
    pub struct SStruct {
        field: usize,
        #[reflect(hidden)]
        hidden: usize,
    }

    // NOTE: property names are in snake_case
    assert_eq!(SStruct::FIELD, "field");

    // hidden fields don't expose their keys
    // assert_eq!(SStruct::HIDDEN, "hidden");

    #[derive(Reflect)]
    pub struct STuple(usize);

    assert_eq!(STuple::F_0, "0");

    #[derive(Reflect)]
    #[allow(unused)]
    pub enum E {
        Tuple(usize),
        Struct { field: usize },
        Unit,
    }

    assert_eq!(E::TUPLE_F_0, "Tuple.0");
    assert_eq!(E::STRUCT_FIELD, "Struct.field");
}

#[test]
fn field_accessors() {
    #[allow(dead_code)]
    #[derive(Reflect)]
    pub struct Struct {
        field: usize,
        #[reflect(hidden)]
        hidden: usize,
    }

    let mut s = Struct {
        field: 10,
        hidden: 10,
    };

    assert_eq!(s.get_field::<usize>(Struct::FIELD), Some(&10));
    assert_eq!(s.get_field_mut::<usize>(Struct::FIELD), Some(&mut 10));
    *s.get_field_mut::<usize>(Struct::FIELD).unwrap() = 100;
    assert_eq!(s.get_field::<usize>(Struct::FIELD), Some(&100));

    assert!(s.get_field::<usize>("HIDDEN").is_none());

    #[allow(dead_code)]
    #[derive(Reflect)]
    pub struct Tuple(usize, usize);

    let mut t = Tuple(0, 1);

    assert_eq!(t.get_field::<usize>(Tuple::F_0), Some(&0));
    assert_eq!(t.get_field::<usize>(Tuple::F_1), Some(&1));

    *t.get_field_mut::<usize>(Tuple::F_0).unwrap() += 10;
    *t.get_field_mut::<usize>(Tuple::F_1).unwrap() += 10;

    assert_eq!(t.get_field::<usize>(Tuple::F_0), Some(&10));
    assert_eq!(t.get_field::<usize>(Tuple::F_1), Some(&11));

    #[allow(dead_code)]
    #[derive(Reflect)]
    pub enum Enum {
        Named { field: usize },
        Tuple(usize),
        Unit,
    }

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
