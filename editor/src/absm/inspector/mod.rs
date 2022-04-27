use crate::{
    absm::{
        command::{
            blend::{
                AddInputCommand, AddPoseSourceCommand, RemoveInputCommand, RemovePoseSourceCommand,
                SetBlendAnimationsByIndexInputBlendTimeCommand,
                SetBlendAnimationsByIndexParameterCommand, SetBlendAnimationsPoseWeightCommand,
                SetPoseWeightConstantCommand, SetPoseWeightParameterCommand,
            },
            AbsmCommand, CommandGroup, MovePoseNodeCommand, MoveStateNodeCommand,
            SetPlayAnimationResourceCommand, SetStateNameCommand, SetTransitionInvertRuleCommand,
            SetTransitionNameCommand, SetTransitionRuleCommand, SetTransitionTimeCommand,
        },
        message::MessageSender,
        AbsmDataModel, SelectedEntity,
    },
    inspector::editors::make_property_editors_container,
    Message, MessageDirection, MSG_SYNC_FLAG,
};
use fyrox::{
    animation::machine::{
        node::{
            blend::{
                BlendAnimationsByIndexDefinition, BlendAnimationsDefinition, BlendPoseDefinition,
                IndexedBlendInputDefinition,
            },
            play::PlayAnimationDefinition,
            BasePoseNodeDefinition, PoseNodeDefinition,
        },
        state::StateDefinition,
        transition::TransitionDefinition,
        MachineDefinition, PoseWeight,
    },
    core::{inspect::Inspect, pool::Handle},
    gui::{
        inspector::{
            editors::{
                collection::VecCollectionPropertyEditorDefinition,
                enumeration::EnumPropertyEditorDefinition,
                inspectable::InspectablePropertyEditorDefinition,
                PropertyEditorDefinitionContainer,
            },
            CollectionChanged, FieldKind, InspectorBuilder, InspectorContext, InspectorMessage,
            PropertyChanged,
        },
        message::UiMessage,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        BuildContext, UiNode, UserInterface,
    },
    utils::log::Log,
};
use std::{any::TypeId, rc::Rc, sync::mpsc::Sender};

pub struct Inspector {
    pub window: Handle<UiNode>,
    inspector: Handle<UiNode>,
    selection: Vec<SelectedEntity>,
    property_editors: Rc<PropertyEditorDefinitionContainer>,
}

impl Inspector {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let inspector = InspectorBuilder::new(WidgetBuilder::new()).build(ctx);
        let window = WindowBuilder::new(WidgetBuilder::new())
            .can_close(false)
            .can_minimize(false)
            .with_title(WindowTitle::text("Inspector"))
            .with_content(inspector)
            .build(ctx);

        let mut property_editors = make_property_editors_container(sender);
        property_editors
            .insert(InspectablePropertyEditorDefinition::<BasePoseNodeDefinition>::new());
        property_editors.insert(InspectablePropertyEditorDefinition::<
            IndexedBlendInputDefinition,
        >::new());
        property_editors.insert(VecCollectionPropertyEditorDefinition::<
            IndexedBlendInputDefinition,
        >::new());
        property_editors.insert(InspectablePropertyEditorDefinition::<BlendPoseDefinition>::new());
        property_editors
            .insert(VecCollectionPropertyEditorDefinition::<BlendPoseDefinition>::new());
        property_editors.insert(EnumPropertyEditorDefinition::<PoseWeight>::new());

