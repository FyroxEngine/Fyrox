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

//! Provides an interface for IO operations that a resource loader will use, this facilliates
//! things such as loading assets within archive files

use fyrox_core::io::FileError;
use std::fs::File;
use std::future::{ready, Future};
use std::io::{BufReader, Write};
use std::iter::empty;
use std::pin::Pin;
use std::{
    fmt::Debug,
    io::{Cursor, Read, Seek},
    path::{Path, PathBuf},
};

/// Trait for files readers ensuring they implement the required traits
pub trait FileReader: Debug + Send + Sync + Read + Seek + 'static {
    /// Returns the length in bytes, if available
    fn byte_len(&self) -> Option<u64>;
}

impl FileReader for File {
    fn byte_len(&self) -> Option<u64> {
        match self.metadata() {
            Ok(metadata) => Some(metadata.len()),
            _ => None,
        }
    }
}

impl<T> FileReader for Cursor<T>
where
    T: Debug + Send + Sync + std::convert::AsRef<[u8]> + 'static,
{
    fn byte_len(&self) -> Option<u64> {
        let inner = self.get_ref();
        Some(inner.as_ref().len().try_into().unwrap())
    }
}
impl FileReader for BufReader<File> {
    fn byte_len(&self) -> Option<u64> {
        self.get_ref().byte_len()
    }
}

/// Interface wrapping IO operations for doing this like loading files
/// for resources
pub trait ResourceIo: Send + Sync + 'static {
    /// Attempts to load the file at the provided path returning
    /// the entire byte contents of the file or an error
    fn load_file<'a>(&'a self, path: &'a Path) -> ResourceIoFuture<'a, Result<Vec<u8>, FileError>>;

    /// Attempts to write the given set of bytes to the specified path.
    fn write_file<'a>(
        &'a self,
        path: &'a Path,
        data: Vec<u8>,
    ) -> ResourceIoFuture<'a, Result<(), FileError>>;

    /// Attempts to move a file at the given `source` path to the given `dest` path.
    fn move_file<'a>(
        &'a self,
        source: &'a Path,
        dest: &'a Path,
    ) -> ResourceIoFuture<'a, Result<(), FileError>>;

    /// Tries to convert the path to its canonical form (normalize it in other terms). This method
    /// should guarantee correct behaviour for relative paths. Symlinks aren't mandatory to
    /// follow.
    fn canonicalize_path<'a>(
        &'a self,
        path: &'a Path,
    ) -> ResourceIoFuture<'a, Result<PathBuf, FileError>> {
        Box::pin(ready(Ok(path.to_owned())))
    }

    /// Provides an iterator over the paths present in the provided
    /// path, this should only provide paths immediately within the directory
    ///
    /// Default implementation is no-op returning an empty iterator
    fn read_directory<'a>(
        &'a self,
        #[allow(unused)] path: &'a Path,
    ) -> ResourceIoFuture<'a, Result<Box<dyn Iterator<Item = PathBuf> + Send>, FileError>> {
        let iter: Box<dyn Iterator<Item = PathBuf> + Send> = Box::new(empty());
        Box::pin(ready(Ok(iter)))
    }

    /// Provides an iterator over the paths present in the provided
    /// path directory this implementation should walk the directory paths
    ///
    /// Default implementation is no-op returning an empty iterator
    fn walk_directory<'a>(
        &'a self,
        #[allow(unused)] path: &'a Path,
    ) -> ResourceIoFuture<'a, Result<Box<dyn Iterator<Item = PathBuf> + Send>, FileError>> {
        let iter: Box<dyn Iterator<Item = PathBuf> + Send> = Box::new(empty());
        Box::pin(ready(Ok(iter)))
    }

    /// Attempts to open a file reader to the proivded path for
    /// reading its bytes
    ///
    /// Default implementation loads the entire file contents from `load_file`
    /// then uses a cursor as the reader
    fn file_reader<'a>(
        &'a self,
        path: &'a Path,
    ) -> ResourceIoFuture<'a, Result<Box<dyn FileReader>, FileError>> {
        Box::pin(async move {
            let bytes = self.load_file(path).await?;
            let read: Box<dyn FileReader> = Box::new(Cursor::new(bytes));
            Ok(read)
        })
    }

    /// Used to check whether a path exists
    fn exists<'a>(&'a self, path: &'a Path) -> ResourceIoFuture<'a, bool>;

    /// Used to check whether a path is a file
    fn is_file<'a>(&'a self, path: &'a Path) -> ResourceIoFuture<'a, bool>;

    /// Used to check whether a path is a dir
    fn is_dir<'a>(&'a self, path: &'a Path) -> ResourceIoFuture<'a, bool>;
}

