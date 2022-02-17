use crate::{
    define_swap_command,
    scene::commands::{Command, SceneContext},
};
use fyrox::{
    core::{algebra::Vector3, color::Color},
    resource::texture::Texture,
    scene::{
        light::{point::PointLight, spot::SpotLight, BaseLight},
        node::Node,
    },
};

fn node_base_light_mut(node: &mut Node) -> &mut BaseLight {
    node.query_component_mut::<BaseLight>().unwrap()
}

define_swap_command! {
    node_base_light_mut,
    SetLightScatterCommand(Vector3<f32>): scatter, set_scatter, "Set Light Scatter";
    SetLightScatterEnabledCommand(bool): is_scatter_enabled, enable_scatter, "Set Light Scatter Enabled";
    SetLightIntensityCommand(f32): intensity, set_intensity, "Set Light Intensity";
    SetLightCastShadowsCommand(bool): is_cast_shadows, set_cast_shadows, "Set Light Cast Shadows";
    SetLightColorCommand(Color): color, set_color, "Set Light Color";
}

fn node_as_spot_mut(node: &mut Node) -> &mut SpotLight {
    node.as_spot_light_mut()
}

define_swap_command! {
    node_as_spot_mut,
    SetSpotLightHotspotCommand(f32): hotspot_cone_angle, set_hotspot_cone_angle, "Set Spot Light Hotswap";
    SetSpotLightFalloffAngleDeltaCommand(f32): falloff_angle_delta, set_falloff_angle_delta, "Set Spot Light Falloff Angle Delta";
    SetSpotLightShadowBiasCommand(f32): shadow_bias, set_shadow_bias, "Set Spot Light Shadow Bias";
    SetSpotLightDistanceCommand(f32): distance, set_distance, "Set Spot Light Distance";
    SetSpotLightCookieTextureCommand(Option<Texture>): cookie_texture, set_cookie_texture, "Set Spot Light Cookie Texture";
}

fn node_as_point_light_mut(node: &mut Node) -> &mut PointLight {
    node.as_point_light_mut()
}

define_swap_command! {
    node_as_point_light_mut,
    SetPointLightShadowBiasCommand(f32): shadow_bias, set_shadow_bias, "Set Point Light Shadow Bias";
    SetPointLightRadiusCommand(f32): radius, set_radius, "Set Point Light Radius";
}
