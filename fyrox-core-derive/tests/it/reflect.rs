//! Test cases for `fyrox_core::reflect::Reflect`

#![allow(clippy::blacklisted_name)] // Useless in tests

use std::any::TypeId;
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

#[test]
fn reflect_fields_list_of_struct() {
    #[derive(Reflect)]
    struct Foo {
        field_a: f32,
        field_b: String,
    }

    let foo = Foo {
        field_a: 1.23,
        field_b: "Foobar".to_string(),
    };

    assert_eq!(foo.fields().len(), 2);
    assert_eq!(foo.fields()[0].downcast_ref::<f32>().cloned(), Some(1.23));
    assert_eq!(
        foo.fields()[1].downcast_ref::<String>().cloned(),
        Some("Foobar".to_string())
    );
}

#[test]
fn reflect_fields_list_of_enum() {
    #[derive(Reflect)]
    enum Foo {
        Bar { field_a: f32 },
        Baz { field_b: u32, field_c: String },
    }

    let bar_variant = Foo::Bar { field_a: 1.23 };

    assert_eq!(bar_variant.fields().len(), 1);
    assert_eq!(
        bar_variant.fields()[0].downcast_ref::<f32>().cloned(),
        Some(1.23)
    );

    let baz_variant = Foo::Baz {
        field_b: 321,
        field_c: "Foobar".to_string(),
    };

    assert_eq!(baz_variant.fields().len(), 2);
    assert_eq!(
        baz_variant.fields()[0].downcast_ref::<u32>().cloned(),
        Some(321)
    );
    assert_eq!(
        baz_variant.fields()[1].downcast_ref::<String>().cloned(),
        Some("Foobar".to_string())
    );
}

fn default_prop() -> Metadata {
    Metadata {
        owner_type_id: TypeId::of::<()>(),
        name: "",
        display_name: "",
        read_only: false,
        min_value: None,
        max_value: None,
        step: None,
        precision: None,
        description: "",
    }
}

#[test]
fn inspect_default() {
    #[derive(Debug, Default, Reflect)]
    pub struct Data {
        the_field: String,
        another_field: f32,
    }

    let data = Data::default();

    let expected = vec![
        Metadata {
            owner_type_id: TypeId::of::<Data>(),
            name: "the_field",
            display_name: "The Field",
            ..default_prop()
        },
        Metadata {
            owner_type_id: TypeId::of::<Data>(),
            name: "another_field",
            display_name: "Another Field",
            ..default_prop()
        },
    ];

    assert_eq!(data.fields_metadata(), expected);
}

#[test]
fn inspect_attributes() {
    #[derive(Debug, Default, Reflect)]
    pub struct AarGee {
        aar: u32,
        gee: u32,
    }

    #[derive(Debug, Default, Reflect)]
    pub struct Data {
        // NOTE: Even though this field is skipped, the next field is given index `1` for simplicity
        #[reflect(hidden)]
        _skipped: u32,
        #[reflect(display_name = "Super X")]
        x: f32,
        #[reflect(
            read_only,
            min_value = 0.1,
            max_value = 1.1,
            step = 0.1,
            precision = 3,
            description = "This is a property description."
        )]
        y: f32,
    }

    let data = Data::default();

    let expected = vec![
        Metadata {
            owner_type_id: TypeId::of::<Data>(),
            name: "x",
            display_name: "Super X",
            ..default_prop()
        },
        Metadata {
            owner_type_id: TypeId::of::<Data>(),
            name: "y",
            display_name: "Y",
            read_only: true,
            min_value: Some(0.1),
            max_value: Some(1.1),
            step: Some(0.1),
            precision: Some(3),
            description: "This is a property description.",
        },
    ];

    assert_eq!(data.fields_metadata()[0..2], expected);
}

#[test]
fn inspect_struct() {
    #[derive(Debug, Default, Reflect)]
    struct Tuple(f32, f32);

    let x = Tuple::default();
    assert_eq!(
        x.fields_metadata(),
        vec![
            Metadata {
                owner_type_id: TypeId::of::<Tuple>(),
                name: "0",
                display_name: "0",
                ..default_prop()
            },
            Metadata {
                owner_type_id: TypeId::of::<Tuple>(),
                name: "1",
                display_name: "1",
                ..default_prop()
            },
        ]
    );

    #[derive(Debug, Default, Reflect)]
    struct Unit;

    let x = Unit::default();
    assert_eq!(x.fields_metadata(), vec![]);
}

#[test]
fn inspect_enum() {
    #[derive(Debug, Reflect)]
    pub struct NonCopy {
        inner: u32,
    }

    #[derive(Debug, Reflect)]
    pub enum Data {
        Named { x: u32, y: u32, z: NonCopy },
        Tuple(f32, f32),
        Unit,
    }

    let data = Data::Named {
        x: 0,
        y: 1,
        z: NonCopy { inner: 10 },
    };

    assert_eq!(
        data.fields_metadata(),
        vec![
            Metadata {
                owner_type_id: TypeId::of::<Data>(),
                name: "Named@x",
                display_name: "X",
                ..default_prop()
            },
            Metadata {
                owner_type_id: TypeId::of::<Data>(),
                name: "Named@y",
                display_name: "Y",
                ..default_prop()
            },
            Metadata {
                owner_type_id: TypeId::of::<Data>(),
                name: "Named@z",
                display_name: "Z",
                ..default_prop()
            },
        ]
    );

    let data = Data::Tuple(10.0, 20.0);

    assert_eq!(
        data.fields_metadata(),
        vec![
            Metadata {
                owner_type_id: TypeId::of::<Data>(),
                name: "Tuple@0",
                display_name: "0",
                ..default_prop()
            },
            Metadata {
                owner_type_id: TypeId::of::<Data>(),
                name: "Tuple@1",
                display_name: "1",
                ..default_prop()
            },
        ]
    );

    // unit variants don't have fields
    let data = Data::Unit;
    assert_eq!(data.fields_metadata(), vec![]);
}

#[test]
fn inspect_prop_key_constants() {
    #[allow(dead_code)]
    #[derive(Reflect)]
    pub struct SStruct {
        field: usize,
        #[reflect(hidden)]
        hidden: usize,
    }

    // NOTE: property names are in snake_case (not Title Case)
    assert_eq!(SStruct::FIELD, "field");

    // hidden properties
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

    assert_eq!(E::TUPLE_F_0, "Tuple@0");
    assert_eq!(E::TUPLE_F_0, E::Tuple(0).fields_metadata()[0].name);

    assert_eq!(E::STRUCT_FIELD, "Struct@field");
    assert_eq!(
        E::STRUCT_FIELD,
        E::Struct { field: 0 }.fields_metadata()[0].name
    );
}
