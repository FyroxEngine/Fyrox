//! Resource manager controls loading and lifetime of resource in the engine.

use crate::{
    core::{futures::executor::ThreadPool, instant, visitor::prelude::*},
    renderer::TextureUploadSender,
    resource::{
        model::{Model, ModelData},
        texture::{
            CompressionOptions, Texture, TextureData, TextureError, TextureMagnificationFilter,
            TextureMinificationFilter, TexturePixelKind, TextureState, TextureWrapMode,
        },
        Resource, ResourceData, ResourceLoadError, ResourceState,
    },
    sound::buffer::{DataSource, SoundBuffer},
    utils::log::{Log, MessageKind},
};
use std::{
    borrow::Cow,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
};

/// Lifetime of orphaned resource in seconds (with only one strong ref which is resource manager itself)
pub const DEFAULT_RESOURCE_LIFETIME: f32 = 60.0;

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
            time_to_live: DEFAULT_RESOURCE_LIFETIME,
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

impl ResourceData for Arc<Mutex<SoundBuffer>> {
    fn path(&self) -> Cow<Path> {
        self.lock()
            .unwrap()
            .external_data_path()
            .map(|p| Cow::Owned(p.to_owned()))
            .unwrap_or_else(|| Cow::Owned(Path::new("").to_owned()))
    }
}

/// Type alias for sound buffer resource.
pub type SharedSoundBuffer = Resource<Arc<Mutex<SoundBuffer>>, ()>;

impl Into<Arc<Mutex<SoundBuffer>>> for SharedSoundBuffer {
    fn into(self) -> Arc<Mutex<SoundBuffer>> {
        self.data_ref().clone()
    }
}

/// See module docs.
pub struct ResourceManagerState {
    textures: Vec<TimedEntry<Texture>>,
    models: Vec<TimedEntry<Model>>,
    sound_buffers: Vec<TimedEntry<SharedSoundBuffer>>,
    textures_import_options: TextureImportOptions,
    #[cfg(not(target_arch = "wasm32"))]
    thread_pool: ThreadPool,
    pub(in crate) upload_sender: Option<TextureUploadSender>,
}

impl Default for ResourceManagerState {
    fn default() -> Self {
        Self {
            textures: Default::default(),
            models: Default::default(),
            sound_buffers: Default::default(),
            textures_import_options: Default::default(),
            #[cfg(not(target_arch = "wasm32"))]
            thread_pool: ThreadPool::new().unwrap(),
            upload_sender: None,
        }
    }
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
    s_wrap_mode: TextureWrapMode,
    t_wrap_mode: TextureWrapMode,
    anisotropy: f32,
    compression: CompressionOptions,
}

