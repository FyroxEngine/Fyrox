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

//! Test cases for `fyrox_core::reflect::Reflect`

#![allow(clippy::disallowed_names)] // Useless in tests

use std::{
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

#[derive(Reflect, Clone, Debug)]
pub struct Tuple(usize, usize);

#[derive(Reflect, Clone, Debug)]
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
    s.fields_ref(&mut |infos| {
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
    #[derive(Debug, Clone)]
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

    #[derive(Reflect, Clone, Debug)]
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

    #[derive(Reflect, Clone, Debug)]
    #[reflect(bounds = "T: Reflect + Clone")]
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
    #[derive(Reflect, Clone, Debug)]
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

    #[derive(Reflect, Clone, Debug)]
    struct X {
        data: Vec<usize>,
    }

    #[derive(Reflect, Clone, Debug)]
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
    #[derive(Reflect, Clone, Debug)]
    #[reflect(bounds = "T: Reflect + Clone")]
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
    #[derive(Reflect, Clone, Debug)]
    struct Foo {
        field_a: f32,
        field_b: String,
    }

    let foo = Foo {
        field_a: 1.23,
        field_b: "Foobar".to_string(),
    };

    foo.fields_ref(&mut |fields| assert_eq!(fields.len(), 2));
    foo.fields_ref(&mut |fields| {
        fields[0]
            .value
            .field_value_as_reflect()
            .downcast_ref::<f32>(&mut |result| assert_eq!(result.cloned(), Some(1.23)))
    });
    foo.fields_ref(&mut |fields| {
        fields[1]
            .value
            .field_value_as_reflect()
            .downcast_ref::<String>(&mut |result| {
                assert_eq!(result.cloned(), Some("Foobar".to_string()))
            })
    });
}

#[test]
fn reflect_fields_list_of_enum() {
    #[derive(Reflect, Clone, Debug)]
    enum Foo {
        Bar { field_a: f32 },
        Baz { field_b: u32, field_c: String },
    }

    let bar_variant = Foo::Bar { field_a: 1.23 };

    bar_variant.fields_ref(&mut |fields| assert_eq!(fields.len(), 1));
    bar_variant.fields_ref(&mut |fields| {
        fields[0]
            .value
            .field_value_as_reflect()
            .downcast_ref::<f32>(&mut |result| assert_eq!(result.cloned(), Some(1.23)))
    });

    let baz_variant = Foo::Baz {
        field_b: 321,
        field_c: "Foobar".to_string(),
    };

    baz_variant.fields_ref(&mut |fields| assert_eq!(fields.len(), 2));
    baz_variant.fields_ref(&mut |fields| {
        fields[0]
            .value
            .field_value_as_reflect()
            .downcast_ref::<u32>(&mut |result| assert_eq!(result.cloned(), Some(321)))
    });
    baz_variant.fields_ref(&mut |fields| {
        fields[1]
            .value
            .field_value_as_reflect()
            .downcast_ref::<String>(&mut |result| {
                assert_eq!(result.cloned(), Some("Foobar".to_string()))
            })
    });
}

fn default_prop_metadata() -> FieldMetadata<'static> {
    FieldMetadata {
        name: "",
        display_name: "",
        read_only: false,
        immutable_collection: false,
        min_value: None,
        max_value: None,
        step: None,
        precision: None,
        tag: "",
        doc: "",
    }
}

#[test]
fn inspect_default() {
    #[derive(Debug, Default, Clone, Reflect)]
    pub struct Data {
        the_field: String,
        another_field: f32,
    }

    let data = Data::default();

    let the_field_metadata = FieldMetadata {
        name: "the_field",
        display_name: "The Field",
        ..default_prop_metadata()
    };

    let another_field_metadata = FieldMetadata {
        name: "another_field",
        display_name: "Another Field",
        ..default_prop_metadata()
    };

    let expected = vec![
        FieldRef {
            metadata: &the_field_metadata,
            value: &data.the_field,
        },
        FieldRef {
            metadata: &another_field_metadata,
            value: &data.another_field,
        },
    ];

    data.fields_ref(&mut |fields_ref| assert_eq!(fields_ref, expected));
}

#[test]
fn inspect_attributes() {
    #[derive(Debug, Default, Clone, Reflect)]
    pub struct AarGee {
        aar: u32,
        gee: u32,
    }

    #[derive(Debug, Default, Clone, Reflect)]
    pub struct Data {
        // NOTE: Even though this field is skipped, the next field is given index `1` for simplicity
        #[reflect(hidden)]
        _skipped: u32,
        #[reflect(display_name = "Super X")]
        x: f32,
        /// This is a property description.
        #[reflect(
            read_only,
            min_value = 0.1,
            max_value = 1.1,
            step = 0.1,
            precision = 3,
            tag = "SomeTag"
        )]
        y: f32,
    }

    let data = Data::default();

    let x_metadata = FieldMetadata {
        name: "x",
        display_name: "Super X",
        ..default_prop_metadata()
    };

    let expected = vec![
        FieldRef {
            metadata: &x_metadata,
            value: &data.x,
        },
        FieldRef {
            metadata: &FieldMetadata {
                name: "y",
                display_name: "Y",
                read_only: true,
                immutable_collection: false,
                min_value: Some(0.1),
                max_value: Some(1.1),
                step: Some(0.1),
                precision: Some(3),
                tag: "SomeTag",
                doc: "",
            },
            value: &data.y,
        },
    ];

    data.fields_ref(&mut |fields_ref| assert_eq!(fields_ref[0..2], expected));
}

