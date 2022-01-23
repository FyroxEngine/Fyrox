use crate::{
    define_node_command, define_swap_command,
    scene::commands::{Command, SceneContext},
};
use fyrox::scene::camera::Projection;
use fyrox::{
    core::{math::Rect, pool::Handle},
    resource::texture::Texture,
    scene::{
        camera::{Camera, ColorGradingLut, Exposure, SkyBox},
        graph::Graph,
        node::Node,
    },
};

fn node_as_camera_mut(node: &mut Node) -> &mut Camera {
    node.as_camera_mut()
}

define_swap_command! {
    node_as_camera_mut,
    SetViewportCommand(Rect<f32>): viewport, set_viewport, "Set Viewport";
    SetEnvironmentMap(Option<Texture>): environment_map, set_environment, "Set Camera Environment Map";
    SetProjectionCommand(Projection): projection_value, set_projection, "Set Camera Projection";
    SetExposureCommand(Exposure): exposure, set_exposure, "Set Camera Exposure";
    SetColorGradingLutCommand(Option<ColorGradingLut>): color_grading_lut, set_color_grading_map, "Set Color Grading Lut";
    SetColorGradingEnabledCommand(bool): color_grading_enabled, set_color_grading_enabled, "Set Color Grading Enabled";
}

define_node_command! {
    SetSkyBoxCommand("Set Sky Box Command", Option<Box<SkyBox>>) where fn swap(self, node) {
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

#[derive(Debug)]
pub struct SetCameraPreviewCommand {
    handle: Handle<Node>,
    value: bool,
    old_value: bool,
    prev_active: Vec<Handle<Node>>,
}

impl SetCameraPreviewCommand {
    pub fn new(node: Handle<Node>, value: bool) -> Self {
        Self {
            handle: node,
            value,
            old_value: false,
            prev_active: Vec::new(),
        }
    }
}

impl Command for SetCameraPreviewCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Set camera preview".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        let camera = context.scene.graph[self.handle].as_camera_mut();
        self.old_value = camera.is_enabled();
        camera.set_enabled(self.value);

        let editor_camera_handle = context.editor_scene.camera_controller.camera;
        let editor_camera = context.scene.graph[editor_camera_handle].as_camera_mut();
        editor_camera.set_enabled(!self.value);

        // disable other cameras and save their handles to be able to revert
        if self.value {
            self.prev_active = context
                .scene
                .graph
                .pair_iter_mut()
                .filter(|(handle, _)| handle != &self.handle && handle != &editor_camera_handle)
                .filter_map(|(handle, node)| {
                    if let Node::Camera(cam) = node {
                        if cam.is_enabled() {
                            cam.set_enabled(false);
                            Some(handle)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();
        }
    }

    fn revert(&mut self, context: &mut SceneContext) {
        for handle in self.prev_active.iter_mut() {
            let camera = context.scene.graph[*handle].as_camera_mut();
            camera.set_enabled(true);
        }

        let camera = context.scene.graph[self.handle].as_camera_mut();
        camera.set_enabled(self.old_value);

        let editor_camera_handle = context.editor_scene.camera_controller.camera;
        let editor_camera = context.scene.graph[editor_camera_handle].as_camera_mut();
        let editor_camera_enabled = !self.old_value && self.prev_active.is_empty();
        editor_camera.set_enabled(editor_camera_enabled);
    }
}
