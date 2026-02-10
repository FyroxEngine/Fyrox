// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! A module that handles asynchronous scene loading and is able to create derived scenes.

use crate::{
    asset::manager::ResourceManager,
    core::{dyntype::DynTypeConstructorContainer, log::Log, visitor::error::VisitError},
    engine::SerializationContext,
    resource::model::Model,
    scene::{Scene, SceneLoader},
};
use fxhash::FxHashMap;
use std::{
    path::{Path, PathBuf},
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
};
use uuid::Uuid;

pub(crate) struct SceneLoadingOptions {
    pub derived: bool,
}

/// A helper that is used to load scenes asynchronously.
///
/// ## Examples
///
/// ```rust
/// use fyrox_impl::{
///     core::{color::Color, visitor::prelude::*, reflect::prelude::*, log::Log, pool::Handle},
///     plugin::{Plugin, PluginContext, error::GameResult},
///     scene::Scene,
/// };
/// use std::path::Path;
///
/// #[derive(Visit, Reflect, Debug)]
/// #[reflect(non_cloneable)]
/// struct MyGame {
///     scene: Handle<Scene>,
/// }
///
/// impl MyGame {
///     pub fn new(scene_path: Option<&str>, context: PluginContext) -> Self {
///         context
///             .async_scene_loader
///             .request(scene_path.unwrap_or("data/scene.rgs"));
///
///         Self {
///             scene: Handle::NONE,
///         }
///     }
/// }
///
/// impl Plugin for MyGame {
///     fn on_scene_begin_loading(&mut self, path: &Path, _context: &mut PluginContext) -> GameResult {
///         Log::info(format!("{} scene has started loading.", path.display()));
///
///         // Use this method if you need to so something when a scene started loading.
///         Ok(())
///     }
///
///     fn on_scene_loaded(
///         &mut self,
///         path: &Path,
///         scene: Handle<Scene>,
///         data: &[u8],
///         context: &mut PluginContext,
///     ) -> GameResult {
///         // Optionally remove previous scene.
///         if self.scene.is_some() {
///             context.scenes.remove(self.scene);
///         }
///
///         // Remember new scene handle.
///         self.scene = scene;
///
///         Log::info(format!("{} scene was loaded!", path.display()));
///
///         // Do something with a newly loaded scene.
///         let scene_ref = &mut context.scenes[scene];
///
///         scene_ref.rendering_options.ambient_lighting_color = Color::opaque(20, 20, 20);
///
///         Ok(())
///     }
/// }
/// ```
///
/// This example shows a typical usage of the loader, an instance of which is available in the
/// plugin context. `Game::new` requests a new scene, which internally asks a resource manager to
/// load the scene. Then, when the scene is fully loaded, the engine calls `Plugin::on_scene_loaded`
/// method which allows you to do something with the newly loaded scene by taking a reference of it.
pub struct AsyncSceneLoader {
    resource_manager: ResourceManager,
    serialization_context: Arc<SerializationContext>,
    dyn_type_constructors: Arc<DynTypeConstructorContainer>,
    pub(crate) receiver: Receiver<SceneLoadingResult>,
    sender: Sender<SceneLoadingResult>,
    pub(crate) loading_scenes: FxHashMap<PathBuf, LoadingScene>,
}

pub(crate) struct LoadingScene {
    pub reported: bool,
    pub path: PathBuf,
    pub options: SceneLoadingOptions,
}

pub(crate) struct SceneLoadingResult {
    pub uuid: Uuid,
    pub path: PathBuf,
    pub result: Result<(Scene, Vec<u8>), VisitError>,
}

impl AsyncSceneLoader {
    pub(crate) fn new(
        resource_manager: ResourceManager,
        serialization_context: Arc<SerializationContext>,
        dyn_type_constructors: Arc<DynTypeConstructorContainer>,
    ) -> Self {
        let (sender, receiver) = channel();
        Self {
            resource_manager,
            serialization_context,
            dyn_type_constructors,
            receiver,
            sender,
            loading_scenes: Default::default(),
        }
    }

    fn request_with_options<P: AsRef<Path>>(&mut self, path: P, opts: SceneLoadingOptions) {
        let path = path.as_ref().to_path_buf();

        if self.loading_scenes.contains_key(&path) {
            Log::warn(format!("A scene {} is already loading!", path.display()))
        } else {
            // Register a new request.
            self.loading_scenes.insert(
                path.clone(),
                LoadingScene {
                    reported: false,
                    path: path.clone(),
                    options: opts,
                },
            );

            // Start loading in a separate off-thread task.
            let sender = self.sender.clone();
            let serialization_context = self.serialization_context.clone();
            let dyn_type_constructors = self.dyn_type_constructors.clone();
            let resource_manager = self.resource_manager.clone();
            let uuid = resource_manager.find::<Model>(&path).resource_uuid();

            // Acquire the resource IO from the resource manager
            let io = resource_manager.resource_io();
            resource_manager.task_pool().spawn_task(async move {
                match SceneLoader::from_file(
                    path.clone(),
                    io.as_ref(),
                    serialization_context,
                    dyn_type_constructors,
                    resource_manager.clone(),
                )
                .await
                {
                    Ok((loader, data)) => {
                        let scene = loader.finish().await;
                        Log::verify(sender.send(SceneLoadingResult {
                            uuid,
                            path,
                            result: Ok((scene, data)),
                        }));
                    }
                    Err(e) => {
                        Log::verify(sender.send(SceneLoadingResult {
                            uuid,
                            path,
                            result: Err(e),
                        }));
                    }
                }
            });
        }
    }

    /// Requests a scene for loading as derived scene. See [`AsyncSceneLoader`] for usage example.
    ///
    /// ## Raw vs Derived Scene
    ///
    /// Derived scene means its nodes will derive their properties from the nodes from the source
    /// scene. Derived scene is useful for saved games - you can serialize your scene as usual and
    /// it will only contain a "difference" between the original scene and yours. To load the same
    /// scene as raw scene use [`Self::request_raw`] method.
    ///
    /// Raw scene, on other hand, loads the scene as-is without any additional markings for the
    /// scene nodes. It could be useful to load saved games.
    pub fn request<P: AsRef<Path>>(&mut self, path: P) {
        self.request_with_options(path, SceneLoadingOptions { derived: true });
    }

    /// Requests a scene for loading in raw mode. See [`Self::request`] docs for more info.
    pub fn request_raw<P: AsRef<Path>>(&mut self, path: P) {
        self.request_with_options(path, SceneLoadingOptions { derived: false });
    }
}