impl Default for TextureImportOptions {
    fn default() -> Self {
        Self {
            minification_filter: TextureMinificationFilter::LinearMipMapLinear,
            magnification_filter: TextureMagnificationFilter::Linear,
            s_wrap_mode: TextureWrapMode::Repeat,
            t_wrap_mode: TextureWrapMode::Repeat,
            anisotropy: 16.0,
            compression: CompressionOptions::Quality,
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

    /// Sets new S coordinate wrap mode which will be applied to every imported texture as
    /// default value.
    pub fn with_s_wrap_mode(mut self, s_wrap_mode: TextureWrapMode) -> Self {
        self.s_wrap_mode = s_wrap_mode;
        self
    }

    /// Sets new T coordinate wrap mode which will be applied to every imported texture as
    /// default value.
    pub fn with_t_wrap_mode(mut self, t_wrap_mode: TextureWrapMode) -> Self {
        self.t_wrap_mode = t_wrap_mode;
        self
    }

    /// Sets new anisotropy level which will be applied to every imported texture as
    /// default value.
    pub fn with_anisotropy(mut self, anisotropy: f32) -> Self {
        self.anisotropy = anisotropy.min(1.0);
        self
    }

    /// Sets desired texture compression.
    pub fn with_compression(mut self, compression: CompressionOptions) -> Self {
        self.compression = compression;
        self
    }
}

/// An error that may occur during texture registration.
#[derive(Debug)]
pub enum TextureRegistrationError {
    /// Texture saving has failed.
    Texture(TextureError),
    /// Texture was in invalid state (Pending, LoadErr)
    InvalidState,
    /// Texture is already registered.
    AlreadyRegistered,
}

impl From<TextureError> for TextureRegistrationError {
    fn from(e: TextureError) -> Self {
        Self::Texture(e)
    }
}

async fn load_texture(
    texture: Texture,
    path: PathBuf,
    options: TextureImportOptions,
    upload_sender: TextureUploadSender,
) {
    let time = instant::Instant::now();
    match TextureData::load_from_file(&path, options.compression).await {
        Ok(mut raw_texture) => {
            Log::writeln(
                MessageKind::Information,
                format!("Texture {:?} is loaded in {:?}!", path, time.elapsed()),
            );

            raw_texture.set_magnification_filter(options.magnification_filter);
            raw_texture.set_minification_filter(options.minification_filter);
            raw_texture.set_anisotropy_level(options.anisotropy);
            raw_texture.set_s_wrap_mode(options.s_wrap_mode);
            raw_texture.set_t_wrap_mode(options.t_wrap_mode);

            texture.state().commit(ResourceState::Ok(raw_texture));

            // Ask renderer to upload texture to GPU.
            upload_sender.request_upload(texture);
        }
        Err(error) => {
            Log::writeln(
                MessageKind::Error,
                format!("Unable to load texture {:?}! Reason {:?}", &path, &error),
            );

            texture.state().commit(ResourceState::LoadError {
                path,
                error: Some(Arc::new(error)),
            });
        }
    }
}

async fn load_model(
    model: Model,
    path: PathBuf,
    resource_manager: ResourceManager,
    material_search_options: MaterialSearchOptions,
) {
    match ModelData::load(&path, resource_manager, material_search_options).await {
        Ok(raw_model) => {
            Log::writeln(
                MessageKind::Information,
                format!("Model {:?} is loaded!", path),
            );

            model.state().commit(ResourceState::Ok(raw_model));
        }
        Err(error) => {
            Log::writeln(
                MessageKind::Error,
                format!("Unable to load model from {:?}! Reason {:?}", path, error),
            );

            model.state().commit(ResourceState::LoadError {
                path,
                error: Some(Arc::new(error)),
            });
        }
    }
}

async fn load_sound_buffer(resource: SharedSoundBuffer, path: PathBuf, stream: bool) {
    match DataSource::from_file(&path).await {
        Ok(source) => {
            let buffer = if stream {
                SoundBuffer::new_streaming(source)
            } else {
                SoundBuffer::new_generic(source)
            };
            match buffer {
                Ok(sound_buffer) => {
                    Log::writeln(
                        MessageKind::Information,
                        format!("Sound buffer {:?} is loaded!", path),
                    );

                    resource.state().commit(ResourceState::Ok(sound_buffer));
                }
                Err(_) => {
                    Log::writeln(
                        MessageKind::Error,
                        format!("Unable to load sound buffer from {:?}!", path),
                    );

                    resource.state().commit(ResourceState::LoadError {
                        path: path.clone(),
                        error: Some(Arc::new(())),
                    })
                }
            }
        }
        Err(e) => {
            Log::writeln(MessageKind::Error, format!("Invalid data source: {:?}", e));

            resource.state().commit(ResourceState::LoadError {
                path: path.clone(),
                error: Some(Arc::new(())),
            })
        }
    }
}

async fn reload_texture(texture: Texture, path: PathBuf, compression: CompressionOptions) {
    match TextureData::load_from_file(&path, compression).await {
        Ok(data) => {
            Log::writeln(
                MessageKind::Information,
                format!("Texture {:?} successfully reloaded!", path,),
            );

            texture.state().commit(ResourceState::Ok(data));
        }
        Err(e) => {
            Log::writeln(
                MessageKind::Error,
                format!("Unable to reload {:?} texture! Reason: {:?}", path, e),
            );

            texture.state().commit(ResourceState::LoadError {
                path,
                error: Some(Arc::new(e)),
            });
        }
    };
}

async fn reload_model(
    model: Model,
    path: PathBuf,
    resource_manager: ResourceManager,
    material_search_options: MaterialSearchOptions,
) {
    match ModelData::load(&path, resource_manager, material_search_options).await {
        Ok(data) => {
            Log::writeln(
                MessageKind::Information,
                format!("Model {:?} successfully reloaded!", path,),
            );

            model.state().commit(ResourceState::Ok(data));
        }
        Err(e) => {
            Log::writeln(
                MessageKind::Error,
                format!("Unable to reload {:?} model! Reason: {:?}", path, e),
            );

            model.state().commit(ResourceState::LoadError {
                path,
                error: Some(Arc::new(e)),
            })
        }
    };
}

async fn reload_sound_buffer(
    resource: SharedSoundBuffer,
    path: PathBuf,
    stream: bool,
    inner_buffer: Arc<Mutex<SoundBuffer>>,
) {
    if let Ok(data_source) = DataSource::from_file(&path).await {
        let new_sound_buffer = match stream {
            false => SoundBuffer::raw_generic(data_source),
            true => SoundBuffer::raw_streaming(data_source),
        };
        match new_sound_buffer {
            Ok(new_sound_buffer) => {
                Log::writeln(
                    MessageKind::Information,
                    format!("Sound buffer {:?} successfully reloaded!", path,),
                );

                *inner_buffer.lock().unwrap() = new_sound_buffer;
                resource.state().commit(ResourceState::Ok(inner_buffer));
            }
            Err(_) => {
                Log::writeln(
                    MessageKind::Error,
                    format!("Unable to reload {:?} sound buffer!", path),
                );

                resource.state().commit(ResourceState::LoadError {
                    path,
                    error: Some(Arc::new(())),
                })
            }
        }
    }
}

impl ResourceManager {
    pub(in crate) fn new(upload_sender: TextureUploadSender) -> Self {
        Self {
            state: Some(Arc::new(Mutex::new(ResourceManagerState::new(
                upload_sender,
            )))),
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
    /// and then use pattern matching to get TextureData which contains actual texture data.
    ///
    /// # Async/.await
    ///
    /// Each Texture implements Future trait and can be used in async contexts.
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

        let texture = Texture::new(ResourceState::new_pending(path.as_ref().to_owned()));
        state.textures.push(TimedEntry {
            value: texture.clone(),
            time_to_live: DEFAULT_RESOURCE_LIFETIME,
        });
        let result = texture.clone();
        let options = state.textures_import_options.clone();
        let path = path.as_ref().to_owned();
        let upload_sender = state
            .upload_sender
            .as_ref()
            .expect("Upload sender must be set!")
            .clone();

        #[cfg(target_arch = "wasm32")]
        crate::core::wasm_bindgen_futures::spawn_local(async move {
            load_texture(texture, path, options, upload_sender).await;
        });

        #[cfg(not(target_arch = "wasm32"))]
        state.thread_pool.spawn_ok(async move {
            load_texture(texture, path, options, upload_sender).await;
        });

        result
    }

    /// Saves given texture in the specified path and registers it in resource manager, so
    /// it will be accessible through it later.
    pub fn register_texture<P: AsRef<Path>>(
        &self,
        texture: Texture,
        path: P,
    ) -> Result<(), TextureRegistrationError> {
        let mut state = self.state();
        if state.find_texture(path.as_ref()).is_some() {
            Err(TextureRegistrationError::AlreadyRegistered)
        } else {
            let mut texture_state = texture.state();
            match &mut *texture_state {
                TextureState::Ok(texture_data) => {
                    texture_data.set_path(path);
                    if let Err(e) = texture_data.save() {
                        Err(TextureRegistrationError::Texture(e))
                    } else {
                        std::mem::drop(texture_state);
                        state.textures.push(TimedEntry {
                            value: texture,
                            time_to_live: DEFAULT_RESOURCE_LIFETIME,
                        });
                        Ok(())
                    }
                }
                _ => Err(TextureRegistrationError::InvalidState),
            }
        }
    }

    /// Tries to load new model resource from given path or get instance of existing, if any.
    /// This method is asynchronous, it immediately returns a model which can be shared across
    /// multiple places, the loading may fail, but it is internal state of the model. If you need
    /// to access internals of the texture you have to get state first and then use pattern matching
    /// to get ModelData which contains actual model data.
    ///
    /// # Async/.await
    ///
    /// Each model implements Future trait and can be used in async contexts.
    ///
    /// # Supported formats
    ///
    /// Currently only FBX (common format in game industry for storing complex 3d models)
    /// and RGS (native rusty-editor format) formats are supported.
    pub fn request_model<P: AsRef<Path>>(
        &self,
        path: P,
        material_search_options: MaterialSearchOptions,
    ) -> Model {
        let mut state = self.state();

        if let Some(model) = state.find_model(path.as_ref()) {
            return model;
        }

        let model = Model::new(ResourceState::new_pending(path.as_ref().to_owned()));
        state.models.push(TimedEntry {
            value: model.clone(),
            time_to_live: DEFAULT_RESOURCE_LIFETIME,
        });

        let result = model.clone();
        let path = path.as_ref().to_owned();
        let resource_manager = self.clone();

        #[cfg(target_arch = "wasm32")]
        crate::core::wasm_bindgen_futures::spawn_local(async move {
            load_model(model, path, resource_manager, material_search_options).await;
        });

        #[cfg(not(target_arch = "wasm32"))]
        state.thread_pool.spawn_ok(async move {
            load_model(model, path, resource_manager, material_search_options).await;
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
    pub fn request_sound_buffer<P: AsRef<Path>>(&self, path: P, stream: bool) -> SharedSoundBuffer {
        let mut state = self.state();

        if let Some(sound_buffer) = state.find_sound_buffer(path.as_ref()) {
            return sound_buffer;
        }

        let resource = SharedSoundBuffer::new(ResourceState::new_pending(path.as_ref().to_owned()));
        state.sound_buffers.push(TimedEntry {
            value: resource.clone(),
            time_to_live: DEFAULT_RESOURCE_LIFETIME,
        });
        let result = resource.clone();
        let path = path.as_ref().to_owned();

        #[cfg(target_arch = "wasm32")]
        crate::core::wasm_bindgen_futures::spawn_local(async move {
            load_sound_buffer(resource, path, stream).await;
        });

        #[cfg(not(target_arch = "wasm32"))]
        state.thread_pool.spawn_ok(async move {
            load_sound_buffer(resource, path, stream).await;
        });

        result
    }

    /// Reloads every loaded texture. This method is asynchronous, internally it uses thread pool
    /// to run reload on separate thread per texture.
    pub async fn reload_textures(&self) {
        // Separate block to release lock on state before await.
        let textures = {
            let state = self.state();

            let textures = state
                .textures
                .iter()
                .map(|e| e.value.clone())
                .collect::<Vec<Texture>>();

            for resource in textures.iter().cloned() {
                let path = resource.state().path().to_path_buf();
                let compression = if let ResourceState::Ok(ref data) = *resource.state() {
                    match data.pixel_kind() {
                        TexturePixelKind::DXT1RGB => CompressionOptions::Speed,
                        TexturePixelKind::DXT1RGBA => CompressionOptions::Speed,
                        TexturePixelKind::DXT3RGBA => CompressionOptions::NoCompression, // TODO
                        TexturePixelKind::DXT5RGBA => CompressionOptions::Quality,
                        _ => CompressionOptions::NoCompression,
                    }
                } else {
                    CompressionOptions::NoCompression
                };
                *resource.state() = ResourceState::new_pending(path.clone());

                #[cfg(target_arch = "wasm32")]
                crate::core::wasm_bindgen_futures::spawn_local(async move {
                    reload_texture(resource, path, compression).await;
                });

                #[cfg(not(target_arch = "wasm32"))]
                state.thread_pool.spawn_ok(async move {
                    reload_texture(resource, path, compression).await;
                });
            }

            textures
        };

        crate::core::futures::future::join_all(textures).await;
    }

    /// Reloads every loaded model. This method is asynchronous, internally it uses thread pool
    /// to run reload on separate thread per model.
    pub async fn reload_models(&self) {
        let models = {
            let this = self.clone();
            let state = self.state();

            let models = state
                .models
                .iter()
                .map(|m| m.value.clone())
                .collect::<Vec<Model>>();

            for model in models.iter().cloned() {
                let this = this.clone();
                let path = model.state().path().to_path_buf();
                let material_search_options = model.data_ref().material_search_options().clone();
                *model.state() = ResourceState::new_pending(path.clone());

                #[cfg(target_arch = "wasm32")]
                crate::core::wasm_bindgen_futures::spawn_local(async move {
                    reload_model(model, path, this, material_search_options).await;
                });

                #[cfg(not(target_arch = "wasm32"))]
                state.thread_pool.spawn_ok(async move {
                    reload_model(model, path, this, material_search_options).await;
                })
            }

            models
        };

        crate::core::futures::future::join_all(models).await;

        Log::writeln(
            MessageKind::Information,
            "All model resources reloaded!".to_owned(),
        );
    }

    /// Reloads every loaded sound buffer. This method is asynchronous, internally it uses thread pool
    /// to run reload on separate thread per sound buffer.
    pub async fn reload_sound_buffers(&self) {
        let buffers = {
            let state = self.state();

            let sound_buffers = state
                .sound_buffers
                .iter()
                .map(|b| b.value.clone())
                .collect::<Vec<SharedSoundBuffer>>();

            for resource in sound_buffers.iter().cloned() {
                let (stream, path, inner_buffer) = {
                    let inner_buffer_ref = resource.data_ref();
                    let inner_buffer = inner_buffer_ref.lock().unwrap();
                    let stream = match *inner_buffer {
                        SoundBuffer::Generic(_) => false,
                        SoundBuffer::Streaming(_) => true,
                    };
                    (
                        stream,
                        inner_buffer.external_data_path().map(|p| p.to_owned()),
                        inner_buffer_ref.clone(),
                    )
                };
                if let Some(ext_path) = path {
                    *resource.state() = ResourceState::new_pending(ext_path.clone());

                    #[cfg(target_arch = "wasm32")]
                    crate::core::wasm_bindgen_futures::spawn_local(async move {
                        reload_sound_buffer(resource, ext_path, stream, inner_buffer).await;
                    });

                    #[cfg(not(target_arch = "wasm32"))]
                    state.thread_pool.spawn_ok(async move {
                        reload_sound_buffer(resource, ext_path, stream, inner_buffer).await;
                    });
                }
            }

            sound_buffers
        };

        crate::core::futures::future::join_all(buffers).await;
    }

    /// Reloads all loaded resources. Normally it should never be called, because it is **very** heavy
    /// method! This method is asynchronous, it uses all available CPU power to reload resources as
    /// fast as possible.
    pub async fn reload_resources(&self) {
        crate::core::futures::join!(
            self.reload_textures(),
            self.reload_models(),
            self.reload_sound_buffers()
        );
    }
}

fn count_pending_resources<T, E>(resources: &[TimedEntry<Resource<T, E>>]) -> usize
where
    T: ResourceData,
    E: ResourceLoadError,
{
    let mut count = 0;
    for entry in resources.iter() {
        if let ResourceState::Pending { .. } = *entry.value.state() {
            count += 1;
        }
    }
    count
}

fn count_loaded_resources<T, E>(resources: &[TimedEntry<Resource<T, E>>]) -> usize
where
    T: ResourceData,
    E: ResourceLoadError,
{
    let mut count = 0;
    for entry in resources.iter() {
        match *entry.value.state() {
            ResourceState::LoadError { .. } | ResourceState::Ok(_) => {
                count += 1;
            }
            _ => {}
        }
    }
    count
}

/// Defines a way of searching materials when loading a model resource.
#[derive(Clone, Debug, Visit, PartialEq)]
pub enum MaterialSearchOptions {
    /// Search in specified materials directory. It is suitable for cases when
    /// your model resource use shared textures.
    ///
    /// # Platform specific
    ///
    /// Works on every platform.
    MaterialsDirectory(PathBuf),

    /// Recursive-up search. It is suitable for cases when textures are placed
    /// near your model resource. This is default option.
    ///
    /// # Platform specific
    ///
    /// Works on every platform.
    RecursiveUp,

    /// Global search starting from working directory. Slowest option with a lot of ambiguities -
    /// it may load unexpected file in cases when there are two or more files with same name but
    /// lying in different directories.
    ///
    /// # Platform specific
    ///
    /// WebAssembly - **not supported** due to lack of file system.
    WorkingDirectory,

    /// Try to use paths stored in the model resource directly. This options has limited usage,
    /// it is suitable to load animations, or any other model which does not have any materials.
    ///
    /// # Important notes
    ///
    /// RGS (native engine scenes) files should be loaded with this option by default, otherwise
    /// the engine won't be able to correctly find materials.
    UsePathDirectly,
}

impl Default for MaterialSearchOptions {
    fn default() -> Self {
        Self::RecursiveUp
    }
}

impl MaterialSearchOptions {
    /// A helper to create MaterialsDirectory variant.
    pub fn materials_directory<P: AsRef<Path>>(path: P) -> Self {
        Self::MaterialsDirectory(path.as_ref().to_path_buf())
    }
}

impl ResourceManagerState {
    pub(in crate::engine) fn new(upload_sender: TextureUploadSender) -> Self {
        Self {
            textures: Vec::new(),
            models: Vec::new(),
            sound_buffers: Vec::new(),
            textures_import_options: Default::default(),
            #[cfg(not(target_arch = "wasm32"))]
            thread_pool: ThreadPool::new().unwrap(),
            upload_sender: Some(upload_sender),
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
            if sound_buffer.state().path() == path.as_ref() {
                return Some(sound_buffer.value.clone());
            }
        }
        None
    }

    /// Returns total amount of textures in pending state.
    pub fn count_pending_textures(&self) -> usize {
        count_pending_resources(&self.textures)
    }

    /// Returns total amount of loaded textures (including textures, that failed to load).
    pub fn count_loaded_textures(&self) -> usize {
        count_loaded_resources(&self.textures)
    }

    /// Returns total amount of sound buffers in pending state.
    pub fn count_pending_sound_buffers(&self) -> usize {
        count_pending_resources(&self.sound_buffers)
    }

    /// Returns total amount of loaded sound buffers (including sound buffers, that failed to load).
    pub fn count_loaded_sound_buffers(&self) -> usize {
        count_loaded_resources(&self.sound_buffers)
    }

    /// Returns total amount of models in pending state.
    pub fn count_pending_models(&self) -> usize {
        count_pending_resources(&self.models)
    }

    /// Returns total amount of loaded models (including models, that failed to load).
    pub fn count_loaded_models(&self) -> usize {
        count_loaded_resources(&self.models)
    }

    /// Returns total amount of resources in pending state.
    pub fn count_pending_resources(&self) -> usize {
        self.count_pending_textures()
            + self.count_pending_sound_buffers()
            + self.count_pending_models()
    }

    /// Returns total amount of loaded resources.
    pub fn count_loaded_resources(&self) -> usize {
        self.count_loaded_textures()
            + self.count_loaded_sound_buffers()
            + self.count_loaded_models()
    }

    /// Returns total amount of registered resources.
    pub fn count_registered_resources(&self) -> usize {
        self.textures.len() + self.sound_buffers.len() + self.models.len()
    }

    /// Returns percentage of loading progress. This method is useful to show progress on
    /// loading screen in your game. This method could be used alone if your game depends
    /// only on external resources, or if your game doing some heavy calculations this value
    /// can be combined with progress of your tasks.  
    pub fn loading_progress(&self) -> usize {
        let registered = self.count_registered_resources();
        if registered > 0 {
            self.count_loaded_resources() * 100 / registered
        } else {
            100
        }
    }

    /// Immediately destroys all unused resources.
    pub fn purge_unused_resources(&mut self) {
        self.sound_buffers
            .retain(|buffer| buffer.value.use_count() > 1);
        self.models.retain(|buffer| buffer.value.use_count() > 1);
        self.textures.retain(|buffer| buffer.value.use_count() > 1);
    }

    fn update_textures(&mut self, dt: f32) {
        for texture in self.textures.iter_mut() {
            if matches!(*texture.state(), ResourceState::Ok(_)) {
                texture.time_to_live -= dt;
                if texture.use_count() > 1 {
                    texture.time_to_live = DEFAULT_RESOURCE_LIFETIME;
                }
            }
        }
        self.textures.retain(|texture| {
            let retain = texture.time_to_live > 0.0;
            if !retain && texture.state().path().exists() {
                Log::writeln(
                    MessageKind::Information,
                    format!(
                        "Texture resource {:?} destroyed because it not used anymore!",
                        texture.state().path()
                    ),
                );
            }
            retain
        });
    }

    fn update_model(&mut self, dt: f32) {
        for model in self.models.iter_mut() {
            model.time_to_live -= dt;
            if model.use_count() > 1 {
                model.time_to_live = DEFAULT_RESOURCE_LIFETIME;
            }
        }
        self.models.retain(|model| {
            let retain = model.time_to_live > 0.0;
            if !retain && model.state().path().exists() {
                Log::writeln(
                    MessageKind::Information,
                    format!(
                        "Model resource {:?} destroyed because it not used anymore!",
                        model.state().path()
                    ),
                );
            }
            retain
        });
    }

    fn update_sound_buffers(&mut self, dt: f32) {
        for buffer in self.sound_buffers.iter_mut() {
            buffer.time_to_live -= dt;
            if buffer.use_count() > 1 {
                buffer.time_to_live = DEFAULT_RESOURCE_LIFETIME;
            }
        }
        self.sound_buffers.retain(|buffer| {
            let retain = buffer.time_to_live > 0.0;
            if !retain {
                Log::writeln(
                    MessageKind::Information,
                    format!(
                        "Sound resource {:?} destroyed because it not used anymore!",
                        buffer.state().path()
                    ),
                );
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

        crate::core::futures::executor::block_on(crate::core::futures::future::join_all(
            self.textures.iter().map(|t| t.value.clone()),
        ));
        crate::core::futures::executor::block_on(crate::core::futures::future::join_all(
            self.models.iter().map(|m| m.value.clone()),
        ));
        crate::core::futures::executor::block_on(crate::core::futures::future::join_all(
            self.sound_buffers.iter().map(|m| m.value.clone()),
        ));

        self.textures.visit("Textures", visitor)?;
        self.models.visit("Models", visitor)?;
        self.sound_buffers.visit("SoundBuffers", visitor)?;

        visitor.leave_region()
    }
}
