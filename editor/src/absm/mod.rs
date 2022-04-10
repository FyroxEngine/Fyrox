use crate::absm::menu::Menu;
use crate::{
    absm::{
        command::{AbsmCommand, AbsmCommandStack, AbsmEditorContext},
        inspector::Inspector,
        message::{AbsmMessage, MessageSender},
        node::{AbsmNode, AbsmNodeMessage},
        preview::Previewer,
        state_graph::Document,
        state_viewer::StateViewer,
    },
    utils::{create_file_selector, open_file_selector},
    Message,
};
use fyrox::animation::machine::node::PoseNodeDefinition;
use fyrox::{
    animation::machine::{
        state::StateDefinition, transition::TransitionDefinition, MachineDefinition,
    },
    core::{
        color::Color,
        futures::executor::block_on,
        pool::Handle,
        visitor::{Visit, VisitResult, Visitor},
    },
    engine::Engine,
    gui::{
        dock::{DockingManagerBuilder, TileBuilder, TileContent},
        file_browser::{FileBrowserMode, FileSelectorMessage},
        grid::{Column, GridBuilder, Row},
        message::UiMessage,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        UiNode, UserInterface,
    },
    utils::log::Log,
};
use std::{
    path::{Path, PathBuf},
    sync::mpsc::{channel, Receiver, Sender},
};

mod canvas;
mod command;
mod inspector;
mod menu;
mod message;
mod node;
mod preview;
mod selectable;
mod socket;
mod state_graph;
mod state_viewer;
mod transition;

const NORMAL_BACKGROUND: Color = Color::opaque(60, 60, 60);
const SELECTED_BACKGROUND: Color = Color::opaque(80, 80, 80);
const BORDER_COLOR: Color = Color::opaque(70, 70, 70);

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum SelectedEntity {
    Transition(Handle<TransitionDefinition>),
    State(Handle<StateDefinition>),
    PoseNode(Handle<PoseNodeDefinition>),
}

#[derive(Default)]
pub struct AbsmDataModel {
    path: PathBuf,
    preview_model_path: PathBuf,
    selection: Vec<SelectedEntity>,
    absm_definition: MachineDefinition,
}

impl AbsmDataModel {
    pub fn ctx(&mut self) -> AbsmEditorContext {
        AbsmEditorContext {
            selection: &mut self.selection,
            definition: &mut self.absm_definition,
        }
    }

    // Manual implementation is needed to store editor data alongside the engine data.
    fn visit(&mut self, visitor: &mut Visitor) -> VisitResult {
        // Visit engine data first.
        self.absm_definition.visit("Machine", visitor)?;

        // Visit editor-specific data. These fields are optional so we ignore any errors here.
        let _ = self.preview_model_path.visit("PreviewModelPath", visitor);

        Ok(())
    }
}

pub struct AbsmEditor {
    #[allow(dead_code)] // TODO
    window: Handle<UiNode>,
    command_stack: AbsmCommandStack,
    data_model: Option<AbsmDataModel>,
    message_sender: MessageSender,
    message_receiver: Receiver<AbsmMessage>,
    inspector: Inspector,
    document: Document,
    save_dialog: Handle<UiNode>,
    load_dialog: Handle<UiNode>,
    previewer: Previewer,
    state_viewer: StateViewer,
    menu: Menu,
}

