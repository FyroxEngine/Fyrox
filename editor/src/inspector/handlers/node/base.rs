use crate::{
    handle_properties,
    inspector::handlers::node::transform::handle_transform_property_changed,
    scene::commands::{graph::*, lod::*},
    SceneCommand,
};
use fyrox::scene::base::{LodControlledObject, LodGroup};
use fyrox::{
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
    base: &Base,
) -> Option<SceneCommand> {
    match args.value {
        FieldKind::Object(ref value) => {
            handle_properties!(args.name.as_ref(), handle, value,
                Base::NAME => SetNameCommand,
                Base::TAG => SetTagCommand,
                Base::FRUSTUM_CULLING => SetFrustumCullingCommand,
                Base::VISIBILITY => SetVisibleCommand,
                Base::MOBILITY => SetMobilityCommand,
                Base::LIFETIME => SetLifetimeCommand,
                Base::DEPTH_OFFSET => SetDepthOffsetCommand,
                Base::LOD_GROUP => SetLodGroupCommand,
                Base::CAST_SHADOWS => SetCastShadowsCommand
            )
        }
        FieldKind::Collection(ref collection_changed) => match args.name.as_ref() {
            Base::PROPERTIES => match **collection_changed {
                CollectionChanged::Add => Some(SceneCommand::new(AddPropertyCommand {
                    handle,
                    value: Default::default(),
                })),
                CollectionChanged::Remove(i) => Some(SceneCommand::new(RemovePropertyCommand {
                    handle,
                    index: i,
                    value: None,
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
                            name: value.cast_clone()?,
                        })),
                        _ => None,
                    },
                    FieldKind::Inspectable(ref property_changed) => {
                        match property_changed.name.as_ref() {
                            "0" => {
                                if let FieldKind::Object(ref value) = property_changed.value {
                                    let value = value
                                        .cast_value()
                                        .map(|v| PropertyValue::I64(*v))
                                        .or_else(|| value.cast_clone().map(PropertyValue::U64))
                                        .or_else(|| value.cast_clone().map(PropertyValue::I32))
                                        .or_else(|| value.cast_clone().map(PropertyValue::U32))
                                        .or_else(|| value.cast_clone().map(PropertyValue::I16))
                                        .or_else(|| value.cast_clone().map(PropertyValue::U16))
                                        .or_else(|| value.cast_clone().map(PropertyValue::I8))
                                        .or_else(|| value.cast_clone().map(PropertyValue::U8))
                                        .or_else(|| value.cast_clone().map(PropertyValue::F32))
                                        .or_else(|| value.cast_clone().map(PropertyValue::F64))
                                        .or_else(|| value.cast_clone().map(PropertyValue::String))
                                        .or_else(|| {
                                            value.cast_clone().map(PropertyValue::NodeHandle)
                                        })
                                        .or_else(|| value.cast_clone().map(PropertyValue::Handle));

                                    value.map(|value| {
                                        SceneCommand::new(SetPropertyValueCommand {
                                            handle,
                                            index,
                                            value,
                                        })
                                    })
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
                                                            value: value.cast_clone()?,
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
            Base::LOCAL_TRANSFORM => handle_transform_property_changed(inner_value, handle, base),
            _ => None,
        },
    }
}
