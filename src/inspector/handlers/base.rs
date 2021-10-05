use crate::{
    inspector::SenderHelper,
    scene::commands::graph::{
        SetDepthOffsetCommand, SetLifetimeCommand, SetMobilityCommand, SetNameCommand,
        SetPhysicsBindingCommand, SetTagCommand, SetVisibleCommand,
    },
};
use rg3d::{
    core::pool::Handle,
    gui::message::{FieldKind, PropertyChanged},
    scene::{base::Base, node::Node},
};

pub fn handle_base_property_changed(
    args: &PropertyChanged,
    node_handle: Handle<Node>,
    helper: &SenderHelper,
) {
    if let FieldKind::Object(ref value) = args.value {
        match args.name.as_ref() {
            Base::NAME => {
                helper.do_scene_command(SetNameCommand::new(
                    node_handle,
                    value.cast_value::<String>().unwrap().clone(),
                ));
            }
            Base::TAG => {
                helper.do_scene_command(SetTagCommand::new(
                    node_handle,
                    value.cast_value::<String>().unwrap().clone(),
                ));
            }
            Base::VISIBILITY => {
                helper.do_scene_command(SetVisibleCommand::new(
                    node_handle,
                    *value.cast_value().unwrap(),
                ));
            }
            Base::MOBILITY => helper.do_scene_command(SetMobilityCommand::new(
                node_handle,
                *value.cast_value().unwrap(),
            )),
            Base::PHYSICS_BINDING => helper.do_scene_command(SetPhysicsBindingCommand::new(
                node_handle,
                *value.cast_value().unwrap(),
            )),
            Base::LIFETIME => helper.do_scene_command(SetLifetimeCommand::new(
                node_handle,
                *value.cast_value().unwrap(),
            )),
            Base::DEPTH_OFFSET => helper.do_scene_command(SetDepthOffsetCommand::new(
                node_handle,
                *value.cast_value().unwrap(),
            )),
            _ => println!("Unhandled property of Base: {:?}", args),
        }
    }
}
