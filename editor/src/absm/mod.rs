use crate::{
    absm::{
        canvas::{AbsmCanvas, AbsmCanvasBuilder},
        command::{AbsmCommand, AbsmCommandStack, AbsmEditorContext, AddStateCommand},
        menu::Menu,
        node::{AbsmStateNode, AbsmStateNodeBuilder},
    },
    BuildContext, Color, MessageDirection, Row, UiMessage, WidgetBuilder,
};
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
    pub fn new(ctx: &mut BuildContext) -> Self {
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

        Self {
            window,
            canvas_context_menu,
            node_context_menu,
            command_stack: AbsmCommandStack::new(false),
            canvas,
            absm_definition: Some(Default::default()),
            menu,
        }
    }

    fn sync_to_model(&mut self, ui: &mut UserInterface) {
        if let Some(definition) = self.absm_definition.as_ref() {
            let canvas = ui
                .node(self.canvas)
                .cast::<AbsmCanvas>()
                .expect("Must be AbsmCanvas!");

            let states = canvas
                .children()
                .iter()
                .cloned()
                .filter(|c| ui.node(*c).has_component::<AbsmStateNode>())
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
                        let node_handle = AbsmStateNodeBuilder::new(
                            WidgetBuilder::new()
                                .with_context_menu(self.node_context_menu.menu)
                                .with_desired_position(state.position),
                        )
                        .build(state_handle, &mut ui.build_ctx());

                        ui.send_message(WidgetMessage::link(
                            node_handle,
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

            // Sync states.
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
