use crate::{
    define_swap_command,
    scene::commands::{Command, SceneContext},
};
use fyrox::{
    core::{algebra::Vector3, color::Color},
    resource::texture::Texture,
    scene::{
        light::{point::PointLight, spot::SpotLight},
        node::Node,
    },
};

define_swap_command! {
    Node::as_light_mut,
    SetLightScatterCommand(Vector3<f32>): scatter, set_scatter, "Set Light Scatter";
    SetLightScatterEnabledCommand(bool): is_scatter_enabled, enable_scatter, "Set Light Scatter Enabled";
    SetLightIntensityCommand(f32): intensity, set_intensity, "Set Light Intensity";
    SetLightCastShadowsCommand(bool): is_cast_shadows, set_cast_shadows, "Set Light Cast Shadows";
    SetLightColorCommand(Color): color, set_color, "Set Light Color";
}

fn node_as_spot_mut(node: &mut Node) -> &mut SpotLight {
    node.as_light_mut().as_spot_mut()
}

define_swap_command! {
    node_as_spot_mut,
    SetSpotLightHotspotCommand(Vector3<f32>): scatter, set_scatter, "Set Spot Light Hotswap";
    SetPointLightShadowBiasCommand(f32): shadow_bias, set_shadow_bias, "Set Point Light Shadow Bias";
    SetSpotLightFalloffAngleDeltaCommand(f32): falloff_angle_delta, set_falloff_angle_delta, "Set Spot Light Falloff Angle Delta";
    SetSpotLightShadowBiasCommand(f32): shadow_bias, set_shadow_bias, "Set Spot Light Shadow Bias";
    SetSpotLightDistanceCommand(f32): distance, set_distance, "Set Spot Light Distance";
    SetSpotLightCookieTextureCommand(Option<Texture>): cookie_texture, set_cookie_texture, "Set Spot Light Cookie Texture";
}

fn node_as_point_light_mut(node: &mut Node) -> &mut PointLight {
    node.as_light_mut().as_point_mut()
}

define_swap_command! {
    node_as_point_light_mut,
    SetPointLightRadiusCommand(f32): radius, set_radius, "Set Point Light Radius";
}