impl AbsmEditor {
    pub fn new(engine: &mut Engine) -> Self {
        let (tx, rx) = channel();

        let previewer = Previewer::new(engine);

        let ui = &mut engine.user_interface;
        let ctx = &mut ui.build_ctx();

        let menu = Menu::new(ctx);

        let inspector = Inspector::new(ctx);
        let document = Document::new(ctx);
        let state_viewer = StateViewer::new(ctx);

        let docking_manager = DockingManagerBuilder::new(
            WidgetBuilder::new().on_row(1).with_child(
                TileBuilder::new(WidgetBuilder::new())
                    .with_content(TileContent::HorizontalTiles {
                        splitter: 0.8,
                        tiles: [
                            TileBuilder::new(WidgetBuilder::new())
                                .with_content(TileContent::HorizontalTiles {
                                    splitter: 0.3,
                                    tiles: [
                                        TileBuilder::new(WidgetBuilder::new())
                                            .with_content(TileContent::Window(previewer.window))
                                            .build(ctx),
                                        TileBuilder::new(WidgetBuilder::new())
                                            .with_content(TileContent::HorizontalTiles {
                                                splitter: 0.5,
                                                tiles: [
                                                    TileBuilder::new(WidgetBuilder::new())
                                                        .with_content(TileContent::Window(
                                                            document.window,
                                                        ))
                                                        .build(ctx),
                                                    TileBuilder::new(WidgetBuilder::new())
                                                        .with_content(TileContent::Window(
                                                            state_viewer.window,
                                                        ))
                                                        .build(ctx),
                                                ],
                                            })
                                            .build(ctx),
                                    ],
                                })
                                .build(ctx),
                            TileBuilder::new(WidgetBuilder::new())
                                .with_content(TileContent::Window(inspector.window))
                                .build(ctx),
                        ],
                    })
                    .build(ctx),
            ),
        )
        .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(1600.0).with_height(800.0))
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(menu.menu)
                        .with_child(docking_manager),
                )
                .add_row(Row::strict(24.0))
                .add_row(Row::stretch())
                .add_column(Column::stretch())
                .build(ctx),
            )
            .with_title(WindowTitle::text("ABSM Editor"))
            .build(ctx);

        let load_dialog = create_file_selector(ctx, "absm", FileBrowserMode::Open);
        let save_dialog = create_file_selector(
            ctx,
            "absm",
            FileBrowserMode::Save {
                default_file_name: PathBuf::from("unnamed.absm"),
            },
        );

        Self {
            window,
            message_sender: MessageSender::new(tx),
            message_receiver: rx,
            command_stack: AbsmCommandStack::new(false),
            data_model: None,
            menu,
            document,
            inspector,
            save_dialog,
            load_dialog,
            previewer,
            state_viewer,
        }
    }

    fn sync_to_model(&mut self, ui: &mut UserInterface, sender: Sender<Message>) {
        if let Some(data_model) = self.data_model.as_ref() {
            self.document.sync_to_model(data_model, ui);
            self.state_viewer
                .sync_to_model(&data_model.absm_definition, ui, data_model);
            self.inspector.sync_to_model(ui, data_model, sender);
        }
    }

    fn do_command(&mut self, command: AbsmCommand) -> bool {
        if let Some(data_model) = self.data_model.as_mut() {
            self.command_stack
                .do_command(command.into_inner(), data_model.ctx());
            true
        } else {
            false
        }
    }

    fn undo_command(&mut self) -> bool {
        if let Some(data_model) = self.data_model.as_mut() {
            self.command_stack.undo(data_model.ctx());
            true
        } else {
            false
        }
    }

    fn redo_command(&mut self) -> bool {
        if let Some(data_model) = self.data_model.as_mut() {
            self.command_stack.redo(data_model.ctx());
            true
        } else {
            false
        }
    }

    fn clear_command_stack(&mut self) -> bool {
        if let Some(data_model) = self.data_model.as_mut() {
            self.command_stack.clear(data_model.ctx());
            true
        } else {
            false
        }
    }

    fn create_new_absm(&mut self) {
        self.clear_command_stack();

        self.data_model = Some(AbsmDataModel::default());
    }

    fn open_save_dialog(&self, ui: &UserInterface) {
        open_file_selector(self.save_dialog, ui);
    }

    fn open_load_dialog(&self, ui: &UserInterface) {
        open_file_selector(self.load_dialog, ui);
    }

    fn save_current_absm(&mut self, path: PathBuf) {
        if let Some(data_model) = self.data_model.as_mut() {
            data_model.path = path.clone();

            let mut visitor = Visitor::new();
            Log::verify(data_model.visit(&mut visitor));
            Log::verify(visitor.save_binary(path));
        }
    }

    fn set_preview_model(&mut self, engine: &mut Engine, path: &Path) {
        if let Some(data_model) = self.data_model.as_mut() {
            self.previewer
                .set_preview_model(engine, path, &data_model.absm_definition);

            data_model.preview_model_path = path.to_path_buf();
        }
    }

    fn load_absm(&mut self, path: &Path, engine: &mut Engine) {
        match block_on(Visitor::load_binary(path)) {
            Ok(mut visitor) => {
                let mut data_model = AbsmDataModel::default();
                if let Err(e) = data_model.visit(&mut visitor) {
                    Log::err(format!(
                        "Unable to read ABSM from {}. Reason: {}",
                        path.display(),
                        e
                    ));
                } else {
                    data_model.path = path.to_path_buf();
                    let preview_model_path = data_model.preview_model_path.clone();
                    self.data_model = Some(data_model);
                    self.message_sender.sync();
                    self.set_preview_model(engine, &preview_model_path);
                }
            }
            Err(e) => Log::err(format!(
                "Unable to load ABSM from {}. Reason: {}",
                path.display(),
                e
            )),
        };
    }

    pub fn update(&mut self, engine: &mut Engine, sender: Sender<Message>) {
        let mut need_sync = false;

        while let Ok(message) = self.message_receiver.try_recv() {
            match message {
                AbsmMessage::DoCommand(command) => {
                    need_sync |= self.do_command(command);
                }
                AbsmMessage::Undo => {
                    need_sync |= self.undo_command();
                }
                AbsmMessage::Redo => {
                    need_sync |= self.redo_command();
                }
                AbsmMessage::ClearCommandStack => {
                    need_sync |= self.clear_command_stack();
                }
                AbsmMessage::CreateNewAbsm => {
                    self.create_new_absm();
                    need_sync = true;
                }
                AbsmMessage::LoadAbsm => {
                    self.open_load_dialog(&engine.user_interface);
                }
                AbsmMessage::SaveCurrentAbsm => {
                    if let Some(data_model) = self.data_model.as_ref() {
                        if data_model.path.exists() {
                            let path = data_model.path.clone();
                            self.save_current_absm(path)
                        } else {
                            self.open_save_dialog(&engine.user_interface);
                        }
                    }
                }
                AbsmMessage::Sync => {
                    need_sync = true;
                }
                AbsmMessage::SetPreviewModel(path) => self.set_preview_model(engine, &path),
            }
        }

        if need_sync {
            self.sync_to_model(&mut engine.user_interface, sender);
        }

        self.previewer.update(engine);
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, engine: &mut Engine) {
        self.previewer
            .handle_message(message, &self.message_sender, engine);

        let ui = &mut engine.user_interface;
        self.menu.handle_ui_message(&self.message_sender, message);

        if let Some(data_model) = self.data_model.as_ref() {
            self.state_viewer
                .handle_ui_message(message, ui, &self.message_sender, data_model);
            self.document
                .handle_ui_message(message, ui, &self.message_sender, data_model);
        }

        if let Some(FileSelectorMessage::Commit(path)) = message.data() {
            if message.destination() == self.save_dialog {
                self.save_current_absm(path.clone())
            } else if message.destination() == self.load_dialog {
                self.load_absm(path, engine);
            }
        } else if let Some(AbsmNodeMessage::Enter) = message.data() {
            if let Some(node) = ui
                .node(message.destination())
                .query_component::<AbsmNode<StateDefinition>>()
            {
                self.state_viewer.set_state(node.model_handle);
                self.message_sender.sync();
            }
        }
    }
}
