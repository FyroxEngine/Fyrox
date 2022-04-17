use crate::{
    absm::{
        canvas::{AbsmCanvasMessage, Mode},
        command::{
            AbsmCommand, AddStateCommand, ChangeSelectionCommand, CommandGroup, DeleteStateCommand,
            DeleteTransitionCommand,
        },
        message::MessageSender,
        transition::Transition,
        AbsmDataModel, SelectedEntity,
    },
    menu::create_menu_item,
};
use fyrox::{
    animation::machine::state::StateDefinition,
    core::pool::Handle,
    gui::{
        menu::MenuItemMessage,
        message::{MessageDirection, UiMessage},
        popup::{Placement, PopupBuilder, PopupMessage},
        stack_panel::StackPanelBuilder,
        widget::WidgetBuilder,
        BuildContext, UiNode, UserInterface,
    },
};

pub struct CanvasContextMenu {
    create_state: Handle<UiNode>,
    pub menu: Handle<UiNode>,
    pub canvas: Handle<UiNode>,
    pub node_context_menu: Handle<UiNode>,
}

impl CanvasContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let create_state;
        let menu = PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
            .with_content(
                StackPanelBuilder::new(WidgetBuilder::new().with_child({
                    create_state = create_menu_item("Create State", vec![], ctx);
                    create_state
                }))
                .build(ctx),
            )
            .build(ctx);

        Self {
            create_state,
            menu,
            canvas: Default::default(),
            node_context_menu: Default::default(),
        }
    }

    pub fn handle_ui_message(
        &mut self,
        sender: &MessageSender,
        message: &UiMessage,
        ui: &mut UserInterface,
    ) {
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.create_state {
                let screen_position = ui.node(self.menu).screen_position();

                sender.do_command(AddStateCommand::new(StateDefinition {
                    position: ui.node(self.canvas).screen_to_local(screen_position),
                    name: "New State".to_string(),
                    root: Default::default(),
                }));
            }
        }
    }
}

pub struct NodeContextMenu {
    create_transition: Handle<UiNode>,
    remove: Handle<UiNode>,
    pub menu: Handle<UiNode>,
    pub canvas: Handle<UiNode>,
    placement_target: Handle<UiNode>,
}

impl NodeContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let create_transition;
        let remove;
        let menu = PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
            .with_content(
                StackPanelBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            create_transition = create_menu_item("Create Transition", vec![], ctx);
                            create_transition
                        })
                        .with_child({
                            remove = create_menu_item("Remove", vec![], ctx);
                            remove
                        }),
                )
                .build(ctx),
            )
            .build(ctx);

        Self {
            create_transition,
            menu,
            remove,
            canvas: Default::default(),
            placement_target: Default::default(),
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        ui: &mut UserInterface,
        data_model: &AbsmDataModel,
        sender: &MessageSender,
    ) {
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.create_transition {
                ui.send_message(AbsmCanvasMessage::switch_mode(
                    self.canvas,
                    MessageDirection::ToWidget,
                    Mode::CreateTransition {
                        source: self.placement_target,
                        source_pos: ui.node(self.placement_target).center(),
                        dest_pos: ui.node(self.canvas).screen_to_local(ui.cursor_position()),
                    },
                ))
            } else if message.destination == self.remove {
                let states_to_remove = data_model
                    .selection
                    .iter()
                    .cloned()
                    .filter_map(|e| {
                        if let SelectedEntity::State(handle) = e {
                            Some(handle)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();

                // Gather every transition that leads from/to any of states to remove.
                let transitions_to_remove = data_model
                    .absm_definition
                    .transitions
                    .pair_iter()
                    .filter_map(|(handle, transition)| {
                        if states_to_remove.iter().cloned().any(|state_to_remove| {
                            state_to_remove == transition.source
                                || state_to_remove == transition.dest
                        }) {
                            Some(handle)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();

                let mut group = vec![AbsmCommand::new(ChangeSelectionCommand {
                    selection: vec![],
                })];

                group.extend(
                    transitions_to_remove.into_iter().map(|transition| {
                        AbsmCommand::new(DeleteTransitionCommand::new(transition))
                    }),
                );

                group.extend(
                    states_to_remove
                        .into_iter()
                        .map(|state| AbsmCommand::new(DeleteStateCommand::new(state))),
                );

                sender.do_command(CommandGroup::from(group));
            }
        } else if let Some(PopupMessage::Placement(Placement::Cursor(target))) = message.data() {
            if message.destination() == self.menu {
                self.placement_target = *target;
            }
        }
    }
}

pub struct TransitionContextMenu {
    remove: Handle<UiNode>,
    pub menu: Handle<UiNode>,
    placement_target: Handle<UiNode>,
}

impl TransitionContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let remove;
        let menu = PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
            .with_content(
                StackPanelBuilder::new(WidgetBuilder::new().with_child({
                    remove = create_menu_item("Remove Transition", vec![], ctx);
                    remove
                }))
                .build(ctx),
            )
            .build(ctx);

        Self {
            menu,
            remove,
            placement_target: Default::default(),
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        ui: &mut UserInterface,
        sender: &MessageSender,
    ) {
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination == self.remove {
                let transition_ref = ui
                    .node(self.placement_target)
                    .query_component::<Transition>()
                    .unwrap();

                let group = vec![
                    AbsmCommand::new(ChangeSelectionCommand { selection: vec![] }),
                    AbsmCommand::new(DeleteTransitionCommand::new(transition_ref.model_handle)),
                ];

                sender.do_command(CommandGroup::from(group));
            }
        } else if let Some(PopupMessage::Placement(Placement::Cursor(target))) = message.data() {
            if message.destination() == self.menu {
                self.placement_target = *target;
            }
        }
    }
}
