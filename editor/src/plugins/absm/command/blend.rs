// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::command::CommandContext;
use crate::fyrox::{
    core::{algebra::Vector2, pool::Handle},
    generic_animation::machine::{
        node::blendspace::BlendSpacePoint, BlendPose, IndexedBlendInput, PoseNode,
    },
};
use crate::plugins::absm::command::fetch_machine;
use crate::{
    command::CommandTrait, define_push_element_to_collection_command,
    define_set_collection_element_command,
};
use fyrox::core::reflect::Reflect;
use std::fmt::Debug;

define_push_element_to_collection_command!(AddInputCommand<Handle<PoseNode<Handle<N>>>, IndexedBlendInput<Handle<N>>>(self, context) {
    let machine = fetch_machine(context, self.node_handle);
    match &mut machine.layers_mut()[self.layer_index].nodes_mut()[self.handle] {
        PoseNode::BlendAnimationsByIndex(definition) => &mut definition.inputs,
        _ => unreachable!(),
    }
});

define_push_element_to_collection_command!(AddPoseSourceCommand<Handle<PoseNode<Handle<N>>>, BlendPose<Handle<N>>>(self, context) {
    let machine = fetch_machine(context, self.node_handle);
    match &mut machine.layers_mut()[self.layer_index].nodes_mut()[self.handle] {
        PoseNode::BlendAnimations(definition) => &mut definition.pose_sources,
        _ => unreachable!(),
    }
});

define_push_element_to_collection_command!(AddBlendSpacePointCommand<Handle<PoseNode<Handle<N>>>, BlendSpacePoint<Handle<N>>>(self, context) {
    let machine = fetch_machine(context, self.node_handle);
    match &mut machine.layers_mut()[self.layer_index].nodes_mut()[self.handle] {
        PoseNode::BlendSpace(definition) => definition.points_mut(),
        _ => unreachable!(),
    }
});

define_set_collection_element_command!(
    SetBlendAnimationByIndexInputPoseSourceCommand<Handle<PoseNode<Handle<N>>>, Handle<PoseNode<Handle<N>>>>(self, context) {
        let machine = fetch_machine(context, self.node_handle);
        if let PoseNode::BlendAnimationsByIndex(ref mut definition) = machine.layers_mut()[self.layer_index].nodes_mut()[self.handle] {
            std::mem::swap(&mut definition.inputs[self.index].pose_source, &mut self.value)
        }
    }
);

define_set_collection_element_command!(
    SetBlendAnimationsPoseSourceCommand<Handle<PoseNode<Handle<N>>>, Handle<PoseNode<Handle<N>>>>(self, context) {
        let machine = fetch_machine(context, self.node_handle);
        if let PoseNode::BlendAnimations(ref mut definition) = machine.layers_mut()[self.layer_index].nodes_mut()[self.handle] {
            std::mem::swap(&mut definition.pose_sources[self.index].pose_source, &mut self.value);
        }
    }
);

define_set_collection_element_command!(
    SetBlendSpacePoseSourceCommand<Handle<PoseNode<Handle<N>>>, Handle<PoseNode<Handle<N>>>>(self, context) {
        let machine = fetch_machine(context, self.node_handle);
        if let PoseNode::BlendSpace(ref mut definition) = machine.layers_mut()[self.layer_index].nodes_mut()[self.handle] {
            std::mem::swap( &mut definition.points_mut()[self.index].pose_source, &mut self.value);
        }
    }
);

define_set_collection_element_command!(
    SetBlendSpacePointPositionCommand<Handle<PoseNode<Handle<N>>>, Vector2<f32>>(self, context) {
        let machine = fetch_machine(context, self.node_handle);
        if let PoseNode::BlendSpace(ref mut definition) = machine.layers_mut()[self.layer_index].nodes_mut()[self.handle] {
            std::mem::swap( &mut definition.points_mut()[self.index].position, &mut self.value);
            definition.try_snap_points();
        }
    }
);

#[derive(Debug)]
pub struct RemoveBlendSpacePointCommand<N: Reflect> {
    pub scene_node_handle: Handle<N>,
    pub layer_index: usize,
    pub node_handle: Handle<PoseNode<Handle<N>>>,
    pub point_index: usize,
    pub point: Option<BlendSpacePoint<Handle<N>>>,
}

impl<N: Reflect> CommandTrait for RemoveBlendSpacePointCommand<N> {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Remove Blend Space Point".to_string()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        let machine = fetch_machine(context, self.scene_node_handle);
        if let PoseNode::BlendSpace(ref mut definition) =
            machine.layers_mut()[self.layer_index].nodes_mut()[self.node_handle]
        {
            self.point = Some(definition.points_mut().remove(self.point_index));
        }
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
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
