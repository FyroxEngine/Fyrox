use crate::{
    inspector::handlers::node::transform::handle_transform_property_changed,
    make_command,
    scene::commands::{graph::*, lod::*},
    ErasedHandle, SceneCommand,
};
use rg3d::scene::base::{LodControlledObject, LodGroup};
use rg3d::{
    core::pool::Handle,
    gui::inspector::{CollectionChanged, FieldKind, PropertyChanged},
    scene::{
        base::{Base, LevelOfDetail, Property, PropertyValue},
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
        FieldKind::Collection(ref collection_changed) => match args.name.as_ref() {
            Base::PROPERTIES => match **collection_changed {
                CollectionChanged::Add => Some(SceneCommand::new(AddPropertyCommand {
                    handle,
                    property: Default::default(),
                })),
                CollectionChanged::Remove(i) => Some(SceneCommand::new(RemovePropertyCommand {
                    handle,
                    index: i,
                    property: None,
                })),
                CollectionChanged::ItemChanged {
                    index,
                    ref property,
                } => match property.value {
                    FieldKind::Object(ref value) => match property.name.as_ref() {
                        Property::VALUE => Some(SceneCommand::new(SetPropertyValueCommand {
                            handle,
                            index,
                            value: value.cast_value::<PropertyValue>().cloned()?,
                        })),
                        Property::NAME => Some(SceneCommand::new(SetPropertyNameCommand {
                            handle,
                            index,
                            name: value.cast_value_cloned()?,
                        })),
                        _ => None,
                    },
                    FieldKind::Inspectable(ref property_changed) => {
                        match property_changed.name.as_ref() {
                            "0" => {
                                if let FieldKind::Object(ref value) = property_changed.value {
                                    let value = if let Some(int64) =
                                        value.cast_value_cloned::<i64>()
                                    {
                                        Some(PropertyValue::I64(int64))
                                    } else if let Some(uint64) = value.cast_value_cloned::<u64>() {
                                        Some(PropertyValue::U64(uint64))
                                    } else if let Some(int32) = value.cast_value_cloned::<i32>() {
                                        Some(PropertyValue::I32(int32))
                                    } else if let Some(uint32) = value.cast_value_cloned::<u32>() {
                                        Some(PropertyValue::U32(uint32))
                                    } else if let Some(int16) = value.cast_value_cloned::<i16>() {
                                        Some(PropertyValue::I16(int16))
                                    } else if let Some(uint16) = value.cast_value_cloned::<u16>() {
                                        Some(PropertyValue::U16(uint16))
                                    } else if let Some(int8) = value.cast_value_cloned::<i8>() {
                                        Some(PropertyValue::I8(int8))
                                    } else if let Some(uint8) = value.cast_value_cloned::<u8>() {
                                        Some(PropertyValue::U8(uint8))
                                    } else if let Some(float32) = value.cast_value_cloned::<f32>() {
                                        Some(PropertyValue::F32(float32))
                                    } else if let Some(float64) = value.cast_value_cloned::<f64>() {
                                        Some(PropertyValue::F64(float64))
                                    } else if let Some(string) = value.cast_value_cloned::<String>()
                                    {
                                        Some(PropertyValue::String(string))
                                    } else if let Some(node_handle) =
                                        value.cast_value_cloned::<Handle<Node>>()
                                    {
                                        Some(PropertyValue::NodeHandle(node_handle))
                                    } else if let Some(handle) =
                                        value.cast_value_cloned::<ErasedHandle>()
                                    {
                                        Some(PropertyValue::Handle(handle))
                                    } else {
                                        None
                                    };

                                    if let Some(value) = value {
                                        Some(SceneCommand::new(SetPropertyValueCommand {
                                            handle,
                                            index,
                                            value,
                                        }))
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        }
                    }
                    _ => None,
                },
            },
            _ => None,
        },
        FieldKind::Inspectable(ref inner_value) => match args.name.as_ref() {
            Base::LOD_GROUP => match inner_value.name.as_ref() {
                LodGroup::LEVELS => match inner_value.value {
                    FieldKind::Collection(ref collection_changed) => match **collection_changed {
                        CollectionChanged::Add => Some(SceneCommand::new(
                            AddLodGroupLevelCommand::new(handle, Default::default()),
                        )),
                        CollectionChanged::Remove(i) => Some(SceneCommand::new(
                            RemoveLodGroupLevelCommand::new(handle, i),
                        )),
                        CollectionChanged::ItemChanged {
                            index: lod_index,
                            ref property,
                        } => match property.value {
                            FieldKind::Object(ref value) => match property.name.as_ref() {
                                LevelOfDetail::BEGIN => {
                                    Some(SceneCommand::new(ChangeLodRangeBeginCommand::new(
                                        handle,
                                        lod_index,
                                        *value.cast_value()?,
                                    )))
                                }
                                LevelOfDetail::END => {
                                    Some(SceneCommand::new(ChangeLodRangeEndCommand::new(
                                        handle,
                                        lod_index,
                                        *value.cast_value()?,
                                    )))
                                }
                                _ => None,
                            },
                            FieldKind::Collection(ref collection_changed) => {
                                match property.name.as_ref() {
                                    LevelOfDetail::OBJECTS => match **collection_changed {
                                        CollectionChanged::Add => {
                                            Some(SceneCommand::new(AddLodObjectCommand::new(
                                                handle,
                                                lod_index,
                                                Default::default(),
                                            )))
                                        }
                                        CollectionChanged::Remove(object_index) => {
                                            Some(SceneCommand::new(RemoveLodObjectCommand::new(
                                                handle,
                                                lod_index,
                                                object_index,
                                            )))
                                        }
                                        CollectionChanged::ItemChanged {
                                            index,
                                            ref property,
                                        } => match property.name.as_ref() {
                                            LodControlledObject::F_0 => {
                                                if let FieldKind::Object(ref value) = property.value
                                                {
                                                    Some(SceneCommand::new(
                                                        SetLodGroupLodObjectValue {
                                                            handle,
                                                            lod_index,
                                                            object_index: index,
                                                            value: value.cast_value_cloned()?,
                                                        },
                                                    ))
                                                } else {
                                                    None
                                                }
                                            }
                                            _ => None,
                                        },
                                    },
                                    _ => None,
                                }
                            }
                            _ => None,
                        },
                    },
                    _ => None,
                },
                _ => None,
            },
            Base::LOCAL_TRANSFORM => handle_transform_property_changed(inner_value, handle, node),
            _ => None,
        },
    }
}
