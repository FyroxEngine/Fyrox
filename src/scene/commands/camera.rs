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

impl<'a> Command<'a> for SetCameraPreviewCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Set camera preview".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
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

    fn revert(&mut self, context: &mut Self::Context) {
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
