//! Resource manager controls loading and lifetime of resource in the engine.

use crate::{
    core::futures::future::join_all,
    engine::resource_manager::{
        container::ResourceContainer,
        loader::{
            curve::CurveLoader,
            model::ModelLoader,
            shader::ShaderLoader,
            sound::{SoundBufferImportOptions, SoundBufferLoader},
            texture::TextureLoader,
        },
        task::TaskPool,
    },
    material::shader::{Shader, ShaderImportOptions},
    renderer::TextureUploadSender,
    resource::{
        curve::{CurveImportOptions, CurveResource},
        model::{Model, ModelImportOptions},
        texture::{Texture, TextureError, TextureImportOptions, TextureState},
    },
};
use fyrox_sound::buffer::SoundBufferResource;
use std::{
    path::Path,
    sync::{Arc, Mutex, MutexGuard},
};

pub mod container;
mod loader;
pub mod options;
mod task;

/// See module docs.
pub struct ResourceManagerState {
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

    pub(in crate) upload_sender: Option<TextureUploadSender>,
}

impl Default for ResourceManagerState {
    fn default() -> Self {
        let task_pool = Arc::new(TaskPool::new());
        Self {
            textures: ResourceContainer::new(task_pool.clone(), TextureLoader),
            models: ResourceContainer::new(task_pool.clone(), ModelLoader),
            sound_buffers: ResourceContainer::new(task_pool.clone(), SoundBufferLoader),
            shaders: ResourceContainer::new(task_pool.clone(), ShaderLoader),
            curves: ResourceContainer::new(task_pool, CurveLoader),
            upload_sender: None,
        }
    }
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
    pub(in crate) fn new(upload_sender: TextureUploadSender) -> Self {
        Self {
            state: Arc::new(Mutex::new(ResourceManagerState::new(upload_sender))),
        }
    }

    /// Returns a guarded reference to internal state of resource manager.
    pub fn state(&self) -> MutexGuard<'_, ResourceManagerState> {
        self.state.lock().unwrap()
    }

    /// Tries to get actual version of the texture. This is a helper function that mainly used to
    /// restore "shallow" textures when loading scenes. "Shallow" texture (or resource) is a resource
    /// that does not have any data loaded, but only path to data.
    #[must_use]
    pub fn map_texture(&self, texture: Option<Texture>) -> Option<Texture> {
        texture.map(|texture| self.request_texture(texture.state().path()))
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
    /// To load images and decode them, Fyrox uses image create which supports following image formats: png, tga, bmp, dds,
    /// jpg, gif, tiff, dxt.
    pub fn request_texture<P: AsRef<Path>>(&self, path: P) -> Texture {
        self.state().textures.request(path, self.clone())
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
    /// and RGS (native Fyroxed format) formats are supported.
    pub fn request_model<P: AsRef<Path>>(&self, path: P) -> Model {
        self.state().models.request(path, self.clone())
    }

    /// Tries to load new sound buffer from given path or get instance of existing, if any.
    /// This method is **blocking**, so it will block current thread until sound buffer is
    /// loading. On failure it returns None and prints failure reason to log.
    ///
    /// # Supported formats
    ///
    /// Currently only WAV and OGG are supported.
    pub fn request_sound_buffer<P: AsRef<Path>>(&self, path: P) -> SoundBufferResource {
        self.state().sound_buffers.request(path, self.clone())
    }

    /// Tries to load a new shader resource from given path or get instance of existing, if any.
    /// This method is asynchronous, it immediately returns a shader which can be shared across
    /// multiple places, the loading may fail, but it is internal state of the shader.
    ///
    /// # Async/.await
    ///
    /// Each shader implements Future trait and can be used in async contexts.
    pub fn request_shader<P: AsRef<Path>>(&self, path: P) -> Shader {
        self.state().shaders.request(path, self.clone())
    }

    /// Tries to load a new curve resource from given path or get instance of existing, if any.
    /// This method is asynchronous, it immediately returns a curve which can be shared across
    /// multiple places, the loading may fail, but it is internal state of the curve resource.
    ///
    /// # Async/.await
    ///
    /// Each shader implements Future trait and can be used in async contexts.
    pub fn request_curve_resource<P: AsRef<Path>>(&self, path: P) -> CurveResource {
        self.state().curves.request(path, self.clone())
    }

    /// Reloads given texture, forces the engine to re-upload the texture to the GPU.
    pub fn reload_texture(&self, texture: Texture) {
        self.state().textures.reload_resource(texture, self.clone());
    }

    /// Reloads every loaded texture. This method is asynchronous, internally it uses thread pool
    /// to run reload on separate thread per texture.
    pub async fn reload_textures(&self) {
        let resources = self.state().textures.reload_resources(self.clone());
        join_all(resources).await;
    }

    /// Reloads every loaded model. This method is asynchronous, internally it uses thread pool
    /// to run reload on separate thread per model.
    pub async fn reload_models(&self) {
        let resources = self.state().models.reload_resources(self.clone());
        join_all(resources).await;
    }

    /// Reloads every loaded shader. This method is asynchronous, internally it uses thread pool
    /// to run reload on separate thread per shader.
    pub async fn reload_shaders(&self) {
        let resources = self.state().shaders.reload_resources(self.clone());
        join_all(resources).await;
    }

    /// Reloads every loaded curve resource. This method is asynchronous, internally it uses thread pool
    /// to run reload on separate thread per resource.
    pub async fn reload_curve_resources(&self) {
        let resources = self.state().curves.reload_resources(self.clone());
        join_all(resources).await;
    }

    /// Reloads every loaded sound buffer. This method is asynchronous, internally it uses thread pool
    /// to run reload on separate thread per sound buffer.
    pub async fn reload_sound_buffers(&self) {
        let resources = self.state().sound_buffers.reload_resources(self.clone());
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
    pub(in crate::engine) fn new(upload_sender: TextureUploadSender) -> Self {
        let task_pool = Arc::new(TaskPool::new());
        Self {
            textures: ResourceContainer::new(task_pool.clone(), TextureLoader),
            models: ResourceContainer::new(task_pool.clone(), ModelLoader),
            sound_buffers: ResourceContainer::new(task_pool.clone(), SoundBufferLoader),
            shaders: ResourceContainer::new(task_pool.clone(), ShaderLoader),
            curves: ResourceContainer::new(task_pool, CurveLoader),
            upload_sender: Some(upload_sender),
        }
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
