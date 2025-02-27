use std::{
    fs::File,
    io::{Read, Write},
};
use toml_edit::DocumentMut;

fn write_version_file(src: &str, dest: &str) {
    let mut fyrox_toml = File::open(src).expect("Cargo.toml must exist!");

    let mut toml = String::new();
    fyrox_toml
        .read_to_string(&mut toml)
        .expect("File must be readable");

    let document = toml.parse::<DocumentMut>().unwrap();

    let package = document
        .get("package")
        .and_then(|i| i.as_table())
        .expect("package section must exist!");

    let version = package.get("version").expect("version must be set!");

    let mut file = File::create(dest).expect("version file must be accessible for writing!");

    let version = version.to_string().replace('\"', "");
    file.write_all(version.as_bytes())
        .expect("fyrox.version must be writable!");
    drop(file);
}

fn main() {
    write_version_file("../fyrox/Cargo.toml", "./engine.version");
    write_version_file("../editor/Cargo.toml", "./editor.version");
    write_version_file("../fyrox-scripts/Cargo.toml", "./scripts.version");
}
