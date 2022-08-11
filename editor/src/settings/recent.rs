use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Default, Eq)]
pub struct RecentFiles {
    pub scenes: Vec<PathBuf>,
}
