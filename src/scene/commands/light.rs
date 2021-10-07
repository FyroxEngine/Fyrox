use crate::{
    define_node_command, get_set_swap,
    scene::commands::{Command, SceneContext},
};
use rg3d::{
    core::{algebra::Vector3, color::Color, pool::Handle},
    resource::texture::Texture,
    scene::{graph::Graph, node::Node},
};

define_node_command!(SetLightScatterCommand("Set Light Scatter", Vector3<f32>) where fn swap(self, node) {
    get_set_swap!(self, node.as_light_mut(), scatter, set_scatter)
});

define_node_command!(SetLightScatterEnabledCommand("Set Light Scatter Enabled", bool) where fn swap(self, node) {
    get_set_swap!(self, node.as_light_mut(), is_scatter_enabled, enable_scatter)
});

define_node_command!(SetLightCastShadowsCommand("Set Light Cast Shadows", bool) where fn swap(self, node) {
    get_set_swap!(self, node.as_light_mut(), is_cast_shadows, set_cast_shadows)
});

define_node_command!(SetLightIntensityCommand("Set Light Intensity", f32) where fn swap(self, node) {
    get_set_swap!(self, node.as_light_mut(), intensity, set_intensity)
});

define_node_command!(SetPointLightRadiusCommand("Set Point Light Radius", f32) where fn swap(self, node) {
    get_set_swap!(self, node.as_light_mut().as_point_mut(), radius, set_radius)
});

define_node_command!(SetPointLightShadowBiasCommand("Set Point Light Shadow Bias", f32) where fn swap(self, node) {
    get_set_swap!(self, node.as_light_mut().as_point_mut(), shadow_bias, set_shadow_bias)
});

define_node_command!(SetSpotLightHotspotCommand("Set Spot Light Hotspot", f32) where fn swap(self, node) {
    get_set_swap!(self, node.as_light_mut().as_spot_mut(), hotspot_cone_angle, set_hotspot_cone_angle)
});

define_node_command!(SetSpotLightFalloffAngleDeltaCommand("Set Spot Light Falloff Angle Delta", f32) where fn swap(self, node) {
    get_set_swap!(self, node.as_light_mut().as_spot_mut(), falloff_angle_delta, set_falloff_angle_delta)
});

define_node_command!(SetSpotLightShadowBiasCommand("Set Spot Light Shadow Bias", f32) where fn swap(self, node) {
    get_set_swap!(self, node.as_light_mut().as_spot_mut(), shadow_bias, set_shadow_bias)
});

define_node_command!(SetSpotLightDistanceCommand("Set Spot Light Distance", f32) where fn swap(self, node) {
    get_set_swap!(self, node.as_light_mut().as_spot_mut(), distance, set_distance);
});

define_node_command!(SetSpotLightCookieTextureCommand("Set Spot Light Cookie Texture", Option<Texture>) where fn swap(self, node) {
    get_set_swap!(self, node.as_light_mut().as_spot_mut(), cookie_texture, set_cookie_texture);
});

define_node_command!(SetLightColorCommand("Set Light Color", Color) where fn swap(self, node) {
    get_set_swap!(self, node.as_light_mut(), color, set_color)
});
