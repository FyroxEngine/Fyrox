//! Provides an interface for IO operations that a resource loader will use, this facilliates
//! things such as loading assets within archive files

use std::{
    fmt::Debug,
    io::{Cursor, Read, Seek},
    path::{Path, PathBuf},
};

use fyrox_core::{futures::future::BoxFuture, io::FileLoadError};
use walkdir::WalkDir;

/// Trait for files readers ensuring they implement the required traits
pub trait FileReader: Debug + Send + Read + Seek + 'static {}

impl<F> FileReader for F where F: Debug + Send + Read + Seek + 'static {}

/// Interface wrapping IO operations for doing this like loading files
/// for resources
pub trait ResourceIo: Send + Sync + 'static {
    /// Attempts to load the file at the provided path returning
    /// the entire byte contents of the file or an error
    fn load_file<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, Result<Vec<u8>, FileLoadError>>;

    /// Provides an iterator over the paths present in the provided
    /// path directory FsResourceIo uses WalkDir bu
    fn walk_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxFuture<'a, Result<Box<dyn Iterator<Item = PathBuf> + Send>, FileLoadError>>;

    /// Attempts to open a file reader to the proivded path for
    /// reading its bytes
    ///
    /// Default implementation loads the entire file contents from `load_file`
    /// then uses a cursor as the reader
    fn file_reader<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxFuture<'a, Result<Box<dyn FileReader>, FileLoadError>> {
        Box::pin(async move {
            let bytes = self.load_file(path).await?;
            let read: Box<dyn FileReader> = Box::new(Cursor::new(bytes));
            Ok(read)
        })
    }

    /// Used to check whether a path exists
    fn exists<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, bool>;

    /// Used to check whether a path is a file
    fn is_file<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, bool>;

    /// Used to check whether a path is a dir
    fn is_dir<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, bool>;
}

/// Standard resource IO provider that uses the file system to
/// load the file bytes
#[derive(Default)]
pub struct FsResourceIo;

impl ResourceIo for FsResourceIo {
    fn load_file<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, Result<Vec<u8>, FileLoadError>> {
        Box::pin(fyrox_core::io::load_file(path))
    }

    fn walk_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxFuture<'a, Result<Box<dyn Iterator<Item = PathBuf> + Send>, FileLoadError>> {
        // I dont think directory walking works on android or wasm so this is no-op with an empty iterator
        #[cfg(any(target_os = "android", target_arch = "wasm32"))]
        {
            use std::future::ready;

            let iter: Box<dyn Iterator<Item = PathBuf> + Send> = Box::new(None.into_iter());
            return Box::pin(ready(Ok(iter)));
        }

        // Use walkdir for normal use
        #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
        {
            Box::pin(async move {
                // TODO: I'm not sure if WalkDir is acceptable for wasm and mobile platforms
                // might need to be updated for those platforms

                let iter = WalkDir::new(path)
                    .into_iter()
                    .flatten()
                    .map(|value| value.into_path());

                let iter: Box<dyn Iterator<Item = PathBuf> + Send> = Box::new(iter);

                Ok(iter)
            })
        }
    }

    /// Only use file reader when not targetting android or wasm
    ///
    /// Note: Might be possible to using the android Asset for reading as
    /// long as its Send + Sync + 'static (It already implements Debug + Read + Seek)
    #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
    fn file_reader<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxFuture<'a, Result<Box<dyn FileReader>, FileLoadError>> {
        Box::pin(async move {
            let file = match std::fs::File::open(path) {
                Ok(file) => file,
                Err(e) => return Err(FileLoadError::Io(e)),
            };

            let read: Box<dyn FileReader> = Box::new(std::io::BufReader::new(file));
            Ok(read)
        })
    }

    fn exists<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, bool> {
        Box::pin(fyrox_core::io::exists(path))
    }

    fn is_file<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, bool> {
        Box::pin(fyrox_core::io::is_file(path))
    }

    fn is_dir<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, bool> {
        Box::pin(fyrox_core::io::is_dir(path))
    }
}
