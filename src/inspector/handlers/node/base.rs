use crate::inspector::handlers::node::transform::handle_transform_property_changed;
use crate::{
    do_command,
    inspector::SenderHelper,
    scene::commands::{graph::*, lod::*},
};
use rg3d::{
    core::pool::Handle,
    gui::message::{CollectionChanged, FieldKind, PropertyChanged},
    scene::{
        base::{Base, LevelOfDetail},
        node::Node,
    },
};

pub fn handle_base_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    node: &Node,
    helper: &SenderHelper,
) -> Option<()> {
    match args.value {
        FieldKind::Object(ref value) => match args.name.as_ref() {
            Base::NAME => {
                do_command!(helper, SetNameCommand, handle, value)
            }
            Base::TAG => {
                do_command!(helper, SetTagCommand, handle, value)
            }
            Base::VISIBILITY => {
                do_command!(helper, SetVisibleCommand, handle, value)
            }
            Base::MOBILITY => {
                do_command!(helper, SetMobilityCommand, handle, value)
            }
            Base::PHYSICS_BINDING => {
                do_command!(helper, SetPhysicsBindingCommand, handle, value)
            }
            Base::LIFETIME => {
                do_command!(helper, SetLifetimeCommand, handle, value)
            }
            Base::DEPTH_OFFSET => {
                do_command!(helper, SetDepthOffsetCommand, handle, value)
            }
            Base::LOD_GROUP => {
                do_command!(helper, SetLodGroupCommand, handle, value)
            }
            _ => None,
        },
        FieldKind::Inspectable(ref inner_value) => match args.name.as_ref() {
            Base::LOD_GROUP => match inner_value.value {
                FieldKind::Collection(ref collection_changed) => match **collection_changed {
                    CollectionChanged::Add => helper
                        .do_scene_command(AddLodGroupLevelCommand::new(handle, Default::default())),
                    CollectionChanged::Remove(i) => {
                        helper.do_scene_command(RemoveLodGroupLevelCommand::new(handle, i))
                    }
                    CollectionChanged::ItemChanged {
                        index,
                        ref property,
                    } => {
                        if let FieldKind::Object(ref value) = property.value {
                            match property.name.as_ref() {
                                LevelOfDetail::BEGIN => {
                                    helper.do_scene_command(ChangeLodRangeBeginCommand::new(
                                        handle,
                                        index,
                                        *value.cast_value()?,
                                    ))
                                }
                                LevelOfDetail::END => {
                                    helper.do_scene_command(ChangeLodRangeEndCommand::new(
                                        handle,
                                        index,
                                        *value.cast_value()?,
                                    ))
                                }
                                _ => None,
                            }
                        } else {
                            None
                        }
                    }
                },
                _ => None,
            },
            Base::LOCAL_TRANSFORM => {
                handle_transform_property_changed(inner_value, handle, node, helper)
            }
            _ => None,
        },
        _ => None,
    }
}
