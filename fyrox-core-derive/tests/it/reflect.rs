//! Test cases for `fyrox_core::reflect::Reflect`

#![allow(clippy::disallowed_names)] // Useless in tests

use std::{
    any::TypeId,
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use fyrox_core::{parking_lot::Mutex, reflect::*};

/// Struct doc comment.
#[allow(dead_code)]
#[derive(Reflect, Debug, Clone)]
pub struct Struct {
    /// This is a
    /// multiline doc comment.
    field: usize,
    #[reflect(hidden)]
    hidden: usize,
}

#[derive(Reflect, Debug)]
pub struct Tuple(usize, usize);

#[derive(Reflect, Debug)]
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
fn doc_comments() {
    let s = Struct {
        field: 0,
        hidden: 0,
    };
    s.fields_info(&mut |infos| {
        assert_eq!(
            infos[0].doc,
            " This is a \
 multiline doc comment."
        );
    });
    assert_eq!(s.doc(), " Struct doc comment.");
}

#[test]
fn reflect_field_accessors() {
    let mut s = Struct {
        field: 10,
        hidden: 10,
    };

    s.get_field::<usize>(Struct::FIELD, &mut |field| assert_eq!(field, Some(&10)));
    s.get_field_mut::<usize>(Struct::FIELD, &mut |field| assert_eq!(field, Some(&mut 10)));
    s.get_field_mut::<usize>(Struct::FIELD, &mut |field| *field.unwrap() = 100);
    s.get_field::<usize>(Struct::FIELD, &mut |field| assert_eq!(field, Some(&100)));

    s.get_field::<usize>("HIDDEN", &mut |field| assert!(field.is_none()));

    let mut t = Tuple(0, 1);

    t.get_field::<usize>(Tuple::F_0, &mut |field| assert_eq!(field, Some(&0)));
    t.get_field::<usize>(Tuple::F_1, &mut |field| assert_eq!(field, Some(&1)));

    t.get_field_mut::<usize>(Tuple::F_0, &mut |field| *field.unwrap() += 10);
    t.get_field_mut::<usize>(Tuple::F_1, &mut |field| *field.unwrap() += 10);

    t.get_field::<usize>(Tuple::F_0, &mut |field| assert_eq!(field, Some(&10)));
    t.get_field::<usize>(Tuple::F_1, &mut |field| assert_eq!(field, Some(&11)));

    let mut e_named = Enum::Named { field: 10 };

    e_named.get_field::<usize>(Enum::NAMED_FIELD, &mut |field| assert_eq!(field, Some(&10)));
    e_named.get_field::<usize>(Enum::TUPLE_F_0, &mut |field| assert_eq!(field, None));
    e_named.get_field_mut::<usize>(Enum::NAMED_FIELD, &mut |field| *field.unwrap() = 20usize);
    e_named.get_field::<usize>(Enum::NAMED_FIELD, &mut |field| assert_eq!(field, Some(&20)));

    let mut e_tuple = Enum::Tuple(30);

    e_tuple.get_field::<usize>(Enum::NAMED_FIELD, &mut |field| assert_eq!(field, None));
    e_tuple.get_field::<usize>(Enum::TUPLE_F_0, &mut |field| assert_eq!(field, Some(&30)));
    e_tuple.get_field_mut::<usize>(Enum::TUPLE_F_0, &mut |field| *field.unwrap() = 40usize);
    e_tuple.get_field::<usize>(Enum::TUPLE_F_0, &mut |field| assert_eq!(field, Some(&40)));
}

#[test]
fn reflect_containers() {
    #[derive(Debug)]
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

    #[derive(Reflect, Debug)]
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

    x.get_resolve_path::<usize>("container.data.field", &mut |result| {
        assert!(result.is_err())
    });

    x.get_resolve_path::<usize>("container.field", &mut |result| assert_eq!(result, Ok(&0)));

    #[derive(Reflect, Debug)]
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

    b.get_resolve_path::<usize>("data.field", &mut |result| assert_eq!(result, Ok(&10)));
    x.get_resolve_path::<usize>("data.hidden", &mut |result| assert!(result.is_err()));
}

#[test]
fn reflect_path() {
    #[derive(Reflect, Debug)]
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

    hie.get_resolve_path::<usize>("s.field", &mut |result| assert_eq!(result, Ok(&1)));
    hie.get_resolve_path::<usize>("e.Tuple@0", &mut |result| assert_eq!(result, Ok(&10)));

    hie.get_resolve_path_mut::<usize>("s.field", &mut |result| assert_eq!(result, Ok(&mut 1)));
    hie.get_resolve_path_mut::<usize>("e.Tuple@0", &mut |result| assert_eq!(result, Ok(&mut 10)));
}

#[test]
fn reflect_list() {
    let mut data = vec![10usize, 11usize];

    let data = &mut data as &mut dyn Reflect;
    data.as_list_mut(&mut |data| {
        let data = data.unwrap();
        data.get_reflect_index(0, &mut |result| assert_eq!(result, Some(&10usize)));
        data.get_reflect_index::<usize>(2, &mut |result| assert_eq!(result, None));

        data.reflect_push(Box::new(12usize)).unwrap();
        data.get_reflect_index(2, &mut |result| assert_eq!(result, Some(&12usize)));
    });
}

#[test]
fn reflect_list_path() {
    let data = vec![vec![0usize, 1], vec![2, 3, 4]];
    data.get_resolve_path("[0][1]", &mut |result| assert_eq!(result, Ok(&1usize)));

    #[derive(Reflect, Debug)]
    struct X {
        data: Vec<usize>,
    }

    #[derive(Reflect, Debug)]
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

    a.get_resolve_path("xs[0].data[1]", &mut |result| {
        assert_eq!(result, Ok(&1usize))
    });
}

#[test]
fn reflect_custom_setter() {
    #[derive(Reflect, Debug)]
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
    wrapper.set_field(Wrapper::<()>::VALUE, Box::new(value), &mut |result| {
        assert!(result.is_ok())
    });
    assert!(wrapper.is_dirty);
}

#[test]
fn reflect_fields_list_of_struct() {
    #[derive(Reflect, Debug)]
    struct Foo {
        field_a: f32,
        field_b: String,
    }

    let foo = Foo {
        field_a: 1.23,
        field_b: "Foobar".to_string(),
    };

    foo.fields(&mut |fields| assert_eq!(fields.len(), 2));
    foo.fields(&mut |fields| {
        fields[0].downcast_ref::<f32>(&mut |result| assert_eq!(result.cloned(), Some(1.23)))
    });
    foo.fields(&mut |fields| {
        fields[1].downcast_ref::<String>(&mut |result| {
            assert_eq!(result.cloned(), Some("Foobar".to_string()))
        })
    });
}

#[test]
fn reflect_fields_list_of_enum() {
    #[derive(Reflect, Debug)]
    enum Foo {
        Bar { field_a: f32 },
        Baz { field_b: u32, field_c: String },
    }

    let bar_variant = Foo::Bar { field_a: 1.23 };

    bar_variant.fields(&mut |fields| assert_eq!(fields.len(), 1));
    bar_variant.fields(&mut |fields| {
        fields[0].downcast_ref::<f32>(&mut |result| assert_eq!(result.cloned(), Some(1.23)))
    });

    let baz_variant = Foo::Baz {
        field_b: 321,
        field_c: "Foobar".to_string(),
    };

    baz_variant.fields(&mut |fields| assert_eq!(fields.len(), 2));
    baz_variant.fields(&mut |fields| {
        fields[0].downcast_ref::<u32>(&mut |result| assert_eq!(result.cloned(), Some(321)))
    });
    baz_variant.fields(&mut |fields| {
        fields[1].downcast_ref::<String>(&mut |result| {
            assert_eq!(result.cloned(), Some("Foobar".to_string()))
        })
    });
}

fn default_prop() -> FieldInfo<'static, 'static> {
    FieldInfo {
        owner_type_id: TypeId::of::<()>(),
        name: "",
        value: &(),
        reflect_value: &(),
        display_name: "",
        read_only: false,
        immutable_collection: false,
        min_value: None,
        max_value: None,
        step: None,
        precision: None,
        description: "",
        type_name: "",
        doc: "",
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
        FieldInfo {
            owner_type_id: TypeId::of::<Data>(),
            name: "the_field",
            display_name: "The Field",
            value: &data.the_field,
            ..default_prop()
        },
        FieldInfo {
            owner_type_id: TypeId::of::<Data>(),
            name: "another_field",
            display_name: "Another Field",
            value: &data.another_field,
            ..default_prop()
        },
    ];

    data.fields_info(&mut |fields_info| assert_eq!(fields_info, expected));
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
        FieldInfo {
            owner_type_id: TypeId::of::<Data>(),
            name: "x",
            display_name: "Super X",
            value: &data.x,
            type_name: std::any::type_name::<f32>(),
            ..default_prop()
        },
        FieldInfo {
            owner_type_id: TypeId::of::<Data>(),
            name: "y",
            display_name: "Y",
            value: &data.y,
            reflect_value: &data.y,
            read_only: true,
            immutable_collection: false,
            min_value: Some(0.1),
            max_value: Some(1.1),
            step: Some(0.1),
            precision: Some(3),
            description: "This is a property description.",
            type_name: std::any::type_name::<f32>(),
            doc: "",
        },
    ];

    data.fields_info(&mut |fields_info| assert_eq!(fields_info[0..2], expected));
}

