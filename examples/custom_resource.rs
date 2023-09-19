use fyrox::event_loop::EventLoop;
use fyrox::{
    asset::{
        event::ResourceEventBroadcaster,
        loader::{BoxedLoaderFuture, ResourceLoader},
        untyped::UntypedResource,
        ResourceData,
    },
    core::{
        futures::executor,
        io::{self},
        pool::Handle,
        reflect::prelude::*,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
        TypeUuidProvider,
    },
    engine::{executor::Executor, GraphicsContextParams},
    plugin::{Plugin, PluginConstructor, PluginContext},
    scene::Scene,
    window::WindowAttributes,
};
use std::{
    any::Any,
    borrow::Cow,
    path::{Path, PathBuf},
};

#[derive(Debug, Visit, Reflect)]
struct CustomResource {
    // Your resource must store the path.
    path: PathBuf,
    some_data: String,
}

impl TypeUuidProvider for CustomResource {
    // Every resource must provide a unique identifier, that is used for dynamic type
    // casting, serialization, etc.
    fn type_uuid() -> Uuid {
        uuid!("15551157-651b-4f1d-a5fb-6874fbfe8637")
    }
}

impl ResourceData for CustomResource {
    fn path(&self) -> Cow<Path> {
        Cow::Borrowed(&self.path)
    }

    fn set_path(&mut self, path: PathBuf) {
        self.path = path;
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn type_uuid(&self) -> Uuid {
        <Self as TypeUuidProvider>::type_uuid()
    }

    fn is_procedural(&self) -> bool {
        false
    }
}

struct CustomResourceLoader;

impl ResourceLoader for CustomResourceLoader {
    fn extensions(&self) -> &[&str] {
        &["my_resource"]
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn load(
        &self,
        resource: UntypedResource,
        event_broadcaster: ResourceEventBroadcaster,
        reload: bool,
    ) -> BoxedLoaderFuture {
        Box::pin(async move {
            let path = resource.path();
            match io::load_file(&path).await {
                Ok(content) => {
                    let my_resource = CustomResource {
                        path,
                        some_data: String::from_utf8(content).unwrap(),
                    };

                    resource.commit_ok(my_resource);

                    // Notify potential subscribers that the resource was loaded.
                    event_broadcaster.broadcast_loaded_or_reloaded(resource, reload);
                }
                Err(err) => {
                    resource.commit_error(path, err);
                }
            }
        })
    }
}

struct Game {}

impl Plugin for Game {}

struct GameConstructor;

impl PluginConstructor for GameConstructor {
    fn create_instance(
        &self,
        _override_scene: Handle<Scene>,
        _context: PluginContext,
    ) -> Box<dyn Plugin> {
        _context
            .resource_manager
            .state()
            .loaders
            .set(CustomResourceLoader);

        let resource = executor::block_on(
            _context
                .resource_manager
                .request::<CustomResource, _>("examples/data/custom.my_resource"),
        )
        .unwrap();

        println!("{}", resource.data_ref().some_data);

        Box::new(Game {})
    }
}

fn main() {
    let mut executor = Executor::from_params(
        EventLoop::new().unwrap(),
        GraphicsContextParams {
            window_attributes: WindowAttributes {
                title: "Example - Custom Resource".to_string(),
                ..Default::default()
            },
            vsync: true,
        },
    );
    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}
