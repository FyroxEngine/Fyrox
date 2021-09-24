//! Test cases for `rg3d::gui::Inspect`

use rg3d_core::inspect::{Inspect, PropertyInfo};

use rg3d::scene::transform::Transform;
use std::any::TypeId;

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
    pub struct Data {
        #[inspect(skip)]
        _skipped: u32,
        #[inspect(group = "Pos", name = "the_x", display_name = "Super X")]
        x: f32,
        // Expand properties are added to the end of the list
        #[inspect(expand)]
        tfm: Transform,
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
    assert_eq!(data.properties().len(), 2 + data.tfm.properties().len());

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
