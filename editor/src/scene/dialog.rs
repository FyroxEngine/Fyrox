use crate::fyrox::graph::SceneGraph;
use crate::fyrox::{
    core::pool::Handle,
    engine::Engine,
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        formatted_text::WrapMode,
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessage},
        scroll_viewer::ScrollViewerBuilder,
        stack_panel::StackPanelBuilder,
        text::{TextBuilder, TextMessage},
        text_box::TextBoxBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowMessage},
        BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode,
    },
};
use crate::scene::Selection;
use crate::{
    message::MessageSender,
    scene::{
        commands::{make_delete_selection_command, selection_to_delete},
        GameScene,
    },
    Message,
};

pub struct NodeRemovalDialog {
    pub window: Handle<UiNode>,
    info_text: Handle<UiNode>,
    ok: Handle<UiNode>,
    cancel: Handle<UiNode>,
}

impl NodeRemovalDialog {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let info_text;
        let ok;
        let cancel;
        let text = "You're trying to delete scene node(s), that are referenced in some \
            other scene nodes, which may cause various issues in the engine or your \
            game. Are you sure you want to continue? You can always undo your changes. This \
            validation step could be disabled in editor settings.\n\n
            The full list of reference pairs is listed below:";
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(400.0).with_height(500.0))
            .open(false)
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(
                            TextBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(0)
                                    .with_margin(Thickness::uniform(1.0)),
                            )
                            .with_wrap(WrapMode::Word)
                            .with_text(text)
                            .build(ctx),
                        )
                        .with_child(
                            ScrollViewerBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(1)
                                    .with_margin(Thickness::uniform(1.0)),
                            )
                            .with_content({
                                info_text = TextBoxBuilder::new(
                                    WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                                )
                                .with_multiline(true)
                                .with_wrap(WrapMode::Word)
                                .build(ctx);
                                info_text
                            })
                            .build(ctx),
                        )
                        .with_child(
                            StackPanelBuilder::new(
                                WidgetBuilder::new()
                                    .with_horizontal_alignment(HorizontalAlignment::Right)
                                    .on_row(2)
                                    .with_margin(Thickness::uniform(1.0))
                                    .with_child({
                                        ok = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_width(100.0)
                                                .with_height(20.0)
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_text("OK")
                                        .build(ctx);
                                        ok
                                    })
                                    .with_child({
                                        cancel = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_width(100.0)
                                                .with_height(20.0)
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_text("Cancel")
                                        .build(ctx);
                                        cancel
                                    }),
                            )
                            .with_orientation(Orientation::Horizontal)
                            .build(ctx),
                        ),
                )
                .add_row(Row::auto())
                .add_row(Row::stretch())
                .add_row(Row::auto())
                .add_column(Column::auto())
                .build(ctx),
            )
            .build(ctx);

        Self {
            info_text,
            window,
            ok,
            cancel,
        }
    }

    pub fn open(&mut self, editor_selection: &Selection, game_scene: &GameScene, engine: &Engine) {
        let ui = &engine.user_interfaces.first();
        let graph = &engine.scenes[game_scene.scene].graph;

        ui.send_message(WindowMessage::open_modal(
            self.window,
            MessageDirection::ToWidget,
            true,
            true,
        ));

        let mut text = String::new();

        let selection = selection_to_delete(editor_selection, game_scene);
        for root in selection.nodes.iter() {
            for node_handle in graph.traverse_handle_iter(*root) {
                let node = &graph[node_handle];
                for reference_handle in graph.find_references_to(node_handle) {
                    let reference = &graph[reference_handle];
                    text += &format!(
                        "Scene node `{}`({}:{}) referenced in `{}`({}:{}) scene node.\n",
                        node.name(),
                        node_handle.index(),
                        node_handle.generation(),
                        reference.name(),
                        reference_handle.index(),
                        reference_handle.generation()
                    );
                }
            }
        }

        ui.send_message(TextMessage::text(
            self.info_text,
            MessageDirection::ToWidget,
            text,
        ));
    }

    pub fn handle_ui_message(
        &mut self,
        editor_selection: &Selection,
        game_scene: &GameScene,
        message: &UiMessage,
        engine: &Engine,
        sender: &MessageSender,
    ) {
        let ui = &engine.user_interfaces.first();
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.ok {
                ui.send_message(WindowMessage::close(
                    self.window,
                    MessageDirection::ToWidget,
                ));

                sender.send(Message::DoCommand(make_delete_selection_command(
                    editor_selection,
                    game_scene,
                    engine,
                )));
            } else if message.destination() == self.cancel {
                ui.send_message(WindowMessage::close(
                    self.window,
                    MessageDirection::ToWidget,
                ));
            }
        }
    }
}