        Self {
            window,
            inspector,
            selection: Default::default(),
            property_editors: Rc::new(property_editors),
        }
    }

    fn first_selected_entity<'a>(
        &self,
        definition: &'a MachineDefinition,
    ) -> Option<&'a dyn Inspect> {
        self.selection.first().map(|first| match first {
            SelectedEntity::Transition(transition) => {
                &definition.transitions[*transition] as &dyn Inspect
            }
            SelectedEntity::State(state) => &definition.states[*state] as &dyn Inspect,
            SelectedEntity::PoseNode(pose) => &definition.nodes[*pose] as &dyn Inspect,
        })
    }

    pub fn clear(&mut self, ui: &UserInterface) {
        self.selection.clear();

        ui.send_message(InspectorMessage::context(
            self.inspector,
            MessageDirection::ToWidget,
            Default::default(),
        ));
    }

    pub fn sync_to_model(&mut self, ui: &mut UserInterface, data_model: &AbsmDataModel) {
        let guard = data_model.resource.data_ref();

        if self.selection != data_model.selection {
            self.selection = data_model.selection.clone();

            if let Some(obj_ref) = self.first_selected_entity(&guard.absm_definition) {
                let ctx = InspectorContext::from_object(
                    obj_ref,
                    &mut ui.build_ctx(),
                    self.property_editors.clone(),
                    None,
                    MSG_SYNC_FLAG,
                    0,
                );

                ui.send_message(InspectorMessage::context(
                    self.inspector,
                    MessageDirection::ToWidget,
                    ctx,
                ));
            }
        } else if let Some(obj_ref) = self.first_selected_entity(&guard.absm_definition) {
            let ctx = ui
                .node(self.inspector)
                .cast::<fyrox::gui::inspector::Inspector>()
                .unwrap()
                .context()
                .clone();

            if let Err(sync_errors) = ctx.sync(obj_ref, ui, 0) {
                for error in sync_errors {
                    Log::err(format!("Failed to sync property. Reason: {:?}", error))
                }
            }
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        data_model: &AbsmDataModel,
        sender: &MessageSender,
    ) {
        if message.destination() == self.inspector
            && message.direction() == MessageDirection::FromWidget
        {
            if let Some(InspectorMessage::PropertyChanged(args)) =
                message.data::<InspectorMessage>()
            {
                let group = data_model
                    .selection
                    .iter()
                    .filter_map(|entry| match entry {
                        SelectedEntity::Transition(transition) => {
                            handle_transition_property_changed(args, *transition)
                        }
                        SelectedEntity::State(state) => handle_state_property_changed(
                            args,
                            *state,
                            &data_model.resource.data_ref().absm_definition.states[*state],
                        ),
                        SelectedEntity::PoseNode(pose_node) => {
                            let node =
                                &data_model.resource.data_ref().absm_definition.nodes[*pose_node];
                            if args.owner_type_id == TypeId::of::<PlayAnimationDefinition>() {
                                handle_play_animation_node_property_changed(args, *pose_node, node)
                            } else if args.owner_type_id
                                == TypeId::of::<BlendAnimationsByIndexDefinition>()
                            {
                                handle_blend_animations_by_index_node_property_changed(
                                    args, *pose_node, node,
                                )
                            } else if args.owner_type_id
                                == TypeId::of::<BlendAnimationsDefinition>()
                            {
                                handle_blend_animations_node_property_changed(
                                    args, *pose_node, node,
                                )
                            } else {
                                None
                            }
                        }
                    })
                    .collect::<Vec<_>>();

                if group.is_empty() {
                    Log::err(format!("Failed to handle a property {}", args.path()))
                } else {
                    sender.do_command(CommandGroup::from(group));
                }
            }
        }
    }
}

fn handle_transition_property_changed(
    args: &PropertyChanged,
    handle: Handle<TransitionDefinition>,
) -> Option<AbsmCommand> {
    match args.value {
        FieldKind::Object(ref value) => match args.name.as_ref() {
            TransitionDefinition::NAME => Some(AbsmCommand::new(SetTransitionNameCommand {
                handle,
                value: value.cast_clone()?,
            })),
            TransitionDefinition::RULE => Some(AbsmCommand::new(SetTransitionRuleCommand {
                handle,
                value: value.cast_clone()?,
            })),
            TransitionDefinition::TRANSITION_TIME => {
                Some(AbsmCommand::new(SetTransitionTimeCommand {
                    handle,
                    value: value.cast_clone()?,
                }))
            }
            TransitionDefinition::INVERT_RULE => {
                Some(AbsmCommand::new(SetTransitionInvertRuleCommand {
                    handle,
                    value: value.cast_clone()?,
                }))
            }
            _ => None,
        },
        _ => None,
    }
}

fn handle_state_property_changed(
    args: &PropertyChanged,
    handle: Handle<StateDefinition>,
    state_definition: &StateDefinition,
) -> Option<AbsmCommand> {
    match args.value {
        FieldKind::Object(ref value) => match args.name.as_ref() {
            StateDefinition::POSITION => Some(AbsmCommand::new(MoveStateNodeCommand::new(
                handle,
                state_definition.position,
                value.cast_clone()?,
            ))),
            StateDefinition::NAME => Some(AbsmCommand::new(SetStateNameCommand {
                handle,
                value: value.cast_clone()?,
            })),
            _ => None,
        },
        _ => None,
    }
}

fn handle_play_animation_node_property_changed(
    args: &PropertyChanged,
    handle: Handle<PoseNodeDefinition>,
    node: &PoseNodeDefinition,
) -> Option<AbsmCommand> {
    match args.value {
        FieldKind::Object(ref value) => match args.name.as_ref() {
            PlayAnimationDefinition::ANIMATION => {
                Some(AbsmCommand::new(SetPlayAnimationResourceCommand {
                    handle,
                    value: value.cast_clone()?,
                }))
            }
            _ => None,
        },
        FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
            PlayAnimationDefinition::BASE => {
                handle_base_pose_node_property_changed(inner, handle, node)
            }
            _ => None,
        },
        _ => None,
    }
}

