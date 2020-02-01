use std::{
    path::{PathBuf, Path},
    sync::{Arc, Mutex},
    time,
};
use crate::{
    sound::buffer::{SoundBuffer, DataSource},
    core::{
        visitor::{Visitor, VisitResult, Visit}
    },
    resource::{
        texture::Texture,
        model::Model,
        texture::TextureKind,
    },
    utils::log::Log,
};
use std::ops::Deref;

pub struct Entry<T> {
    pub value: T,
    time_to_live: f32,
}

impl<T> Deref for Entry<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> Default for Entry<T> where T: Default {
    fn default() -> Self {
        Self {
            value: Default::default(),
            time_to_live: ResourceManager::MAX_RESOURCE_TTL,
        }
    }
}

impl<T> Clone for Entry<T> where T: Clone {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            time_to_live: self.time_to_live
        }
    }
}

impl<T> Visit for Entry<T> where T: Default + Visit {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.value.visit("Value", visitor)?;
        self.time_to_live.visit("TimeToLive", visitor)?;

        visitor.leave_region()
    }
}

pub type SharedTexture = Arc<Mutex<Texture>>;
pub type SharedModel = Arc<Mutex<Model>>;
pub type SharedSoundBuffer = Arc<Mutex<SoundBuffer>>;

pub struct ResourceManager {
    textures: Vec<Entry<SharedTexture>>,
    models: Vec<Entry<SharedModel>>,
    sound_buffers: Vec<Entry<SharedSoundBuffer>>,
    /// Path to textures, extensively used for resource files which stores path in weird
    /// format (either relative or absolute) which is obviously not good for engine.
    textures_path: PathBuf,
}

impl ResourceManager {
    /// Lifetime of orphaned resource in seconds (with only one strong ref which is resource manager itself)
    pub const MAX_RESOURCE_TTL: f32 = 20.0;

    pub(in crate::engine) fn new() -> ResourceManager {
        Self {
            textures: Vec::new(),
            models: Vec::new(),
            sound_buffers: Vec::new(),
            textures_path: PathBuf::from("data/textures/"),
        }
    }

    /// Experimental async texture loader. Always returns valid texture object which could still
    /// be not loaded, you should check is_loaded flag to ensure.
    ///
    /// It extensively used in model loader to speed up loading.
    pub fn request_texture_async<P: AsRef<Path>>(&mut self, path: P, kind: TextureKind) -> SharedTexture {
        if let Some(texture) = self.find_texture(path.as_ref()) {
            return texture;
        }

        let texture = Arc::new(Mutex::new(Texture::default()));
        self.textures.push(Entry {
            value: texture.clone(),
            time_to_live: Self::MAX_RESOURCE_TTL,
        });
        let result = texture.clone();

        let path = PathBuf::from(path.as_ref());
        std::thread::spawn(move || {
            if let Ok(mut texture) = texture.lock() {
                let time = time::Instant::now();
                match Texture::load_from_file(&path, kind) {
                    Ok(raw_texture) => {
                        *texture = raw_texture;
                        Log::writeln(format!("Texture {:?} is loaded in {:?}!", path, time.elapsed()));
                    }
                    Err(e) => {
                        Log::writeln(format!("Unable to load texture {:?}! Reason {}", path, e));
                    }
                }
            }
        });

        result
    }

    pub fn request_texture<P: AsRef<Path>>(&mut self, path: P, kind: TextureKind) -> Option<SharedTexture> {
        if let Some(texture) = self.find_texture(path.as_ref()) {
            return Some(texture);
        }

        match Texture::load_from_file(path.as_ref(), kind) {
            Ok(texture) => {
                let shared_texture = Arc::new(Mutex::new(texture));
                self.textures.push(Entry {
                    value: shared_texture.clone(),
                    time_to_live: Self::MAX_RESOURCE_TTL,
                });
                Log::writeln(format!("Texture {} is loaded!", path.as_ref().display()));
                Some(shared_texture)
            }
            Err(e) => {
                Log::writeln(format!("Unable to load texture {}! Reason {}", path.as_ref().display(), e));
                None
            }
        }
    }

    pub fn request_model<P: AsRef<Path>>(&mut self, path: P) -> Option<SharedModel> {
        if let Some(model) = self.find_model(path.as_ref()) {
            return Some(model);
        }

        match Model::load(path.as_ref(), self) {
            Ok(model) => {
                let model = Arc::new(Mutex::new(model));
                model.lock().unwrap().self_weak_ref = Some(Arc::downgrade(&model));
                self.models.push(Entry {
                    value: model.clone(),
                    time_to_live: Self::MAX_RESOURCE_TTL
                });
                Log::writeln(format!("Model {} is loaded!", path.as_ref().display()));
                Some(model)
            }
            Err(e) => {
                Log::writeln(format!("Unable to load model from {}! Reason {}", path.as_ref().display(), e));
                None
            }
        }
    }

    pub fn request_sound_buffer<P: AsRef<Path>>(&mut self, path: P, stream: bool) -> Option<SharedSoundBuffer> {
        if let Some(sound_buffer) = self.find_sound_buffer(path.as_ref()) {
            return Some(sound_buffer);
        }

        let source = match DataSource::from_file(path.as_ref()) {
            Ok(source) => source,
            Err(e) => {
                Log::writeln(format!("Invalid data source: {:?}", e));
                return None;
            }
        };

        let buffer = if stream {
            SoundBuffer::new_streaming(source)
        } else {
            SoundBuffer::new_generic(source)
        };
        match buffer {
            Ok(sound_buffer) => {
                self.sound_buffers.push(Entry {
                    value: sound_buffer.clone(),
                    time_to_live: Self::MAX_RESOURCE_TTL
                });
                Log::writeln(format!("Sound buffer {} is loaded!", path.as_ref().display()));
                Some(sound_buffer)
            }
            Err(_) => {
                Log::writeln(format!("Unable to load sound buffer from {}!", path.as_ref().display()));
                None
            }
        }
    }

