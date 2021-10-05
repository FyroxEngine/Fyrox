use crate::scene::commands::terrain::SetTerrainDecalLayerIndexCommand;
use crate::{
    inspector::SenderHelper,
    scene::commands::terrain::{
        AddTerrainLayerCommand, DeleteTerrainLayerCommand, SetTerrainLayerMaskPropertyNameCommand,
    },
};
use rg3d::scene::terrain::Terrain;
use rg3d::{
    core::pool::Handle,
    gui::message::{CollectionChanged, FieldKind, PropertyChanged},
    scene::{graph::Graph, node::Node, terrain::Layer},
};
use std::any::TypeId;

pub fn handle_terrain_property_changed(
    args: &PropertyChanged,
    node_handle: Handle<Node>,
    helper: &SenderHelper,
    graph: &Graph,
) {
    match args.name.as_ref() {
        Terrain::LAYERS => {
            if let FieldKind::Collection(ref collection_changed) = args.value {
                match &**collection_changed {
                    CollectionChanged::Add => {
                        helper.do_scene_command(AddTerrainLayerCommand::new(node_handle, graph))
                    }
                    CollectionChanged::Remove(index) => {
                        helper.do_scene_command(DeleteTerrainLayerCommand::new(node_handle, *index))
                    }
                    CollectionChanged::ItemChanged { index, property } => {
                        assert_eq!(property.owner_type_id, TypeId::of::<Layer>());
                        if let FieldKind::Object(ref args) = property.value {
                            match property.name.as_ref() {
                                "mask_property_name" => helper.do_scene_command(
                                    SetTerrainLayerMaskPropertyNameCommand {
                                        handle: node_handle,
                                        layer_index: *index,
                                        value: args
                                            .cast_value::<String>()
                                            .expect("mask_property_name must be String!")
                                            .clone(),
                                    },
                                ),
                                _ => (),
                            }
                        }
                    }
                }
            }
        }
        Terrain::DECAL_LAYER_INDEX => {
            if let FieldKind::Object(ref args) = args.value {
                helper.do_scene_command(SetTerrainDecalLayerIndexCommand::new(
                    node_handle,
                    *args.cast_value::<u8>().expect("Must be u8"),
                ))
            }
        }
        _ => println!("Unhandled property of Camera: {:?}", args),
    }
}
