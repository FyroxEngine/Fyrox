use crate::{
    scene::commands::{make_set_node_property_command, terrain::AddTerrainLayerCommandConstructor},
    SceneCommand,
};
use fyrox::{
    core::pool::Handle,
    fxhash::FxHashMap,
    gui::inspector::PropertyChanged,
    scene::{node::Node, terrain::Terrain},
};

/// A set of constructors for special cases, when simple reflection is not enough.
pub trait CommandConstructor {
    fn make_command(&self, handle: Handle<Node>, node: &mut Node) -> SceneCommand;
}

pub struct SceneNodePropertyChangedHandler {
    unique_handlers: FxHashMap<String, Box<dyn CommandConstructor>>,
}

impl SceneNodePropertyChangedHandler {
    pub fn new() -> Self {
        let mut unique_handlers = FxHashMap::<String, Box<dyn CommandConstructor>>::default();

        unique_handlers.insert(
            Terrain::LAYERS.to_string(),
            Box::new(AddTerrainLayerCommandConstructor),
        );

        Self { unique_handlers }
    }
}

impl SceneNodePropertyChangedHandler {
    pub fn handle(
        &self,
        args: &PropertyChanged,
        handle: Handle<Node>,
        node: &mut Node,
    ) -> Option<SceneCommand> {
        if let Some(constructor) = self.unique_handlers.get(&args.path()) {
            Some(constructor.make_command(handle, node))
        } else {
            Some(make_set_node_property_command(handle, args))
        }
    }
}
