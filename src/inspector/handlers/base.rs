use crate::{
    inspector::SenderHelper,
    scene::commands::{
        graph::{SetNameCommand, SetTagCommand, SetVisibleCommand},
        SceneCommand,
    },
};
use rg3d::{core::pool::Handle, gui::message::PropertyChanged, scene::node::Node};

pub fn handle_base_property_changed(
    args: &PropertyChanged,
    node_handle: Handle<Node>,
    helper: &SenderHelper,
) {
    match args.name.as_ref() {
        "name" => {
            helper.do_scene_command(SceneCommand::SetName(SetNameCommand::new(
                node_handle,
                args.cast_value::<String>().unwrap().clone(),
            )));
        }
        "tag" => {
            helper.do_scene_command(SceneCommand::SetTag(SetTagCommand::new(
                node_handle,
                args.cast_value::<String>().unwrap().clone(),
            )));
        }
        "visibility" => {
            helper.do_scene_command(SceneCommand::SetVisible(SetVisibleCommand::new(
                node_handle,
                *args.cast_value::<bool>().unwrap(),
            )));
        }
        _ => println!("Unhandled property of Base: {:?}", args),
    }
}
