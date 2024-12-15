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

use crate::fyrox::core::reflect::prelude::*;
use fyrox_build_tools::BuildProfile;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Reflect)]
pub struct BuildSettings {
    #[reflect(hidden)]
    pub selected_profile: usize,
    pub profiles: Vec<BuildProfile>,
}

impl Default for BuildSettings {
    fn default() -> Self {
        let debug = BuildProfile::debug();
        let release = BuildProfile::release();
        let debug_hot_reloading = BuildProfile::debug_hot_reloading();
        let release_hot_reloading = BuildProfile::release_hot_reloading();
        Self {
            selected_profile: 0,
            profiles: vec![debug, debug_hot_reloading, release, release_hot_reloading],
        }
    }
}