#[test]
fn inspect_struct() {
    #[derive(Debug, Default, Clone, Reflect)]
    struct Tuple(f32, f32);

    let x = Tuple::default();

    x.fields_ref(&mut |fields_ref| {
        assert_eq!(
            fields_ref,
            vec![
                FieldRef {
                    metadata: &FieldMetadata {
                        name: "0",
                        display_name: "0",
                        ..default_prop_metadata()
                    },
                    value: &x.0,
                },
                FieldRef {
                    metadata: &FieldMetadata {
                        name: "1",
                        display_name: "1",
                        ..default_prop_metadata()
                    },
                    value: &x.1,
                },
            ]
        )
    });

    #[derive(Debug, Default, Clone, Reflect)]
    struct Unit;

    let x = Unit;
    x.fields_ref(&mut |fields_ref| assert_eq!(fields_ref, vec![]));
}

#[test]
fn inspect_enum() {
    #[derive(Debug, Clone, Reflect)]
    pub struct NonCopy {
        inner: u32,
    }

    #[derive(Debug, Clone, Reflect)]
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

    data.fields_ref(&mut |fields_ref| {
        assert_eq!(
            fields_ref,
            vec![
                FieldRef {
                    metadata: &FieldMetadata {
                        name: "Named@x",
                        display_name: "X",
                        ..default_prop_metadata()
                    },
                    value: match data {
                        Data::Named { ref x, .. } => x,
                        _ => unreachable!(),
                    },
                },
                FieldRef {
                    metadata: &FieldMetadata {
                        name: "Named@y",
                        display_name: "Y",
                        ..default_prop_metadata()
                    },
                    value: match data {
                        Data::Named { ref y, .. } => y,
                        _ => unreachable!(),
                    },
                },
                FieldRef {
                    metadata: &FieldMetadata {
                        name: "Named@z",
                        display_name: "Z",
                        ..default_prop_metadata()
                    },
                    value: match data {
                        Data::Named { ref z, .. } => z,
                        _ => unreachable!(),
                    },
                },
            ]
        )
    });

    let data = Data::Tuple(10.0, 20.0);

    data.fields_ref(&mut |fields_ref| {
        assert_eq!(
            fields_ref,
            vec![
                FieldRef {
                    metadata: &FieldMetadata {
                        name: "Tuple@0",
                        display_name: "0",
                        ..default_prop_metadata()
                    },
                    value: match data {
                        Data::Tuple(ref f0, ref _f1) => f0,
                        _ => unreachable!(),
                    },
                },
                FieldRef {
                    metadata: &FieldMetadata {
                        name: "Tuple@1",
                        display_name: "1",
                        ..default_prop_metadata()
                    },
                    value: match data {
                        Data::Tuple(ref _f0, ref f1) => f1,
                        _ => unreachable!(),
                    },
                },
            ]
        )
    });

    // unit variants don't have fields
    let data = Data::Unit;
    data.fields_ref(&mut |fields_ref| assert_eq!(fields_ref, vec![]));
}

#[test]
fn inspect_prop_key_constants() {
    #[allow(dead_code)]
    #[derive(Reflect, Clone, Debug)]
    pub struct SStruct {
        field: usize,
        #[reflect(hidden)]
        hidden: usize,
    }

    // NOTE: property names are in snake_case (not Title Case)
    assert_eq!(SStruct::FIELD, "field");

    // hidden properties
    // assert_eq!(SStruct::HIDDEN, "hidden");

    #[derive(Reflect, Clone, Debug)]
    pub struct STuple(usize);
    assert_eq!(STuple::F_0, "0");

    #[derive(Reflect, Clone, Debug)]
    #[allow(unused)]
    pub enum E {
        Tuple(usize),
        Struct { field: usize },
        Unit,
    }

    assert_eq!(E::TUPLE_F_0, "Tuple@0");
    E::Tuple(0).fields_ref(&mut |fields_ref| assert_eq!(E::TUPLE_F_0, fields_ref[0].name));

    assert_eq!(E::STRUCT_FIELD, "Struct@field");

    E::Struct { field: 0 }
        .fields_ref(&mut |fields_ref| assert_eq!(E::STRUCT_FIELD, fields_ref[0].name));
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
    #[derive(Reflect, Clone, Debug)]
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
