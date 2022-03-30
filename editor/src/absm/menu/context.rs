use crate::absm::{
    canvas::{AbsmCanvasMessage, Mode},
    command::{AbsmCommand, AddStateCommand},
    message::AbsmMessage,
    node::AbsmStateNode,
};
use fyrox::{
    animation::machine::state::StateDefinition,
    core::{algebra::Vector2, pool::Handle},
    gui::{
        menu::{MenuItemBuilder, MenuItemContent, MenuItemMessage},
        message::{MessageDirection, UiMessage},
        popup::{Placement, PopupBuilder, PopupMessage},
        stack_panel::StackPanelBuilder,
        widget::WidgetBuilder,
        BuildContext, UiNode, UserInterface,
    },
};
use std::sync::mpsc::Sender;

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
        sender: &Sender<AbsmMessage>,
        message: &UiMessage,
        ui: &mut UserInterface,
    ) {
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.create_state {
                let screen_position = ui.node(self.menu).screen_position();

                sender
                    .send(AbsmMessage::DoCommand(AbsmCommand::new(
                        AddStateCommand::new(StateDefinition {
                            position: ui.node(self.canvas).screen_to_local(screen_position),
                            name: "New State".to_string(),
                            root: Default::default(),
                        }),
                    )))
                    .unwrap();
            }
        }
    }
}

pub struct NodeContextMenu {
    create_transition: Handle<UiNode>,
    pub menu: Handle<UiNode>,
    pub canvas: Handle<UiNode>,
    placement_target: Handle<UiNode>,
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
            canvas: Default::default(),
            placement_target: Default::default(),
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, ui: &mut UserInterface) {
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.create_transition {
                assert!(ui
                    .node(self.placement_target)
                    .query_component::<AbsmStateNode>()
                    .is_some());

                ui.send_message(AbsmCanvasMessage::switch_mode(
                    self.canvas,
                    MessageDirection::ToWidget,
                    Mode::CreateTransition {
                        source: self.placement_target,
                        source_pos: ui.node(self.placement_target).center(),
                        dest_pos: ui.node(self.canvas).screen_to_local(ui.cursor_position()),
                    },
                ))
            }
        } else if let Some(PopupMessage::Placement(Placement::Cursor(target))) = message.data() {
            if message.destination() == self.menu {
                self.placement_target = *target;
            }
        }
    }
}