fn handle_blend_animations_by_index_node_property_changed(
    args: &PropertyChanged,
    handle: Handle<PoseNodeDefinition>,
    node: &PoseNodeDefinition,
) -> Option<AbsmCommand> {
    match args.value {
        FieldKind::Object(ref value) => match args.name.as_ref() {
            BlendAnimationsByIndexDefinition::INDEX_PARAMETER => Some(AbsmCommand::new(
                SetBlendAnimationsByIndexParameterCommand {
                    handle,
                    value: value.cast_clone()?,
                },
            )),
            _ => None,
        },
        FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
            BlendAnimationsByIndexDefinition::BASE => {
                handle_base_pose_node_property_changed(inner, handle, node)
            }
            _ => None,
        },
        FieldKind::Collection(ref collection_changed) => match args.name.as_ref() {
            BlendAnimationsByIndexDefinition::INPUTS => match **collection_changed {
                CollectionChanged::Add => Some(AbsmCommand::new(AddInputCommand {
                    handle,
                    value: Some(Default::default()),
                })),
                CollectionChanged::Remove(i) => {
                    Some(AbsmCommand::new(RemoveInputCommand::new(handle, i)))
                }
                CollectionChanged::ItemChanged {
                    index,
                    ref property,
                } => match property.value {
                    FieldKind::Object(ref value) => match property.name.as_ref() {
                        IndexedBlendInputDefinition::BLEND_TIME => Some(AbsmCommand::new(
                            SetBlendAnimationsByIndexInputBlendTimeCommand {
                                handle,
                                index,
                                value: value.cast_clone()?,
                            },
                        )),
                        _ => None,
                    },
                    _ => None,
                },
            },
            _ => None,
        },
    }
}

#[allow(clippy::manual_map)]
fn handle_blend_animations_node_property_changed(
    args: &PropertyChanged,
    handle: Handle<PoseNodeDefinition>,
    node: &PoseNodeDefinition,
) -> Option<AbsmCommand> {
    match args.value {
        FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
            BlendAnimationsDefinition::BASE => {
                handle_base_pose_node_property_changed(inner, handle, node)
            }
            _ => None,
        },
        FieldKind::Collection(ref collection_changed) => match args.name.as_ref() {
            BlendAnimationsDefinition::POSE_SOURCES => match **collection_changed {
                CollectionChanged::Add => Some(AbsmCommand::new(AddPoseSourceCommand {
                    handle,
                    value: Some(Default::default()),
                })),
                CollectionChanged::Remove(i) => {
                    Some(AbsmCommand::new(RemovePoseSourceCommand::new(handle, i)))
                }
                CollectionChanged::ItemChanged {
                    index,
                    ref property,
                } => match property.value {
                    FieldKind::Object(ref value) => match property.name.as_ref() {
                        BlendPoseDefinition::WEIGHT => {
                            Some(AbsmCommand::new(SetBlendAnimationsPoseWeightCommand {
                                handle,
                                index,
                                value: value.cast_clone()?,
                            }))
                        }
                        _ => None,
                    },
                    FieldKind::Inspectable(ref inner) => match inner.name.as_ref() {
                        "0" => match inner.value {
                            FieldKind::Object(ref value) => {
                                if let Some(constant) = value.cast_clone::<f32>() {
                                    Some(AbsmCommand::new(SetPoseWeightConstantCommand {
                                        handle,
                                        value: constant,
                                        index,
                                    }))
                                } else if let Some(parameter) = value.cast_clone::<String>() {
                                    Some(AbsmCommand::new(SetPoseWeightParameterCommand {
                                        handle,
                                        value: parameter,
                                        index,
                                    }))
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        },
                        _ => None,
                    },
                    _ => None,
                },
            },
            _ => None,
        },
        _ => None,
    }
}

fn handle_base_pose_node_property_changed(
    args: &PropertyChanged,
    handle: Handle<PoseNodeDefinition>,
    base: &BasePoseNodeDefinition,
) -> Option<AbsmCommand> {
    match args.value {
        FieldKind::Object(ref value) => {
            match args.name.as_ref() {
                BasePoseNodeDefinition::POSITION => Some(AbsmCommand::new(
                    MovePoseNodeCommand::new(handle, base.position, value.cast_clone()?),
                )),
                _ => None,
            }
        }
        _ => None,
    }
}
