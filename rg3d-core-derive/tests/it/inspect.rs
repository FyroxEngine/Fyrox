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
            group: "Data",
            value: &data.the_field,
        },
        PropertyInfo {
            owner_type_id: TypeId::of::<Data>(),
            name: "another_field",
            display_name: "Another Field",
            group: "Data",
            value: &data.another_field,
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
        #[inspect(skip)]
        _skipped: u32,
        #[inspect(group = "Pos", name = "the_x", display_name = "Super X")]
        x: f32,
        // Expand properties are added to the end of the list
        #[inspect(expand)]
        aar_gee: AarGee,
        #[inspect(group = "Pos")]
        y: f32,
    }

    let data = Data::default();

    let expected = vec![
        PropertyInfo {
            owner_type_id: TypeId::of::<Data>(),
            name: "the_x",
            display_name: "Super X",
            group: "Pos",
            value: &data.x,
        },
        PropertyInfo {
            owner_type_id: TypeId::of::<Data>(),
            name: "y",
            display_name: "Y",
            group: "Pos",
            value: &data.y,
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
                group: "Tuple",
                value: &x.0,
            },
            PropertyInfo {
                owner_type_id: TypeId::of::<Tuple>(),
                name: "1",
                display_name: "1",
                group: "Tuple",
                value: &x.1
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
                group: "Data",
                value: match data {
                    Data::Named { ref x, .. } => x,
                    _ => unreachable!(),
                },
            },
            PropertyInfo {
                owner_type_id: TypeId::of::<Data>(),
                name: "y",
                display_name: "Y",
                group: "Data",
                value: match data {
                    Data::Named { ref y, .. } => y,
                    _ => unreachable!(),
                },
            },
            PropertyInfo {
                owner_type_id: TypeId::of::<Data>(),
                name: "z",
                display_name: "Z",
                group: "Data",
                value: match data {
                    Data::Named { ref z, .. } => z,
                    _ => unreachable!(),
                },
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
                group: "Data",
                value: match data {
                    Data::Tuple(ref f0, ref _f1) => f0,
                    _ => unreachable!(),
                },
            },
            PropertyInfo {
                owner_type_id: TypeId::of::<Data>(),
                name: "1",
                display_name: "1",
                group: "Data",
                value: match data {
                    Data::Tuple(ref _f0, ref f1) => f1,
                    _ => unreachable!(),
                },
            },
        ]
    );

    // unit variants don't have fields
    let data = Data::Unit;
    assert_eq!(data.properties(), vec![]);
}
