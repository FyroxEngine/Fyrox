//! Fight the compatibility hell with attributes! .. someday :)

use fyrox_core::visitor::prelude::*;

// Comment it out and make sure it panics
// #[derive(Debug, Clone, PartialEq, Visit)]
// pub struct CantCompile {
//     pub a: f32,
//     #[visit(rename = "A")]
//     pub duplicate_name: f32,
// }

#[derive(Debug, Clone, PartialEq, Visit)]
pub struct Rename {
    #[visit(rename = "Renamed")]
    pub x: f32,
}

#[test]
fn rename() {
    let mut data = Rename { x: 100.0 };
    let mut data_default = Rename { x: 0.0 };

    super::save_load("rename", &mut data, &mut data_default);

    assert_eq!(data, data_default);
}
