use crate::utils;
use rg3d::core::algebra::Vector3;
use rg3d::core::pool::Handle;
use rg3d::sound::context::SoundContext;
use rg3d::sound::source::SoundSource;

#[derive(Debug, Clone)]
pub struct SoundSelection {
    pub sources: Vec<Handle<SoundSource>>,
}

impl SoundSelection {
    pub fn sources(&self) -> &[Handle<SoundSource>] {
        &self.sources
    }

    pub fn is_single_selection(&self) -> bool {
        self.sources.len() == 1
    }

    pub fn first(&self) -> Option<Handle<SoundSource>> {
        self.sources.first().cloned()
    }

    pub fn center(&self, sound_context: &SoundContext) -> Option<Vector3<f32>> {
        let state = sound_context.state();
        let mut count = 0;
        let position_sum = self
            .sources
            .iter()
            .filter_map(|&handle| match state.source(handle) {
                SoundSource::Generic(_) => None,
                SoundSource::Spatial(spatial) => Some(spatial.position()),
            })
            .fold(Vector3::default(), |acc, source_position| {
                count += 1;
                acc + source_position
            });
        if count > 0 {
            Some(position_sum.scale(1.0 / count as f32))
        } else {
            None
        }
    }
}

impl PartialEq for SoundSelection {
    fn eq(&self, other: &Self) -> bool {
        utils::is_slice_equal_permutation(self.sources(), other.sources())
    }
}

impl Eq for SoundSelection {}
