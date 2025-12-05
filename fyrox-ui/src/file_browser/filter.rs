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

use crate::core::{reflect::prelude::*, visitor::prelude::*};
use std::fmt::Display;
use std::ops::Index;
use std::{
    fmt::{Debug, Formatter},
    path::Path,
};

#[derive(Default, Clone, Debug, Visit, Reflect, PartialEq)]
pub struct FileType {
    pub description: String,
    pub extension: String,
}

impl FileType {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_extension(extension: impl AsRef<str>) -> Self {
        Self {
            description: Default::default(),
            extension: extension.as_ref().to_string(),
        }
    }

    pub fn with_description(mut self, description: impl AsRef<str>) -> Self {
        self.description = description.as_ref().to_string();
        self
    }

    pub fn with_extension(mut self, extension: impl AsRef<str>) -> Self {
        self.extension = extension.as_ref().to_string();
        self
    }

    pub fn matches(&self, path: &Path) -> bool {
        path.extension()
            .is_some_and(|ext| ext.to_string_lossy() == self.extension)
    }
}

impl Display for FileType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (*.{})", self.description, self.extension)
    }
}

#[derive(Default, Clone, Debug, Visit, Reflect, PartialEq)]
pub struct PathFilter {
    pub folders_only: bool,
    pub types: Vec<FileType>,
}

impl Index<usize> for PathFilter {
    type Output = FileType;

    fn index(&self, index: usize) -> &Self::Output {
        self.types.index(index)
    }
}

impl PathFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_file_type(mut self, file_type: FileType) -> Self {
        self.types.push(file_type);
        self
    }

    pub fn folder() -> Self {
        Self {
            folders_only: true,
            types: Default::default(),
        }
    }

    pub fn supports(&self, path: &Path) -> bool {
        if self.folders_only {
            path.is_dir()
        } else {
            path.is_dir()
                || self.is_empty()
                || self.types.iter().any(|file_type| file_type.matches(path))
        }
    }

    pub fn len(&self) -> usize {
        self.types.len()
    }

    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &FileType> {
        self.types.iter()
    }

    pub fn get(&self, i: usize) -> Option<&FileType> {
        self.types.get(i)
    }
}