    #[inline]
    pub fn textures(&self) -> &[Entry<SharedTexture>] {
        &self.textures
    }

    pub fn find_texture<P: AsRef<Path>>(&self, path: P) -> Option<SharedTexture> {
        for texture_entry in self.textures.iter() {
            if texture_entry.lock().unwrap().path.as_path() == path.as_ref() {
                return Some(texture_entry.value.clone());
            }
        }
        None
    }

    #[inline]
    pub fn models(&self) -> &[Entry<SharedModel>] {
        &self.models
    }

    pub fn find_model<P: AsRef<Path>>(&self, path: P) -> Option<SharedModel> {
        for model in self.models.iter() {
            if model.lock().unwrap().path.as_path() == path.as_ref() {
                return Some(model.value.clone());
            }
        }
        None
    }

    #[inline]
    pub fn sound_buffers(&self) -> &[Entry<SharedSoundBuffer>] {
        &self.sound_buffers
    }

    pub fn find_sound_buffer<P: AsRef<Path>>(&self, path: P) -> Option<SharedSoundBuffer> {
        for sound_buffer in self.sound_buffers.iter() {
            if let Some(ext_path) = sound_buffer.lock().unwrap().generic().external_data_path() {
                if ext_path == path.as_ref() {
                    return Some(sound_buffer.value.clone());
                }
            }
        }
        None
    }

    #[inline]
    pub fn get_textures_path(&self) -> &Path {
        self.textures_path.as_path()
    }

    fn update_textures(&mut self, dt: f32) {
        for texture in self.textures.iter_mut() {
            texture.time_to_live -= dt;
            if texture.lock().unwrap().loaded && Arc::strong_count(texture) > 1 {
                texture.time_to_live = Self::MAX_RESOURCE_TTL;
            }
        }
        self.textures.retain(|texture| {
            let retain = texture.time_to_live > 0.0;
            if !retain && texture.lock().unwrap().path.exists() {
                Log::writeln(format!("Texture resource {:?} destroyed because it not used anymore!", texture.lock().unwrap().path));
            }
            retain
        });
    }

    fn update_model(&mut self, dt: f32) {
        for model in self.models.iter_mut() {
            model.time_to_live -= dt;
            if Arc::strong_count(model) > 1 {
                model.time_to_live = Self::MAX_RESOURCE_TTL;
            }
        }
        self.models.retain(|model| {
            let retain = model.time_to_live > 0.0;
            if !retain && model.lock().unwrap().path.exists() {
                Log::writeln(format!("Model resource {:?} destroyed because it not used anymore!", model.lock().unwrap().path.exists()));
            }
            retain
        });
    }

    fn update_sound_buffers(&mut self, dt: f32) {
        for buffer in self.sound_buffers.iter_mut() {
            buffer.time_to_live -= dt;
            if Arc::strong_count(buffer) > 1 {
                buffer.time_to_live = Self::MAX_RESOURCE_TTL;
            }
        }
        self.sound_buffers.retain(|buffer| {
            let retain = buffer.time_to_live > 0.0;
            if !retain {
                if let Some(path) = buffer.lock().unwrap().generic().external_data_path().as_ref() {
                    Log::writeln(format!("Sound resource {:?} destroyed because it not used anymore!", path));
                }
            }
            retain
        });
    }

    pub(in crate) fn update(&mut self, dt: f32) {
        self.update_textures(dt);
        self.update_model(dt);
        self.update_sound_buffers(dt);
    }

    pub fn reload_resources(&mut self) {
        for old_texture in self.textures.iter() {
            let mut old_texture = old_texture.lock().unwrap();
            let new_texture = match Texture::load_from_file(old_texture.path.as_path(), old_texture.kind) {
                Ok(texture) => texture,
                Err(e) => {
                    Log::writeln(format!("Unable to reload {:?} texture! Reason: {}", old_texture.path, e));
                    continue;
                }
            };
            old_texture.path = Default::default();
            *old_texture = new_texture;
        }

        for old_model in self.models().to_vec() {
            let old_model_arc = old_model.clone();
            let mut old_model = old_model.lock().unwrap();
            let mut new_model = match Model::load(old_model.path.as_path(), self) {
                Ok(new_model) => new_model,
                Err(e) => {
                    Log::writeln(format!("Unable to reload {:?} model! Reason: {}", old_model.path, e));
                    continue;
                }
            };
            new_model.self_weak_ref = Some(Arc::downgrade(&old_model_arc));
            old_model.path = Default::default();
            *old_model = new_model;
        }

        for old_sound_buffer in self.sound_buffers() {
            let mut old_sound_buffer = old_sound_buffer.lock().unwrap();
            if let Some(ext_path) = old_sound_buffer.generic().external_data_path() {
                if let Ok(data_source) = DataSource::from_file(ext_path.as_path()) {
                    let new_sound_buffer = match *old_sound_buffer {
                        SoundBuffer::Generic(_) => SoundBuffer::raw_generic(data_source),
                        SoundBuffer::Streaming(_) => SoundBuffer::raw_streaming(data_source),
                    };
                    let new_sound_buffer = match new_sound_buffer {
                        Ok(new_sound_buffer) => new_sound_buffer,
                        Err(_) => {
                            Log::writeln(format!("Unable to reload {:?} sound buffer!", ext_path));
                            continue;
                        }
                    };
                    *old_sound_buffer = new_sound_buffer;
                }
            }
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