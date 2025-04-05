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

use crate::io::ResourceIo;
use fyrox_core::{io::FileError, Uuid};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub struct ResourceMetadata {
    pub resource_id: Uuid,
}

impl ResourceMetadata {
    pub const EXTENSION: &'static str = "meta";

    pub fn new_with_random_id() -> Self {
        Self {
            resource_id: Uuid::new_v4(),
        }
    }

    pub async fn load_from_file_async(
        path: &Path,
        resource_io: &dyn ResourceIo,
    ) -> Result<Self, FileError> {
        resource_io.load_file(path).await.and_then(|metadata| {
            ron::de::from_bytes::<Self>(&metadata).map_err(|err| {
                FileError::Custom(format!(
                    "Unable to deserialize the resource metadata. Reason: {:?}",
                    err
                ))
            })
        })
    }

    fn serialize(&self, path: &Path) -> Result<String, FileError> {
        ron::ser::to_string_pretty(self, PrettyConfig::default()).map_err(|err| {
            FileError::Custom(format!(
                "Unable to serialize resource metadata for {} resource! Reason: {}",
                path.display(),
                err
            ))
        })
    }

    pub async fn save_async(
        &self,
        path: &Path,
        resource_io: &dyn ResourceIo,
    ) -> Result<(), FileError> {
        let string = self.serialize(path)?;
        resource_io.write_file(path, string.into_bytes()).await
    }

    pub fn save_sync(&self, path: &Path) -> Result<(), FileError> {
        let string = self.serialize(path)?;
        let mut file = File::create(path)?;
        file.write_all(string.as_bytes())?;
        Ok(())
    }
}
