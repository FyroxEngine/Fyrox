//! Test cases for `rg3d::gui::Inspect`

use rg3d_core::inspect::{Inspect, PropertyInfo};

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
