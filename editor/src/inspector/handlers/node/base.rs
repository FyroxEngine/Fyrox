use crate::{
    inspector::handlers::node::transform::handle_transform_property_changed,
    scene::commands::{graph::*, lod::*, SetNodePropertyCommand},
    SceneCommand,
};
use fyrox::{
    core::pool::Handle,
    gui::inspector::{CollectionChanged, FieldKind, PropertyChanged},
    scene::{
        base::{
            serialize_script, Base, LevelOfDetail, LodControlledObject, LodGroup, Property,
            PropertyValue,
        },
        node::Node,
    },
};

pub fn handle_base_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    base: &mut Base,
) -> Option<SceneCommand> {
    match args.value {
        FieldKind::Object(ref value) => Some(SceneCommand::new(SetNodePropertyCommand::new(
            handle,
            args.path(),
            value.clone().into_box_reflect(),
        ))),
        FieldKind::Collection(ref collection_changed) => match args.name.as_ref() {
            Base::PROPERTIES => match **collection_changed {
                CollectionChanged::Add(_) => Some(SceneCommand::new(AddPropertyCommand {
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
                    ..
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
                        if let FieldKind::Object(ref value) = property_changed.value {
                            let value = match property_changed.name.as_ref() {
                                PropertyValue::I_64_F_0 => {
                                    value.cast_clone().map(PropertyValue::I64)
                                }
                                PropertyValue::U_64_F_0 => {
                                    value.cast_clone().map(PropertyValue::U64)
                                }
                                PropertyValue::I_32_F_0 => {
                                    value.cast_clone().map(PropertyValue::I32)
                                }
                                PropertyValue::U_32_F_0 => {
                                    value.cast_clone().map(PropertyValue::U32)
                                }
                                PropertyValue::I_16_F_0 => {
                                    value.cast_clone().map(PropertyValue::I16)
                                }
                                PropertyValue::U_16_F_0 => {
                                    value.cast_clone().map(PropertyValue::U16)
                                }
                                PropertyValue::I_8_F_0 => value.cast_clone().map(PropertyValue::I8),
                                PropertyValue::U_8_F_0 => value.cast_clone().map(PropertyValue::U8),
                                PropertyValue::F_32_F_0 => {
                                    value.cast_clone().map(PropertyValue::F32)
                                }
                                PropertyValue::F_64_F_0 => {
                                    value.cast_clone().map(PropertyValue::F64)
                                }
                                PropertyValue::STRING_F_0 => {
                                    value.cast_clone().map(PropertyValue::String)
                                }
                                PropertyValue::NODE_HANDLE_F_0 => {
                                    value.cast_clone().map(PropertyValue::NodeHandle)
                                }
                                PropertyValue::HANDLE_F_0 => {
                                    value.cast_clone().map(PropertyValue::Handle)
                                }
                                _ => None,
                            };
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
                },
            },
            _ => None,
        },
        FieldKind::Inspectable(ref inner_value) => match args.name.as_ref() {
            Base::LOD_GROUP => match inner_value.name.as_ref() {
                LodGroup::LEVELS => match inner_value.value {
                    FieldKind::Collection(ref collection_changed) => match **collection_changed {
                        CollectionChanged::Add(_) => Some(SceneCommand::new(
                            AddLodGroupLevelCommand::new(handle, Default::default()),
                        )),
                        CollectionChanged::Remove(i) => Some(SceneCommand::new(
                            RemoveLodGroupLevelCommand::new(handle, i),
                        )),
                        CollectionChanged::ItemChanged {
                            index: lod_index,
                            ref property,
                            ..
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
                                        CollectionChanged::Add(_) => {
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
                                            ..
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
            Base::SCRIPT => handle_script_property_changed(inner_value, handle, base),
            Base::LOCAL_TRANSFORM => handle_transform_property_changed(args, handle),
            _ => None,
        },
    }
}

fn handle_script_property_changed(
    args: &PropertyChanged,
    node_handle: Handle<Node>,
    base: &mut Base,
) -> Option<SceneCommand> {
    if let Some(script) = base.script_mut() {
        let old_data = serialize_script(script).expect("Script must be serializable!");

        if script.on_property_changed(args) {
            let new_data = serialize_script(script).expect("Script must be serializable!");

            return Some(SceneCommand::new(ScriptDataBlobCommand {
                handle: node_handle,
                old_value: old_data,
                new_value: new_data,
            }));
        }
    }
    None
}
