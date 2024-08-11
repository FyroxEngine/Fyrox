// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

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
