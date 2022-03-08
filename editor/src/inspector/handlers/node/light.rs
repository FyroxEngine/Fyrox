use crate::{
    handle_properties, inspector::handlers::node::base::handle_base_property_changed,
    scene::commands::light::*, SceneCommand,
};
use fyrox::{
    core::pool::Handle,
    gui::inspector::{CollectionChanged, FieldKind, PropertyChanged},
    scene::{
        light::{
            directional::{CsmOptions, DirectionalLight, FrustumSplitOptions},
            point::PointLight,
            spot::SpotLight,
            BaseLight,
        },
        node::Node,
    },
};

pub fn handle_base_light_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    node: &mut Node,
) -> Option<SceneCommand> {
    match args.value {
        FieldKind::Object(ref value) => {
            handle_properties!(args.name.as_ref(), handle, value,
                BaseLight::COLOR => SetLightColorCommand,
                BaseLight::CAST_SHADOWS => SetLightCastShadowsCommand,
                BaseLight::SCATTER => SetLightScatterCommand,
                BaseLight::SCATTER_ENABLED => SetLightScatterEnabledCommand,
                BaseLight::INTENSITY => SetLightIntensityCommand
            )
        }
        FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
            BaseLight::BASE => handle_base_property_changed(inner, handle, node),
            _ => None,
        },
        _ => None,
    }
}

pub fn handle_spot_light_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    node: &mut Node,
) -> Option<SceneCommand> {
    if node.is_spot_light() {
        match args.value {
            FieldKind::Object(ref value) => {
                handle_properties!(args.name.as_ref(), handle, value,
                    SpotLight::HOTSPOT_CONE_ANGLE => SetSpotLightHotspotCommand,
                    SpotLight::FALLOFF_ANGLE_DELTA => SetSpotLightFalloffAngleDeltaCommand,
                    SpotLight::SHADOW_BIAS => SetSpotLightShadowBiasCommand,
                    SpotLight::DISTANCE => SetSpotLightDistanceCommand,
                    SpotLight::COOKIE_TEXTURE => SetSpotLightCookieTextureCommand
                )
            }
            FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
                SpotLight::BASE_LIGHT => handle_base_light_property_changed(inner, handle, node),
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}

pub fn handle_point_light_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    node: &mut Node,
) -> Option<SceneCommand> {
    if node.is_point_light() {
        match args.value {
            FieldKind::Object(ref value) => {
                handle_properties!(args.name.as_ref(), handle, value,
                    PointLight::SHADOW_BIAS => SetPointLightShadowBiasCommand,
                    PointLight::RADIUS => SetPointLightRadiusCommand
                )
            }
            FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
                PointLight::BASE_LIGHT => handle_base_light_property_changed(inner, handle, node),
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}

pub fn handle_directional_light_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    node: &mut Node,
) -> Option<SceneCommand> {
    if node.is_directional_light() {
        match args.value {
            FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
                DirectionalLight::BASE_LIGHT => {
                    handle_base_light_property_changed(inner, handle, node)
                }
                DirectionalLight::CSM_OPTIONS => match inner.name.as_ref() {
                    CsmOptions::SPLIT_OPTIONS => match inner.value {
                        FieldKind::Inspectable(ref split_options_value) => {
                            if let FieldKind::Collection(ref collection_changed) =
                                split_options_value.value
                            {
                                if let CollectionChanged::ItemChanged { .. } = **collection_changed
                                {
                                    match split_options_value.name.as_ref() {
                                        FrustumSplitOptions::ABSOLUTE_FAR_PLANES => None,
                                        FrustumSplitOptions::RELATIVE_FRACTIONS => None,
                                        _ => None,
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        }
                        _ => None,
                    },
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}
