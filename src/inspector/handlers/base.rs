use crate::{
    inspector::SenderHelper,
    scene::commands::graph::{SetNameCommand, SetTagCommand, SetVisibleCommand},
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
                    *value.cast_value::<bool>().unwrap(),
                ));
            }
            Base::MOBILITY => {
                // TODO
            }
            Base::PHYSICS_BINDING => {
                // TODO
            }
            _ => println!("Unhandled property of Base: {:?}", args),
        }
    }
}
