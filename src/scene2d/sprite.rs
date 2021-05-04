use crate::core::color::Color;
use crate::{core::visitor::prelude::*, resource::texture::Texture, scene2d::base::Base};
use std::ops::{Deref, DerefMut};

#[derive(Default, Visit)]
pub struct Sprite {
    base: Base,
    texture: Option<Texture>,
    color: Color,
}

impl Deref for Sprite {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Sprite {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Sprite {
    pub fn texture(&self) -> Option<&Texture> {
        self.texture.as_ref()
    }

    pub fn color(&self) -> Color {
        self.color
    }
}
