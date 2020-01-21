use std::{path::{PathBuf, Path}, sync::{Arc, Mutex}, time};
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

pub struct ResourceManager {
    textures: Vec<Arc<Mutex<Texture>>>,
    models: Vec<Arc<Mutex<Model>>>,
    sound_buffers: Vec<Arc<Mutex<SoundBuffer>>>,
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

    pub fn request_texture_async<P: AsRef<Path>>(&mut self, path: P, kind: TextureKind) -> Option<Arc<Mutex<Texture>>> {
        if let Some(texture) = self.find_texture(path.as_ref()) {
            return Some(texture);
        }

        let texture = Arc::new(Mutex::new(Texture::default()));
        self.textures.push(texture.clone());
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

        Some(result)
    }

    pub fn request_texture<P: AsRef<Path>>(&mut self, path: P, kind: TextureKind) -> Option<Arc<Mutex<Texture>>> {
        if let Some(texture) = self.find_texture(path.as_ref()) {
            return Some(texture);
        }

        match Texture::load_from_file(path.as_ref(), kind) {
            Ok(texture) => {
                let shared_texture = Arc::new(Mutex::new(texture));
                self.textures.push(shared_texture.clone());
                Log::writeln(format!("Texture {} is loaded!", path.as_ref().display()));
                Some(shared_texture)
            }
            Err(e) => {
                Log::writeln(format!("Unable to load texture {}! Reason {}", path.as_ref().display(), e));
                None
            }
        }
    }

    pub fn request_model<P: AsRef<Path>>(&mut self, path: P) -> Option<Arc<Mutex<Model>>> {
        if let Some(model) = self.find_model(path.as_ref()) {
            return Some(model);
        }

        let extension = path.as_ref().extension().
            and_then(|os| os.to_str()).
            map_or(String::from(""), |s| s.to_ascii_lowercase());

        match extension.as_str() {
            "fbx" => match Model::load(path.as_ref(), self) {
                Ok(model) => {
                    let model = Arc::new(Mutex::new(model));
                    model.lock().unwrap().self_weak_ref = Some(Arc::downgrade(&model));
                    self.models.push(model.clone());
                    Log::writeln(format!("Model {} is loaded!", path.as_ref().display()));
                    Some(model)
                }
                Err(e) => {
                    Log::writeln(format!("Unable to load model from {}! Reason {}", path.as_ref().display(), e));
                    None
                }
            },
            _ => {
                Log::writeln(format!("Unsupported model type {}!", path.as_ref().display()));
                None
            }
        }
    }

    pub fn request_sound_buffer<P: AsRef<Path>>(&mut self, path: P, stream: bool) -> Option<Arc<Mutex<SoundBuffer>>> {
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
                self.sound_buffers.push(sound_buffer.clone());
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
    pub fn get_textures(&self) -> &[Arc<Mutex<Texture>>] {
        &self.textures
    }

    pub fn find_texture<P: AsRef<Path>>(&self, path: P) -> Option<Arc<Mutex<Texture>>> {
        for texture in self.textures.iter() {
            if texture.lock().unwrap().path.as_path() == path.as_ref() {
                return Some(texture.clone());
            }
        }
        None
    }

    #[inline]
    pub fn get_models(&self) -> &[Arc<Mutex<Model>>] {
        &self.models
    }

    pub fn find_model<P: AsRef<Path>>(&self, path: P) -> Option<Arc<Mutex<Model>>> {
        for model in self.models.iter() {
            if model.lock().unwrap().path.as_path() == path.as_ref() {
                return Some(model.clone());
            }
        }
        None
    }

    #[inline]
    pub fn get_sound_buffers(&self) -> &[Arc<Mutex<SoundBuffer>>] {
        &self.sound_buffers
    }

    pub fn find_sound_buffer<P: AsRef<Path>>(&self, path: P) -> Option<Arc<Mutex<SoundBuffer>>> {
        for sound_buffer in self.sound_buffers.iter() {
            if let Some(ext_path) = sound_buffer.lock().unwrap().generic().external_data_path() {
                if ext_path == path.as_ref() {
                    return Some(sound_buffer.clone());
                }
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
            !resource.lock().unwrap().loaded || Arc::strong_count(resource) > 1
        });

        self.models.retain(|models| {
            Arc::strong_count(models) > 1
        });

        self.sound_buffers.retain(|sound_buffer| {
            Arc::strong_count(sound_buffer) > 1
        });
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

        for old_model in self.get_models().to_vec() {
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

        for old_sound_buffer in self.get_sound_buffers() {
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