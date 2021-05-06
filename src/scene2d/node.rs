use crate::{
    core::{define_is_as, visitor::prelude::*},
    scene2d::{base::Base, camera::Camera, light::Light, sprite::Sprite},
};
use std::ops::{Deref, DerefMut};

#[derive(Visit)]
pub enum Node {
    Base(Base),
    Camera(Camera),
    Light(Light),
    Sprite(Sprite),
}

macro_rules! static_dispatch_deref {
    ($self:ident) => {
        match $self {
            Node::Base(v) => v,
            Node::Camera(v) => v,
            Node::Light(v) => v,
            Node::Sprite(v) => v,
        }
    };
}

impl Deref for Node {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        static_dispatch_deref!(self)
    }
}

impl DerefMut for Node {
    fn deref_mut(&mut self) -> &mut Self::Target {
        static_dispatch_deref!(self)
    }
}

impl Default for Node {
    fn default() -> Self {
        Self::Base(Default::default())
    }
}

impl Node {
    define_is_as!(Node : Camera -> ref Camera => fn is_camera, fn as_camera, fn as_camera_mut);
    define_is_as!(Node : Light -> ref Light => fn is_light, fn as_light, fn as_light_mut);
    define_is_as!(Node : Sprite -> ref Sprite => fn is_sprite, fn as_sprite, fn as_sprite_mut);
}
