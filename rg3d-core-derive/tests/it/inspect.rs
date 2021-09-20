//! Test cases for `rg3d::gui::Inspect`

use rg3d_core::{
    algebra::Vector3,
    inspect::{Inspect, PropertyInfo},
};

#[test]
fn inspect_default() {
    #[derive(Inspect)]
    pub struct A {
        the_field: String,
        another_field: f32,
    }

    let a = A {
        the_field: "Name!".to_string(),
        another_field: 100.0,
    };

    let expected = vec![
        PropertyInfo {
            name: "THE_FIELD",
            group: "Common",
            value: &a.the_field,
        },
        PropertyInfo {
            name: "ANOTHER_FIELD",
            group: "Common",
            value: &a.another_field,
        },
    ];

    assert_eq!(a.properties(), expected);
}

#[test]
fn inspect_attributes() {
    #[derive(Inspect)]
    pub struct A {
        #[inspect(skip)]
        skipped: Vector3<f32>,
        #[inspect(group = "Pos")]
        x: f32,
        #[inspect(group = "Pos")]
        y: f32,
    }

    let a = A {
        skipped: Default::default(),
        x: 10.0,
        y: 20.0,
    };

    let expected = vec![
        PropertyInfo {
            name: "X",
            group: "Pos",
            value: &a.x,
        },
        PropertyInfo {
            name: "Y",
            group: "Pos",
            value: &a.y,
        },
    ];

    assert_eq!(a.properties(), expected);
}
