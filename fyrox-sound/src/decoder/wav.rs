use crate::{buffer::DataSource, error::SoundError};
use hound::WavReader;
use std::{
    fmt::{Debug, Formatter},
    io::{Read, Seek, SeekFrom},
    sync::{Arc, Mutex},
    time::Duration,
};

/// Wav decoder
pub(crate) struct WavDecoder {
    reader: WavReader<DataSource>,
}

impl Debug for WavDecoder {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "WavDecoder")
    }
}

#[derive(Clone)]
struct WrappedDataSource {
    data_source: Arc<Mutex<DataSource>>,
}

impl WrappedDataSource {
    fn into_inner(self) -> DataSource {
        Arc::try_unwrap(self.data_source)
            .unwrap()
            .into_inner()
            .unwrap()
    }
}

impl Read for WrappedDataSource {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        self.data_source.lock().unwrap().read(buf)
    }
}

impl Seek for WrappedDataSource {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, std::io::Error> {
        self.data_source.lock().unwrap().seek(pos)
    }
}

impl WavDecoder {
    pub fn new(mut source: DataSource) -> Result<Self, DataSource> {
        let pos = source.stream_position().unwrap();
        let mut wrapped_source = WrappedDataSource {
            data_source: Arc::new(Mutex::new(source)),
        };

        let reader = match WavReader::new(wrapped_source.clone()) {
            Ok(old_reader) => {
                drop(old_reader);
                // Once we ensure that this is correct WAV source we need to re-create reader
                // with inner value of WrappedDataSource to eliminate mutex locking overhead.
                // This is some sort of a hack to bypass design flaws of the `hound` crate.
                wrapped_source.seek(SeekFrom::Start(pos)).unwrap();
                WavReader::new(wrapped_source.into_inner()).unwrap()
            }
            Err(_) => {
                wrapped_source.seek(SeekFrom::Start(pos)).unwrap();
                return Err(wrapped_source.into_inner());
            }
        };

        Ok(Self { reader })
    }

    pub fn rewind(&mut self) -> Result<(), SoundError> {
        self.reader.seek(0)?;
        Ok(())
    }

    pub fn time_seek(&mut self, location: Duration) {
        let _ = self
            .reader
            .seek((location.as_secs_f64() * self.reader.spec().sample_rate as f64) as u32);
    }

    pub fn channel_duration_in_samples(&self) -> usize {
        self.reader.duration() as usize
    }

    pub fn channel_count(&self) -> usize {
        self.reader.spec().channels as usize
    }

    pub fn sample_rate(&self) -> usize {
        self.reader.spec().sample_rate as usize
    }
}

impl Iterator for WavDecoder {
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let spec = self.reader.spec();
        match (spec.bits_per_sample, spec.sample_format) {
            (8, hound::SampleFormat::Int) => self
                .reader
                .samples::<i8>()
                .next()
                .and_then(|s| s.ok().map(|s| s as f32 / i8::MAX as f32)),
            (16, hound::SampleFormat::Int) => self
                .reader
                .samples::<i16>()
                .next()
                .and_then(|s| s.ok().map(|s| s as f32 / i16::MAX as f32)),
            (24, hound::SampleFormat::Int) => self
                .reader
                .samples::<i32>()
                .next()
                .and_then(|s| s.ok().map(|s| s as f32 / 0x7fffff as f32)),
            (32, hound::SampleFormat::Int) => self
                .reader
                .samples::<i32>()
                .next()
                .and_then(|s| s.ok().map(|s| s as f32 / i32::MAX as f32)),
            (32, hound::SampleFormat::Float) => {
                self.reader.samples::<f32>().next().and_then(|s| s.ok())
            }
            _ => None,
        }
    }
}
