#[derive(Debug)]
pub enum SoundError {
    /// Generic input error.
    Io(std::io::Error),

    /// No backend is provided for current OS.
    NoBackend,

    /// Unable to initialize device, exact reason stored in inner value.
    FailedToInitializeDevice(String),

    /// Invalid header of sound file.
    InvalidHeader,

    /// Unsupported format of sound file.
    UnsupportedFormat,

    /// It means that some thread panicked while holding a MutexGuard, the data mutex
    /// protected can be corrupted.
    PoisonedMutex,
}

impl From<std::io::Error> for SoundError {
    fn from(e: std::io::Error) -> Self {
        SoundError::Io(e)
    }
}

impl<'a, T> From<std::sync::PoisonError<std::sync::MutexGuard<'a, T>>> for SoundError {
    fn from(_: std::sync::PoisonError<std::sync::MutexGuard<'a, T>>) -> Self {
        SoundError::PoisonedMutex
    }
}