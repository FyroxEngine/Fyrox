//! Resource manager controls loading and lifetime of resource in the engine.

use crate::{
    asset::{Resource, ResourceData, ResourceLoadError, ResourceState},
    core::{append_extension, instant, io, visitor::prelude::*, VecExtensions},
    material::shader::{Shader, ShaderState},
    renderer::TextureUploadSender,
    resource::{
        curve::{CurveResource, CurveResourceState},
        model::{Model, ModelData},
        texture::{Texture, TextureData, TextureError, TextureImportOptions, TextureState},
    },
    sound::buffer::{
        DataSource, SoundBufferResource, SoundBufferResourceLoadError, SoundBufferState,
    },
    utils::log::{Log, MessageKind},
};
use serde::de::DeserializeOwned;
use std::{
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
};

#[cfg(not(target_arch = "wasm32"))]
use crate::core::futures::executor::ThreadPool;
use crate::resource::model::ModelImportOptions;

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

/// Generic container for any resource in the engine. Main purpose of the container is to
/// track resources life time and remove unused timed-out resources. It also provides useful
/// methods to search resources, count loaded or pending, wait until all resources are loading,
/// etc.
#[derive(Default, Visit)]
pub struct ResourceContainer<T> {
    resources: Vec<TimedEntry<T>>,
}

impl<T, R, E> ResourceContainer<T>
where
    T: Deref<Target = Resource<R, E>>,
    R: ResourceData,
    E: ResourceLoadError,
{
    /// Adds a new resource in the container.
    pub fn push(&mut self, resource: T) {
        self.resources.push(TimedEntry {
            value: resource,
            time_to_live: DEFAULT_RESOURCE_LIFETIME,
        });
    }

    /// Tries to find a resources by its path. Returns None if no resource was found.
    ///
    /// # Complexity
    ///
    /// O(n)
    pub fn find<P: AsRef<Path>>(&self, path: P) -> Option<&T> {
        for resource in self.resources.iter() {
            if resource.state().path() == path.as_ref() {
                return Some(&resource.value);
            }
        }
        None
    }

    /// Tracks life time of resource and removes unused resources after some time of idling.
    pub fn update(&mut self, dt: f32) {
        self.resources.retain_mut_ext(|resource| {
            // One usage means that the resource has single owner, and that owner
            // is this container. Such resources have limited life time, if the time
            // runs out before it gets shared again, the resource will be deleted.
            if resource.use_count() <= 1 {
                resource.time_to_live -= dt;
                if resource.time_to_live <= 0.0 {
                    Log::writeln(
                        MessageKind::Information,
                        format!(
                            "Resource {:?} destroyed because it not used anymore!",
                            resource.state().path()
                        ),
                    );

                    false
                } else {
                    // Keep resource alive for short period of time.
                    true
                }
            } else {
                // Make sure to reset timer if a resource is used by more than one owner.
                resource.time_to_live = DEFAULT_RESOURCE_LIFETIME;

                // Keep resource alive while it has more than one owner.
                true
            }
        });
    }

    /// Returns total amount of resources in the container.
    pub fn len(&self) -> usize {
        self.resources.len()
    }

    /// Returns true if container has no resources.
    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
    }

    /// Creates an iterator over resources in the container.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.resources.iter().map(|entry| &entry.value)
    }

    /// Immediately destroys all resources in the container that are not used anywhere else.
    pub fn destroy_unused(&mut self) {
        self.resources
            .retain(|resource| resource.value.use_count() > 1);
    }

    /// Returns total amount of resources that still loading.
    pub fn count_pending_resources(&self) -> usize {
        self.resources.iter().fold(0, |counter, resource| {
            if let ResourceState::Pending { .. } = *resource.state() {
                counter + 1
            } else {
                counter
            }
        })
    }

    /// Returns total amount of completely loaded resources.
    pub fn count_loaded_resources(&self) -> usize {
        self.resources.iter().fold(0, |counter, resource| {
            if let ResourceState::Ok(_) = *resource.state() {
                counter + 1
            } else {
                counter
            }
        })
    }

    /// Locks current thread until every resource is loaded (or failed to load).
    ///
    /// # Platform specific
    ///
    /// WASM: WebAssembly uses simple loop to wait for all resources, which means
    /// full load of single CPU core.
    pub fn wait(&self) {
        #[cfg(target_arch = "wasm32")]
        {
            // In case of WebAssembly, spin until everything is loaded.
            loop {
                let mut loaded_count = 0;
                for resource in self.resources.iter() {
                    if !matches!(*resource.value.state(), ResourceState::Pending { .. }) {
                        loaded_count += 1;
                    }
                }
                if loaded_count == self.resources.len() {
                    break;
                }
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            crate::core::futures::executor::block_on(crate::core::futures::future::join_all(
                self.resources.iter().map(|t| t.value.clone()),
            ));
        }
    }
}

