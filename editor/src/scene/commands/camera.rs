use crate::{
    define_node_command, define_swap_command,
    scene::commands::{Command, SceneContext},
};
use fyrox::{
    core::{math::Rect, pool::Handle},
    resource::texture::Texture,
    scene::{
        camera::{Camera, ColorGradingLut, Exposure, Projection, SkyBox},
        graph::Graph,
        node::Node,
    },
};

fn node_as_camera_mut(node: &mut Node) -> &mut Camera {
    node.as_camera_mut()
}

define_swap_command! {
    node_as_camera_mut,
    SetCameraEnabled(bool): is_enabled, set_enabled, "Set Enabled";
    SetViewportCommand(Rect<f32>): viewport, set_viewport, "Set Viewport";
    SetEnvironmentMap(Option<Texture>): environment_map, set_environment, "Set Camera Environment Map";
    SetProjectionCommand(Projection): projection_value, set_projection, "Set Camera Projection";
    SetExposureCommand(Exposure): exposure, set_exposure, "Set Camera Exposure";
    SetColorGradingLutCommand(Option<ColorGradingLut>): color_grading_lut, set_color_grading_map, "Set Color Grading Lut";
    SetColorGradingEnabledCommand(bool): color_grading_enabled, set_color_grading_enabled, "Set Color Grading Enabled";
}

define_node_command! {
    SetSkyBoxCommand("Set Sky Box Command", Option<SkyBox>) where fn swap(self, node) {
        let camera = node.as_camera_mut();
        let temp = camera.replace_skybox(self.value.take());
        self.value = temp;
    }

    SetPerspectiveZNear("Set Z Near", f32) where fn swap(self, node) {
        if let Projection::Perspective(proj) = node.as_camera_mut().projection_mut() {
            std::mem::swap(&mut proj.z_near, &mut self.value)
        } else {
            unreachable!()
        }
    }

    SetPerspectiveZFar("Set Z Far", f32) where fn swap(self, node) {
        if let Projection::Perspective(proj) = node.as_camera_mut().projection_mut() {
            std::mem::swap(&mut proj.z_far, &mut self.value)
        } else {
            unreachable!()
        }
    }

    SetPerspectiveFov("Set Fov", f32) where fn swap(self, node) {
        if let Projection::Perspective(proj) = node.as_camera_mut().projection_mut() {
            std::mem::swap(&mut proj.fov, &mut self.value)
        } else {
            unreachable!()
        }
    }

    SetOrthoZNear("Set Z Near", f32) where fn swap(self, node) {
        if let Projection::Orthographic(proj) = node.as_camera_mut().projection_mut() {
            std::mem::swap(&mut proj.z_near, &mut self.value)
        } else {
            unreachable!()
        }
    }

    SetOrthoZFar("Set Z Far", f32) where fn swap(self, node) {
        if let Projection::Orthographic(proj) = node.as_camera_mut().projection_mut() {
            std::mem::swap(&mut proj.z_far, &mut self.value)
        } else {
            unreachable!()
        }
    }

    SetOrthoVerticalSize("Set Vertical Size", f32) where fn swap(self, node) {
        if let Projection::Orthographic(proj) = node.as_camera_mut().projection_mut() {
            std::mem::swap(&mut proj.vertical_size, &mut self.value)
        } else {
            unreachable!()
        }
    }
}
