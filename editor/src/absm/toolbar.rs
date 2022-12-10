use crate::{
    absm::selection::AbsmSelection,
    gui::make_dropdown_list_option,
    scene::{commands::ChangeSelectionCommand, EditorScene, Selection},
    Message,
};
use fyrox::{
    core::pool::Handle,
    gui::{
        check_box::{CheckBoxBuilder, CheckBoxMessage},
        dropdown_list::{DropdownListBuilder, DropdownListMessage},
        message::{MessageDirection, UiMessage},
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        widget::WidgetBuilder,
        BuildContext, Orientation, Thickness, UiNode, UserInterface, VerticalAlignment,
    },
    scene::animation::absm::AnimationBlendingStateMachine,
};
use std::sync::mpsc::Sender;

pub struct Toolbar {
    pub panel: Handle<UiNode>,
    pub preview: Handle<UiNode>,
    pub layers: Handle<UiNode>,
}

pub enum ToolbarAction {
    None,
    EnterPreviewMode,
    LeavePreviewMode,
}

impl Toolbar {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let preview;
        let layers;
        let panel = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_child({
                    preview = CheckBoxBuilder::new(
                        WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                    )
                    .with_content(
                        TextBuilder::new(
                            WidgetBuilder::new().with_vertical_alignment(VerticalAlignment::Center),
                        )
                        .with_text("Preview")
                        .build(ctx),
                    )
                    .build(ctx);
                    preview
                })
                .with_child({
                    layers = DropdownListBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .with_width(100.0),
                    )
                    .build(ctx);
                    layers
                }),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        Self {
            panel,
            preview,
            layers,
        }
    }

    pub fn handle_ui_message(
        &self,
        message: &UiMessage,
        editor_scene: &EditorScene,
        sender: &Sender<Message>,
    ) -> ToolbarAction {
        if let Some(CheckBoxMessage::Check(Some(value))) = message.data() {
            if message.destination() == self.preview
                && message.direction() == MessageDirection::FromWidget
            {
                return if *value {
                    ToolbarAction::EnterPreviewMode
                } else {
                    ToolbarAction::LeavePreviewMode
                };
            }
        } else if let Some(DropdownListMessage::SelectionChanged(Some(index))) = message.data() {
            if message.destination() == self.layers
                && message.direction() == MessageDirection::FromWidget
            {
                if let Selection::Absm(ref selection) = editor_scene.selection {
                    let mut new_selection = selection.clone();
                    new_selection.layer = *index;
                    sender
                        .send(Message::do_scene_command(ChangeSelectionCommand::new(
                            Selection::Absm(new_selection),
                            editor_scene.selection.clone(),
                        )))
                        .unwrap();
                }
            }
        }

        ToolbarAction::None
    }

    pub fn sync_to_model(
        &mut self,
        absm_node: &AnimationBlendingStateMachine,
        ui: &mut UserInterface,
        selection: &AbsmSelection,
    ) {
        let layers = absm_node
            .machine()
            .layers()
            .iter()
            .map(|l| make_dropdown_list_option(&mut ui.build_ctx(), l.name()))
            .collect();

        ui.send_message(DropdownListMessage::items(
            self.layers,
            MessageDirection::ToWidget,
            layers,
        ));

        ui.send_message(DropdownListMessage::selection(
            self.layers,
            MessageDirection::ToWidget,
            Some(selection.layer),
        ));
    }
}
