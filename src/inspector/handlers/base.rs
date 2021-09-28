use crate::{
    inspector::SenderHelper,
    scene::commands::graph::{SetNameCommand, SetTagCommand, SetVisibleCommand},
};
use rg3d::gui::message::FieldKind;
use rg3d::{core::pool::Handle, gui::message::PropertyChanged, scene::node::Node};

pub fn handle_base_property_changed(
    args: &PropertyChanged,
    node_handle: Handle<Node>,
    helper: &SenderHelper,
) {
    if let FieldKind::Object(ref value) = args.value {
        match args.name.as_ref() {
            "name" => {
                helper.do_scene_command(SetNameCommand::new(
                    node_handle,
                    value.cast_value::<String>().unwrap().clone(),
                ));
            }
            "tag" => {
                helper.do_scene_command(SetTagCommand::new(
                    node_handle,
                    value.cast_value::<String>().unwrap().clone(),
                ));
            }
            "visibility" => {
                helper.do_scene_command(SetVisibleCommand::new(
                    node_handle,
                    *value.cast_value::<bool>().unwrap(),
                ));
            }
            _ => println!("Unhandled property of Base: {:?}", args),
        }
    }
}
