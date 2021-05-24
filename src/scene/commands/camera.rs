use crate::scene::commands::Command;
use crate::scene::commands::SceneContext;
use crate::{define_node_command, get_set_swap};
use rg3d::core::pool::Handle;
use rg3d::scene::graph::Graph;
use rg3d::scene::node::Node;

define_node_command!(SetFovCommand("Set Fov", f32) where fn swap(self, node) {
    get_set_swap!(self, node.as_camera_mut(), fov, set_fov);
});

define_node_command!(SetZNearCommand("Set Camera Z Near", f32) where fn swap(self, node) {
    get_set_swap!(self, node.as_camera_mut(), z_near, set_z_near);
});

define_node_command!(SetZFarCommand("Set Camera Z Far", f32) where fn swap(self, node) {
    get_set_swap!(self, node.as_camera_mut(), z_far, set_z_far);
});
