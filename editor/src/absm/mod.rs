use crate::absm::node::AbsmStateNodeMessage;
use crate::absm::transition::{Transition, TransitionBuilder};
use crate::{
    absm::{
        canvas::{AbsmCanvas, AbsmCanvasBuilder},
        command::{AbsmCommand, AbsmCommandStack, AbsmEditorContext, AddStateCommand},
        menu::Menu,
        node::{AbsmStateNode, AbsmStateNodeBuilder},
    },
    BuildContext, Color, MessageDirection, Row, UiMessage, WidgetBuilder,
};
use fyrox::animation::machine::transition::TransitionDefinition;
use fyrox::{
    animation::machine::{state::StateDefinition, MachineDefinition},
    core::{algebra::Point2, algebra::Vector2, pool::Handle},
    gui::{
        border::BorderBuilder,
        grid::{Column, GridBuilder},
        menu::{MenuItemBuilder, MenuItemContent, MenuItemMessage},
        popup::PopupBuilder,
        stack_panel::StackPanelBuilder,
        widget::WidgetMessage,
        window::{WindowBuilder, WindowTitle},
        UiNode, UserInterface,
    },
};

mod canvas;
mod command;
mod menu;
mod node;
mod transition;

const NORMAL_BACKGROUND: Color = Color::opaque(60, 60, 60);
const SELECTED_BACKGROUND: Color = Color::opaque(80, 80, 80);
const BORDER_COLOR: Color = Color::opaque(70, 70, 70);

pub struct AbsmEditor {
    #[allow(dead_code)] // TODO
    window: Handle<UiNode>,
    canvas_context_menu: CanvasContextMenu,
    node_context_menu: NodeContextMenu,
    command_stack: AbsmCommandStack,
    canvas: Handle<UiNode>,
    absm_definition: Option<MachineDefinition>,
    menu: Menu,
}

pub struct CanvasContextMenu {
    create_state: Handle<UiNode>,
    menu: Handle<UiNode>,
    pub canvas: Handle<UiNode>,
    pub node_context_menu: Handle<UiNode>,
}

impl CanvasContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let create_state;
        let menu = PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
            .with_content(
                StackPanelBuilder::new(WidgetBuilder::new().with_child({
                    create_state = MenuItemBuilder::new(
                        WidgetBuilder::new().with_min_size(Vector2::new(120.0, 20.0)),
                    )
                    .with_content(MenuItemContent::text("Create State"))
                    .build(ctx);
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
        message: &UiMessage,
        ui: &mut UserInterface,
    ) -> Option<AbsmCommand> {
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.create_state {
                let screen_position = ui.node(self.menu).screen_position();

                let local_position = ui
                    .node(self.canvas)
                    .visual_transform()
                    .try_inverse()
                    .unwrap_or_default()
                    .transform_point(&Point2::from(screen_position))
                    .coords;

                return Some(AbsmCommand::new(AddStateCommand::new(StateDefinition {
                    position: local_position,
                    name: "New State".to_string(),
                    root: Default::default(),
                })));
            }
        }

        None
    }
}

pub struct NodeContextMenu {
    create_transition: Handle<UiNode>,
    menu: Handle<UiNode>,
}

impl NodeContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let create_transition;
        let menu = PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
            .with_content(
                StackPanelBuilder::new(WidgetBuilder::new().with_child({
                    create_transition = MenuItemBuilder::new(
                        WidgetBuilder::new().with_min_size(Vector2::new(120.0, 20.0)),
                    )
                    .with_content(MenuItemContent::text("Create Transition"))
                    .build(ctx);
                    create_transition
                }))
                .build(ctx),
            )
            .build(ctx);

        Self {
            create_transition,
            menu,
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, ui: &mut UserInterface) {
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.create_transition {}
        }
    }
}

