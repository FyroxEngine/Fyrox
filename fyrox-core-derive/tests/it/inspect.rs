//! Test cases for `rg3d::gui::Inspect`

use std::any::TypeId;

use rg3d_core::inspect::{Inspect, PropertyInfo};

#[test]
fn inspect_default() {
    #[derive(Debug, Default, Inspect)]
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
            read_only: false,
            min_value: None,
            max_value: None,
            step: None,
            precision: None,
            description: "".to_string(),
        },
        PropertyInfo {
            owner_type_id: TypeId::of::<Data>(),
            name: "another_field",
            display_name: "Another Field",
            value: &data.another_field,
            read_only: false,
            min_value: None,
            max_value: None,
            step: None,
            precision: None,
            description: "".to_string(),
        },
    ];

    assert_eq!(data.properties(), expected);
}

#[test]
fn inspect_attributes() {
    #[derive(Debug, Default, Inspect)]
    pub struct AarGee {
        aar: u32,
        gee: u32,
    }

    #[derive(Debug, Default, Inspect)]
    pub struct Data {
        // NOTE: Even though this field is skipped, the next field is given index `1` for simplicity
        #[inspect(skip)]
        _skipped: u32,
        #[inspect(name = "the_x", display_name = "Super X")]
        x: f32,
        // Expand properties are added to the end of the list.
        // NOTE: Even though this field inspection is postponed, this field is given index `2`
        #[inspect(expand)]
        aar_gee: AarGee,
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
            name: "the_x",
            display_name: "Super X",
            value: &data.x,
            read_only: false,
            min_value: None,
            max_value: None,
            step: None,
            precision: None,
            description: "".to_string(),
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
    assert_eq!(data.properties().len(), 2 + data.aar_gee.properties().len());

    #[derive(Debug, Default, Inspect)]
    pub struct X {
        #[inspect(expand_subtree)]
        y: Y,
    }

    #[derive(Debug, Default, Inspect)]
    pub struct Y {
        a: u32,
    }

    let x = X::default();
    assert_eq!(x.properties().len(), 1 + x.y.properties().len());
}

#[test]
fn inspect_struct() {
    #[derive(Debug, Default, Inspect)]
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
                read_only: false,
                min_value: None,
                max_value: None,
                step: None,
                precision: None,
                description: "".to_string()
            },
            PropertyInfo {
                owner_type_id: TypeId::of::<Tuple>(),
                name: "1",
                display_name: "1",
                value: &x.1,
                read_only: false,
                min_value: None,
                max_value: None,
                step: None,
                precision: None,
                description: "".to_string()
            },
        ]
    );

    #[derive(Debug, Default, Inspect)]
    struct Unit;

    let x = Unit::default();
    assert_eq!(x.properties(), vec![]);
}

#[test]
fn inspect_enum() {
    #[derive(Debug, Inspect)]
    pub struct NonCopy {
        inner: u32,
    }

    #[derive(Debug, Inspect)]
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
                name: "x",
                display_name: "X",
                value: match data {
                    Data::Named { ref x, .. } => x,
                    _ => unreachable!(),
                },
                read_only: false,
                min_value: None,
                max_value: None,
                step: None,
                precision: None,
                description: "".to_string()
            },
            PropertyInfo {
                owner_type_id: TypeId::of::<Data>(),
                name: "y",
                display_name: "Y",
                value: match data {
                    Data::Named { ref y, .. } => y,
                    _ => unreachable!(),
                },
                read_only: false,
                min_value: None,
                max_value: None,
                step: None,
                precision: None,
                description: "".to_string()
            },
            PropertyInfo {
                owner_type_id: TypeId::of::<Data>(),
                name: "z",
                display_name: "Z",
                value: match data {
                    Data::Named { ref z, .. } => z,
                    _ => unreachable!(),
                },
                read_only: false,
                min_value: None,
                max_value: None,
                step: None,
                precision: None,
                description: "".to_string()
            },
        ]
    );

    let data = Data::Tuple(10.0, 20.0);

    assert_eq!(
        data.properties(),
        vec![
            PropertyInfo {
                owner_type_id: TypeId::of::<Data>(),
                name: "0",
                display_name: "0",
                value: match data {
                    Data::Tuple(ref f0, ref _f1) => f0,
                    _ => unreachable!(),
                },
                read_only: false,
                min_value: None,
                max_value: None,
                step: None,
                precision: None,
                description: "".to_string()
            },
            PropertyInfo {
                owner_type_id: TypeId::of::<Data>(),
                name: "1",
                display_name: "1",
                value: match data {
                    Data::Tuple(ref _f0, ref f1) => f1,
                    _ => unreachable!(),
                },
                read_only: false,
                min_value: None,
                max_value: None,
                step: None,
                precision: None,
                description: "".to_string()
            },
        ]
    );

    // unit variants don't have fields
    let data = Data::Unit;
    assert_eq!(data.properties(), vec![]);
}

#[test]
fn inspect_prop_key_constants() {
    #[derive(Debug, Inspect)]
    pub struct X;

    #[allow(dead_code)]
    #[derive(Inspect)]
    pub struct SStruct {
        field: usize,
        #[inspect(skip)]
        hidden: usize,
        #[inspect(expand)]
        expand: X,
        #[inspect(expand_subtree)]
        expand_subtree: X,
    }

    // NOTE: property names are in snake_case (not Title Case)
    assert_eq!(SStruct::FIELD, "field");
    // property keys for uninspectable fields are NOT emitted
    // assert_eq!(SStruct::HIDDEN, "hidden");
    // assert_eq!(SStruct::EXPAND, "expand");
    assert_eq!(SStruct::EXPAND_SUBTREE, "expand_subtree");

    #[derive(Inspect)]
    pub struct STuple(usize);
    assert_eq!(STuple::F_0, "0");

    #[derive(Inspect)]
    #[allow(unused)]
    pub enum E {
        Tuple(usize),
        Struct { field: usize },
        Unit,
    }

    assert_eq!(E::TUPLE_F_0, "0");
    assert_eq!(E::STRUCT_FIELD, "field");

    // variant itself it not a property
    // assert_eq!(E::UNIT, "unit");
}