#[test]
fn inspect_struct() {
    #[derive(Debug, Default, Reflect)]
    struct Tuple(f32, f32);

    let x = Tuple::default();

    x.fields_info(&mut |fields_info| {
        assert_eq!(
            fields_info,
            vec![
                FieldInfo {
                    owner_type_id: TypeId::of::<Tuple>(),
                    name: "0",
                    display_name: "0",
                    value: &x.0,
                    type_name: std::any::type_name::<f32>(),
                    ..default_prop()
                },
                FieldInfo {
                    owner_type_id: TypeId::of::<Tuple>(),
                    name: "1",
                    display_name: "1",
                    value: &x.1,
                    type_name: std::any::type_name::<f32>(),
                    ..default_prop()
                },
            ]
        )
    });

    #[derive(Debug, Default, Reflect)]
    struct Unit;

    let x = Unit;
    x.fields_info(&mut |fields_info| assert_eq!(fields_info, vec![]));
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

    data.fields_info(&mut |fields_info| {
        assert_eq!(
            fields_info,
            vec![
                FieldInfo {
                    owner_type_id: TypeId::of::<Data>(),
                    name: "Named@x",
                    display_name: "X",
                    value: match data {
                        Data::Named { ref x, .. } => x,
                        _ => unreachable!(),
                    },
                    type_name: std::any::type_name::<u32>(),
                    ..default_prop()
                },
                FieldInfo {
                    owner_type_id: TypeId::of::<Data>(),
                    name: "Named@y",
                    display_name: "Y",
                    type_name: std::any::type_name::<u32>(),
                    value: match data {
                        Data::Named { ref y, .. } => y,
                        _ => unreachable!(),
                    },
                    ..default_prop()
                },
                FieldInfo {
                    owner_type_id: TypeId::of::<Data>(),
                    name: "Named@z",
                    display_name: "Z",
                    type_name: std::any::type_name::<NonCopy>(),
                    value: match data {
                        Data::Named { ref z, .. } => z,
                        _ => unreachable!(),
                    },
                    ..default_prop()
                },
            ]
        )
    });

    let data = Data::Tuple(10.0, 20.0);

    data.fields_info(&mut |fields_info| {
        assert_eq!(
            fields_info,
            vec![
                FieldInfo {
                    owner_type_id: TypeId::of::<Data>(),
                    name: "Tuple@0",
                    display_name: "0",
                    type_name: std::any::type_name::<f32>(),
                    value: match data {
                        Data::Tuple(ref f0, ref _f1) => f0,
                        _ => unreachable!(),
                    },
                    ..default_prop()
                },
                FieldInfo {
                    owner_type_id: TypeId::of::<Data>(),
                    name: "Tuple@1",
                    display_name: "1",
                    type_name: std::any::type_name::<f32>(),
                    value: match data {
                        Data::Tuple(ref _f0, ref f1) => f1,
                        _ => unreachable!(),
                    },
                    ..default_prop()
                },
            ]
        )
    });

    // unit variants don't have fields
    let data = Data::Unit;
    data.fields_info(&mut |fields_info| assert_eq!(fields_info, vec![]));
}

