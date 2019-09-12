#[derive(Debug)]
pub enum SoundError {
    InvalidOutputBufferSize,
    Io(std::io::Error),
    FailedToInitializeDevice,
    InvalidHeader,
    UnsupportedFormat,
}

impl From<std::io::Error> for SoundError {
    fn from(e: std::io::Error) -> Self {
        SoundError::Io(e)
    }
}

