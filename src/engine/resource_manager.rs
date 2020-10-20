//! Resource manager controls loading and lifetime of resource in the engine.

use crate::{
    core::visitor::{Visit, VisitResult, Visitor},
    resource::{
        model::{Model, ModelData},
        texture::{Texture, TextureData, TextureMagnificationFilter, TextureMinificationFilter},
        ResourceState,
    },
    sound::buffer::{DataSource, SoundBuffer},
    utils::log::Log,
};
use std::{
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
    time,
};

/// Lifetime of orphaned resource in seconds (with only one strong ref which is resource manager itself)
pub const MAX_RESOURCE_TTL: f32 = 20.0;

/// Resource container with fixed TTL (time-to-live). Resource will be removed
/// (and unloaded) if there were no other strong references to it in given time
/// span.
pub struct TimedEntry<T> {
    /// Payload of entry.
    pub value: T,
    /// Time to live in seconds.
    pub time_to_live: f32,
}

impl<T> Deref for TimedEntry<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for TimedEntry<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T> Default for TimedEntry<T>
where
    T: Default,
{
    fn default() -> Self {
        Self {
            value: Default::default(),
            time_to_live: MAX_RESOURCE_TTL,
        }
    }
}

impl<T> Clone for TimedEntry<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            time_to_live: self.time_to_live,
        }
    }
}

impl<T> Visit for TimedEntry<T>
where
    T: Default + Visit,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.value.visit("Value", visitor)?;
        self.time_to_live.visit("TimeToLive", visitor)?;

        visitor.leave_region()
    }
}

/// Type alias for Arc<Mutex<SoundBuffer>> to make code less noisy.
pub type SharedSoundBuffer = Arc<Mutex<SoundBuffer>>;

/// See module docs.
#[derive(Default)]
pub struct ResourceManagerState {
    textures: Vec<TimedEntry<Texture>>,
    models: Vec<TimedEntry<Model>>,
    sound_buffers: Vec<TimedEntry<SharedSoundBuffer>>,
    /// Path to textures, extensively used for resource files which stores path in weird
    /// format (either relative or absolute) which is obviously not good for engine.
    textures_path: PathBuf,
    textures_import_options: TextureImportOptions,
}

/// See module docs.
#[derive(Clone)]
pub struct ResourceManager {
    state: Option<Arc<Mutex<ResourceManagerState>>>,
}

impl Visit for ResourceManager {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.state.visit("State", visitor)?;

        visitor.leave_region()
    }
}

/// Allows you to define a set of defaults for every imported texture.
#[derive(Clone)]
pub struct TextureImportOptions {
    minification_filter: TextureMinificationFilter,
    magnification_filter: TextureMagnificationFilter,
    anisotropy: f32,
}

impl Default for TextureImportOptions {
    fn default() -> Self {
        Self {
            minification_filter: TextureMinificationFilter::LinearMipMapLinear,
            magnification_filter: TextureMagnificationFilter::Linear,
            anisotropy: 16.0,
        }
    }
}

impl TextureImportOptions {
    /// Sets new minification filter which will be applied to every imported texture as
    /// default value.
    pub fn with_minification_filter(
        mut self,
        minification_filter: TextureMinificationFilter,
    ) -> Self {
        self.minification_filter = minification_filter;
        self
    }

    /// Sets new magnification filter which will be applied to every imported texture as
    /// default value.
    pub fn with_magnification_filter(
        mut self,
        magnification_filter: TextureMagnificationFilter,
    ) -> Self {
        self.magnification_filter = magnification_filter;
        self
    }

    /// Sets new anisotropy level which will be applied to every imported texture as
    /// default value.
    pub fn with_anisotropy(mut self, anisotropy: f32) -> Self {
        self.anisotropy = anisotropy.min(1.0);
        self
    }
}

impl ResourceManager {
    pub(in crate) fn new() -> Self {
        Self {
            state: Some(Arc::new(Mutex::new(ResourceManagerState::new()))),
        }
    }

