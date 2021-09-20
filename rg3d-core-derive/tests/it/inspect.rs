//! Test cases for `rg3d::gui::Inspect`

use rg3d_core::inspect::{Inspect, PropertyInfo};

use rg3d::scene::transform::Transform;

#[test]
fn inspect_default() {
    #[derive(Default, Inspect)]
    pub struct Data {
        the_field: String,
        another_field: f32,
    }

    let data = Data::default();

    let expected = vec![
        PropertyInfo {
            name: "The Field",
            group: "Common",
            value: &data.the_field,
        },
        PropertyInfo {
            name: "Another Field",
            group: "Common",
            value: &data.another_field,
        },
    ];

    assert_eq!(data.properties(), expected);
}

#[test]
fn inspect_attributes() {
    #[derive(Default, Inspect)]
    pub struct Data {
        #[inspect(skip)]
        skipped: u32,
        #[inspect(group = "Pos", name = "Super X")]
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
            name: "Super X",
            group: "Pos",
            value: &data.x,
        },
        PropertyInfo {
            name: "Y",
            group: "Pos",
            value: &data.y,
        },
    ];

    assert_eq!(data.properties()[0..2], expected);
    assert_eq!(data.properties().len(), 2 + data.tfm.properties().len());
}
