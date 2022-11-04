//! Resource manager controls loading and lifetime of resource in the engine.

use crate::engine::resource_manager::loader::animation::AnimationLoader;
use crate::{
    core::{
        futures::future::join_all,
        make_relative_path,
        parking_lot::{Mutex, MutexGuard},
    },
    engine::{
        resource_manager::{
            container::{Container, ResourceContainer},
            loader::{
                absm::AbsmLoader,
                curve::CurveLoader,
                model::ModelLoader,
                shader::ShaderLoader,
                sound::{SoundBufferImportOptions, SoundBufferLoader},
                texture::TextureLoader,
                ResourceLoader,
            },
            task::TaskPool,
        },
        SerializationContext,
    },
    material::shader::{Shader, ShaderImportOptions},
    resource::{
        absm::{AbsmImportOptions, AbsmResource},
        animation::{AnimationImportOptions, AnimationResource},
        curve::{CurveImportOptions, CurveResource},
        model::{Model, ModelImportOptions},
        texture::{Texture, TextureError, TextureImportOptions, TextureState},
    },
    utils::{log::Log, watcher::FileSystemWatcher},
};
use fyrox_sound::buffer::SoundBufferResource;
use std::fmt::{Debug, Display, Formatter};
use std::{path::Path, sync::Arc};

pub mod container;
pub mod loader;
pub mod options;
mod task;

/// Storage of resource containers.
pub struct ContainersStorage {
    /// Container for texture resources.
    pub textures: ResourceContainer<Texture, TextureImportOptions>,

    /// Container for model resources.
    pub models: ResourceContainer<Model, ModelImportOptions>,

    /// Container for sound buffer resources.
    pub sound_buffers: ResourceContainer<SoundBufferResource, SoundBufferImportOptions>,

    /// Container for shader resources.
    pub shaders: ResourceContainer<Shader, ShaderImportOptions>,

    /// Container for curve resources.
    pub curves: ResourceContainer<CurveResource, CurveImportOptions>,

    /// Container for ABSM resources.
    pub absm: ResourceContainer<AbsmResource, AbsmImportOptions>,

    /// Container for animation resources.
    pub animations: ResourceContainer<AnimationResource, AnimationImportOptions>,
}

impl ContainersStorage {
    /// Sets a custom texture loader.
    pub fn set_texture_loader<L>(&mut self, loader: L)
    where
        L: 'static + ResourceLoader<Texture, TextureImportOptions>,
    {
        self.textures.set_loader(loader);
    }

    /// Sets a custom model loader.
    pub fn set_model_loader<L>(&mut self, loader: L)
    where
        L: 'static + ResourceLoader<Model, ModelImportOptions>,
    {
        self.models.set_loader(loader);
    }

    /// Sets a custom sound buffer loader.
    pub fn set_sound_buffer_loader<L>(&mut self, loader: L)
    where
        L: 'static + ResourceLoader<SoundBufferResource, SoundBufferImportOptions>,
    {
        self.sound_buffers.set_loader(loader);
    }

    /// Sets a custom shader loader.
    pub fn set_shader_loader<L>(&mut self, loader: L)
    where
        L: 'static + ResourceLoader<Shader, ShaderImportOptions>,
    {
        self.shaders.set_loader(loader);
    }

    /// Sets a custom curve loader.
    pub fn set_curve_loader<L>(&mut self, loader: L)
    where
        L: 'static + ResourceLoader<CurveResource, CurveImportOptions>,
    {
        self.curves.set_loader(loader);
    }

    /// Sets a custom ABSM loader.
    pub fn set_absm_loader<L>(&mut self, loader: L)
    where
        L: 'static + ResourceLoader<AbsmResource, AbsmImportOptions>,
    {
        self.absm.set_loader(loader);
    }

    /// Sets a custom animation loader.
    pub fn set_animation_loader<L>(&mut self, loader: L)
    where
        L: 'static + ResourceLoader<AnimationResource, AnimationImportOptions>,
    {
        self.animations.set_loader(loader);
    }

    /// Wait until all resources are loaded (or failed to load).
    pub fn wait_concurrent(&self) -> ResourceWaitContext {
        ResourceWaitContext {
            models: self.models.resources(),
            absm: self.absm.resources(),
            curves: self.curves.resources(),
            shaders: self.shaders.resources(),
            textures: self.textures.resources(),
            sound_buffers: self.sound_buffers.resources(),
            animations: self.animations.resources(),
        }
    }
}

