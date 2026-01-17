use crate::{
    command::{CommandContext, CommandTrait},
    message::MessageSender,
    scene::Selection,
    ui_scene::{commands::UiSceneContext, UiScene},
    Message,
};
use fyrox::gui::text_box::TextBox;
use fyrox::{
    core::{pool::Handle, variable::InheritableVariable},
    engine::Engine,
    graph::SceneGraph,
    gui::{
        formatted_text::{RunSet, WrapMode},
        message::UiMessage,
        stack_panel::StackPanelBuilder,
        text::{Text, TextBuilder, TextMessage},
        text_box::{TextBoxBuilder, TextCommitMode},
        widget::{WidgetBuilder, WidgetMessage},
        BBCode, BuildContext, UiNode, UserInterface,
    },
};

pub struct BBCodePanel {
    pub root_widget: Handle<UiNode>,
    pub text_box: Handle<TextBox>,
}

impl BBCodePanel {
    pub fn new(inspector_head: Handle<UiNode>, ctx: &mut BuildContext) -> Self {
        let text_box = TextBoxBuilder::new(WidgetBuilder::new())
            .with_text_commit_mode(TextCommitMode::Changed)
            .with_multiline(true)
            .with_wrap(WrapMode::Word)
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
        ctx.inner()
            .send(root_widget, WidgetMessage::LinkWith(inspector_head));
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
            if let Ok(text) = ui_scene.ui.try_get_mut_of_type::<Text>(*handle) {
                bbcode = text.bbcode.clone_inner();
                break;
            }
        }
        ui.send_sync(self.text_box, TextMessage::Text(bbcode));
    }
    pub fn handle_message(
        &mut self,
        message: &Message,
        editor_selection: &Selection,
        ui_scene: Option<&UiScene>,
        engine: &mut Engine,
    ) {
        if let Message::SelectionChanged { .. } = message {
            let text_selected = editor_selection.as_ui().is_some_and(|s| {
                s.widgets.iter().any(|n| {
                    ui_scene
                        .and_then(|s| {
                            s.ui.try_get_node(*n)
                                .ok()
                                .map(|n| n.cast::<Text>().is_some())
                        })
                        .unwrap_or_default()
                })
            });
            engine
                .user_interfaces
                .first_mut()
                .send(self.root_widget, WidgetMessage::Visibility(text_selected));
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
        let Some(TextMessage::Text(bbcode)) = message.data_from::<TextMessage>(self.text_box)
        else {
            return;
        };
        let Some(selection) = editor_selection.as_ui() else {
            return;
        };
        for handle in selection.widgets.iter().copied() {
            if let Ok(text) = ui_scene.ui.try_get_mut_of_type::<Text>(handle) {
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
            .try_get_mut_of_type::<Text>(self.text_handle)
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
