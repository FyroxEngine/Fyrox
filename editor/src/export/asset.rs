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

//! Asset processing module.

use crate::export::utils;
use fyrox::{
    asset::manager::ResourceManager,
    core::{
        futures::executor, futures::future::join_all, log::Log, platform::TargetPlatform, SafeLock,
    },
};
use std::{fs, io, path::Path};

pub fn copy_and_convert_assets(
    src_folder: impl AsRef<Path>,
    dst_folder: impl AsRef<Path>,
    target_platform: TargetPlatform,
    filter: &dyn Fn(&Path) -> bool,
    resource_manager: &ResourceManager,
    convert: bool,
) -> io::Result<()> {
    if convert {
        let rm = resource_manager.state();
        let io = rm.resource_io.clone();
        let loaders = rm.loaders.safe_lock();
        let mut tasks = Vec::new();

        // Iterate over the file system and try to convert all the supported resources.
        utils::copy_dir_ex(
            src_folder,
            dst_folder,
            &filter,
            &mut |src_file, dst_file| {
                if let Some(loader) = loaders.loader_for(src_file) {
                    tasks.push(loader.convert(
                        src_file.to_path_buf(),
                        dst_file.to_path_buf(),
                        target_platform,
                        io.clone(),
                    ));
                    Ok(())
                } else {
                    fs::copy(src_file, dst_file)?;
                    Ok(())
                }
            },
        )?;

        // Wait until everything is converted and copied.
        for result in executor::block_on(join_all(tasks)) {
            Log::verify(result);
        }

        Ok(())
    } else {
        utils::copy_dir(src_folder, dst_folder, &filter)
    }
}
