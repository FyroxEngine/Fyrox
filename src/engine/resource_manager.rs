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
use rg3d_sound::buffer::{Buffer, BufferKind};
use crate::resource::texture::TextureKind;

pub struct ResourceManager {
    textures: Vec<Arc<Mutex<Texture>>>,
    models: Vec<Arc<Mutex<Model>>>,
    sound_buffers: Vec<Arc<Mutex<Buffer>>>,
    /// Path to textures, extensively used for resource files which stores path in weird
    /// format (either relative or absolute) which is obviously not good for engine.
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

    pub fn request_texture(&mut self, path: &Path, kind: TextureKind) -> Option<Arc<Mutex<Texture>>> {
        if let Some(texture) = self.find_texture(path) {
            return Some(texture);
        }

        let extension = path.extension().
            and_then(|os| os.to_str()).
            map_or(String::from(""), |s| s.to_ascii_lowercase());

        match extension.as_str() {
            "jpg" | "jpeg" | "png" | "tif" | "tiff" | "tga" | "bmp" => match Texture::load_from_file(path, kind) {
                Ok(texture) => {
                    let shared_texture = Arc::new(Mutex::new(texture));
                    self.textures.push(shared_texture.clone());
                    println!("Texture {} is loaded!", path.display());
                    Some(shared_texture)
                }
                Err(e) => {
                    println!("Unable to load texture {}! Reason {}", path.display(), e);
                    None
                }
            }
            _ => {
                println!("Unsupported texture type {}!", path.display());
                None
            }
        }
    }

    pub fn request_model(&mut self, path: &Path) -> Option<Arc<Mutex<Model>>> {
        if let Some(model) = self.find_model(path) {
            return Some(model);
        }

        let extension = path.extension().
            and_then(|os| os.to_str()).
            map_or(String::from(""), |s| s.to_ascii_lowercase());

        match extension.as_str() {
            "fbx" => match Model::load(path, self) {
                Ok(model) => {
                    let model = Arc::new(Mutex::new(model));
                    self.models.push(model.clone());
                    println!("Model {} is loaded!", path.display());
                    Some(model)
                }
                Err(e) => {
                    println!("Unable to load model from {}! Reason {}", path.display(), e);
                    None
                }
            },
            _ => {
                println!("Unsupported model type {}!", path.display());
                None
            }
        }
    }

    pub fn request_sound_buffer(&mut self, path: &Path, kind: BufferKind) -> Option<Arc<Mutex<Buffer>>> {
        if let Some(sound_buffer) = self.find_sound_buffer(path) {
            return Some(sound_buffer);
        }

        let extension = path.extension().
            and_then(|os| os.to_str()).
            map_or(String::from(""), |s| s.to_ascii_lowercase());

        match extension.as_str() {
            "wav" => match Buffer::new(path, kind) {
                Ok(sound_buffer) => {
                    let sound_buffer = Arc::new(Mutex::new(sound_buffer));
                    self.sound_buffers.push(sound_buffer.clone());
                    println!("Model {} is loaded!", path.display());
                    Some(sound_buffer)
                }
                Err(e) => {
                    println!("Unable to load sound buffer from {}! Reason {}", path.display(), e);
                    None
                }
            },
            _ => {
                println!("Unsupported sound buffer type {}!", path.display());
                None
            }
        }
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

    pub(in crate) fn update(&mut self) {
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

    pub fn reload_resources(&mut self) {
        for old_texture in self.get_textures() {
            let mut old_texture = old_texture.lock().unwrap();
            let new_texture = match Texture::load_from_file(old_texture.path.as_path(), old_texture.kind) {
                Ok(texture) => texture,
                Err(e) => {
                    println!("Unable to reload {:?} texture! Reason: {}", old_texture.path, e);
                    continue;
                }
            };

            *old_texture = new_texture;
        }

        for old_model in self.get_models().to_vec() {
            let mut old_model = old_model.lock().unwrap();
            let new_model = match Model::load(old_model.path.as_path(),self) {
                Ok(new_model) => new_model,
                Err(e) => {
                    println!("Unable to reload {:?} model! Reason: {}", old_model.path, e);
                    continue;
                }
            };

            *old_model = new_model;
        }

        for old_sound_buffer in self.get_sound_buffers() {
            let mut old_sound_buffer = old_sound_buffer.lock().unwrap();
            let new_sound_buffer = match Buffer::new(old_sound_buffer.get_source_path(), old_sound_buffer.get_kind()) {
                Ok(new_sound_buffer) => new_sound_buffer,
                Err(e) => {
                    println!("Unable to reload {:?} sound buffer! Reason: {}", old_sound_buffer.get_source_path(), e);
                    continue;
                }
            };

            *old_sound_buffer = new_sound_buffer;
        }
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