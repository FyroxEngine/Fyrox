use crate::{
    absm::command::fetch_machine, command::Command, define_push_element_to_collection_command,
    define_set_collection_element_command, scene::commands::SceneContext,
};
use fyrox::animation::machine::node::blendspace::BlendSpacePoint;
use fyrox::core::algebra::Vector2;
use fyrox::{
    animation::machine::node::{
        blend::{BlendPose, IndexedBlendInput},
        PoseNode,
    },
    core::pool::Handle,
    scene::node::Node,
};

define_push_element_to_collection_command!(AddInputCommand<Handle<PoseNode>, IndexedBlendInput>(self, context) {
    let machine = fetch_machine(context, self.node_handle);
    match &mut machine.layers_mut()[self.layer_index].nodes_mut()[self.handle] {
        PoseNode::BlendAnimationsByIndex(definition) => &mut definition.inputs,
        _ => unreachable!(),
    }
});

define_push_element_to_collection_command!(AddPoseSourceCommand<Handle<PoseNode>, BlendPose>(self, context) {
    let machine = fetch_machine(context, self.node_handle);
    match &mut machine.layers_mut()[self.layer_index].nodes_mut()[self.handle] {
        PoseNode::BlendAnimations(definition) => &mut definition.pose_sources,
        _ => unreachable!(),
    }
});

define_push_element_to_collection_command!(AddBlendSpacePointCommand<Handle<PoseNode>, BlendSpacePoint>(self, context) {
    let machine = fetch_machine(context, self.node_handle);
    match &mut machine.layers_mut()[self.layer_index].nodes_mut()[self.handle] {
        PoseNode::BlendSpace(definition) => definition.points_mut(),
        _ => unreachable!(),
    }
});

define_set_collection_element_command!(
    SetBlendAnimationByIndexInputPoseSourceCommand<Handle<PoseNode>, Handle<PoseNode>>(self, context) {
        let machine = fetch_machine(context, self.node_handle);
        if let PoseNode::BlendAnimationsByIndex(ref mut definition) = machine.layers_mut()[self.layer_index].nodes_mut()[self.handle] {
            std::mem::swap(&mut definition.inputs[self.index].pose_source, &mut self.value)
        }
    }
);

define_set_collection_element_command!(
    SetBlendAnimationsPoseSourceCommand<Handle<PoseNode>, Handle<PoseNode>>(self, context) {
        let machine = fetch_machine(context, self.node_handle);
        if let PoseNode::BlendAnimations(ref mut definition) = machine.layers_mut()[self.layer_index].nodes_mut()[self.handle] {
            std::mem::swap(&mut definition.pose_sources[self.index].pose_source, &mut self.value);
        }
    }
);

define_set_collection_element_command!(
    SetBlendSpacePoseSourceCommand<Handle<PoseNode>, Handle<PoseNode>>(self, context) {
        let machine = fetch_machine(context, self.node_handle);
        if let PoseNode::BlendSpace(ref mut definition) = machine.layers_mut()[self.layer_index].nodes_mut()[self.handle] {
            std::mem::swap( &mut definition.points_mut()[self.index].pose_source, &mut self.value);
        }
    }
);

define_set_collection_element_command!(
    SetBlendSpacePointPositionCommand<Handle<PoseNode>, Vector2<f32>>(self, context) {
        let machine = fetch_machine(context, self.node_handle);
        if let PoseNode::BlendSpace(ref mut definition) = machine.layers_mut()[self.layer_index].nodes_mut()[self.handle] {
            std::mem::swap( &mut definition.points_mut()[self.index].position, &mut self.value);
            definition.try_snap_points();
        }
    }
);
