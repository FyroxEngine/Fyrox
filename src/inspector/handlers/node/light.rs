use crate::{
    do_command,
    inspector::{handlers::node::base::handle_base_property_changed, SenderHelper},
    scene::commands::light::*,
};
use rg3d::{
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
    node: &Node,
    helper: &SenderHelper,
) -> Option<()> {
    match args.value {
        FieldKind::Object(ref value) => match args.name.as_ref() {
            BaseLight::COLOR => {
                do_command!(helper, SetLightColorCommand, handle, value)
            }
            BaseLight::CAST_SHADOWS => {
                do_command!(helper, SetLightCastShadowsCommand, handle, value)
            }
            BaseLight::SCATTER => {
                do_command!(helper, SetLightScatterCommand, handle, value)
            }
            BaseLight::SCATTER_ENABLED => {
                do_command!(helper, SetLightScatterEnabledCommand, handle, value)
            }
            BaseLight::INTENSITY => {
                do_command!(helper, SetLightIntensityCommand, handle, value)
            }
            _ => None,
        },
        FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
            BaseLight::BASE => handle_base_property_changed(inner, handle, node, helper),
            _ => None,
        },
        _ => None,
    }
}

pub fn handle_spot_light_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    node: &Node,
    helper: &SenderHelper,
) -> Option<()> {
    match args.value {
        FieldKind::Object(ref value) => match args.name.as_ref() {
            SpotLight::HOTSPOT_CONE_ANGLE => {
                do_command!(helper, SetSpotLightHotspotCommand, handle, value)
            }
            SpotLight::FALLOFF_ANGLE_DELTA => {
                do_command!(helper, SetSpotLightFalloffAngleDeltaCommand, handle, value)
            }
            SpotLight::SHADOW_BIAS => {
                do_command!(helper, SetSpotLightShadowBiasCommand, handle, value)
            }
            SpotLight::DISTANCE => {
                do_command!(helper, SetSpotLightDistanceCommand, handle, value)
            }
            SpotLight::COOKIE_TEXTURE => {
                do_command!(helper, SetSpotLightCookieTextureCommand, handle, value)
            }
            _ => None,
        },
        FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
            SpotLight::BASE_LIGHT => {
                handle_base_light_property_changed(inner, handle, node, helper)
            }
            _ => None,
        },
        _ => None,
    }
}

pub fn handle_point_light_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    node: &Node,
    helper: &SenderHelper,
) -> Option<()> {
    match args.value {
        FieldKind::Object(ref value) => match args.name.as_ref() {
            PointLight::SHADOW_BIAS => {
                do_command!(helper, SetPointLightShadowBiasCommand, handle, value)
            }
            PointLight::RADIUS => {
                do_command!(helper, SetPointLightRadiusCommand, handle, value)
            }
            _ => None,
        },
        FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
            PointLight::BASE_LIGHT => {
                handle_base_light_property_changed(inner, handle, node, helper)
            }
            _ => None,
        },
        _ => None,
    }
}

pub fn handle_directional_light_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    node: &Node,
    helper: &SenderHelper,
) -> Option<()> {
    match args.value {
        FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
            DirectionalLight::BASE_LIGHT => {
                handle_base_light_property_changed(inner, handle, node, helper)
            }
            DirectionalLight::CSM_OPTIONS => match inner.name.as_ref() {
                CsmOptions::SPLIT_OPTIONS => match inner.value {
                    FieldKind::Inspectable(ref split_options_value) => {
                        if let FieldKind::Collection(ref collection_changed) =
                            split_options_value.value
                        {
                            if let CollectionChanged::ItemChanged { .. } = **collection_changed {
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
}
