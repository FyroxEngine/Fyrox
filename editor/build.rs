use std::{env, fs, path::Path};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir).join("biosphere_docs.rs");

    let scan_dirs = [
        ("../biosphere-templates", ""),
        ("../biosphere-canonical/specs", "SPEC_"),
    ];

    let mut entries: Vec<(String, String)> = Vec::new();

    for (dir, prefix) in &scan_dirs {
        let dir_path = Path::new(dir);
        println!("cargo:rerun-if-changed={}", dir_path.display());
        if let Ok(read) = fs::read_dir(dir_path) {
            for entry in read.flatten() {
                let path = entry.path();
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if ext == "md" || ext == "txt" {
                    if let (Some(stem), Ok(abs)) =
                        (path.file_stem(), fs::canonicalize(&path))
                    {
                        let key = format!("{}{}", prefix, stem.to_string_lossy());
                        let abs_str = abs.to_string_lossy().to_string();
                        println!("cargo:rerun-if-changed={}", abs_str);
                        entries.push((key, abs_str));
                    }
                }
            }
        }
    }

    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut code = String::from("pub const BIOSPHERE_DOCS: &[(&str, &str)] = &[\n");
    for (name, abs_path) in &entries {
        code.push_str(&format!("    ({name:?}, include_str!({abs_path:?})),\n"));
    }
    code.push_str("];\n");

    fs::write(&out_path, code).unwrap();
}
