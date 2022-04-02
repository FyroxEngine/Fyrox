use crate::absm::{
    canvas::{AbsmCanvasBuilder, AbsmCanvasMessage},
    command::{AbsmCommand, AddTransitionCommand, CommandGroup, MoveStateNodeCommand},
    message::MessageSender,
    node::AbsmStateNode,
};
use fyrox::{
    animation::machine::{state::StateDefinition, transition::TransitionDefinition},
    core::pool::Handle,
    gui::{
        border::BorderBuilder,
        message::{MessageDirection, UiMessage},
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        BuildContext, UiNode, UserInterface,
    },
};

pub struct Document {
    pub window: Handle<UiNode>,
    pub canvas: Handle<UiNode>,
}

fn fetch_state_node_model_handle(
    handle: Handle<UiNode>,
    ui: &UserInterface,
) -> Handle<StateDefinition> {
    ui.node(handle)
        .query_component::<AbsmStateNode>()
        .unwrap()
        .model_handle
}

impl Document {
    pub fn new(context_menu: Handle<UiNode>, ctx: &mut BuildContext) -> Self {
        let canvas =
            AbsmCanvasBuilder::new(WidgetBuilder::new().with_context_menu(context_menu)).build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new())
            .with_title(WindowTitle::text("Document"))
            .with_content(BorderBuilder::new(WidgetBuilder::new().with_child(canvas)).build(ctx))
            .build(ctx);

        Self { window, canvas }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        ui: &mut UserInterface,
        sender: &MessageSender,
    ) {
        if let Some(msg) = message.data::<AbsmCanvasMessage>() {
            match msg {
                AbsmCanvasMessage::CommitTransition { source, dest } => {
                    if message.destination() == self.canvas
                        && message.direction() == MessageDirection::FromWidget
                    {
                        let source = fetch_state_node_model_handle(*source, ui);
                        let dest = fetch_state_node_model_handle(*dest, ui);

                        sender.do_command(AddTransitionCommand::new(TransitionDefinition {
                            name: "Transition".to_string(),
                            transition_time: 1.0,
                            source,
                            dest,
                            rule: "".to_string(),
                        }));
                    }
                }
                AbsmCanvasMessage::CommitDrag { entries } => {
                    let commands = entries
                        .iter()
                        .map(|e| {
                            let state_handle = fetch_state_node_model_handle(e.node, ui);
                            let new_position = ui.node(e.node).actual_local_position();

                            AbsmCommand::new(MoveStateNodeCommand::new(
                                state_handle,
                                e.initial_position,
                                new_position,
                            ))
                        })
                        .collect::<Vec<_>>();

                    sender.do_command(CommandGroup::from(commands));
                }
                _ => (),
            }
        }
    }
}