/// A set of resources that can be waited for.
#[must_use]
pub struct ResourceWaitContext {
    models: Vec<Model>,
    absm: Vec<AbsmResource>,
    curves: Vec<CurveResource>,
    shaders: Vec<Shader>,
    textures: Vec<Texture>,
    sound_buffers: Vec<SoundBufferResource>,
    animations: Vec<AnimationResource>,
}

impl ResourceWaitContext {
    /// Wait until all resources are loaded (or failed to load).
    pub async fn wait_concurrent(self) {
        join_all(self.models).await;
        join_all(self.absm).await;
        join_all(self.curves).await;
        join_all(self.shaders).await;
        join_all(self.textures).await;
        join_all(self.sound_buffers).await;
        join_all(self.animations).await;
    }
}

/// See module docs.
pub struct ResourceManagerState {
    containers_storage: Option<ContainersStorage>,
    watcher: Option<FileSystemWatcher>,
}

/// See module docs.
#[derive(Clone)]
pub struct ResourceManager {
    state: Arc<Mutex<ResourceManagerState>>,
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

impl Display for TextureRegistrationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TextureRegistrationError::Texture(v) => Display::fmt(v, f),
            TextureRegistrationError::InvalidState => {
                write!(f, "A texture was in invalid state!")
            }
            TextureRegistrationError::AlreadyRegistered => {
                write!(f, "A texture is already registered!")
            }
        }
    }
}

impl From<TextureError> for TextureRegistrationError {
    fn from(e: TextureError) -> Self {
        Self::Texture(e)
    }
}

impl ResourceManager {
    /// Creates a resource manager with default settings and loaders.
    pub fn new(serialization_context: Arc<SerializationContext>) -> Self {
        let resource_manager = Self {
            state: Arc::new(Mutex::new(ResourceManagerState::new())),
        };

        let task_pool = Arc::new(TaskPool::new());

        resource_manager.state().containers_storage = Some(ContainersStorage {
            textures: ResourceContainer::new(task_pool.clone(), Box::new(TextureLoader)),
            models: ResourceContainer::new(
                task_pool.clone(),
                Box::new(ModelLoader {
                    resource_manager: resource_manager.clone(),
                    serialization_context,
                }),
            ),
            sound_buffers: ResourceContainer::new(task_pool.clone(), Box::new(SoundBufferLoader)),
            animations: ResourceContainer::new(task_pool.clone(), Box::new(AnimationLoader)),
            shaders: ResourceContainer::new(task_pool.clone(), Box::new(ShaderLoader)),
            curves: ResourceContainer::new(task_pool.clone(), Box::new(CurveLoader)),
            absm: ResourceContainer::new(task_pool, Box::new(AbsmLoader)),
        });

        resource_manager
    }

    /// Returns a guarded reference to internal state of resource manager.
    pub fn state(&self) -> MutexGuard<'_, ResourceManagerState> {
        self.state.lock()
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
    /// See [set_default_import_options](ResourceContainer::set_default_import_options) for more info.
    ///
    /// # Async/.await
    ///
    /// Each Texture implements Future trait and can be used in async contexts.
    ///
    /// # Supported formats
    ///
    /// To load images and decode them, Fyrox uses image create which supports following image formats: png, tga, bmp, dds,
    /// jpg, gif, tiff, dxt.
    pub fn request_texture<P: AsRef<Path>>(&self, path: P) -> Texture {
        self.state().containers_mut().textures.request(path)
    }

