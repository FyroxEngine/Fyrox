use crate::{
    do_command,
    inspector::SenderHelper,
    scene::commands::terrain::{
        AddTerrainLayerCommand, DeleteTerrainLayerCommand, SetTerrainDecalLayerIndexCommand,
        SetTerrainLayerMaskPropertyNameCommand,
    },
};
use rg3d::{
    core::pool::Handle,
    gui::message::{CollectionChanged, FieldKind, PropertyChanged},
    scene::{graph::Graph, node::Node, terrain::Layer, terrain::Terrain},
};
use std::any::TypeId;

pub fn handle_terrain_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    helper: &SenderHelper,
    graph: &Graph,
) -> Option<()> {
    match args.name.as_ref() {
        Terrain::LAYERS => {
            if let FieldKind::Collection(ref collection_changed) = args.value {
                match &**collection_changed {
                    CollectionChanged::Add => {
                        helper.do_scene_command(AddTerrainLayerCommand::new(handle, graph))
                    }
                    CollectionChanged::Remove(index) => {
                        helper.do_scene_command(DeleteTerrainLayerCommand::new(handle, *index))
                    }
                    CollectionChanged::ItemChanged { index, property } => {
                        assert_eq!(property.owner_type_id, TypeId::of::<Layer>());
                        if let FieldKind::Object(ref args) = property.value {
                            match property.name.as_ref() {
                                "mask_property_name" => helper.do_scene_command(
                                    SetTerrainLayerMaskPropertyNameCommand {
                                        handle,
                                        layer_index: *index,
                                        value: args.cast_value::<String>().cloned()?,
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
                do_command!(helper, SetTerrainDecalLayerIndexCommand, handle, args)
            }
        }
        _ => println!("Unhandled property of Camera: {:?}", args),
    }
    Some(())
}