#[test]
fn inspect_prop_key_constants() {
    #[allow(dead_code)]
    #[derive(Reflect, Debug)]
    pub struct SStruct {
        field: usize,
        #[reflect(hidden)]
        hidden: usize,
    }

    // NOTE: property names are in snake_case (not Title Case)
    assert_eq!(SStruct::FIELD, "field");

    // hidden properties
    // assert_eq!(SStruct::HIDDEN, "hidden");

    #[derive(Reflect, Debug)]
    pub struct STuple(usize);
    assert_eq!(STuple::F_0, "0");

    #[derive(Reflect, Debug)]
    #[allow(unused)]
    pub enum E {
        Tuple(usize),
        Struct { field: usize },
        Unit,
    }

    assert_eq!(E::TUPLE_F_0, "Tuple@0");
    E::Tuple(0).fields_info(&mut |fields_info| assert_eq!(E::TUPLE_F_0, fields_info[0].name));

    assert_eq!(E::STRUCT_FIELD, "Struct@field");

    E::Struct { field: 0 }
        .fields_info(&mut |fields_info| assert_eq!(E::STRUCT_FIELD, fields_info[0].name));
}

#[test]
fn test_reflect_mutex() {
    let foo = Mutex::new(Struct {
        field: 123,
        hidden: 0,
    });

    foo.get_resolve_path::<usize>("field", &mut |result| {
        assert_eq!(result, Ok(&123));
    });

    foo.get_resolve_path::<u32>("hidden", &mut |result| {
        assert!(result.is_err());
    });
}

