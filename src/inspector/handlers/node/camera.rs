use crate::inspector::handlers::node::base::handle_base_property_changed;
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
        match args.value {
            FieldKind::Object(ref value) => match args.name.as_ref() {
                Camera::EXPOSURE => helper.do_scene_command(SetExposureCommand::new(
                    handle,
                    value.cast_value::<Exposure>()?.clone(),
                )),
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
                _ => None,
            },
            FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
                Camera::EXPOSURE => {
                    if let FieldKind::Object(ref value) = inner.value {
                        match inner.name.as_ref() {
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
                            Exposure::MANUAL_F_0 => {
                                helper.do_scene_command(SetExposureCommand::new(
                                    handle,
                                    Exposure::Manual(value.cast_value::<f32>().cloned()?),
                                ))
                            }
                            _ => None,
                        }
                    } else {
                        None
                    }
                }
                Camera::BASE => handle_base_property_changed(inner, handle, node, helper),
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}
