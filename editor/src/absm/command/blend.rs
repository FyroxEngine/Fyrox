use crate::{
    absm::command::fetch_machine, command::GameSceneCommandTrait,
    define_push_element_to_collection_command, define_set_collection_element_command,
    scene::commands::GameSceneContext,
};
use fyrox::{
    animation::machine::node::{
        blend::{BlendPose, IndexedBlendInput},
        blendspace::BlendSpacePoint,
        PoseNode,
    },
    core::{algebra::Vector2, pool::Handle},
    scene::node::Node,
};

define_push_element_to_collection_command!(AddInputCommand<Handle<PoseNode<Handle<Node>>>, IndexedBlendInput<Handle<Node>>>(self, context) {
    let machine = fetch_machine(context, self.node_handle);
    match &mut machine.layers_mut()[self.layer_index].nodes_mut()[self.handle] {
        PoseNode::BlendAnimationsByIndex(definition) => &mut definition.inputs,
        _ => unreachable!(),
    }
});

define_push_element_to_collection_command!(AddPoseSourceCommand<Handle<PoseNode<Handle<Node>>>, BlendPose<Handle<Node>>>(self, context) {
    let machine = fetch_machine(context, self.node_handle);
    match &mut machine.layers_mut()[self.layer_index].nodes_mut()[self.handle] {
        PoseNode::BlendAnimations(definition) => &mut definition.pose_sources,
        _ => unreachable!(),
    }
});

define_push_element_to_collection_command!(AddBlendSpacePointCommand<Handle<PoseNode<Handle<Node>>>, BlendSpacePoint<Handle<Node>>>(self, context) {
    let machine = fetch_machine(context, self.node_handle);
    match &mut machine.layers_mut()[self.layer_index].nodes_mut()[self.handle] {
        PoseNode::BlendSpace(definition) => definition.points_mut(),
        _ => unreachable!(),
    }
});

define_set_collection_element_command!(
    SetBlendAnimationByIndexInputPoseSourceCommand<Handle<PoseNode<Handle<Node>>>, Handle<PoseNode<Handle<Node>>>>(self, context) {
        let machine = fetch_machine(context, self.node_handle);
        if let PoseNode::BlendAnimationsByIndex(ref mut definition) = machine.layers_mut()[self.layer_index].nodes_mut()[self.handle] {
            std::mem::swap(&mut definition.inputs[self.index].pose_source, &mut self.value)
        }
    }
);

define_set_collection_element_command!(
    SetBlendAnimationsPoseSourceCommand<Handle<PoseNode<Handle<Node>>>, Handle<PoseNode<Handle<Node>>>>(self, context) {
        let machine = fetch_machine(context, self.node_handle);
        if let PoseNode::BlendAnimations(ref mut definition) = machine.layers_mut()[self.layer_index].nodes_mut()[self.handle] {
            std::mem::swap(&mut definition.pose_sources[self.index].pose_source, &mut self.value);
        }
    }
);

define_set_collection_element_command!(
    SetBlendSpacePoseSourceCommand<Handle<PoseNode<Handle<Node>>>, Handle<PoseNode<Handle<Node>>>>(self, context) {
        let machine = fetch_machine(context, self.node_handle);
        if let PoseNode::BlendSpace(ref mut definition) = machine.layers_mut()[self.layer_index].nodes_mut()[self.handle] {
            std::mem::swap( &mut definition.points_mut()[self.index].pose_source, &mut self.value);
        }
    }
);

define_set_collection_element_command!(
    SetBlendSpacePointPositionCommand<Handle<PoseNode<Handle<Node>>>, Vector2<f32>>(self, context) {
        let machine = fetch_machine(context, self.node_handle);
        if let PoseNode::BlendSpace(ref mut definition) = machine.layers_mut()[self.layer_index].nodes_mut()[self.handle] {
            std::mem::swap( &mut definition.points_mut()[self.index].position, &mut self.value);
            definition.try_snap_points();
        }
    }
);

#[derive(Debug)]
pub struct RemoveBlendSpacePointCommand {
    pub scene_node_handle: Handle<Node>,
    pub layer_index: usize,
    pub node_handle: Handle<PoseNode<Handle<Node>>>,
    pub point_index: usize,
    pub point: Option<BlendSpacePoint<Handle<Node>>>,
}

impl GameSceneCommandTrait for RemoveBlendSpacePointCommand {
    fn name(&mut self, _context: &GameSceneContext) -> String {
        "Remove Blend Space Point".to_string()
    }

    fn execute(&mut self, context: &mut GameSceneContext) {
        let machine = fetch_machine(context, self.scene_node_handle);
        if let PoseNode::BlendSpace(ref mut definition) =
            machine.layers_mut()[self.layer_index].nodes_mut()[self.node_handle]
        {
            self.point = Some(definition.points_mut().remove(self.point_index));
        }
    }

    fn revert(&mut self, context: &mut GameSceneContext) {
        let machine = fetch_machine(context, self.scene_node_handle);
        if let PoseNode::BlendSpace(ref mut definition) =
            machine.layers_mut()[self.layer_index].nodes_mut()[self.node_handle]
        {
            definition
                .points_mut()
                .insert(self.point_index, self.point.take().unwrap());
        }
    }
}
