use std::path::PathBuf;
use rg3d_core::visitor::{
    Visitor,
    VisitResult,
    Visit,
};
use crate::{
    buffer::DataSource,
    decoder::Decoder,
};
use std::time::Duration;

pub struct GenericBuffer {
    /// Interleaved decoded samples (mono sounds: L..., stereo sounds: LR...)
    /// For streaming buffers it contains only small part of decoded data
    /// (usually something around 1 sec).
    pub(in crate) samples: Vec<f32>,
    pub(in crate) channel_count: usize,
    pub(in crate) sample_rate: usize,
    pub(in crate) external_source_path: Option<PathBuf>,
}

impl Default for GenericBuffer {
    fn default() -> Self {
        Self {
            samples: Vec::new(),
            channel_count: 0,
            sample_rate: 0,
            external_source_path: None,
        }
    }
}

impl Visit for GenericBuffer {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.external_source_path.visit("Path", visitor)?;

        visitor.leave_region()
    }
}

impl GenericBuffer {
    pub fn new(source: DataSource) -> Result<Self, DataSource> {
        let external_source_path =
            if let DataSource::File { path, .. } = &source {
                Some(path.clone())
            } else {
                None
            };

        let decoder = Decoder::new(source)?;

        Ok(Self {
            sample_rate: decoder.get_sample_rate(),
            channel_count: decoder.get_channel_count(),
            samples: decoder.into_samples(),
            external_source_path,
        })
    }


    #[inline]
    pub fn external_data_path(&self) -> Option<PathBuf> {
        self.external_source_path.clone()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    #[inline]
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    #[inline]
    pub fn channel_count(&self) -> usize {
        self.channel_count
    }

    #[inline]
    pub fn index_of_last_sample(&self) -> usize {
        self.samples.len() - self.channel_count
    }

    #[inline]
    pub fn sample_rate(&self) -> usize {
        self.sample_rate
    }

    #[inline]
    pub fn duration(&self) -> Duration {
        Duration::from_secs_f64((self.samples.len() / (self.channel_count * self.sample_rate)) as f64)
    }
}