#[test]
fn test_hash_map() {
    let mut hash_map = HashMap::new();

    let foo_key = "foo".to_string();
    hash_map.insert(
        foo_key.clone(),
        Struct {
            field: 123,
            hidden: 0,
        },
    );

    let bar_key = "bar".to_string();
    hash_map.insert(
        bar_key.clone(),
        Struct {
            field: 321,
            hidden: 0,
        },
    );

    hash_map.as_hash_map(&mut |result| {
        let hash_map = result.unwrap();
        assert_eq!(hash_map.reflect_len(), 2);

        hash_map.reflect_get(&foo_key, &mut |result| {
            result
                .unwrap()
                .downcast_ref::<Struct>(&mut |result| assert_eq!(result.unwrap().field, 123))
        })
    });

    hash_map.as_hash_map_mut(&mut |result| {
        let hash_map = result.unwrap();
        hash_map.reflect_get_mut(&bar_key, &mut |result| {
            result
                .unwrap()
                .downcast_mut::<Struct>(&mut |result| result.unwrap().field = 555)
        })
    });

    // Check path resolution.
    #[derive(Reflect, Debug)]
    struct Something {
        hash_map: HashMap<String, Struct>,
    }

    let something = Something { hash_map };

    something.get_resolve_path::<usize>("hash_map[foo].field", &mut |result| {
        assert_eq!(*result.unwrap(), 123)
    });
    something.get_resolve_path::<usize>("hash_map[bar].field", &mut |result| {
        assert_eq!(*result.unwrap(), 555)
    });
}
