use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::{fs::File, io::Write, path::PathBuf};

#[derive(Default, Serialize, Deserialize)]
pub struct Settings {
    pub projects: Vec<Project>,
}

impl Settings {
    pub const PATH: &'static str = "pm_settings.ron";

    pub fn load() -> Self {
        if let Ok(file) = File::open(Self::PATH) {
            ron::de::from_reader(file).unwrap_or_default()
        } else {
            Default::default()
        }
    }

    pub fn save(&self) {
        if let Ok(mut file) = File::create(Self::PATH) {
            let pretty_string =
                ron::ser::to_string_pretty(self, PrettyConfig::default()).unwrap_or_default();
            let _ = file.write_all(pretty_string.as_bytes());
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Project {
    pub manifest_path: PathBuf,
    pub name: String,
    pub hot_reload: bool,
}
