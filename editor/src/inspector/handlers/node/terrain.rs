use crate::{
    inspector::handlers::node::base::handle_base_property_changed, make_command,
    scene::commands::terrain::*, SceneCommand,
};
use fyrox::{
    core::pool::Handle,
    gui::inspector::{CollectionChanged, FieldKind, PropertyChanged},
    scene::{graph::Graph, node::Node, terrain::Layer, terrain::Terrain},
};
use std::any::TypeId;

pub fn handle_terrain_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    node: &Node,
    graph: &Graph,
) -> Option<SceneCommand> {
    if node.is_terrain() {
        match args.value {
            FieldKind::Collection(ref collection_changed) => match args.name.as_ref() {
                Terrain::LAYERS => match &**collection_changed {
                    CollectionChanged::Add => Some(SceneCommand::new(AddTerrainLayerCommand::new(
                        handle, graph,
                    ))),
                    CollectionChanged::Remove(index) => Some(SceneCommand::new(
                        DeleteTerrainLayerCommand::new(handle, *index),
                    )),
                    CollectionChanged::ItemChanged { index, property } => {
                        assert_eq!(property.owner_type_id, TypeId::of::<Layer>());
                        match property.value {
                            FieldKind::Object(ref args) => match property.name.as_ref() {
                                Layer::MASK_PROPERTY_NAME => Some(SceneCommand::new(
                                    SetTerrainLayerMaskPropertyNameCommand {
                                        handle,
                                        layer_index: *index,
                                        value: args.cast_value::<String>().cloned()?,
                                    },
                                )),
                                _ => None,
                            },
                            _ => None,
                        }
                    }
                },
                _ => None,
            },
            FieldKind::Object(ref value) => match args.name.as_ref() {
                Terrain::DECAL_LAYER_INDEX => {
                    make_command!(SetTerrainDecalLayerIndexCommand, handle, value)
                }
                _ => None,
            },
            FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
                Terrain::BASE => handle_base_property_changed(inner, handle, node),
                _ => None,
            },
        }
    } else {
        None
    }
}
