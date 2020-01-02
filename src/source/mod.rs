use crate::source::{
    generic::GenericSource,
    spatial::SpatialSource,
};
use rg3d_core::visitor::{
    Visit,
    Visitor,
    VisitResult,
    VisitError,
};

pub mod generic;
pub mod spatial;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum Status {
    Stopped,
    Playing,
    Paused,
}

pub enum SoundSource {
    Generic(GenericSource),
    Spatial(SpatialSource),
}

impl SoundSource {
    pub fn generic(&self) -> &GenericSource {
        match self {
            SoundSource::Generic(generic) => generic,
            SoundSource::Spatial(spatial) => &spatial.generic(),
        }
    }

    pub fn generic_mut(&mut self) -> &mut GenericSource {
        match self {
            SoundSource::Generic(generic) => generic,
            SoundSource::Spatial(spatial) => spatial.generic_mut(),
        }
    }

    pub fn spatial(&self) -> &SpatialSource {
        match self {
            SoundSource::Generic(_) => panic!("Cast as spatial sound failed!"),
            SoundSource::Spatial(ref spatial) => spatial,
        }
    }

    pub fn spatial_mut(&mut self) -> &mut SpatialSource {
        match self {
            SoundSource::Generic(_) => panic!("Cast as spatial sound failed!"),
            SoundSource::Spatial(ref mut spatial) => spatial,
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
                _ => return Err(VisitError::User("invalid status".to_string()))
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
                _ => return Err(VisitError::User("invalid source kind".to_string()))
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