    /// Saves given texture in the specified path and registers it in resource manager, so
    /// it will be accessible through it later.
    pub fn register_texture<P: AsRef<Path>>(
        &self,
        texture: Texture,
        path: P,
    ) -> Result<(), TextureRegistrationError> {
        let mut state = self.state();
        if state.containers().textures.find(path.as_ref()).is_some() {
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
                        state.containers_mut().textures.push(texture);
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
    /// and RGS (native Fyroxed format) formats are supported.
    pub fn request_model<P: AsRef<Path>>(&self, path: P) -> Model {
        self.state().containers_mut().models.request(path)
    }

    /// Tries to load new sound buffer from given path or get instance of existing, if any.
    /// This method is **blocking**, so it will block current thread until sound buffer is
    /// loading. On failure it returns None and prints failure reason to log.
    ///
    /// # Supported formats
    ///
    /// Currently only WAV and OGG are supported.
    pub fn request_sound_buffer<P: AsRef<Path>>(&self, path: P) -> SoundBufferResource {
        self.state().containers_mut().sound_buffers.request(path)
    }

    /// Tries to load a new shader resource from given path or get instance of existing, if any.
    /// This method is asynchronous, it immediately returns a shader which can be shared across
    /// multiple places, the loading may fail, but it is internal state of the shader.
    ///
    /// # Async/.await
    ///
    /// Each shader implements Future trait and can be used in async contexts.
    pub fn request_shader<P: AsRef<Path>>(&self, path: P) -> Shader {
        self.state().containers_mut().shaders.request(path)
    }

    /// Tries to load a new curve resource from given path or get instance of existing, if any.
    /// This method is asynchronous, it immediately returns a curve which can be shared across
    /// multiple places, the loading may fail, but it is internal state of the curve resource.
    ///
    /// # Async/.await
    ///
    /// Each curve implements Future trait and can be used in async contexts.
    pub fn request_curve<P: AsRef<Path>>(&self, path: P) -> CurveResource {
        self.state().containers_mut().curves.request(path)
    }

    /// Tries to load a new ABSM resource from given path or get instance of existing, if any.
    /// This method is asynchronous, it immediately returns a ABSM which can be shared across
    /// multiple places, the loading may fail, but it is internal state of the ABSM resource.
    ///
    /// # Async/.await
    ///
    /// Each ABSM implements Future trait and can be used in async contexts.
    pub fn request_absm<P: AsRef<Path>>(&self, path: P) -> AbsmResource {
        self.state().containers_mut().absm.request(path)
    }

    /// Tries to load a new ABSM resource from given path or get instance of existing, if any.
    /// This method is asynchronous, it immediately returns an animation which can be shared across
    /// multiple places, the loading may fail, but it is internal state of the animation resource.
    ///
    /// # Async/.await
    ///
    /// Each animation implements Future trait and can be used in async contexts.
    pub fn request_animation<P: AsRef<Path>>(&self, path: P) -> AnimationResource {
        self.state().containers_mut().animations.request(path)
    }

    /// Reloads every loaded texture. This method is asynchronous, internally it uses thread pool
    /// to run reload on separate thread per texture.
    pub async fn reload_textures(&self) {
        let resources = self.state().containers_mut().textures.reload_resources();
        join_all(resources).await;
    }

    /// Reloads every loaded model. This method is asynchronous, internally it uses thread pool
    /// to run reload on separate thread per model.
    pub async fn reload_models(&self) {
        let resources = self.state().containers_mut().models.reload_resources();
        join_all(resources).await;
    }

    /// Reloads every loaded shader. This method is asynchronous, internally it uses thread pool
    /// to run reload on separate thread per shader.
    pub async fn reload_shaders(&self) {
        let resources = self.state().containers_mut().shaders.reload_resources();
        join_all(resources).await;
    }

    /// Reloads every loaded curve resource. This method is asynchronous, internally it uses thread pool
    /// to run reload on separate thread per resource.
    pub async fn reload_curve_resources(&self) {
        let resources = self.state().containers_mut().curves.reload_resources();
        join_all(resources).await;
    }

    /// Reloads every loaded ABSM resource. This method is asynchronous, internally it uses thread pool
    /// to run reload on separate thread per resource.
    pub async fn reload_absm_resources(&self) {
        let resources = self.state().containers_mut().absm.reload_resources();
        join_all(resources).await;
    }

    /// Reloads every loaded animation resource. This method is asynchronous, internally it uses thread pool
    /// to run reload on separate thread per resource.
    pub async fn reload_animations(&self) {
        let resources = self.state().containers_mut().animations.reload_resources();
        join_all(resources).await;
    }

    /// Reloads every loaded sound buffer. This method is asynchronous, internally it uses thread pool
    /// to run reload on separate thread per sound buffer.
    pub async fn reload_sound_buffers(&self) {
        let resources = self
            .state()
            .containers_mut()
            .sound_buffers
            .reload_resources();
        join_all(resources).await;
    }

    /// Reloads all loaded resources. Normally it should never be called, because it is **very** heavy
    /// method! This method is asynchronous, it uses all available CPU power to reload resources as
    /// fast as possible.
    pub async fn reload_resources(&self) {
        crate::core::futures::join!(
            self.reload_textures(),
            self.reload_models(),
            self.reload_sound_buffers(),
            self.reload_animations(),
            self.reload_shaders(),
            self.reload_curve_resources(),
            self.reload_absm_resources(),
        );
    }
}

impl ResourceManagerState {
    pub(in crate::engine) fn new() -> Self {
        Self {
            containers_storage: None,
            watcher: None,
        }
    }

    /// Sets resource watcher which will track any modifications in file system and forcing
    /// the manager to reload changed resources. By default there is no watcher, since it
    /// may be an undesired effect to reload resources at runtime. This is very useful thing
    /// for fast iterative development.
    pub fn set_watcher(&mut self, watcher: Option<FileSystemWatcher>) {
        self.watcher = watcher;
    }

    /// Returns a reference to resource containers storage.
    pub fn containers(&self) -> &ContainersStorage {
        self.containers_storage
            .as_ref()
            .expect("Corrupted resource manager!")
    }

    /// Returns a reference to resource containers storage.
    pub fn containers_mut(&mut self) -> &mut ContainersStorage {
        self.containers_storage
            .as_mut()
            .expect("Corrupted resource manager!")
    }

    /// Returns total amount of resources in pending state.
    pub fn count_pending_resources(&self) -> usize {
        let containers = self.containers();
        containers.textures.count_pending_resources()
            + containers.sound_buffers.count_pending_resources()
            + containers.models.count_pending_resources()
            + containers.shaders.count_pending_resources()
            + containers.curves.count_pending_resources()
            + containers.absm.count_pending_resources()
            + containers.animations.count_pending_resources()
    }

    /// Returns total amount of loaded resources.
    pub fn count_loaded_resources(&self) -> usize {
        let containers = self.containers();
        containers.textures.count_loaded_resources()
            + containers.sound_buffers.count_loaded_resources()
            + containers.models.count_loaded_resources()
            + containers.shaders.count_loaded_resources()
            + containers.curves.count_loaded_resources()
            + containers.absm.count_loaded_resources()
            + containers.animations.count_loaded_resources()
    }

    /// Returns total amount of registered resources.
    pub fn count_registered_resources(&self) -> usize {
        let containers = self.containers();
        containers.textures.len()
            + containers.sound_buffers.len()
            + containers.models.len()
            + containers.shaders.len()
            + containers.curves.len()
            + containers.absm.len()
            + containers.animations.len()
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
        let containers = self.containers_mut();
        containers.sound_buffers.destroy_unused();
        containers.models.destroy_unused();
        containers.textures.destroy_unused();
        containers.shaders.destroy_unused();
        containers.curves.destroy_unused();
        containers.absm.destroy_unused();
        containers.animations.destroy_unused();
    }

    /// Update resource containers and do hot-reloading.
    ///
    /// Resources are removed if they're not used
    /// or reloaded if they have changed in disk.
    ///
    /// Normally, this is called from `Engine::update()`.
    /// You should only call this manually if you don't use that method.
    pub fn update(&mut self, dt: f32) {
        let containers = self.containers_mut();
        containers.textures.update(dt);
        containers.models.update(dt);
        containers.sound_buffers.update(dt);
        containers.shaders.update(dt);
        containers.curves.update(dt);
        containers.absm.update(dt);
        containers.animations.update(dt);

        if let Some(watcher) = self.watcher.as_ref() {
            if let Some(evt) = watcher.try_get_event() {
                if let notify::EventKind::Modify(_) = evt.kind {
                    for path in evt.paths {
                        if let Ok(relative_path) = make_relative_path(path) {
                            let containers = self.containers_mut();
                            for container in [
                                &mut containers.textures as &mut dyn Container,
                                &mut containers.models as &mut dyn Container,
                                &mut containers.sound_buffers as &mut dyn Container,
                                &mut containers.animations as &mut dyn Container,
                                &mut containers.shaders as &mut dyn Container,
                                &mut containers.curves as &mut dyn Container,
                                &mut containers.absm as &mut dyn Container,
                            ] {
                                if container.try_reload_resource_from_path(&relative_path) {
                                    Log::info(format!(
                                        "File {} was changed, trying to reload a respective resource...",
                                        relative_path.display()
                                    ));

                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
