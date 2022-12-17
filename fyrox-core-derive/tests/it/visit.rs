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
            root.join(format!("{}.bin", test_name)),
            root.join(format!("{}.txt", test_name)),
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
