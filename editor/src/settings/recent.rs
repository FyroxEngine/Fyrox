use crate::fyrox::core::make_relative_path;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::PathBuf};

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Default, Eq)]
pub struct RecentFiles {
    pub scenes: Vec<PathBuf>,
}

impl RecentFiles {
    /// Does few main things:
    /// - Removes path to non-existent files.
    /// - Removes all duplicated paths.
    /// - Forces all paths to be in canonical form and replaces slashes to be OS-independent.
    /// - Sorts all paths in alphabetic order, which makes it easier to find specific path when there are many.
    pub fn deduplicate_and_refresh(&mut self) {
        self.scenes = self
            .scenes
            .iter()
            .filter_map(|p| make_relative_path(p).ok())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
    }
}
