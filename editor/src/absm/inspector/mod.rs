use crate::{
    absm::{
        command::{
            pose::make_set_pose_property_command, state::make_set_state_property_command,
            transition::make_set_transition_property_command, CommandGroup,
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
            play::{PlayAnimationDefinition, TimeSlice},
            BasePoseNodeDefinition,
        },
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
            InspectorBuilder, InspectorContext, InspectorMessage,
        },
        message::UiMessage,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        BuildContext, UiNode, UserInterface,
    },
    utils::log::Log,
};
use std::{rc::Rc, sync::mpsc::Sender};

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

        let property_editors = make_property_editors_container(sender);
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
        property_editors.insert(InspectablePropertyEditorDefinition::<PoseWeight>::new());
        property_editors.insert(InspectablePropertyEditorDefinition::<TimeSlice>::new());
        property_editors.insert(EnumPropertyEditorDefinition::<TimeSlice>::new_optional());
        property_editors.insert(InspectablePropertyEditorDefinition::<TimeSlice>::new());
        property_editors.insert(InspectablePropertyEditorDefinition::<
            BlendAnimationsByIndexDefinition,
        >::new());
        property_editors.insert(InspectablePropertyEditorDefinition::<
            BlendAnimationsDefinition,
        >::new());
        property_editors
            .insert(InspectablePropertyEditorDefinition::<PlayAnimationDefinition>::new());

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
                    .map(|entry| match entry {
                        SelectedEntity::Transition(transition) => {
                            make_set_transition_property_command(*transition, args)
                        }
                        SelectedEntity::State(state) => {
                            make_set_state_property_command(*state, args)
                        }
                        SelectedEntity::PoseNode(pose_node) => {
                            make_set_pose_property_command(*pose_node, args)
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
