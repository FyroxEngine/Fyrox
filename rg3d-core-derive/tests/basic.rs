use std::{env, fs::File, io::Write, path::PathBuf};

use futures::executor::block_on;
use rg3d::core::visitor::{Visit, VisitResult, Visitor};

#[derive(Debug, Clone, Default, PartialEq, Visit)]
struct NamedFields {
    a: f32,
    snake_case: u32,
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
