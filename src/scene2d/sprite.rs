use crate::resource::texture::Texture;
use crate::scene2d::base::Base;
use std::ops::{Deref, DerefMut};

#[derive(Default)]
pub struct Sprite {
    base: Base,
    texture: Option<Texture>,
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
