use crate::{
    animation::{track::Track, Animation, NodeTrack},
    core::{pool::Handle, visitor::prelude::*},
    scene::{base::InstanceId, node::Node, Scene},
};

pub type ResourceTrack = Track<InstanceId>;

#[derive(Visit, Default, Debug)]
pub struct AnimationDefinition {
    tracks: Vec<ResourceTrack>,
}

impl AnimationDefinition {
    pub fn instantiate(&self, root: Handle<Node>, scene: &mut Scene) -> Handle<Animation> {
        let mut animation = Animation::default();

        for track in self.tracks.iter() {
            let mut node_track = NodeTrack::new(track.frames_container().clone());

            node_track.set_binding(track.binding().clone());
            node_track.set_target(
                scene
                    .graph
                    .find(root, &mut |n| n.instance_id() == track.target()),
            );

            animation.add_track(node_track);
        }

        scene.animations.add(animation)
    }
}