/// See module docs.
pub struct ResourceManagerState {
    textures: ResourceContainer<Texture>,
    models: ResourceContainer<Model>,
    sound_buffers: ResourceContainer<SoundBufferResource>,
    shaders: ResourceContainer<Shader>,
    curves: ResourceContainer<CurveResource>,
    textures_import_options: TextureImportOptions,
    model_import_options: ModelImportOptions,
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
            shaders: Default::default(),
            curves: Default::default(),
            textures_import_options: Default::default(),
            model_import_options: Default::default(),
            #[cfg(not(target_arch = "wasm32"))]
            thread_pool: ThreadPool::new().unwrap(),
            upload_sender: None,
        }
    }
}

/// See module docs.
#[derive(Clone, Visit)]
pub struct ResourceManager {
    state: Option<Arc<Mutex<ResourceManagerState>>>,
}

/// An error that may occur during texture registration.
#[derive(Debug, thiserror::Error)]
pub enum TextureRegistrationError {
    /// Texture saving has failed.
    #[error(transparent)]
    Texture(TextureError),
    /// Texture was in invalid state (Pending, LoadErr)
    #[error("A texture was in invalid state!")]
    InvalidState,
    /// Texture is already registered.
    #[error("A texture is already registered!")]
    AlreadyRegistered,
}

impl From<TextureError> for TextureRegistrationError {
    fn from(e: TextureError) -> Self {
        Self::Texture(e)
    }
}

/// Tries to load import settings for a resource.
pub async fn try_get_import_settings<T: DeserializeOwned>(resource_path: &Path) -> Option<T> {
    let settings_path = append_extension(resource_path, "options");

    match io::load_file(&settings_path).await {
        Ok(bytes) => match ron::de::from_bytes::<T>(&bytes) {
            Ok(options) => Some(options),
            Err(e) => {
                Log::writeln(
                    MessageKind::Error,
                    format!(
                        "Malformed options file {} for {} resource! Reason: {:?}",
                        settings_path.display(),
                        resource_path.display(),
                        e
                    ),
                );

                None
            }
        },
        Err(e) => {
            Log::writeln(
                MessageKind::Warning,
                format!(
                    "Unable to load options file {} for {} resource, fallback to defaults! Reason: {:?}",
                    settings_path.display(),
                    resource_path.display(),
                    e
                ),
            );

            None
        }
    }
}

