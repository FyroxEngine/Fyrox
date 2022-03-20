//! Animation blending state machine resource.

use crate::animation::machine::{State, Transition};
use crate::{
    animation::machine::{
        node::PoseNodeDefinition, BlendPose, IndexedBlendInput, Machine, MachineDefinition,
        PoseNode,
    },
    asset::{define_new_resource, Resource, ResourceData},
    core::{io::FileLoadError, pool::Handle, visitor::prelude::*},
    engine::resource_manager::{options::ImportOptions, ResourceManager},
    resource::model::ModelLoadError,
    scene::{node::Node, Scene},
};
use fxhash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    sync::Arc,
};

/// An error that may occur during ABSM resource loading.
#[derive(Debug, thiserror::Error)]
pub enum AbsmResourceError {
    /// An i/o error has occurred.
    #[error("A file load error has occurred {0:?}")]
    Io(FileLoadError),

    /// An error that may occur due to version incompatibilities.
    #[error("An error that may occur due to version incompatibilities. {0:?}")]
    Visit(VisitError),

    /// An error that may occur during instantiation of the ABSM. It means that an external
    /// animation resource wasn't able to load correctly.
    #[error("An error that may occur during instantiation of the ABSM. {0:?}")]
    AnimationLoadError(Option<Arc<ModelLoadError>>),

    /// An animation is not valid.
    #[error("An animation is not valid.")]
    InvalidAnimation,
}

impl From<FileLoadError> for AbsmResourceError {
    fn from(e: FileLoadError) -> Self {
        Self::Io(e)
    }
}

impl From<VisitError> for AbsmResourceError {
    fn from(e: VisitError) -> Self {
        Self::Visit(e)
    }
}

impl From<Option<Arc<ModelLoadError>>> for AbsmResourceError {
    fn from(e: Option<Arc<ModelLoadError>>) -> Self {
        Self::AnimationLoadError(e)
    }
}

/// State of the [`AbsmResource`]
#[derive(Debug, Visit, Default)]
pub struct AbsmResourceState {
    pub(in crate) path: PathBuf,
    /// Animation blending state machine definition.
    pub absm_definition: MachineDefinition,
}

impl ResourceData for AbsmResourceState {
    fn path(&self) -> Cow<Path> {
        Cow::Borrowed(&self.path)
    }

    fn set_path(&mut self, path: PathBuf) {
        self.path = path;
    }
}

impl AbsmResourceState {
    /// Load a ABSM resource from the specific file path.
    pub async fn from_file(path: &Path) -> Result<Self, AbsmResourceError> {
        let mut visitor = Visitor::load_binary(path).await?;
        let mut absm_definition = MachineDefinition::default();
        absm_definition.visit("Machine", &mut visitor)?;
        Ok(Self {
            absm_definition,
            path: path.to_path_buf(),
        })
    }
}

define_new_resource!(
    /// See module docs.
    AbsmResource<AbsmResourceState, AbsmResourceError>
);

