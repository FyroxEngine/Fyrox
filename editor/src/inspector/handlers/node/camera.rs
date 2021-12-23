use crate::{
    inspector::handlers::node::base::handle_base_property_changed, make_command,
    scene::commands::camera::*, SceneCommand,
};
use rg3d::scene::camera::{OrthographicProjection, PerspectiveProjection, Projection};
use rg3d::{
    core::{futures::executor::block_on, pool::Handle},
    gui::inspector::{FieldKind, PropertyChanged},
    resource::texture::{Texture, TextureWrapMode},
    scene::{
        camera::{Camera, ColorGradingLut, Exposure, SkyBox, SkyBoxBuilder},
        node::Node,
    },
};
use std::any::TypeId;

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
enum SkyBoxFace {
    Left,
    Right,
    Top,
    Bottom,
    Front,
    Back,
}

fn modify_skybox(
    camera: &Camera,
    texture: Option<Texture>,
    face: SkyBoxFace,
) -> Option<Box<SkyBox>> {
    if let Some(skybox) = camera.skybox_ref() {
        if let Some(texture) = texture.clone() {
            block_on(texture).ok()?;
        }

        let skybox = SkyBoxBuilder {
            front: if face == SkyBoxFace::Front {
                texture.clone()
            } else {
                skybox.front()
            },
            back: if face == SkyBoxFace::Back {
                texture.clone()
            } else {
                skybox.back()
            },
            left: if face == SkyBoxFace::Left {
                texture.clone()
            } else {
                skybox.left()
            },
            right: if face == SkyBoxFace::Right {
                texture.clone()
            } else {
                skybox.right()
            },
            top: if face == SkyBoxFace::Top {
                texture.clone()
            } else {
                skybox.top()
            },
            bottom: if face == SkyBoxFace::Bottom {
                texture
            } else {
                skybox.bottom()
            },
        }
        .build()
        .unwrap();

        // Set S and T coordinate wrap mode, ClampToEdge will remove any possible seams on edges
        // of the skybox.
        if let Some(cubemap) = skybox.cubemap() {
            let mut data = cubemap.data_ref();
            data.set_s_wrap_mode(TextureWrapMode::ClampToEdge);
            data.set_t_wrap_mode(TextureWrapMode::ClampToEdge);
        }

        Some(Box::new(skybox))
    } else {
        None
    }
}

