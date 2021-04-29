use std::{env, fs::File, io::Write, path::PathBuf};

use futures::executor::block_on;
use rg3d::core::visitor::{Visit, Visitor};

#[derive(Debug, Clone, Default, PartialEq, Visit)]
struct NamedFields {
    a: f32,
    snake_case: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Visit)]
struct SkipAttr {
    visited: f32,
    #[visit(skip)]
    skipped: u32,
}

#[test]
fn named_fields() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let root = PathBuf::from(manifest_dir).join("test_output");

    let mut data = NamedFields {
        a: 100.0,
        snake_case: 200,
    };
    let bin = root.join("named_fields.bin");
    let txt = root.join("named_fields.txt");

    // Save
    {
        let mut visitor = Visitor::new();
        data.visit("Data", &mut visitor).unwrap();

        visitor.save_binary(&bin).unwrap();
        let mut file = File::create(&txt).unwrap();
        file.write(visitor.save_text().as_bytes()).unwrap();
    }

    // Load
    {
        let mut visitor = block_on(Visitor::load_binary(&bin)).unwrap();
        let mut loaded_data = NamedFields::default();
        loaded_data.visit("Data", &mut visitor).unwrap();

        assert_eq!(data, loaded_data);
    }
}

#[test]
fn skip_attr() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let root = PathBuf::from(manifest_dir).join("test_output");

    let mut data = SkipAttr {
        visited: 10.0,
        skipped: 20,
    };
    let bin = root.join("skip_attr.bin");
    let txt = root.join("skip_attr.txt");

    // Save
    {
        let mut visitor = Visitor::new();
        data.visit("Data", &mut visitor).unwrap();

        visitor.save_binary(&bin).unwrap();
        let mut file = File::create(&txt).unwrap();
        file.write(visitor.save_text().as_bytes()).unwrap();
    }

    // Load
    {
        let mut visitor = block_on(Visitor::load_binary(&bin)).unwrap();
        let mut loaded_data = SkipAttr::default();
        loaded_data.visit("Data", &mut visitor).unwrap();

        assert_eq!(
            loaded_data,
            SkipAttr {
                visited: data.visited,
                skipped: Default::default(),
            }
        );
    }
}
