//! Provides an interface for IO operations that a resource loader will use, this facilliates
//! things such as loading assets within archive files

use std::{
    fmt::Debug,
    io::{Cursor, Read, Seek},
    path::Path,
};

use fyrox_core::{futures::future::BoxFuture, io::FileLoadError};

/// Trait for files readers ensuring they implement the required traits
pub trait FileReader: Debug + Send + Read + Seek + 'static {}

impl<F> FileReader for F where F: Debug + Send + Read + Seek + 'static {}

/// Interface wrapping IO operations for doing this like loading files
/// for resources
pub trait ResourceIo: Send + Sync + 'static {
    /// Attempts to load the file at the provided path returning
    /// the entire byte contents of the file or an error
    fn load_file<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, Result<Vec<u8>, FileLoadError>>;

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
}

/// Standard resource IO provider that uses the file system to
/// load the file bytes
#[derive(Default)]
pub struct FsResourceIo;

impl ResourceIo for FsResourceIo {
    fn load_file<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, Result<Vec<u8>, FileLoadError>> {
        Box::pin(fyrox_core::io::load_file(path))
    }

    // Only use file reader when not targetting android or wasm
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
}