/// Standard resource IO provider that uses the file system to
/// load the file bytes
#[derive(Default)]
pub struct FsResourceIo;

/// Future for resource io loading
#[cfg(target_arch = "wasm32")]
pub type ResourceIoFuture<'a, V> = Pin<Box<dyn Future<Output = V> + 'a>>;
/// Future for resource io loading
#[cfg(not(target_arch = "wasm32"))]
pub type ResourceIoFuture<'a, V> = Pin<Box<dyn Future<Output = V> + Send + 'a>>;

/// Iterator of paths
#[cfg(target_arch = "wasm32")]
pub type PathIter = Box<dyn Iterator<Item = PathBuf>>;
/// Iterator of paths
#[cfg(not(target_arch = "wasm32"))]
pub type PathIter = Box<dyn Iterator<Item = PathBuf> + Send>;

impl ResourceIo for FsResourceIo {
    fn load_file<'a>(&'a self, path: &'a Path) -> ResourceIoFuture<'a, Result<Vec<u8>, FileError>> {
        Box::pin(fyrox_core::io::load_file(path))
    }

    fn write_file<'a>(
        &'a self,
        path: &'a Path,
        data: Vec<u8>,
    ) -> ResourceIoFuture<'a, Result<(), FileError>> {
        Box::pin(async move {
            let mut file = File::create(path)?;
            file.write_all(&data)?;
            Ok(())
        })
    }

    fn move_file<'a>(
        &'a self,
        source: &'a Path,
        dest: &'a Path,
    ) -> ResourceIoFuture<'a, Result<(), FileError>> {
        Box::pin(async move {
            std::fs::rename(source, dest)?;
            Ok(())
        })
    }

    fn canonicalize_path<'a>(
        &'a self,
        path: &'a Path,
    ) -> ResourceIoFuture<'a, Result<PathBuf, FileError>> {
        Box::pin(async move { Ok(std::fs::canonicalize(path)?) })
    }

    /// wasm should fallback to the default no-op impl as im not sure if they
    /// can directly read a directory
    ///
    /// Note: Android directory reading should be possible just I have not created
    /// an implementation for this yet
    #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
    fn read_directory<'a>(
        &'a self,
        #[allow(unused)] path: &'a Path,
    ) -> ResourceIoFuture<'a, Result<PathIter, FileError>> {
        Box::pin(async move {
            let iter = std::fs::read_dir(path)?.flatten().map(|entry| entry.path());
            let iter: PathIter = Box::new(iter);
            Ok(iter)
        })
    }

    /// Android and wasm should fallback to the default no-op impl as they cant be
    /// walked with WalkDir
    #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
    fn walk_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> ResourceIoFuture<'a, Result<PathIter, FileError>> {
        Box::pin(async move {
            use walkdir::WalkDir;

            let iter = WalkDir::new(path)
                .into_iter()
                .flatten()
                .map(|value| value.into_path());

            let iter: PathIter = Box::new(iter);

            Ok(iter)
        })
    }

    /// Only use file reader when not targetting android or wasm
    ///
    /// Note: Might be possible to use the Android Asset struct for reading as
    /// long as its Send + Sync + 'static (It already implements Debug + Read + Seek)
    #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
    fn file_reader<'a>(
        &'a self,
        path: &'a Path,
    ) -> ResourceIoFuture<'a, Result<Box<dyn FileReader>, FileError>> {
        Box::pin(async move {
            let file = match std::fs::File::open(path) {
                Ok(file) => file,
                Err(e) => return Err(FileError::Io(e)),
            };

            let read: Box<dyn FileReader> = Box::new(std::io::BufReader::new(file));
            Ok(read)
        })
    }

    fn exists<'a>(&'a self, path: &'a Path) -> ResourceIoFuture<'a, bool> {
        Box::pin(fyrox_core::io::exists(path))
    }

    fn is_file<'a>(&'a self, path: &'a Path) -> ResourceIoFuture<'a, bool> {
        Box::pin(fyrox_core::io::is_file(path))
    }

    fn is_dir<'a>(&'a self, path: &'a Path) -> ResourceIoFuture<'a, bool> {
        Box::pin(fyrox_core::io::is_dir(path))
    }
}
