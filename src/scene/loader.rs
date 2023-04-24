//! Async scene loader helper. See [`AsyncSceneLoader`] docs for more info.

use crate::{
    asset::manager::ResourceManager,
    core::parking_lot::Mutex,
    engine::SerializationContext,
    scene::{Scene, SceneLoader},
};
use std::{path::PathBuf, sync::Arc};

struct LoaderState {
    scene: Option<Result<Scene, String>>,
}

/// Asynchronous scene loader is a cross-platform scene loader, including platforms
/// with no true multi-threading (like WebAssembly). It is easy and straightforward to use:
///
/// ```rust
/// use std::path::Path;
/// use fyrox::event_loop::ControlFlow;
/// use fyrox::plugin::{Plugin, PluginContext};
/// use fyrox::scene::loader::AsyncSceneLoader;
/// use fyrox::utils::log::Log;
///
/// struct Game {
///     loader: Option<AsyncSceneLoader>,
/// }
///
/// impl Game {
///     // Step 1. Call this method once when you need to "kick off" scene loading.
///     fn load_scene(&mut self, path: &Path, context: &mut PluginContext) {
///         // Request asynchronous scene loading.
///         self.loader = Some(AsyncSceneLoader::begin_loading(
///             path.into(),
///             context.serialization_context.clone(),
///             context.resource_manager.clone(),
///         ));
///     }
///
///     // Step 2. Call this method in your game loop to continuously check loading progress.
///     fn check_loading_progress(&mut self, context: &mut PluginContext) {
///         if let Some(loader) = self.loader.as_ref() {
///             if let Some(result) = loader.fetch_result() {
///                 // Loading could end in either successfully loaded scene or some error.
///                 match result {
///                     Ok(scene) => {
///                         // Add the scene to the engine, so it will be included in engine processing pipeline.
///                         context.scenes.add(scene);
///                     }
///                     Err(err) => Log::err(err),
///                 }
///
///                 // Discard the loader once it is finished its job.
///                 self.loader = None;
///             }
///         }
///     }
/// }
///
/// impl Plugin for Game {
///     fn update(&mut self, context: &mut PluginContext, _control_flow: &mut ControlFlow) {
///         // Check whether the scene is loaded or not. While it is loading, we can show progress bar
///         // or even loading screen with useful information.
///         self.check_loading_progress(context)
///     }
/// }
/// ```
#[derive(Clone)]
pub struct AsyncSceneLoader {
    state: Arc<Mutex<LoaderState>>,
}

impl AsyncSceneLoader {
    /// Begins scene loading. See [`AsyncSceneLoader`] docs for usage examples.
    pub fn begin_loading(
        path: PathBuf,
        serialization_context: Arc<SerializationContext>,
        resource_manager: ResourceManager,
    ) -> Self {
        let state = Arc::new(Mutex::new(LoaderState { scene: None }));

        let inner_state = state.clone();
        let future = async move {
            match SceneLoader::from_file(&path, serialization_context, resource_manager.clone())
                .await
            {
                Ok(loader) => {
                    inner_state.lock().scene = Some(Ok(loader.finish().await));
                }
                Err(e) => {
                    inner_state.lock().scene = Some(Err(format!(
                        "Unable to load {} override scene! Reason: {:?}",
                        path.display(),
                        e
                    )));
                }
            }
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            std::thread::spawn(move || crate::core::futures::executor::block_on(future));
        }

        #[cfg(target_arch = "wasm32")]
        {
            crate::core::wasm_bindgen_futures::spawn_local(future);
        }

        Self { state }
    }

    /// Tries to get scene loading result. See [`AsyncSceneLoader`] docs for usage examples.
    pub fn fetch_result(&self) -> Option<Result<Scene, String>> {
        self.state.lock().scene.take()
    }
}
