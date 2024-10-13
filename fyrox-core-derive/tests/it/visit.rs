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

//! Test cases

mod basic;
mod compat;

use std::{env, fs::File, io::Write, path::PathBuf};

use futures::executor::block_on;
use fyrox_core::visitor::prelude::*;

/// Saves given `data` and overwrites `data_default` with the saved data.
///
/// Test the equality after running this method!
pub fn save_load<T: Visit>(test_name: &str, data: &mut T, data_default: &mut T) {
    // Locate output path
    let (bin, txt) = {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        let root = PathBuf::from(manifest_dir).join("test_output");
        let _ = std::fs::create_dir(&root);
        (
            root.join(format!("{test_name}.bin")),
            root.join(format!("{test_name}.txt")),
        )
    };

    // Save `data`
    {
        let mut visitor = Visitor::new();
        data.visit("Data", &mut visitor).unwrap();

        visitor.save_binary(&bin).unwrap();
        let mut file = File::create(txt).unwrap();
        file.write_all(visitor.save_text().as_bytes()).unwrap();
    }

    // Load the saved data to `data_default`
    {
        let mut visitor = block_on(Visitor::load_binary(&bin)).unwrap();
        // overwrite the default data with saved data
        data_default.visit("Data", &mut visitor).unwrap();
    }

    // Test function would do like:
    // assert_eq!(data, default_data);
}
