use crate::{
    do_command,
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
    handle: Handle<Node>,
    helper: &SenderHelper,
) -> Option<()> {
    if let FieldKind::Object(ref value) = args.value {
        match args.name.as_ref() {
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
            _ => println!("Unhandled property of Base: {:?}", args),
        }
    }
    Some(())
}
