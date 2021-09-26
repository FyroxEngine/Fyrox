use crate::{
    inspector::SenderHelper,
    scene::commands::{camera::SetExposureCommand, SceneCommand},
};
use rg3d::gui::message::FieldKind;
use rg3d::{
    core::pool::Handle,
    gui::message::PropertyChanged,
    scene::{camera::Exposure, node::Node},
};

pub fn handle_exposure_property_changed(
    args: &PropertyChanged,
    node_handle: Handle<Node>,
    node: &Node,
    helper: &SenderHelper,
) {
    if let FieldKind::Object(ref value) = args.value {
        if let Node::Camera(camera) = node {
            match args.name.as_ref() {
                "key_value" => {
                    let mut current_auto_exposure = camera.exposure().clone();
                    if let Exposure::Auto {
                        ref mut key_value, ..
                    } = current_auto_exposure
                    {
                        *key_value = *value.cast_value::<f32>().unwrap();
                    }

                    helper.do_scene_command(SceneCommand::SetExposure(SetExposureCommand::new(
                        node_handle,
                        current_auto_exposure,
                    )))
                }
                "min_luminance" => {
                    let mut current_auto_exposure = camera.exposure().clone();
                    if let Exposure::Auto {
                        ref mut min_luminance,
                        ..
                    } = current_auto_exposure
                    {
                        *min_luminance = *value.cast_value::<f32>().unwrap();
                    }

                    helper.do_scene_command(SceneCommand::SetExposure(SetExposureCommand::new(
                        node_handle,
                        current_auto_exposure,
                    )))
                }
                "max_luminance" => {
                    let mut current_auto_exposure = camera.exposure().clone();
                    if let Exposure::Auto {
                        ref mut max_luminance,
                        ..
                    } = current_auto_exposure
                    {
                        *max_luminance = *value.cast_value::<f32>().unwrap();
                    }

                    helper.do_scene_command(SceneCommand::SetExposure(SetExposureCommand::new(
                        node_handle,
                        current_auto_exposure,
                    )))
                }
                _ => println!("Unhandled property of Camera: {:?}", args),
            }
        }
    }
}
