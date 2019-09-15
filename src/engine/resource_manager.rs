use std::{
    path::{PathBuf, Path},
    sync::{Arc, Mutex},
};
use crate::resource::{
    texture::Texture,
    model::Model,
};
use rg3d_core::{
    visitor::{Visitor, VisitResult, Visit}
};
use rg3d_sound::buffer::Buffer;

pub struct ResourceManager {
    textures: Vec<Arc<Mutex<Texture>>>,
    models: Vec<Arc<Mutex<Model>>>,
    sound_buffers: Vec<Arc<Mutex<Buffer>>>,
    /// Path to textures, extensively used for resource files
    /// which stores path in weird format (either relative or absolute) which
    /// is obviously not good for engine.
    textures_path: PathBuf,
}

impl ResourceManager {
    pub(in crate::engine) fn new() -> ResourceManager {
        Self {
            textures: Vec::new(),
            models: Vec::new(),
            sound_buffers: Vec::new(),
            textures_path: PathBuf::from("data/textures/"),
        }
    }

    #[inline]
    pub fn add_texture(&mut self, texture: Arc<Mutex<Texture>>) {
        self.textures.push(texture)
    }

    #[inline]
    pub fn get_textures(&self) -> &[Arc<Mutex<Texture>>] {
        &self.textures
    }

    pub fn find_texture(&self, path: &Path) -> Option<Arc<Mutex<Texture>>> {
        for texture in self.textures.iter() {
            if texture.lock().unwrap().path.as_path() == path {
                return Some(texture.clone());
            }
        }
        None
    }

    #[inline]
    pub fn add_model(&mut self, model: Arc<Mutex<Model>>) {
        self.models.push(model)
    }

    #[inline]
    pub fn get_models(&self) -> &[Arc<Mutex<Model>>] {
        &self.models
    }

    pub fn find_model(&self, path: &Path) -> Option<Arc<Mutex<Model>>> {
        for model in self.models.iter() {
            if model.lock().unwrap().path.as_path() == path {
                return Some(model.clone());
            }
        }
        None
    }

    #[inline]
    pub fn add_sound_buffer(&mut self, sound_buffer: Arc<Mutex<Buffer>>) {
        self.sound_buffers.push(sound_buffer)
    }

    #[inline]
    pub fn get_sound_buffers(&self) -> &[Arc<Mutex<Buffer>>] {
        &self.sound_buffers
    }

    pub fn find_sound_buffer(&self, path: &Path) -> Option<Arc<Mutex<Buffer>>> {
        for sound_buffer in self.sound_buffers.iter() {
            if sound_buffer.lock().unwrap().get_source_path() == path {
                return Some(sound_buffer.clone());
            }
        }
        None
    }

    #[inline]
    pub fn get_textures_path(&self) -> &Path {
        self.textures_path.as_path()
    }

    pub fn update(&mut self) {
        self.textures.retain(|resource| {
            Arc::strong_count(resource) > 1
        });

        self.models.retain(|models| {
            Arc::strong_count(models) > 1
        });

        self.sound_buffers.retain(|sound_buffer| {
            Arc::strong_count(sound_buffer) > 1
        });
    }
}

impl Visit for ResourceManager {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.textures.visit("Textures", visitor)?;
        self.models.visit("Models", visitor)?;
        self.sound_buffers.visit("SoundBuffers", visitor)?;

        visitor.leave_region()
    }
}