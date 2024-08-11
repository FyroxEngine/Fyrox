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
