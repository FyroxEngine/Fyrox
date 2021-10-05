use crate::{
    inspector::SenderHelper,
    scene::commands::camera::{
        SetCameraPreviewCommand, SetColorGradingEnabledCommand, SetColorGradingLutCommand,
        SetEnvironmentMap, SetExposureCommand, SetFovCommand, SetSkyBoxCommand, SetViewportCommand,
        SetZFarCommand, SetZNearCommand,
    },
};
use rg3d::{
    core::pool::Handle,
    gui::message::{FieldKind, PropertyChanged},
    resource::texture::Texture,
    scene::{
        camera::{Camera, ColorGradingLut, Exposure, SkyBox},
        node::Node,
    },
};

pub fn handle_camera_property_changed(
    args: &PropertyChanged,
    node_handle: Handle<Node>,
    node: &Node,
    helper: &SenderHelper,
) {
    if let Node::Camera(camera) = node {
        match args.name.as_ref() {
            Camera::EXPOSURE => {
                match args.value {
                    // Exposure variant has changed.
                    FieldKind::Object(ref value) => {
                        helper.do_scene_command(SetExposureCommand::new(
                            node_handle,
                            value.cast_value::<Exposure>().unwrap().clone(),
                        ))
                    }
                    // Some inner property has changed
                    FieldKind::EnumerationVariant(ref args) => {
                        if let FieldKind::Object(ref value) = args.value {
                            match args.name.as_ref() {
                                Exposure::AUTO_KEY_VALUE => {
                                    let mut current_auto_exposure = camera.exposure().clone();
                                    if let Exposure::Auto {
                                        ref mut key_value, ..
                                    } = current_auto_exposure
                                    {
                                        *key_value = *value.cast_value::<f32>().unwrap();
                                    }

                                    helper.do_scene_command(SetExposureCommand::new(
                                        node_handle,
                                        current_auto_exposure,
                                    ))
                                }
                                Exposure::AUTO_MIN_LUMINANCE => {
                                    let mut current_auto_exposure = camera.exposure().clone();
                                    if let Exposure::Auto {
                                        ref mut min_luminance,
                                        ..
                                    } = current_auto_exposure
                                    {
                                        *min_luminance = *value.cast_value::<f32>().unwrap();
                                    }

                                    helper.do_scene_command(SetExposureCommand::new(
                                        node_handle,
                                        current_auto_exposure,
                                    ))
                                }
                                Exposure::AUTO_MAX_LUMINANCE => {
                                    let mut current_auto_exposure = camera.exposure().clone();
                                    if let Exposure::Auto {
                                        ref mut max_luminance,
                                        ..
                                    } = current_auto_exposure
                                    {
                                        *max_luminance = *value.cast_value::<f32>().unwrap();
                                    }

                                    helper.do_scene_command(SetExposureCommand::new(
                                        node_handle,
                                        current_auto_exposure,
                                    ))
                                }
                                _ => println!("Unhandled property of Camera: {:?}", args),
                            }
                        }
                    }
                    _ => {}
                }
            }
            // TODO: Confusing "double-match"
            _ => {
                if let FieldKind::Object(ref value) = args.value {
                    match args.name.as_ref() {
                        Camera::Z_NEAR => helper.do_scene_command(SetZNearCommand::new(
                            node_handle,
                            *value.cast_value().unwrap(),
                        )),
                        Camera::Z_FAR => helper.do_scene_command(SetZFarCommand::new(
                            node_handle,
                            *value.cast_value().unwrap(),
                        )),
                        Camera::FOV => helper.do_scene_command(SetFovCommand::new(
                            node_handle,
                            *value.cast_value().unwrap(),
                        )),
                        Camera::VIEWPORT => helper.do_scene_command(SetViewportCommand::new(
                            node_handle,
                            *value.cast_value().unwrap(),
                        )),
                        Camera::ENABLED => helper.do_scene_command(SetCameraPreviewCommand::new(
                            node_handle,
                            *value.cast_value().unwrap(),
                        )),
                        Camera::SKY_BOX => helper.do_scene_command(SetSkyBoxCommand::new(
                            node_handle,
                            value.cast_value::<Option<Box<SkyBox>>>().unwrap().clone(),
                        )),
                        Camera::ENVIRONMENT => helper.do_scene_command(SetEnvironmentMap::new(
                            node_handle,
                            value.cast_value::<Option<Texture>>().cloned().unwrap(),
                        )),
                        Camera::COLOR_GRADING_LUT => {
                            helper.do_scene_command(SetColorGradingLutCommand::new(
                                node_handle,
                                value
                                    .cast_value::<Option<ColorGradingLut>>()
                                    .unwrap()
                                    .clone(),
                            ))
                        }
                        Camera::COLOR_GRADING_ENABLED => {
                            helper.do_scene_command(SetColorGradingEnabledCommand::new(
                                node_handle,
                                *value.cast_value().unwrap(),
                            ))
                        }
                        _ => println!("Unhandled property of Camera: {:?}", args),
                    }
                }
            }
        }
    }
}