impl AbsmEditor {
    pub fn new(ui: &mut UserInterface) -> Self {
        let ctx = &mut ui.build_ctx();
        let node_context_menu = NodeContextMenu::new(ctx);
        let mut canvas_context_menu = CanvasContextMenu::new(ctx);
        let menu = Menu::new(ctx);

        let canvas;
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(700.0).with_height(400.0))
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new().with_child(menu.menu).with_child(
                        BorderBuilder::new(WidgetBuilder::new().on_row(1).with_child({
                            canvas = AbsmCanvasBuilder::new(
                                WidgetBuilder::new().with_context_menu(canvas_context_menu.menu),
                            )
                            .build(ctx);
                            canvas
                        }))
                        .build(ctx),
                    ),
                )
                .add_row(Row::strict(24.0))
                .add_row(Row::stretch())
                .add_column(Column::stretch())
                .build(ctx),
            )
            .with_title(WindowTitle::text("ABSM Editor"))
            .build(ctx);

        canvas_context_menu.canvas = canvas;
        canvas_context_menu.node_context_menu = node_context_menu.menu;

        let mut absm_definition = MachineDefinition::default();

        let state1 = absm_definition.states.spawn(StateDefinition {
            position: Default::default(),
            name: "State".to_string(),
            root: Default::default(),
        });
        let state2 = absm_definition.states.spawn(StateDefinition {
            position: Vector2::new(300.0, 200.0),
            name: "Other State".to_string(),
            root: Default::default(),
        });
        absm_definition.transitions.spawn(TransitionDefinition {
            name: "Transition".to_string(),
            transition_time: 0.2,
            source: state1,
            dest: state2,
            rule: "Rule1".to_string(),
        });

        let mut editor = Self {
            window,
            canvas_context_menu,
            node_context_menu,
            command_stack: AbsmCommandStack::new(false),
            canvas,
            absm_definition: Some(absm_definition),
            menu,
        };

        editor.sync_to_model(ui);

        editor
    }

    fn sync_to_model(&mut self, ui: &mut UserInterface) {
        if let Some(definition) = self.absm_definition.as_ref() {
            let canvas = ui
                .node(self.canvas)
                .cast::<AbsmCanvas>()
                .expect("Must be AbsmCanvas!");

            let mut states = canvas
                .children()
                .iter()
                .cloned()
                .filter(|c| ui.node(*c).has_component::<AbsmStateNode>())
                .collect::<Vec<_>>();

            let transitions = canvas
                .children()
                .iter()
                .cloned()
                .filter(|c| ui.node(*c).has_component::<Transition>())
                .collect::<Vec<_>>();

            if states.len() < definition.states.alive_count() as usize {
                // A state was added.
                for (state_handle, state) in definition.states.pair_iter() {
                    if states.iter().all(|state_view| {
                        ui.node(*state_view)
                            .query_component::<AbsmStateNode>()
                            .unwrap()
                            .model_handle
                            != state_handle
                    }) {
                        let state_view_handle = AbsmStateNodeBuilder::new(
                            WidgetBuilder::new()
                                .with_context_menu(self.node_context_menu.menu)
                                .with_desired_position(state.position),
                        )
                        .build(state_handle, &mut ui.build_ctx());

                        states.push(state_view_handle);

                        ui.send_message(WidgetMessage::link(
                            state_view_handle,
                            MessageDirection::ToWidget,
                            self.canvas,
                        ));
                    }
                }
            } else if states.len() > definition.states.alive_count() as usize {
                // A state was removed.
                for (state_view_handle, state_model_handle) in
                    states.iter().cloned().map(|state_view| {
                        (
                            state_view,
                            ui.node(state_view)
                                .query_component::<AbsmStateNode>()
                                .unwrap()
                                .model_handle,
                        )
                    })
                {
                    if definition
                        .states
                        .pair_iter()
                        .all(|(h, _)| h != state_model_handle)
                    {
                        ui.send_message(WidgetMessage::remove(
                            state_view_handle,
                            MessageDirection::ToWidget,
                        ));
                    }
                }
            }

            // Sync state nodes.
            for state in states.iter() {
                let state_node = ui.node(*state).query_component::<AbsmStateNode>().unwrap();

                if definition.states[state_node.model_handle].name != state_node.name {
                    ui.send_message(AbsmStateNodeMessage::name(
                        *state,
                        MessageDirection::ToWidget,
                        state_node.name.clone(),
                    ));
                }
            }

            // Force update layout to be able to fetch positions of nodes for transitions.
            ui.update(ui.screen_size(), 0.0);

            // Sync transitions.
            if transitions.len() < definition.transitions.alive_count() as usize {
                // A transition was added.
                for (transition_handle, transition) in definition.transitions.pair_iter() {
                    if transitions.iter().all(|transition_view| {
                        ui.node(*transition_view)
                            .query_component::<Transition>()
                            .unwrap()
                            .model_handle
                            != transition_handle
                    }) {
                        fn find_state_view(
                            state_handle: Handle<StateDefinition>,
                            states: &[Handle<UiNode>],
                            ui: &UserInterface,
                        ) -> Handle<UiNode> {
                            states
                                .iter()
                                .find(|s| {
                                    ui.node(**s)
                                        .query_component::<AbsmStateNode>()
                                        .unwrap()
                                        .model_handle
                                        == state_handle
                                })
                                .cloned()
                                .unwrap_or_default()
                        }

                        let transition_view = TransitionBuilder::new(WidgetBuilder::new())
                            .with_source(find_state_view(transition.source, &states, ui))
                            .with_dest(find_state_view(transition.dest, &states, ui))
                            .build(transition_handle, &mut ui.build_ctx());

                        ui.send_message(WidgetMessage::link(
                            transition_view,
                            MessageDirection::ToWidget,
                            self.canvas,
                        ));
                    }
                }
            } else if transitions.len() > definition.transitions.alive_count() as usize {
                // A transition was removed.
                for (transition_view_handle, transition_model_handle) in
                    transitions.iter().cloned().map(|transition_view| {
                        (
                            transition_view,
                            ui.node(transition_view)
                                .query_component::<Transition>()
                                .unwrap()
                                .model_handle,
                        )
                    })
                {
                    if definition
                        .transitions
                        .pair_iter()
                        .all(|(h, _)| h != transition_model_handle)
                    {
                        ui.send_message(WidgetMessage::remove(
                            transition_view_handle,
                            MessageDirection::ToWidget,
                        ));
                    }
                }
            }
        }
    }

    fn do_command(&mut self, command: AbsmCommand, ui: &mut UserInterface) {
        if let Some(definition) = self.absm_definition.as_mut() {
            self.command_stack
                .do_command(command.into_inner(), AbsmEditorContext { definition });

            self.sync_to_model(ui);
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, ui: &mut UserInterface) {
        if let Some(command) = self.canvas_context_menu.handle_ui_message(message, ui) {
            self.do_command(command, ui);
        }
    }
}