impl AbsmResource {
    /// Instantiates animation blending state machine to the specified scene for a given root node.
    ///
    /// # Steps
    ///
    /// Instantiation involves multiple steps, the most important are:
    ///
    /// - Animation retargeting - it tries to retarget animation stored in PlayAnimation nodes to
    ///   a node hierarchy that starts from `root` node. The step may fail if the animation is not
    ///   suitable for the hierarchy.
    /// - ABSM instantiation - it uses ABSM definition to create a new instance of the ABSM.
    ///
    /// # Important notes
    ///
    /// The method loads multiple animation resources at once and it will fail even if one of them
    /// is faulty. Animation retargeting creates multiple animation instances in the scene, you
    /// **must** delete them manually when deleting the ABSM instance.
    ///
    /// The method is intended to be used with the ABSM resources made in the Fyroxed, any
    /// "hand-crafted" resources may contain invalid data which may cause errors during instantiation
    /// or even panic.  
    pub async fn instantiate(
        &self,
        root: Handle<Node>,
        scene: &mut Scene,
        resource_manager: ResourceManager,
    ) -> Result<Handle<Machine>, AbsmResourceError> {
        let data = self.data_ref();
        let definition = &data.absm_definition;

        let mut machine = Machine::new();

        // Initialize parameters.
        for (name, parameter) in definition.parameters.iter() {
            machine.set_parameter(name, parameter.clone());
        }

        // Instantiate nodes.
        let mut node_map = FxHashMap::default();
        for (definition_handle, node_definition) in definition.nodes.pair_iter() {
            let node = match node_definition {
                PoseNodeDefinition::PlayAnimation(play_animation) => {
                    let animation = *resource_manager
                        .request_model(&play_animation.animation)
                        .await?
                        .retarget_animations(root, scene)
                        .first()
                        .ok_or(AbsmResourceError::InvalidAnimation)?;

                    PoseNode::make_play_animation(animation)
                }
                PoseNodeDefinition::BlendAnimations(blend_animations) => {
                    PoseNode::make_blend_animations(
                        blend_animations
                            .pose_sources
                            .iter()
                            .map(|p| BlendPose {
                                weight: p.weight.clone(),
                                // Will be assigned on the next stage.
                                pose_source: Default::default(),
                            })
                            .collect(),
                    )
                }
                PoseNodeDefinition::BlendAnimationsByIndex(blend_animations) => {
                    PoseNode::make_blend_animations_by_index(
                        blend_animations.index_parameter.clone(),
                        blend_animations
                            .inputs
                            .iter()
                            .map(|i| IndexedBlendInput {
                                blend_time: i.blend_time,
                                // Will be assigned on the next stage.
                                pose_source: Default::default(),
                            })
                            .collect(),
                    )
                }
            };

            let instance_handle = machine.add_node(node);

            node_map.insert(definition_handle, instance_handle);
        }

        // Link nodes.
        for (definition_handle, instance_handle) in node_map.iter() {
            let definition = &definition.nodes[*definition_handle];
            let instance = machine.node_mut(*instance_handle);

            match instance {
                PoseNode::PlayAnimation(_) => {
                    // Do nothing, has no links to other nodes.
                }
                PoseNode::BlendAnimations(blend_animations) => {
                    if let PoseNodeDefinition::BlendAnimations(blend_animations_definition) =
                        definition
                    {
                        for (blend_pose, blend_pose_definition) in blend_animations
                            .pose_sources
                            .iter_mut()
                            .zip(blend_animations_definition.pose_sources.iter())
                        {
                            blend_pose.pose_source = node_map
                                .get(&blend_pose_definition.pose_source)
                                .cloned()
                                .expect(
                                    "There must be a respective pose node for blend pose source!",
                                );
                        }
                    } else {
                        unreachable!()
                    }
                }
                PoseNode::BlendAnimationsByIndex(blend_animations) => {
                    if let PoseNodeDefinition::BlendAnimationsByIndex(blend_animations_definition) =
                        definition
                    {
                        for (input, input_definition) in blend_animations
                            .inputs
                            .iter_mut()
                            .zip(blend_animations_definition.inputs.iter())
                        {
                            input.pose_source = node_map
                                .get(&input_definition.pose_source)
                                .cloned()
                                .expect("There must be a respective pose node for indexed input!");
                        }
                    } else {
                        unreachable!()
                    }
                }
            }
        }

        // Instantiate states.
        let mut state_map = FxHashMap::default();
        for (definition_handle, state_definition) in definition.states.pair_iter() {
            let instance_handle = machine.add_state(State::new(
                state_definition.name.as_ref(),
                node_map
                    .get(&state_definition.root)
                    .cloned()
                    .expect("There must be a respective pose node for root of state!"),
            ));

            state_map.insert(definition_handle, instance_handle);
        }

        // Instantiate transitions.
        for transition_definition in definition.transitions.iter() {
            machine.add_transition(Transition::new(
                transition_definition.name.as_ref(),
                state_map
                    .get(&transition_definition.source)
                    .cloned()
                    .expect("There must be a respective source state!"),
                state_map
                    .get(&transition_definition.dest)
                    .cloned()
                    .expect("There must be a respective dest state!"),
                transition_definition.transition_time,
                transition_definition.rule.as_str(),
            ));
        }

        Ok(scene.animation_machines.add(machine))
    }
}

/// Import options for ABSM resource.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct AbsmImportOptions {}

impl ImportOptions for AbsmImportOptions {}
