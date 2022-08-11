use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Default)]
pub struct RecentFiles {
    pub scenes: Vec<PathBuf>,
}
