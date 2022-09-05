//! Test cases for `fyrox::gui::Inspect`

use std::any::TypeId;

use fyrox_core::{
    inspect::{Inspect, PropertyInfo},
    reflect::Reflect,
};

fn default_prop() -> PropertyInfo<'static> {
    PropertyInfo {
        owner_type_id: TypeId::of::<()>(),
        name: "",
        display_name: "",
        value: &(),
        read_only: false,
        min_value: None,
        max_value: None,
        step: None,
        precision: None,
        description: "".to_string(),
    }
}

#[test]
fn inspect_default() {
    #[derive(Debug, Default, Inspect, Reflect)]
    pub struct Data {
        the_field: String,
        another_field: f32,
    }

    let data = Data::default();

    let expected = vec![
        PropertyInfo {
            owner_type_id: TypeId::of::<Data>(),
            name: "the_field",
            display_name: "The Field",
            value: &data.the_field,
            ..default_prop()
        },
        PropertyInfo {
            owner_type_id: TypeId::of::<Data>(),
            name: "another_field",
            display_name: "Another Field",
            value: &data.another_field,
            ..default_prop()
        },
    ];

    assert_eq!(data.properties(), expected);
}

#[test]
fn inspect_attributes() {
    #[derive(Debug, Default, Inspect, Reflect)]
    pub struct AarGee {
        aar: u32,
        gee: u32,
    }

    #[derive(Debug, Default, Inspect, Reflect)]
    pub struct Data {
        // NOTE: Even though this field is skipped, the next field is given index `1` for simplicity
        #[inspect(skip)]
        _skipped: u32,
        #[inspect(display_name = "Super X")]
        x: f32,
        #[inspect(
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
        PropertyInfo {
            owner_type_id: TypeId::of::<Data>(),
            name: "x",
            display_name: "Super X",
            value: &data.x,
            ..default_prop()
        },
        PropertyInfo {
            owner_type_id: TypeId::of::<Data>(),
            name: "y",
            display_name: "Y",
            value: &data.y,
            read_only: true,
            min_value: Some(0.1),
            max_value: Some(1.1),
            step: Some(0.1),
            precision: Some(3),
            description: "This is a property description.".to_string(),
        },
    ];

    assert_eq!(data.properties()[0..2], expected);
}

#[test]
fn inspect_struct() {
    #[derive(Debug, Default, Inspect, Reflect)]
    struct Tuple(f32, f32);

    let x = Tuple::default();
    assert_eq!(
        x.properties(),
        vec![
            PropertyInfo {
                owner_type_id: TypeId::of::<Tuple>(),
                name: "0",
                display_name: "0",
                value: &x.0,
                ..default_prop()
            },
            PropertyInfo {
                owner_type_id: TypeId::of::<Tuple>(),
                name: "1",
                display_name: "1",
                value: &x.1,
                ..default_prop()
            },
        ]
    );

    #[derive(Debug, Default, Inspect, Reflect)]
    struct Unit;

    let x = Unit::default();
    assert_eq!(x.properties(), vec![]);
}

#[test]
fn inspect_enum() {
    #[derive(Debug, Inspect, Reflect)]
    pub struct NonCopy {
        inner: u32,
    }

    #[derive(Debug, Inspect, Reflect)]
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
        data.properties(),
        vec![
            PropertyInfo {
                owner_type_id: TypeId::of::<Data>(),
                name: "Named@x",
                display_name: "X",
                value: match data {
                    Data::Named { ref x, .. } => x,
                    _ => unreachable!(),
                },
                ..default_prop()
            },
            PropertyInfo {
                owner_type_id: TypeId::of::<Data>(),
                name: "Named@y",
                display_name: "Y",
                value: match data {
                    Data::Named { ref y, .. } => y,
                    _ => unreachable!(),
                },
                ..default_prop()
            },
            PropertyInfo {
                owner_type_id: TypeId::of::<Data>(),
                name: "Named@z",
                display_name: "Z",
                value: match data {
                    Data::Named { ref z, .. } => z,
                    _ => unreachable!(),
                },
                ..default_prop()
            },
        ]
    );

    let data = Data::Tuple(10.0, 20.0);

    assert_eq!(
        data.properties(),
        vec![
            PropertyInfo {
                owner_type_id: TypeId::of::<Data>(),
                name: "Tuple@0",
                display_name: "0",
                value: match data {
                    Data::Tuple(ref f0, ref _f1) => f0,
                    _ => unreachable!(),
                },
                ..default_prop()
            },
            PropertyInfo {
                owner_type_id: TypeId::of::<Data>(),
                name: "Tuple@1",
                display_name: "1",
                value: match data {
                    Data::Tuple(ref _f0, ref f1) => f1,
                    _ => unreachable!(),
                },
                ..default_prop()
            },
        ]
    );

    // unit variants don't have fields
    let data = Data::Unit;
    assert_eq!(data.properties(), vec![]);
}

#[test]
fn inspect_prop_key_constants() {
    #[allow(dead_code)]
    #[derive(Inspect, Reflect)]
    pub struct SStruct {
        field: usize,
        #[inspect(skip)]
        hidden: usize,
    }

    // NOTE: property names are in snake_case (not Title Case)
    assert_eq!(SStruct::FIELD, "field");

    // hidden properties
    // assert_eq!(SStruct::HIDDEN, "hidden");

    #[derive(Inspect, Reflect)]
    pub struct STuple(usize);
    assert_eq!(STuple::F_0, "0");

    #[derive(Inspect, Reflect)]
    #[allow(unused)]
    pub enum E {
        Tuple(usize),
        Struct { field: usize },
        Unit,
    }

    assert_eq!(E::TUPLE_F_0, "Tuple@0");
    assert_eq!(E::TUPLE_F_0, E::Tuple(0).properties()[0].name);

    assert_eq!(E::STRUCT_FIELD, "Struct@field");
    assert_eq!(E::STRUCT_FIELD, E::Struct { field: 0 }.properties()[0].name);
}

#[test]
fn inspect_with_custom_getter() {
    use std::ops::Deref;

    #[derive(Reflect)]
    struct D<T>(T);

    impl<T> Deref for D<T> {
        type Target = T;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    #[derive(Inspect, Reflect)]
    struct A(
        #[inspect(getter = "deref()")] D<u32>,
        #[inspect(deref)] D<u32>,
    );

    let a = A(D(10), D(11));

    assert_eq!(
        a.properties(),
        vec![
            PropertyInfo {
                owner_type_id: TypeId::of::<A>(),
                name: "0",
                display_name: "0",
                value: &*a.0,
                ..default_prop()
            },
            PropertyInfo {
                owner_type_id: TypeId::of::<A>(),
                name: "1",
                display_name: "1",
                value: &*a.1,
                ..default_prop()
            }
        ]
    );
}
