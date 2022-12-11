use crate::absm::fetch_selection;
use crate::{
    absm::{
        command::{AddLayerCommand, SetLayerNameCommand},
        selection::AbsmSelection,
    },
    gui::make_dropdown_list_option,
    scene::{commands::ChangeSelectionCommand, EditorScene, Selection},
    send_sync_message, Message,
};
use fyrox::{
    animation::machine::MachineLayer,
    core::pool::Handle,
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        check_box::{CheckBoxBuilder, CheckBoxMessage},
        dropdown_list::{DropdownListBuilder, DropdownListMessage},
        message::{MessageDirection, UiMessage},
        stack_panel::StackPanelBuilder,
        text::{TextBuilder, TextMessage},
        text_box::{TextBox, TextBoxBuilder},
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
    pub layer_name: Handle<UiNode>,
    pub add_layer: Handle<UiNode>,
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
        let layer_name;
        let add_layer;
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
                    layer_name = TextBoxBuilder::new(
                        WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                    )
                    .build(ctx);
                    layer_name
                })
                .with_child({
                    add_layer = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .with_width(20.0),
                    )
                    .with_text("+")
                    .build(ctx);
                    add_layer
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
            layer_name,
            add_layer,
        }
    }

    pub fn handle_ui_message(
        &self,
        message: &UiMessage,
        editor_scene: &EditorScene,
        sender: &Sender<Message>,
        ui: &UserInterface,
    ) -> ToolbarAction {
        let selection = fetch_selection(&editor_scene.selection);

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
        } else if let Some(TextMessage::Text(text)) = message.data() {
            if message.destination() == self.layer_name
                && message.direction() == MessageDirection::FromWidget
            {
                sender
                    .send(Message::do_scene_command(SetLayerNameCommand {
                        absm_node_handle: selection.absm_node_handle,
                        layer_index: selection.layer,
                        name: text.clone(),
                    }))
                    .unwrap();
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.add_layer {
                let mut layer = MachineLayer::new();

                layer.set_name(
                    ui.node(self.layer_name)
                        .query_component::<TextBox>()
                        .unwrap()
                        .text(),
                );

                sender
                    .send(Message::do_scene_command(AddLayerCommand {
                        absm_node_handle: selection.absm_node_handle,
                        layer: Some(layer),
                    }))
                    .unwrap();
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

        if let Some(layer) = absm_node.machine().layers().get(selection.layer) {
            send_sync_message(
                ui,
                TextMessage::text(
                    self.layer_name,
                    MessageDirection::ToWidget,
                    layer.name().to_string(),
                ),
            );
        }
    }
}
