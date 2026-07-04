use std::path::{Path, PathBuf};

use crate::scene::Component;

// ── Component library — loads/saves JSON files in a components/ folder ────────

pub struct ComponentLibrary {
    pub dir:        PathBuf,
    pub components: Vec<(PathBuf, Component)>,
}

impl ComponentLibrary {
    pub fn load(dir: &Path) -> Self {
        let _ = std::fs::create_dir_all(dir);
        let mut lib = Self { dir: dir.to_path_buf(), components: vec![] };
        lib.reload();
        lib
    }

    pub fn reload(&mut self) {
        self.components.clear();
        if let Ok(entries) = std::fs::read_dir(&self.dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    if let Ok(s) = std::fs::read_to_string(&path) {
                        if let Ok(comp) = serde_json::from_str::<Component>(&s) {
                            self.components.push((path, comp));
                        }
                    }
                }
            }
        }
        self.components.sort_by(|a, b| a.1.name.cmp(&b.1.name));
    }

    pub fn save(&mut self, comp: Component) {
        let fname = format!("{}.json",
            comp.name.to_lowercase().replace(' ', "_").replace('/', "_"));
        let path = self.dir.join(&fname);
        if let Ok(json) = serde_json::to_string_pretty(&comp) {
            let _ = std::fs::write(&path, json);
        }
        self.reload();
    }

    pub fn delete(&mut self, path: &Path) {
        let _ = std::fs::remove_file(path);
        self.reload();
    }
}
