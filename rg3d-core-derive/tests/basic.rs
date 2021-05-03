use std::{env, fs::File, io::Write, path::PathBuf};

use futures::executor::block_on;
use rg3d_core::visitor::prelude::*;

#[derive(Debug, Clone, Default, PartialEq, Visit)]
struct NamedFields {
    a: f32,
    snake_case: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Visit)]
struct UnitStruct;

#[derive(Debug, Clone, Default, PartialEq, Visit)]
struct TupleStruct(f32, u32);

#[derive(Debug, Clone, Default, PartialEq, Visit)]
struct SkipAttr {
    visited: f32,
    #[visit(skip)]
    skipped: u32,
}

#[derive(Debug, Clone, PartialEq, Visit)]
struct Generics<T> {
    items: Vec<T>,
}

// NOTE: This enum doesn't implement copy, but `#[derive(Visit)]` still works
#[derive(Debug, Clone, PartialEq, Eq, Hash, Visit)]
enum PlainEnum {
    A,
    B,
    C,
}

// NOTE: implementing default for `PlainEnum` is REQUIRED to derive `Visit` for `OnefTheTypes`
impl Default for PlainEnum {
    fn default() -> Self {
        Self::A
    }
}

#[derive(Debug, Clone, PartialEq, Visit)]
enum OneOfTheTypes {
    NamedFields(NamedFields),
    PlainEnum(PlainEnum),
    U32(u32),
}

#[derive(Debug, Clone, PartialEq, Visit)]
enum ComplexEnum {
    UnitVariant,
    Tuple(i32, i32),
    NamedFields { a: f32, b: u32 },
}

/// Saves given `data` and overwrites `data_default` with the saved data.
/// Test the equality after running this method!
fn save_load<T: Visit>(test_name: &str, data: &mut T, data_default: &mut T) {
    // Locate output path
    let (bin, txt) = {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        let root = PathBuf::from(manifest_dir).join("test_output");
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
        let mut file = File::create(&txt).unwrap();
        file.write(visitor.save_text().as_bytes()).unwrap();
    }

    // Load the saevd data to `data_default`
    {
        let mut visitor = block_on(Visitor::load_binary(&bin)).unwrap();
        // overwrite the default data with saved data
        data_default.visit("Data", &mut visitor).unwrap();
    }

    // Test function would would do:
    // assert_eq!(data, default_data);
}

#[test]
fn named_fields() {
    let mut data = NamedFields {
        a: 100.0,
        snake_case: 200,
    };
    let mut data_default = NamedFields::default();

    self::save_load("named_fields", &mut data, &mut data_default);

    // The default data was overwritten with saved data.
    // Now it should be same as the original data:
    assert_eq!(data, data_default);
}

#[test]
fn unit_struct() {
    let mut data = UnitStruct;
    let mut data_default = UnitStruct;

    // non seuse.. but anyways,
    // `Visit` is implemented `UnitStruct;` as empty code block `{}`
    self::save_load("unit_struct", &mut data, &mut data_default);

    assert_eq!(data, data_default);
}

#[test]
fn tuple_struct() {
    let mut data = TupleStruct(10.0, 20);
    let mut data_default = TupleStruct(0.0, 0);

    self::save_load("tuple_struct", &mut data, &mut data_default);

    assert_eq!(data, data_default);
}

#[test]
fn skip_attr() {
    let mut data = SkipAttr {
        visited: 10.0,
        skipped: 20,
    };
    let mut data_default = SkipAttr::default();

    self::save_load("skip_attr", &mut data, &mut data_default);

    // The default data was overwritten with saved data,
    // except the `skipped` field:
    assert_eq!(
        data_default,
        SkipAttr {
            visited: data.visited,
            skipped: Default::default(),
        }
    );
}

#[test]
fn generics() {
    struct NotVisit;

    #[allow(warnings)]
    let mut not_compile = Generics {
        items: vec![NotVisit],
    };

    // Compile error because `Generics<NotVisit> is not `Visit`:
    // let mut visitor = Visitor::new();
    // not_compile.visit("Data", &mut visitor).unwrap();

    // But `Generics<T: Visit` is `Visit`
    let mut data = Generics {
        items: vec![100u32],
    };
    let mut data_default = Generics { items: vec![] };

    self::save_load("generics", &mut data, &mut data_default);

    assert_eq!(data, data_default);
}

#[test]
fn plain_enum() {
    let mut data = PlainEnum::C;
    let mut data_default = PlainEnum::A;

    self::save_load("plain_enum", &mut data, &mut data_default);

    assert_eq!(data, data_default);
}

#[test]
fn one_of_the_types() {
    let mut data = OneOfTheTypes::NamedFields(NamedFields {
        a: 1.0,
        snake_case: 10,
    });
    let mut data_default = OneOfTheTypes::U32(0);

    self::save_load("one_of_the_types", &mut data, &mut data_default);

    assert_eq!(data, data_default);
}

#[test]
fn complex_enum() {
    let mut data = ComplexEnum::Tuple(100, 200);
    let mut data_default = ComplexEnum::UnitVariant;

    self::save_load("complex_enum", &mut data, &mut data_default);

    assert_eq!(data, data_default);
}