async fn load_texture(
    texture: Texture,
    path: PathBuf,
    default_options: TextureImportOptions,
    upload_sender: TextureUploadSender,
) {
    let import_options = try_get_import_settings(&path)
        .await
        .unwrap_or(default_options);

    let gen_mip_maps = import_options.minification_filter.is_using_mip_mapping();

    let time = instant::Instant::now();
    match TextureData::load_from_file(&path, import_options.compression, gen_mip_maps).await {
        Ok(mut raw_texture) => {
            Log::writeln(
                MessageKind::Information,
                format!("Texture {:?} is loaded in {:?}!", path, time.elapsed()),
            );

            raw_texture.set_magnification_filter(import_options.magnification_filter);
            raw_texture.set_minification_filter(import_options.minification_filter);
            raw_texture.set_anisotropy_level(import_options.anisotropy);
            raw_texture.set_s_wrap_mode(import_options.s_wrap_mode);
            raw_texture.set_t_wrap_mode(import_options.t_wrap_mode);

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
    default_import_options: ModelImportOptions,
) {
    let import_options = try_get_import_settings(&path)
        .await
        .unwrap_or(default_import_options);

    match ModelData::load(&path, resource_manager, import_options).await {
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

async fn load_shader(shader: Shader, path: PathBuf) {
    match ShaderState::from_file(&path).await {
        Ok(shader_state) => {
            Log::writeln(
                MessageKind::Information,
                format!("Shader {:?} is loaded!", path),
            );

            shader.state().commit(ResourceState::Ok(shader_state));
        }
        Err(error) => {
            Log::writeln(
                MessageKind::Error,
                format!("Unable to load model from {:?}! Reason {:?}", path, error),
            );

            shader.state().commit(ResourceState::LoadError {
                path,
                error: Some(Arc::new(error)),
            });
        }
    }
}

async fn load_curve_resource(curve: CurveResource, path: PathBuf) {
    match CurveResourceState::from_file(&path).await {
        Ok(curve_state) => {
            Log::writeln(
                MessageKind::Information,
                format!("Curve {:?} is loaded!", path),
            );

            curve.state().commit(ResourceState::Ok(curve_state));
        }
        Err(error) => {
            Log::writeln(
                MessageKind::Error,
                format!("Unable to load curve from {:?}! Reason {:?}", path, error),
            );

            curve.state().commit(ResourceState::LoadError {
                path,
                error: Some(Arc::new(error)),
            });
        }
    }
}

async fn load_sound_buffer(resource: SoundBufferResource, path: PathBuf, stream: bool) {
    match DataSource::from_file(&path).await {
        Ok(source) => {
            let buffer = if stream {
                SoundBufferState::raw_streaming(source)
            } else {
                SoundBufferState::raw_generic(source)
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
                        error: Some(Arc::new(SoundBufferResourceLoadError::UnsupportedFormat)),
                    })
                }
            }
        }
        Err(e) => {
            Log::writeln(
                MessageKind::Error,
                format!("Invalid data source for sound buffer: {:?}", e),
            );

            resource.state().commit(ResourceState::LoadError {
                path: path.clone(),
                error: Some(Arc::new(SoundBufferResourceLoadError::Io(e))),
            })
        }
    }
}

async fn reload_sound_buffer(resource: SoundBufferResource, path: PathBuf, stream: bool) {
    if let Ok(data_source) = DataSource::from_file(&path).await {
        let new_sound_buffer = match stream {
            false => SoundBufferState::raw_generic(data_source),
            true => SoundBufferState::raw_streaming(data_source),
        };
        match new_sound_buffer {
            Ok(new_sound_buffer) => {
                Log::writeln(
                    MessageKind::Information,
                    format!("Sound buffer {:?} successfully reloaded!", path,),
                );

                resource.state().commit(ResourceState::Ok(new_sound_buffer));
            }
            Err(_) => {
                Log::writeln(
                    MessageKind::Error,
                    format!("Unable to reload {:?} sound buffer!", path),
                );

                resource.state().commit(ResourceState::LoadError {
                    path,
                    error: Some(Arc::new(SoundBufferResourceLoadError::UnsupportedFormat)),
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
    /// # Import options
    ///
    /// It is possible to define custom import options. Using import options you could set desired compression quality,
    /// filtering, wrapping, etc. Import options should be defined in a separate file with the same name as the source
    /// texture, but with additional extension `options`. For example you have a `foo.jpg` texture, a file with import
    /// options should be called `foo.jpg.options`. It's content may look something like this:
    ///
    /// ```text
    /// (
    ///     minification_filter: Linear,
    ///     magnification_filter: Linear,
    ///     s_wrap_mode: Repeat,
    ///     t_wrap_mode: ClampToEdge,
    ///     anisotropy: 8.0,
    ///     compression: NoCompression,    
    /// )
    /// ```
    ///
    /// Usually there is no need to change this file manually, it can be modified from the editor using the Asset Browser.
    /// When there is no import options file, the engine will use texture import options defined in the resource manager.
    /// See [set_textures_import_options](ResourceManagerState::set_textures_import_options) for more info.
    ///
    /// # Async/.await
    ///
    /// Each Texture implements Future trait and can be used in async contexts.
    ///
    /// # Supported formats
    ///
    /// To load images and decode them, rg3d uses image create which supports following image formats: png, tga, bmp, dds,
    /// jpg, gif, tiff, dxt.
    pub fn request_texture<P: AsRef<Path>>(&self, path: P) -> Texture {
        let path_ref = path.as_ref();
        let mut state = self.state();

        if let Some(texture) = state.textures.find(path_ref) {
            return texture.clone();
        }

        let texture = Texture(Resource::new(ResourceState::new_pending(
            path_ref.to_owned(),
        )));
        state.textures.push(texture.clone());

        let result = texture.clone();
        let default_options = state.textures_import_options.clone();
        let path = path_ref.to_owned();
        let upload_sender = state
            .upload_sender
            .as_ref()
            .expect("Upload sender must be set!")
            .clone();

        #[cfg(target_arch = "wasm32")]
        crate::core::wasm_bindgen_futures::spawn_local(async move {
            load_texture(texture, path, default_options, upload_sender).await;
        });

        #[cfg(not(target_arch = "wasm32"))]
        state.thread_pool.spawn_ok(async move {
            load_texture(texture, path, default_options, upload_sender).await;
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
        if state.textures.find(path.as_ref()).is_some() {
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
                        state.textures.push(texture);
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
    pub fn request_model<P: AsRef<Path>>(&self, path: P) -> Model {
        let mut state = self.state();

        if let Some(model) = state.models.find(path.as_ref()) {
            return model.clone();
        }

        let model = Model(Resource::new(ResourceState::new_pending(
            path.as_ref().to_owned(),
        )));
        state.models.push(model.clone());

        let default_import_options = state.model_import_options.clone();
        let result = model.clone();
        let path = path.as_ref().to_owned();
        let resource_manager = self.clone();

        #[cfg(target_arch = "wasm32")]
        crate::core::wasm_bindgen_futures::spawn_local(async move {
            load_model(model, path, resource_manager, default_import_options).await;
        });

        #[cfg(not(target_arch = "wasm32"))]
        state.thread_pool.spawn_ok(async move {
            load_model(model, path, resource_manager, default_import_options).await;
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
    ) -> SoundBufferResource {
        let mut state = self.state();

        if let Some(sound_buffer) = state.sound_buffers.find(path.as_ref()) {
            return sound_buffer.clone();
        }

        let resource = SoundBufferResource(Resource::new(ResourceState::new_pending(
            path.as_ref().to_owned(),
        )));
        state.sound_buffers.push(resource.clone());
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

    /// Tries to load a new shader resource from given path or get instance of existing, if any.
    /// This method is asynchronous, it immediately returns a shader which can be shared across
    /// multiple places, the loading may fail, but it is internal state of the shader.
    ///
    /// # Async/.await
    ///
    /// Each shader implements Future trait and can be used in async contexts.
    pub fn request_shader<P: AsRef<Path>>(&self, path: P) -> Shader {
        let mut state = self.state();

        if let Some(shader) = state.shaders.find(path.as_ref()) {
            return shader.clone();
        }

        let shader = Shader(Resource::new(ResourceState::new_pending(
            path.as_ref().to_owned(),
        )));
        state.shaders.push(shader.clone());

        let result = shader.clone();
        let path = path.as_ref().to_owned();

        #[cfg(target_arch = "wasm32")]
        crate::core::wasm_bindgen_futures::spawn_local(async move {
            load_shader(shader, path).await;
        });

        #[cfg(not(target_arch = "wasm32"))]
        state.thread_pool.spawn_ok(async move {
            load_shader(shader, path).await;
        });

        result
    }

    /// Tries to load a new curve resource from given path or get instance of existing, if any.
    /// This method is asynchronous, it immediately returns a curve which can be shared across
    /// multiple places, the loading may fail, but it is internal state of the curve resource.
    ///
    /// # Async/.await
    ///
    /// Each shader implements Future trait and can be used in async contexts.
    pub fn request_curve_resource<P: AsRef<Path>>(&self, path: P) -> CurveResource {
        let mut state = self.state();

        if let Some(curve) = state.curves.find(path.as_ref()) {
            return curve.clone();
        }

        let curve = CurveResource(Resource::new(ResourceState::new_pending(
            path.as_ref().to_owned(),
        )));
        state.curves.push(curve.clone());

        let result = curve.clone();
        let path = path.as_ref().to_owned();

        #[cfg(target_arch = "wasm32")]
        crate::core::wasm_bindgen_futures::spawn_local(async move {
            load_curve_resource(curve, path).await;
        });

        #[cfg(not(target_arch = "wasm32"))]
        state.thread_pool.spawn_ok(async move {
            load_curve_resource(curve, path).await;
        });

        result
    }

    /// Reloads every loaded texture. This method is asynchronous, internally it uses thread pool
    /// to run reload on separate thread per texture.
    pub async fn reload_textures(&self) {
        // Separate block to release lock on state before await.
        let textures = {
            let state = self.state();

            let textures = state.textures.iter().cloned().collect::<Vec<Texture>>();

            for resource in textures.iter().cloned() {
                let path = resource.state().path().to_path_buf();
                let default_options = state.textures_import_options.clone();
                let upload_sender = state
                    .upload_sender
                    .clone()
                    .expect("Upload sender must exist at this point!");
                *resource.state() = ResourceState::new_pending(path.clone());

                #[cfg(target_arch = "wasm32")]
                crate::core::wasm_bindgen_futures::spawn_local(async move {
                    load_texture(resource, path, default_options, upload_sender).await;
                });

                #[cfg(not(target_arch = "wasm32"))]
                state.thread_pool.spawn_ok(async move {
                    load_texture(resource, path, default_options, upload_sender).await;
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

            let models = state.models.iter().cloned().collect::<Vec<_>>();

            for model in models.iter().cloned() {
                let this = this.clone();
                let path = model.state().path().to_path_buf();
                let default_import_options = state.model_import_options.clone();
                *model.state() = ResourceState::new_pending(path.clone());

                #[cfg(target_arch = "wasm32")]
                crate::core::wasm_bindgen_futures::spawn_local(async move {
                    load_model(model, path, this, default_import_options).await;
                });

                #[cfg(not(target_arch = "wasm32"))]
                state.thread_pool.spawn_ok(async move {
                    load_model(model, path, this, default_import_options).await;
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

    /// Reloads every loaded shader. This method is asynchronous, internally it uses thread pool
    /// to run reload on separate thread per shader.
    pub async fn reload_shaders(&self) {
        let shaders = {
            let state = self.state();

            let shaders = state.shaders.iter().cloned().collect::<Vec<_>>();

            for shader in shaders.iter().cloned() {
                let path = shader.state().path().to_path_buf();
                *shader.state() = ResourceState::new_pending(path.clone());

                #[cfg(target_arch = "wasm32")]
                crate::core::wasm_bindgen_futures::spawn_local(async move {
                    load_shader(shader, path).await;
                });

                #[cfg(not(target_arch = "wasm32"))]
                state.thread_pool.spawn_ok(async move {
                    load_shader(shader, path).await;
                })
            }

            shaders
        };

        crate::core::futures::future::join_all(shaders).await;

        Log::writeln(
            MessageKind::Information,
            "All shader resources are reloaded!".to_owned(),
        );
    }

    /// Reloads every loaded curve resource. This method is asynchronous, internally it uses thread pool
    /// to run reload on separate thread per resource.
    pub async fn reload_curve_resources(&self) {
        let curves = {
            let state = self.state();

            let curves = state.curves.iter().cloned().collect::<Vec<_>>();

            for curve in curves.iter().cloned() {
                let path = curve.state().path().to_path_buf();
                *curve.state() = ResourceState::new_pending(path.clone());

                #[cfg(target_arch = "wasm32")]
                crate::core::wasm_bindgen_futures::spawn_local(async move {
                    load_curve_resource(curve, path).await;
                });

                #[cfg(not(target_arch = "wasm32"))]
                state.thread_pool.spawn_ok(async move {
                    load_curve_resource(curve, path).await;
                })
            }

            curves
        };

        crate::core::futures::future::join_all(curves).await;

        Log::writeln(
            MessageKind::Information,
            "All curve resources are reloaded!".to_owned(),
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
                .cloned()
                .collect::<Vec<SoundBufferResource>>();

            for resource in sound_buffers.iter().cloned() {
                let (stream, path) = {
                    let inner_buffer = resource.data_ref();
                    let stream = match *inner_buffer {
                        SoundBufferState::Generic(_) => false,
                        SoundBufferState::Streaming(_) => true,
                    };
                    (stream, inner_buffer.external_data_path().to_path_buf())
                };
                if path != PathBuf::default() {
                    *resource.state() = ResourceState::new_pending(path.clone());

                    #[cfg(target_arch = "wasm32")]
                    crate::core::wasm_bindgen_futures::spawn_local(async move {
                        reload_sound_buffer(resource, path, stream).await;
                    });

                    #[cfg(not(target_arch = "wasm32"))]
                    state.thread_pool.spawn_ok(async move {
                        reload_sound_buffer(resource, path, stream).await;
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
            self.reload_sound_buffers(),
            self.reload_shaders(),
            self.reload_curve_resources(),
        );
    }
}

impl ResourceManagerState {
    pub(in crate::engine) fn new(upload_sender: TextureUploadSender) -> Self {
        Self {
            textures: Default::default(),
            models: Default::default(),
            sound_buffers: Default::default(),
            shaders: Default::default(),
            curves: Default::default(),
            textures_import_options: Default::default(),
            model_import_options: Default::default(),
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

    /// Returns a reference to textures container.
    #[inline]
    pub fn textures(&self) -> &ResourceContainer<Texture> {
        &self.textures
    }

    /// Returns a reference to shaders container.
    #[inline]
    pub fn models(&self) -> &ResourceContainer<Model> {
        &self.models
    }

    /// Returns a reference to sound buffers container.
    #[inline]
    pub fn sound_buffers(&self) -> &ResourceContainer<SoundBufferResource> {
        &self.sound_buffers
    }

    /// Returns a reference to shaders container.
    #[inline]
    pub fn shaders(&self) -> &ResourceContainer<Shader> {
        &self.shaders
    }

    /// Returns a reference to curves container.
    #[inline]
    pub fn curves(&self) -> &ResourceContainer<CurveResource> {
        &self.curves
    }

    /// Returns total amount of resources in pending state.
    pub fn count_pending_resources(&self) -> usize {
        self.textures.count_pending_resources()
            + self.sound_buffers.count_pending_resources()
            + self.models.count_pending_resources()
            + self.shaders.count_pending_resources()
            + self.curves.count_pending_resources()
    }

    /// Returns total amount of loaded resources.
    pub fn count_loaded_resources(&self) -> usize {
        self.textures.count_loaded_resources()
            + self.sound_buffers.count_loaded_resources()
            + self.models.count_loaded_resources()
            + self.shaders.count_loaded_resources()
            + self.curves.count_loaded_resources()
    }

    /// Returns total amount of registered resources.
    pub fn count_registered_resources(&self) -> usize {
        self.textures.len()
            + self.sound_buffers.len()
            + self.models.len()
            + self.shaders.len()
            + self.curves.len()
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
    pub fn destroy_unused_resources(&mut self) {
        self.sound_buffers.destroy_unused();
        self.models.destroy_unused();
        self.textures.destroy_unused();
        self.shaders.destroy_unused();
        self.curves.destroy_unused();
    }

    pub(in crate) fn update(&mut self, dt: f32) {
        self.textures.update(dt);
        self.models.update(dt);
        self.sound_buffers.update(dt);
        self.shaders.update(dt);
        self.curves.update(dt);
    }
}

impl Visit for ResourceManagerState {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.textures.wait();
        self.models.wait();
        self.sound_buffers.wait();
        self.shaders.wait();
        self.curves.wait();

        self.textures.visit("Textures", visitor)?;
        self.models.visit("Models", visitor)?;
        self.sound_buffers.visit("SoundBuffers", visitor)?;
        self.shaders.visit("Shaders", visitor)?;
        self.curves.visit("Curves", visitor)?;

        visitor.leave_region()
    }
}
