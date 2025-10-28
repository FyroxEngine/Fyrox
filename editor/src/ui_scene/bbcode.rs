use fyrox::{
    core::{pool::Handle, variable::InheritableVariable},
    engine::Engine,
    gui::{
        formatted_text::RunSet,
        message::{MessageDirection, UiMessage},
        stack_panel::StackPanelBuilder,
        text::{Text, TextBuilder, TextMessage},
        text_box::{TextBoxBuilder, TextCommitMode},
        widget::{WidgetBuilder, WidgetMessage},
        BBCode, BuildContext, UiNode, UserInterface,
    },
};

use crate::{
    command::{CommandContext, CommandTrait},
    message::MessageSender,
    scene::Selection,
    send_sync_message,
    ui_scene::{commands::UiSceneContext, UiScene},
    Message,
};

pub struct BBCodePanel {
    pub root_widget: Handle<UiNode>,
    pub text_box: Handle<UiNode>,
}

impl BBCodePanel {
    pub fn new(inspector_head: Handle<UiNode>, ctx: &mut BuildContext) -> Self {
        let text_box = TextBoxBuilder::new(WidgetBuilder::new())
            .with_text_commit_mode(TextCommitMode::Changed)
            .build(ctx);
        let root_widget = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_visibility(false)
                .with_child(
                    TextBuilder::new(WidgetBuilder::new())
                        .with_text("BBCode")
                        .build(ctx),
                )
                .with_child(text_box),
        )
        .build(ctx);
        ctx.send_message(WidgetMessage::link(
            root_widget,
            MessageDirection::ToWidget,
            inspector_head,
        ));
        Self {
            root_widget,
            text_box,
        }
    }
    pub fn sync_to_model(
        &mut self,
        selection: &Selection,
        ui_scene: &mut UiScene,
        ui: &UserInterface,
    ) {
        let Some(selection) = selection.as_ui() else {
            return;
        };
        let mut bbcode = String::new();
        for handle in &selection.widgets {
            if let Some(text) = ui_scene
                .ui
                .try_get_node_mut(*handle)
                .and_then(|n| n.cast::<Text>())
            {
                bbcode = text.bbcode.clone_inner();
                break;
            }
        }
        send_sync_message(
            ui,
            TextMessage::text(self.text_box, MessageDirection::ToWidget, bbcode),
        );
    }
    pub fn handle_message(
        &mut self,
        message: &Message,
        editor_selection: &Selection,
        ui_scene: &mut UiScene,
        engine: &mut Engine,
    ) {
        if let Message::SelectionChanged { .. } = message {
            let text_selected = editor_selection.as_ui().is_some_and(|s| {
                s.widgets.iter().any(|n| {
                    ui_scene
                        .ui
                        .try_get_node_mut(*n)
                        .map(|n| n.cast::<Text>().is_some())
                        .unwrap_or_default()
                })
            });
            engine
                .user_interfaces
                .first_mut()
                .send_message(WidgetMessage::visibility(
                    self.root_widget,
                    MessageDirection::ToWidget,
                    text_selected,
                ));
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_selection: &Selection,
        ui_scene: &mut UiScene,
        _engine: &mut Engine,
        _sender: &MessageSender,
    ) {
        if message.destination() != self.text_box {
            return;
        }
        if message.direction() != MessageDirection::FromWidget {
            return;
        }
        let Some(TextMessage::Text(bbcode)) = message.data::<TextMessage>() else {
            return;
        };
        let Some(selection) = editor_selection.as_ui() else {
            return;
        };
        for handle in selection.widgets.iter().copied() {
            if let Some(text) = ui_scene
                .ui
                .try_get_node_mut(handle)
                .and_then(|n| n.cast::<Text>())
            {
                let font = text.font();
                let parsed_code: BBCode = bbcode.parse().unwrap();
                let text = parsed_code.text.chars().collect::<Vec<char>>();
                ui_scene.message_sender.do_command(BBCodeCommand {
                    text_handle: handle,
                    code: InheritableVariable::new_modified(bbcode.clone()),
                    text: InheritableVariable::new_modified(text),
                    runs: parsed_code.build_runs(&font),
                });
            }
        }
    }
}

#[derive(Debug)]
struct BBCodeCommand {
    text_handle: Handle<UiNode>,
    code: InheritableVariable<String>,
    text: InheritableVariable<Vec<char>>,
    runs: RunSet,
}

impl BBCodeCommand {
    fn swap(&mut self, context: &mut dyn CommandContext) {
        let ctx = context.get_mut::<UiSceneContext>();
        let text = ctx
            .ui
            .try_get_node_mut(self.text_handle)
            .and_then(|n| n.cast_mut::<Text>())
            .expect("must be Text");
        std::mem::swap(&mut text.bbcode, &mut self.code);
        let formatted_text = text.formatted_text.get_mut();
        std::mem::swap(&mut formatted_text.text, &mut self.text);
        std::mem::swap(&mut formatted_text.runs, &mut self.runs);
        text.invalidate_layout();
    }
}

impl CommandTrait for BBCodeCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Modify BBCode".into()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        self.swap(context);
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        self.swap(context);
    }
}
