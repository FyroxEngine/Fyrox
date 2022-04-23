#![allow(dead_code)] // TODO

use crate::{
    absm::command::{AbsmCommandTrait, AbsmEditorContext},
    define_absm_swap_command, define_push_element_to_collection_command,
    define_remove_collection_element_command, define_set_collection_element_command,
};
use fyrox::{
    animation::machine::{
        node::{
            blend::{BlendPoseDefinition, IndexedBlendInputDefinition},
            PoseNodeDefinition,
        },
        PoseWeight,
    },
    core::pool::Handle,
};

define_push_element_to_collection_command!(AddInputCommand<Handle<PoseNodeDefinition>, IndexedBlendInputDefinition>(self, context) {
    match &mut context.definition.nodes[self.handle] {
        PoseNodeDefinition::BlendAnimationsByIndex(definition) => &mut definition.inputs,
        _ => unreachable!(),
    }
});

define_remove_collection_element_command!(RemoveInputCommand<Handle<PoseNodeDefinition>, IndexedBlendInputDefinition>(self, context) {
    match &mut context.definition.nodes[self.handle] {
        PoseNodeDefinition::BlendAnimationsByIndex(definition) => &mut definition.inputs,
        _ => unreachable!(),
    }
});

define_push_element_to_collection_command!(AddPoseSourceCommand<Handle<PoseNodeDefinition>, BlendPoseDefinition>(self, context) {
    match &mut context.definition.nodes[self.handle] {
        PoseNodeDefinition::BlendAnimations(definition) => &mut definition.pose_sources,
        _ => unreachable!(),
    }
});

define_remove_collection_element_command!(RemovePoseSourceCommand<Handle<PoseNodeDefinition>, BlendPoseDefinition>(self, context) {
    match &mut context.definition.nodes[self.handle] {
        PoseNodeDefinition::BlendAnimations(definition) => &mut definition.pose_sources,
        _ => unreachable!(),
    }
});

define_set_collection_element_command!(
    SetBlendAnimationByIndexInputPoseSourceCommand<Handle<PoseNodeDefinition>, Handle<PoseNodeDefinition>>(self, context) {
        match context.definition.nodes[self.handle] {
            PoseNodeDefinition::BlendAnimationsByIndex(ref mut definition) => {
                &mut definition.inputs[self.index].pose_source
            }
            _ => unreachable!(),
        }
    }
);

define_set_collection_element_command!(
    SetBlendAnimationsPoseSourceCommand<Handle<PoseNodeDefinition>, Handle<PoseNodeDefinition>>(self, context) {
        match context.definition.nodes[self.handle] {
            PoseNodeDefinition::BlendAnimations(ref mut definition) => {
                &mut definition.pose_sources[self.index].pose_source
            }
            _ => unreachable!(),
        }
    }
);

define_absm_swap_command!(SetBlendAnimationsByIndexParameterCommand<Handle<PoseNodeDefinition>, String>[](self, context) {
    if let PoseNodeDefinition::BlendAnimationsByIndex(ref mut definition) = context.definition.nodes[self.handle] {
        &mut definition.index_parameter
    } else {
        unreachable!()
    }
});

define_absm_swap_command!(SetBlendAnimationsByIndexInputBlendTimeCommand<Handle<PoseNodeDefinition>, f32>[index: usize](self, context) {
    if let PoseNodeDefinition::BlendAnimationsByIndex(ref mut definition) = context.definition.nodes[self.handle] {
        &mut definition.inputs[self.index].blend_time
    } else {
        unreachable!()
    }
});

define_absm_swap_command!(SetBlendAnimationsPoseWeightCommand<Handle<PoseNodeDefinition>, PoseWeight>[index: usize](self, context) {
    if let PoseNodeDefinition::BlendAnimations(ref mut definition) = context.definition.nodes[self.handle] {
        &mut definition.pose_sources[self.index].weight
    } else {
        unreachable!()
    }
});

define_absm_swap_command!(SetPoseWeightConstantCommand<Handle<PoseNodeDefinition>, f32>[index: usize](self, context) {
    if let PoseNodeDefinition::BlendAnimations(ref mut definition) = context.definition.nodes[self.handle] {
        if let PoseWeight::Constant(ref mut value) = definition.pose_sources[self.index].weight {
            value
        } else {
            unreachable!()
        }
    } else {
        unreachable!()
    }
});

define_absm_swap_command!(SetPoseWeightParameterCommand<Handle<PoseNodeDefinition>, String>[index: usize](self, context) {
    if let PoseNodeDefinition::BlendAnimations(ref mut definition) = context.definition.nodes[self.handle] {
        if let PoseWeight::Parameter(ref mut value) = definition.pose_sources[self.index].weight {
            value
        } else {
            unreachable!()
        }
    } else {
        unreachable!()
    }
});
