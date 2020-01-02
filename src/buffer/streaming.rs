use crate::{
    decoder::Decoder,
    buffer::{
        generic::GenericBuffer,
        DataSource
    },
    error::SoundError
};
use std::time::Duration;
use rg3d_core::visitor::{
    Visit,
    Visitor,
    VisitResult
};

pub struct StreamingBuffer {
    pub(in crate) generic: GenericBuffer,
    /// Count of sources that share this buffer, it is important to keep only one
    /// user of streaming buffer, because streaming buffer does not allow random
    /// access.
    pub(in crate) use_count: usize,
    decoder: Decoder,
}

impl Default for StreamingBuffer {
    fn default() -> Self {
        Self {
            generic: Default::default(),
            decoder: Decoder::Null,
            use_count: 0,
        }
    }
}

#[inline]
fn read_samples(buffer: &mut Vec<f32>, decoder: &mut Decoder, count: usize) -> usize {
    buffer.clear();
    for _ in 0..count {
        if let Some(sample) = decoder.next() {
            buffer.push(sample)
        } else {
            break;
        }
    }
    buffer.len()
}

impl StreamingBuffer {
    pub const STREAM_SAMPLE_COUNT: usize = 44100;

    pub fn new(source: DataSource) -> Result<Self, DataSource> {
        let external_source_path =
            if let DataSource::File { path, .. } = &source {
                Some(path.clone())
            } else {
                None
            };

        let mut decoder = Decoder::new(source)?;

        let mut samples = Vec::new();
        let channel_count = decoder.get_channel_count();
        read_samples(&mut samples, &mut decoder, Self::STREAM_SAMPLE_COUNT * channel_count);
        debug_assert_eq!(samples.len() % channel_count, 0);

        Ok(Self {
            generic: GenericBuffer {
                samples,
                sample_rate: decoder.get_sample_rate(),
                channel_count: decoder.get_channel_count(),
                external_source_path,
            },
            use_count: 0,
            decoder,
        })
    }

    pub fn generic(&self) -> &GenericBuffer {
        &self.generic
    }

    pub fn generic_mut(&mut self) -> &mut GenericBuffer {
        &mut self.generic
    }

    pub fn duration(&self) -> Option<Duration> {
        self.decoder.duration()
    }

    #[inline]
    pub(in crate) fn read_next_block(&mut self) {
        read_samples(&mut self.generic.samples, &mut self.decoder, self.generic.channel_count * Self::STREAM_SAMPLE_COUNT);
    }

    #[inline]
    pub(in crate) fn rewind(&mut self) -> Result<(), SoundError> {
        self.decoder.rewind()
    }

    #[inline]
    pub(in crate) fn time_seek(&mut self, location: Duration) {
        self.decoder.time_seek(location);
    }
}

impl Visit for StreamingBuffer {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.generic.visit("Generic", visitor)?;

        visitor.leave_region()
    }
}