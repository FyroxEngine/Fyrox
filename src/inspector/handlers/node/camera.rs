use crate::{do_command, inspector::SenderHelper, scene::commands::camera::*};
use rg3d::{
    core::pool::Handle,
    gui::message::{FieldKind, PropertyChanged},
    scene::{
        camera::{Camera, Exposure},
        node::Node,
    },
};

pub fn handle_camera_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    node: &Node,
    helper: &SenderHelper,
) -> Option<()> {
    if let Node::Camera(camera) = node {
        match args.name.as_ref() {
            Camera::EXPOSURE => {
                match args.value {
                    // Exposure variant has changed.
                    FieldKind::Object(ref value) => helper.do_scene_command(
                        SetExposureCommand::new(handle, value.cast_value::<Exposure>()?.clone()),
                    ),
                    // Some inner property has changed
                    FieldKind::Inspectable(ref args) => {
                        if let FieldKind::Object(ref value) = args.value {
                            match args.name.as_ref() {
                                Exposure::AUTO_KEY_VALUE => {
                                    let mut current_auto_exposure = camera.exposure().clone();
                                    if let Exposure::Auto {
                                        ref mut key_value, ..
                                    } = current_auto_exposure
                                    {
                                        *key_value = *value.cast_value::<f32>()?;
                                    }

                                    helper.do_scene_command(SetExposureCommand::new(
                                        handle,
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
                                        *min_luminance = *value.cast_value::<f32>()?;
                                    }

                                    helper.do_scene_command(SetExposureCommand::new(
                                        handle,
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
                                        *max_luminance = *value.cast_value::<f32>()?;
                                    }

                                    helper.do_scene_command(SetExposureCommand::new(
                                        handle,
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
                        Camera::Z_NEAR => {
                            do_command!(helper, SetZNearCommand, handle, value)
                        }
                        Camera::Z_FAR => {
                            do_command!(helper, SetZFarCommand, handle, value)
                        }
                        Camera::FOV => {
                            do_command!(helper, SetFovCommand, handle, value)
                        }
                        Camera::VIEWPORT => {
                            do_command!(helper, SetViewportCommand, handle, value)
                        }
                        Camera::ENABLED => {
                            do_command!(helper, SetCameraPreviewCommand, handle, value)
                        }
                        Camera::SKY_BOX => {
                            do_command!(helper, SetSkyBoxCommand, handle, value)
                        }
                        Camera::ENVIRONMENT => {
                            do_command!(helper, SetEnvironmentMap, handle, value)
                        }
                        Camera::COLOR_GRADING_LUT => {
                            do_command!(helper, SetColorGradingLutCommand, handle, value)
                        }
                        Camera::COLOR_GRADING_ENABLED => {
                            do_command!(helper, SetColorGradingEnabledCommand, handle, value)
                        }
                        _ => println!("Unhandled property of Camera: {:?}", args),
                    }
                }
            }
        }
    }

    Some(())
}
