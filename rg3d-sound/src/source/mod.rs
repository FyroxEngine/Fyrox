//! Sound source module.
//!
//! # Overview
//!
//! Sound source is a "work horse" of the engine. It allows you to play sounds, apply effects (such as positioning, HRTF,
//! etc.), control volume, pitch, panning and other. Exact behaviour defined by a variant of sound buffer (generic or
//! spatial). See docs at those modules for more info.

use crate::source::{generic::GenericSource, spatial::SpatialSource};
use rg3d_core::visitor::{Visit, VisitError, VisitResult, Visitor};
use std::ops::{Deref, DerefMut};

pub mod generic;
pub mod spatial;

/// Status (state) of sound source.
#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum Status {
    /// Sound is stopped - it won't produces any sample and won't load mixer. This is default
    /// state of all sound sources.
    Stopped,

    /// Sound is playing.
    Playing,

    /// Sound is paused, it can stay in this state any amount if time. Playback can be continued by
    /// setting `Playing` status.
    Paused,
}

/// See module docs.
#[derive(Debug, Clone)]
pub enum SoundSource {
    /// See `generic` module docs.
    Generic(GenericSource),

    /// See `spatial` module docs.
    Spatial(SpatialSource),
}

impl SoundSource {
    /// Tries to "cast" sound source to spatial source. It will panic if this is not spatial source.
    /// This is useful method for situations where you definitely know that source is spatial. So there
    /// is no need to use pattern matching to take reference as a spatial source.
    pub fn spatial(&self) -> &SpatialSource {
        match self {
            SoundSource::Generic(_) => panic!("Cast as spatial sound failed!"),
            SoundSource::Spatial(ref spatial) => spatial,
        }
    }

    /// Tries to "cast" sound source to spatial source. It will panic if this is not spatial source.
    /// This is useful method for situations where you definitely know that source is spatial. So there
    /// is no need to use pattern matching to take reference as a spatial source.
    pub fn spatial_mut(&mut self) -> &mut SpatialSource {
        match self {
            SoundSource::Generic(_) => panic!("Cast as spatial sound failed!"),
            SoundSource::Spatial(ref mut spatial) => spatial,
        }
    }
}

impl Deref for SoundSource {
    type Target = GenericSource;

    /// Returns shared reference to generic source of each sound source variant. It is possible because
    /// `Spatial` sources are composed using generic source.
    fn deref(&self) -> &Self::Target {
        match self {
            SoundSource::Generic(v) => v,
            SoundSource::Spatial(v) => v,
        }
    }
}

impl DerefMut for SoundSource {
    /// Returns mutable reference to generic source of each sound source variant. It is possible because
    /// `Spatial` sources are composed using generic source.
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            SoundSource::Generic(v) => v,
            SoundSource::Spatial(v) => v,
        }
    }
}

impl Visit for Status {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut kind: u8 = match self {
            Status::Stopped => 0,
            Status::Playing => 1,
            Status::Paused => 2,
        };

        kind.visit(name, visitor)?;

        if visitor.is_reading() {
            *self = match kind {
                0 => Status::Stopped,
                1 => Status::Playing,
                2 => Status::Paused,
                _ => return Err(VisitError::User("invalid status".to_string())),
            }
        }

        Ok(())
    }
}

impl Visit for SoundSource {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut kind: u8 = match self {
            SoundSource::Generic(_) => 0,
            SoundSource::Spatial(_) => 1,
        };

        kind.visit("Id", visitor)?;

        if visitor.is_reading() {
            *self = match kind {
                0 => SoundSource::Generic(GenericSource::default()),
                1 => SoundSource::Spatial(SpatialSource::default()),
                _ => return Err(VisitError::User("invalid source kind".to_string())),
            }
        }

        match self {
            SoundSource::Generic(generic) => generic.visit("Content", visitor)?,
            SoundSource::Spatial(spatial) => spatial.visit("Content", visitor)?,
        }

        visitor.leave_region()
    }
}

impl Default for SoundSource {
    fn default() -> Self {
        SoundSource::Generic(Default::default())
    }
}
