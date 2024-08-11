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
