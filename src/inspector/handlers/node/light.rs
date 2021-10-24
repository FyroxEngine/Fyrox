use crate::inspector::handlers::node::base::handle_base_property_changed;
use crate::{do_command, inspector::SenderHelper, scene::commands::light::*};
use rg3d::scene::light::directional::DirectionalLight;
use rg3d::{
    core::pool::Handle,
    gui::message::{FieldKind, PropertyChanged},
    scene::{
        light::{point::PointLight, spot::SpotLight, BaseLight},
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
            BaseLight::BASE => handle_base_property_changed(&inner, handle, node, helper),
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
            _ => None,
        },
        _ => None,
    }
}