pub fn handle_camera_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    node: &Node,
) -> Option<SceneCommand> {
    if let Node::Camera(camera) = node {
        match args.value {
            FieldKind::Object(ref value) => match args.name.as_ref() {
                Camera::EXPOSURE => Some(SceneCommand::new(SetExposureCommand::new(
                    handle,
                    *value.cast_value::<Exposure>()?,
                ))),
                Camera::PROJECTION => {
                    make_command!(SetProjectionCommand, handle, value)
                }
                Camera::VIEWPORT => {
                    make_command!(SetViewportCommand, handle, value)
                }
                Camera::ENABLED => {
                    make_command!(SetCameraPreviewCommand, handle, value)
                }
                Camera::SKY_BOX => {
                    make_command!(SetSkyBoxCommand, handle, value)
                }
                Camera::ENVIRONMENT => {
                    make_command!(SetEnvironmentMap, handle, value)
                }
                Camera::COLOR_GRADING_LUT => {
                    make_command!(SetColorGradingLutCommand, handle, value)
                }
                Camera::COLOR_GRADING_ENABLED => {
                    make_command!(SetColorGradingEnabledCommand, handle, value)
                }
                _ => None,
            },
            FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
                Camera::EXPOSURE => {
                    if let FieldKind::Object(ref value) = inner.value {
                        match inner.name.as_ref() {
                            Exposure::AUTO_KEY_VALUE => {
                                let mut current_auto_exposure = camera.exposure();
                                if let Exposure::Auto {
                                    ref mut key_value, ..
                                } = current_auto_exposure
                                {
                                    *key_value = *value.cast_value::<f32>()?;
                                }

                                Some(SceneCommand::new(SetExposureCommand::new(
                                    handle,
                                    current_auto_exposure,
                                )))
                            }
                            Exposure::AUTO_MIN_LUMINANCE => {
                                let mut current_auto_exposure = camera.exposure();
                                if let Exposure::Auto {
                                    ref mut min_luminance,
                                    ..
                                } = current_auto_exposure
                                {
                                    *min_luminance = *value.cast_value::<f32>()?;
                                }

                                Some(SceneCommand::new(SetExposureCommand::new(
                                    handle,
                                    current_auto_exposure,
                                )))
                            }
                            Exposure::AUTO_MAX_LUMINANCE => {
                                let mut current_auto_exposure = camera.exposure();
                                if let Exposure::Auto {
                                    ref mut max_luminance,
                                    ..
                                } = current_auto_exposure
                                {
                                    *max_luminance = *value.cast_value::<f32>()?;
                                }

                                Some(SceneCommand::new(SetExposureCommand::new(
                                    handle,
                                    current_auto_exposure,
                                )))
                            }
                            Exposure::MANUAL_F_0 => {
                                Some(SceneCommand::new(SetExposureCommand::new(
                                    handle,
                                    Exposure::Manual(value.cast_value::<f32>().cloned()?),
                                )))
                            }
                            _ => None,
                        }
                    } else {
                        None
                    }
                }
                Camera::COLOR_GRADING_LUT => {
                    if let FieldKind::Object(ref value) = inner.value {
                        match inner.name.as_ref() {
                            ColorGradingLut::LUT => {
                                if let Some(texture) =
                                    value.cast_value::<Option<Texture>>().cloned()?
                                {
                                    if let Ok(lut) = block_on(ColorGradingLut::new(texture)) {
                                        Some(SceneCommand::new(SetColorGradingLutCommand::new(
                                            handle,
                                            Some(lut),
                                        )))
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        }
                    } else {
                        None
                    }
                }
                Camera::SKY_BOX => {
                    if let FieldKind::Object(ref value) = inner.value {
                        let texture = value.cast_value::<Option<Texture>>().cloned()?;
                        let face = match inner.name.as_ref() {
                            SkyBox::FRONT => Some(SkyBoxFace::Front),
                            SkyBox::BACK => Some(SkyBoxFace::Back),
                            SkyBox::LEFT => Some(SkyBoxFace::Left),
                            SkyBox::RIGHT => Some(SkyBoxFace::Right),
                            SkyBox::BOTTOM => Some(SkyBoxFace::Bottom),
                            SkyBox::TOP => Some(SkyBoxFace::Top),
                            _ => None,
                        };

                        face.map(|face| {
                            SceneCommand::new(SetSkyBoxCommand::new(
                                handle,
                                modify_skybox(camera, texture, face),
                            ))
                        })
                    } else {
                        None
                    }
                }
                Camera::PROJECTION => {
                    if let FieldKind::Inspectable(ref value) = inner.value {
                        if value.owner_type_id == TypeId::of::<PerspectiveProjection>() {
                            handle_perspective_property_changed(&**value, handle)
                        } else if value.owner_type_id == TypeId::of::<OrthographicProjection>() {
                            handle_ortho_property_changed(&**value, handle)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                Camera::BASE => handle_base_property_changed(inner, handle, node),
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}

pub fn handle_perspective_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
) -> Option<SceneCommand> {
    if let FieldKind::Object(ref value) = args.value {
        match args.name.as_ref() {
            PerspectiveProjection::Z_NEAR => {
                make_command!(SetPerspectiveZNear, handle, value)
            }
            PerspectiveProjection::Z_FAR => {
                make_command!(SetPerspectiveZFar, handle, value)
            }
            PerspectiveProjection::FOV => {
                make_command!(SetPerspectiveFov, handle, value)
            }
            _ => None,
        }
    } else {
        None
    }
}

pub fn handle_ortho_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
) -> Option<SceneCommand> {
    if let FieldKind::Object(ref value) = args.value {
        match args.name.as_ref() {
            OrthographicProjection::Z_NEAR => {
                make_command!(SetOrthoZNear, handle, value)
            }
            OrthographicProjection::Z_FAR => {
                make_command!(SetOrthoZFar, handle, value)
            }
            OrthographicProjection::VERTICAL_SIZE => {
                make_command!(SetOrthoVerticalSize, handle, value)
            }
            _ => None,
        }
    } else {
        None
    }
}
