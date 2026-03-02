use std::{
    fs::File,
    io::{Read, Write},
};
use toml_edit::DocumentMut;

fn write_version_file(manifest_path: &str, version_file: &str) {
    let mut toml = String::new();
    File::open(manifest_path)
        .expect("Cargo.toml must exist!")
        .read_to_string(&mut toml)
        .expect("File must be readable");

    let version = toml
        .parse::<DocumentMut>()
        .expect("must be valid toml")
        .get("package")
        .and_then(|i| i.as_table())
        .expect("package section must exist!")
        .get("version")
        .expect("version must be set!")
        .to_string()
        .replace('\"', "")
        .trim()
        .to_string();

    File::create(version_file)
        .expect("version file must be accessible for writing!")
        .write_all(version.as_bytes())
        .expect("version file must be writable!");
}

fn main() {
    write_version_file("./Cargo.toml", "./pm.version");
}
