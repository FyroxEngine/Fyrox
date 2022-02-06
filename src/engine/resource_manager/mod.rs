//! Resource manager controls loading and lifetime of resource in the engine.

use crate::{
    core::{
        futures::future::join_all,
        make_relative_path,
        parking_lot::{Mutex, MutexGuard},
    },
    engine::resource_manager::{
        container::{Container, ResourceContainer},
        loader::{
            curve::CurveLoader,
            model::ModelLoader,
            shader::ShaderLoader,
            sound::{SoundBufferImportOptions, SoundBufferLoader},
            texture::TextureLoader,
        },
        task::TaskPool,
        watcher::ResourceWatcher,
    },
    material::shader::{Shader, ShaderImportOptions},
    resource::{
        curve::{CurveImportOptions, CurveResource},
        model::{Model, ModelImportOptions},
        texture::{Texture, TextureError, TextureImportOptions, TextureState},
    },
};
use fyrox_sound::buffer::SoundBufferResource;
use notify::DebouncedEvent;
use std::{path::Path, sync::Arc};

pub mod container;
mod loader;
pub mod options;
mod task;
pub mod watcher;

/// Storage of resource containers.
pub struct ContainersStorage {
    /// Container for texture resources.
    pub textures: ResourceContainer<Texture, TextureImportOptions, TextureLoader>,

    /// Container for model resources.
    pub models: ResourceContainer<Model, ModelImportOptions, ModelLoader>,

    /// Container for sound buffer resources.
    pub sound_buffers:
        ResourceContainer<SoundBufferResource, SoundBufferImportOptions, SoundBufferLoader>,

    /// Container for shader resources.
    pub shaders: ResourceContainer<Shader, ShaderImportOptions, ShaderLoader>,

    /// Container for curve resources.
    pub curves: ResourceContainer<CurveResource, CurveImportOptions, CurveLoader>,
}

/// See module docs.
pub struct ResourceManagerState {
    containers_storage: Option<ContainersStorage>,
    watcher: Option<ResourceWatcher>,
}

/// See module docs.
#[derive(Clone)]
pub struct ResourceManager {
    state: Arc<Mutex<ResourceManagerState>>,
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

impl ResourceManager {
    pub(in crate) fn new() -> Self {
        let resource_manager = Self {
            state: Arc::new(Mutex::new(ResourceManagerState::new())),
        };

        let task_pool = Arc::new(TaskPool::new());

        resource_manager.state().containers_storage = Some(ContainersStorage {
            textures: ResourceContainer::new(task_pool.clone(), TextureLoader),
            models: ResourceContainer::new(
                task_pool.clone(),
                ModelLoader {
                    resource_manager: resource_manager.clone(),
                },
            ),
            sound_buffers: ResourceContainer::new(task_pool.clone(), SoundBufferLoader),
            shaders: ResourceContainer::new(task_pool.clone(), ShaderLoader),
            curves: ResourceContainer::new(task_pool, CurveLoader),
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
    /// Each shader implements Future trait and can be used in async contexts.
    pub fn request_curve_resource<P: AsRef<Path>>(&self, path: P) -> CurveResource {
        self.state().containers_mut().curves.request(path)
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
            self.reload_shaders(),
            self.reload_curve_resources(),
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
    pub fn set_watcher(&mut self, watcher: Option<ResourceWatcher>) {
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
    }

    /// Returns total amount of loaded resources.
    pub fn count_loaded_resources(&self) -> usize {
        let containers = self.containers();
        containers.textures.count_loaded_resources()
            + containers.sound_buffers.count_loaded_resources()
            + containers.models.count_loaded_resources()
            + containers.shaders.count_loaded_resources()
            + containers.curves.count_loaded_resources()
    }

    /// Returns total amount of registered resources.
    pub fn count_registered_resources(&self) -> usize {
        let containers = self.containers();
        containers.textures.len()
            + containers.sound_buffers.len()
            + containers.models.len()
            + containers.shaders.len()
            + containers.curves.len()
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
    }

    /// Reload resources if they have changed in disk.
    pub fn update(&mut self, dt: f32) {
        let containers = self.containers_mut();
        containers.textures.update(dt);
        containers.models.update(dt);
        containers.sound_buffers.update(dt);
        containers.shaders.update(dt);
        containers.curves.update(dt);

        if let Some(watcher) = self.watcher.as_ref() {
            if let Some(DebouncedEvent::Write(path)) = watcher.try_get_event() {
                let relative_path = make_relative_path(path);
                let containers = self.containers_mut();
                for container in [
                    &mut containers.textures as &mut dyn Container,
                    &mut containers.models as &mut dyn Container,
                    &mut containers.sound_buffers as &mut dyn Container,
                    &mut containers.shaders as &mut dyn Container,
                    &mut containers.curves as &mut dyn Container,
                ] {
                    if container.try_reload_resource_from_path(&relative_path) {
                        break;
                    }
                }
            }
        }
    }
}
