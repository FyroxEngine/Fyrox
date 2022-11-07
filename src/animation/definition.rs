use crate::{
    animation::{track::Track, Animation, AnimationContainer, NodeTrack},
    core::{pool::Handle, reflect::prelude::*, visitor::prelude::*},
    scene::{base::InstanceId, graph::Graph, node::Node},
};

pub type ResourceTrack = Track<InstanceId>;

#[derive(Visit, Reflect, Default, Debug)]
pub struct AnimationDefinition {
    tracks: Vec<ResourceTrack>,
}

impl AnimationDefinition {
    pub fn instantiate(
        &self,
        root: Handle<Node>,
        graph: &Graph,
        animations: &mut AnimationContainer,
    ) -> Handle<Animation> {
        let mut animation = Animation::default();

        animation.set_root(root);

        for track in self.tracks.iter() {
            let mut node_track =
                NodeTrack::new(track.frames_container().clone(), track.binding().clone());

            node_track.set_target(graph.find(root, &mut |n| n.instance_id() == track.target()));

            animation.add_track(node_track);
        }

        animations.add(animation)
    }

    pub fn tracks(&self) -> &[ResourceTrack] {
        &self.tracks
    }

    pub fn tracks_container(&mut self) -> &mut Vec<ResourceTrack> {
        &mut self.tracks
    }
}
