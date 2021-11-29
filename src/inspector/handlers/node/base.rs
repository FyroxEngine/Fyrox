use crate::{
    inspector::handlers::node::transform::handle_transform_property_changed,
    make_command,
    scene::commands::{graph::*, lod::*},
    SceneCommand,
};
use rg3d::{
    core::pool::Handle,
    gui::inspector::{CollectionChanged, FieldKind, PropertyChanged},
    scene::{
        base::{Base, LevelOfDetail},
        node::Node,
    },
};

pub fn handle_base_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    node: &Node,
) -> Option<SceneCommand> {
    match args.value {
        FieldKind::Object(ref value) => match args.name.as_ref() {
            Base::NAME => {
                make_command!(SetNameCommand, handle, value)
            }
            Base::TAG => {
                make_command!(SetTagCommand, handle, value)
            }
            Base::VISIBILITY => {
                make_command!(SetVisibleCommand, handle, value)
            }
            Base::MOBILITY => {
                make_command!(SetMobilityCommand, handle, value)
            }
            Base::PHYSICS_BINDING => {
                make_command!(SetPhysicsBindingCommand, handle, value)
            }
            Base::LIFETIME => {
                make_command!(SetLifetimeCommand, handle, value)
            }
            Base::DEPTH_OFFSET => {
                make_command!(SetDepthOffsetCommand, handle, value)
            }
            Base::LOD_GROUP => {
                make_command!(SetLodGroupCommand, handle, value)
            }
            _ => None,
        },
        FieldKind::Inspectable(ref inner_value) => match args.name.as_ref() {
            Base::LOD_GROUP => match inner_value.value {
                FieldKind::Collection(ref collection_changed) => match **collection_changed {
                    CollectionChanged::Add => Some(SceneCommand::new(
                        AddLodGroupLevelCommand::new(handle, Default::default()),
                    )),
                    CollectionChanged::Remove(i) => Some(SceneCommand::new(
                        RemoveLodGroupLevelCommand::new(handle, i),
                    )),
                    CollectionChanged::ItemChanged {
                        index,
                        ref property,
                    } => {
                        if let FieldKind::Object(ref value) = property.value {
                            match property.name.as_ref() {
                                LevelOfDetail::BEGIN => {
                                    Some(SceneCommand::new(ChangeLodRangeBeginCommand::new(
                                        handle,
                                        index,
                                        *value.cast_value()?,
                                    )))
                                }
                                LevelOfDetail::END => {
                                    Some(SceneCommand::new(ChangeLodRangeEndCommand::new(
                                        handle,
                                        index,
                                        *value.cast_value()?,
                                    )))
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
            Base::LOCAL_TRANSFORM => handle_transform_property_changed(inner_value, handle, node),
            _ => None,
        },
        _ => None,
    }
}
