use std::sync::{Arc, Mutex};
use crate::resource::texture::Texture;
use rg3d_core::{
    color::Color,
    visitor::{Visit, VisitResult, Visitor}
};

#[derive(Clone)]
pub struct Sprite {
    texture: Option<Arc<Mutex<Texture>>>,
    color: Color,
    size: f32,
}

impl Default for Sprite {
    fn default() -> Self {
        Self::new()
    }
}

impl Sprite {
    pub fn new() -> Self {
        Self {
            texture: None,
            color: Color::white(),
            size: 0.2
        }
    }

    pub fn set_size(&mut self, size: f32) {
        self.size = size;
    }

    pub fn get_size(&self) -> f32 {
        self.size
    }

    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    pub fn get_color(&self) -> Color {
        self.color
    }

    pub fn set_texture(&mut self, texture: Arc<Mutex<Texture>>) {
        self.texture = Some(texture);
    }

    pub fn get_texture(&self) -> Option<Arc<Mutex<Texture>>> {
        self.texture.clone()
    }
}

impl Visit for Sprite {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.texture.visit("Texture", visitor)?;
        self.color.visit("Color", visitor)?;

        visitor.leave_region()
    }
}