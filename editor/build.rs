use std::{env, fs, path::Path};

fn main() {
    let templates_dir = Path::new("../biosphere-templates");
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir).join("biosphere_docs.rs");

    println!("cargo:rerun-if-changed={}", templates_dir.display());

    let mut entries: Vec<(String, String)> = Vec::new();
    if let Ok(read) = fs::read_dir(templates_dir) {
        for entry in read.flatten() {
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext == "md" || ext == "txt" {
                if let (Some(stem), Ok(abs)) =
                    (path.file_stem(), fs::canonicalize(&path))
                {
                    let key = stem.to_string_lossy().to_string();
                    let abs_str = abs.to_string_lossy().to_string();
                    println!("cargo:rerun-if-changed={}", abs_str);
                    entries.push((key, abs_str));
                }
            }
        }
    }

    // Sort for deterministic output
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut code = String::from("pub const BIOSPHERE_DOCS: &[(&str, &str)] = &[\n");
    for (name, abs_path) in &entries {
        code.push_str(&format!("    ({name:?}, include_str!({abs_path:?})),\n"));
    }
    code.push_str("];\n");

    fs::write(&out_path, code).unwrap();
}