    /// Returns a guarded reference to internal state of resource manager.
    pub fn state(&self) -> MutexGuard<'_, ResourceManagerState> {
        self.state.as_ref().unwrap().lock().unwrap()
    }

    /// Tries to load texture from given path or get instance of existing, if any. This method is asynchronous,
    /// it immediately returns a texture which can be shared across multiple places, the loading may fail, but it is
    /// internal state of the texture. The engine does not care if texture failed to load, it just won't use
    /// such texture during the rendering. If you need to access internals of the texture you have to get state first
    /// and then use pattern matching to get TextureDetails which contains actual texture data.
    ///
    /// # Async/.await
    ///
    /// Each Texture implements Future trait and can be used in async contexts.
    ///
    /// # Performance
    ///
    /// Currently this method creates a thread which is responsible for actual texture loading, this is very
    /// unoptimal and will be replaced with worker threads in the near future.
    ///
    /// # Supported formats
    ///
    /// To load images and decode them, rg3d uses image create which supports following image
    /// formats: png, tga, bmp, dds, jpg, gif, tiff, dxt.
    pub fn request_texture<P: AsRef<Path>>(&self, path: P) -> Texture {
        let mut state = self.state();

        if let Some(texture) = state.find_texture(path.as_ref()) {
            return texture;
        }

        let texture = Texture::new(ResourceState::Pending {
            path: path.as_ref().to_owned(),
            wakers: Default::default(),
        });
        state.textures.push(TimedEntry {
            value: texture.clone(),
            time_to_live: MAX_RESOURCE_TTL,
        });
        let result = texture.clone();
        let options = state.textures_import_options.clone();

        let path = PathBuf::from(path.as_ref());

        // TODO: Replace with worker threads.
        std::thread::spawn(move || {
            let time = time::Instant::now();
            match TextureData::load_from_file(&path) {
                Ok(mut raw_texture) => {
                    raw_texture.set_magnification_filter(options.magnification_filter);
                    raw_texture.set_minification_filter(options.minification_filter);
                    raw_texture.set_anisotropy_level(options.anisotropy);

                    let mut state = texture.state();

                    let wakers = if let ResourceState::Pending { ref mut wakers, .. } = *state {
                        std::mem::take(wakers)
                    } else {
                        unreachable!()
                    };

                    *state = ResourceState::Ok(raw_texture);

                    Log::writeln(format!(
                        "Texture {:?} is loaded in {:?}!",
                        path,
                        time.elapsed()
                    ));

                    for waker in wakers {
                        waker.wake();
                    }
                }
                Err(error) => {
                    let mut state = texture.state();

                    let wakers = if let ResourceState::Pending { ref mut wakers, .. } = *state {
                        std::mem::take(wakers)
                    } else {
                        unreachable!()
                    };

                    Log::writeln(format!(
                        "Unable to load texture {:?}! Reason {}",
                        &path, &error
                    ));

                    *state = ResourceState::LoadError {
                        path,
                        error: Some(Arc::new(error)),
                    };

                    for waker in wakers {
                        waker.wake();
                    }
                }
            }
        });

        result
    }

    /// Tries to load new model resource from given path or get instance of existing, if any.
    /// This method is **blocking**, so it will block current thread until model is loading
    /// On failure it returns None and prints failure reason to log.
    ///
    /// # Supported formats
    ///
    /// Currently only FBX (common format in game industry for storing complex 3d models)
    /// and RGS (native rusty-editor format) formats are supported.
    pub fn request_model<P: AsRef<Path>>(&self, path: P) -> Model {
        let mut state = self.state();

        if let Some(model) = state.find_model(path.as_ref()) {
            return model;
        }

        let model = Model::new(ResourceState::Pending {
            path: path.as_ref().to_owned(),
            wakers: Default::default(),
        });
        state.models.push(TimedEntry {
            value: model.clone(),
            time_to_live: MAX_RESOURCE_TTL,
        });
        let result = model.clone();
        let path = PathBuf::from(path.as_ref());

        let resource_manager = self.clone();
        // TODO: Replace with worker threads.
        std::thread::spawn(move || match ModelData::load(&path, resource_manager) {
            Ok(raw_model) => {
                let mut state = model.state();

                let wakers = if let ResourceState::Pending { ref mut wakers, .. } = *state {
                    std::mem::take(wakers)
                } else {
                    unreachable!()
                };

                *state = ResourceState::Ok(raw_model);

                Log::writeln(format!("Model {:?} is loaded!", path));

                for waker in wakers {
                    waker.wake();
                }
            }
            Err(error) => {
                let mut state = model.state();

                let wakers = if let ResourceState::Pending { ref mut wakers, .. } = *state {
                    std::mem::take(wakers)
                } else {
                    unreachable!()
                };

                Log::writeln(format!(
                    "Unable to load model from {:?}! Reason {:?}",
                    path, error
                ));

                *state = ResourceState::LoadError {
                    path,
                    error: Some(Arc::new(error)),
                };

                for waker in wakers {
                    waker.wake();
                }
            }
        });

        result
    }

    /// Tries to load new sound buffer from given path or get instance of existing, if any.
    /// This method is **blocking**, so it will block current thread until sound buffer is
    /// loading. On failure it returns None and prints failure reason to log.
    ///
    /// # Supported formats
    ///
    /// Currently only WAV (uncompressed) and OGG are supported.
    pub fn request_sound_buffer<P: AsRef<Path>>(
        &self,
        path: P,
        stream: bool,
    ) -> Option<SharedSoundBuffer> {
        let mut state = self.state();

        if let Some(sound_buffer) = state.find_sound_buffer(path.as_ref()) {
            return Some(sound_buffer);
        }

        match DataSource::from_file(path.as_ref()) {
            Ok(source) => {
                let buffer = if stream {
                    SoundBuffer::new_streaming(source)
                } else {
                    SoundBuffer::new_generic(source)
                };
                match buffer {
                    Ok(sound_buffer) => {
                        state.sound_buffers.push(TimedEntry {
                            value: sound_buffer.clone(),
                            time_to_live: MAX_RESOURCE_TTL,
                        });
                        Log::writeln(format!(
                            "Sound buffer {} is loaded!",
                            path.as_ref().display()
                        ));
                        Some(sound_buffer)
                    }
                    Err(_) => {
                        Log::writeln(format!(
                            "Unable to load sound buffer from {}!",
                            path.as_ref().display()
                        ));
                        None
                    }
                }
            }
            Err(e) => {
                Log::writeln(format!("Invalid data source: {:?}", e));
                None
            }
        }
    }

    /// Reloads every loaded texture. This method is **blocking**.
    ///
    /// TODO: Make this async.
    pub fn reload_textures(&self) {
        let textures = self.state().textures.to_vec();
        for old_texture in textures {
            let mut old_texture_state = old_texture.state();
            let details = match TextureData::load_from_file(old_texture_state.path()) {
                Ok(texture) => {
                    Log::writeln(format!(
                        "Texture {:?} successfully reloaded!",
                        old_texture_state.path(),
                    ));

                    texture
                }
                Err(e) => {
                    Log::writeln(format!(
                        "Unable to reload {:?} texture! Reason: {}",
                        old_texture_state.path(),
                        e
                    ));
                    continue;
                }
            };
            *old_texture_state = ResourceState::Ok(details);
        }
    }

    /// Reloads every loaded model. This method is **blocking**.
    ///
    /// TODO: Make this async.
    pub fn reload_models(&self) {
        let models = self.state().models.to_vec();
        for old_model in models {
            let mut old_model = old_model.state();
            let new_model = match ModelData::load(old_model.path(), self.clone()) {
                Ok(new_model) => new_model,
                Err(e) => {
                    Log::writeln(format!(
                        "Unable to reload {:?} model! Reason: {:?}",
                        old_model.path(),
                        e
                    ));
                    continue;
                }
            };
            *old_model = ResourceState::Ok(new_model);
        }
    }

    /// Reloads every loaded sound buffer. This method is **blocking**.
    ///
    /// TODO: Make this async.
    pub fn reload_sound_buffers(&self) {
        let sound_buffers = self.state().sound_buffers.to_vec();
        for old_sound_buffer in sound_buffers {
            let mut old_sound_buffer = old_sound_buffer.lock().unwrap();
            if let Some(ext_path) = old_sound_buffer.external_data_path() {
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

    /// Reloads all loaded resources. Normally it should never be called, because it is **very** heavy
    /// method! This method is **blocking**.
    ///
    /// TODO: Make this async.
    pub fn reload_resources(&self) {
        self.reload_textures();
        self.reload_models();
        self.reload_sound_buffers();
    }
}

impl ResourceManagerState {
    pub(in crate::engine) fn new() -> Self {
        Self {
            textures: Vec::new(),
            models: Vec::new(),
            sound_buffers: Vec::new(),
            textures_path: PathBuf::from("data/textures/"),
            textures_import_options: Default::default(),
        }
    }

    /// Sets new import options for textures. Previously loaded textures won't be affected by the
    /// new settings.
    pub fn set_textures_import_options(&mut self, options: TextureImportOptions) {
        self.textures_import_options = options;
    }

    /// Returns shared reference to list of available textures.
    #[inline]
    pub fn textures(&self) -> &[TimedEntry<Texture>] {
        &self.textures
    }

    /// Tries to find texture by its path. Returns None if no such texture was found.
    pub fn find_texture<P: AsRef<Path>>(&self, path: P) -> Option<Texture> {
        for texture_entry in self.textures.iter() {
            if texture_entry.state().path() == path.as_ref() {
                return Some(texture_entry.value.clone());
            }
        }
        None
    }

    /// Returns shared reference to list of available models.
    #[inline]
    pub fn models(&self) -> &[TimedEntry<Model>] {
        &self.models
    }

    /// Tries to find model by its path. Returns None if no such model was found.
    pub fn find_model<P: AsRef<Path>>(&self, path: P) -> Option<Model> {
        for model in self.models.iter() {
            if model.state().path() == path.as_ref() {
                return Some(model.value.clone());
            }
        }
        None
    }

    /// Returns shared reference to list of sound buffers.
    #[inline]
    pub fn sound_buffers(&self) -> &[TimedEntry<SharedSoundBuffer>] {
        &self.sound_buffers
    }

    /// Tries to find sound buffer by its path. Returns None if no such sound buffer was found.
    pub fn find_sound_buffer<P: AsRef<Path>>(&self, path: P) -> Option<SharedSoundBuffer> {
        for sound_buffer in self.sound_buffers.iter() {
            if let Some(ext_path) = sound_buffer.lock().unwrap().external_data_path() {
                if ext_path == path.as_ref() {
                    return Some(sound_buffer.value.clone());
                }
            }
        }
        None
    }

    /// Returns current path where to search texture when loading complex model resources.
    #[inline]
    pub fn textures_path(&self) -> &Path {
        self.textures_path.as_path()
    }

    /// Sets new path where engine should search textures when it loads a model from external
    /// non-native format. Most 3d model formats uses absolute paths to textures, this is
    /// bad for engine, because all paths to data must be in relative format, otherwise it
    /// would be tightly coupled with environment where a model was made. This path should
    /// lead to a folder where all textures are located. **CAVEAT** Sub-folders are **not**
    /// supported!
    #[inline]
    pub fn set_textures_path<P: AsRef<Path>>(&mut self, path: P) {
        self.textures_path = path.as_ref().to_owned();
    }

    /// Immediately destroys all unused resources.
    pub fn purge_unused_resources(&mut self) {
        self.sound_buffers
            .retain(|buffer| Arc::strong_count(&buffer.value) > 1);
        self.models.retain(|buffer| buffer.value.use_count() > 1);
        self.textures.retain(|buffer| buffer.value.use_count() > 1);
    }

    fn update_textures(&mut self, dt: f32) {
        for texture in self.textures.iter_mut() {
            let ok = if let ResourceState::Ok(_) = *texture.state() {
                true
            } else {
                false
            };
            if ok {
                texture.time_to_live -= dt;
                if texture.use_count() > 1 {
                    texture.time_to_live = MAX_RESOURCE_TTL;
                }
            }
        }
        self.textures.retain(|texture| {
            let retain = texture.time_to_live > 0.0;
            if !retain && texture.state().path().exists() {
                Log::writeln(format!(
                    "Texture resource {:?} destroyed because it not used anymore!",
                    texture.state().path()
                ));
            }
            retain
        });
    }

    fn update_model(&mut self, dt: f32) {
        for model in self.models.iter_mut() {
            model.time_to_live -= dt;
            if model.use_count() > 1 {
                model.time_to_live = MAX_RESOURCE_TTL;
            }
        }
        self.models.retain(|model| {
            let retain = model.time_to_live > 0.0;
            if !retain && model.state().path().exists() {
                Log::writeln(format!(
                    "Model resource {:?} destroyed because it not used anymore!",
                    model.state().path()
                ));
            }
            retain
        });
    }

    fn update_sound_buffers(&mut self, dt: f32) {
        for buffer in self.sound_buffers.iter_mut() {
            buffer.time_to_live -= dt;
            if Arc::strong_count(buffer) > 1 {
                buffer.time_to_live = MAX_RESOURCE_TTL;
            }
        }
        self.sound_buffers.retain(|buffer| {
            let retain = buffer.time_to_live > 0.0;
            if !retain {
                if let Some(path) = buffer.lock().unwrap().external_data_path().as_ref() {
                    Log::writeln(format!(
                        "Sound resource {:?} destroyed because it not used anymore!",
                        path
                    ));
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
}

impl Visit for ResourceManagerState {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        futures::executor::block_on(futures::future::join_all(
            self.textures.iter().map(|t| t.value.clone()),
        ));
        futures::executor::block_on(futures::future::join_all(
            self.models.iter().map(|m| m.value.clone()),
        ));

        self.textures_path.visit("TexturesPath", visitor)?;
        self.textures.visit("Textures", visitor)?;
        self.models.visit("Models", visitor)?;
        self.sound_buffers.visit("SoundBuffers", visitor)?;

        visitor.leave_region()
    }
}
