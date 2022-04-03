use crate::{
    absm::{
        canvas::{AbsmCanvas, AbsmCanvasMessage},
        command::{AbsmCommand, AbsmCommandStack, AbsmEditorContext},
        document::Document,
        inspector::Inspector,
        menu::{
            context::{CanvasContextMenu, NodeContextMenu},
            Menu,
        },
        message::{AbsmMessage, MessageSender},
        node::{AbsmStateNode, AbsmStateNodeBuilder, AbsmStateNodeMessage},
        transition::{Transition, TransitionBuilder},
    },
    send_sync_message,
    utils::create_file_selector,
};
use fyrox::{
    animation::machine::{
        state::StateDefinition, transition::TransitionDefinition, MachineDefinition,
    },
    core::{
        color::Color,
        futures::executor::block_on,
        pool::Handle,
        visitor::{Visit, Visitor},
    },
    engine::Engine,
    gui::{
        dock::{DockingManagerBuilder, TileBuilder, TileContent},
        file_browser::{FileBrowserMode, FileSelectorMessage},
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessage},
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        UiNode, UserInterface,
    },
    utils::log::Log,
};
use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
    sync::mpsc::{channel, Receiver},
};

mod canvas;
mod command;
mod document;
mod inspector;
mod menu;
mod message;
mod node;
mod transition;

const NORMAL_BACKGROUND: Color = Color::opaque(60, 60, 60);
const SELECTED_BACKGROUND: Color = Color::opaque(80, 80, 80);
const BORDER_COLOR: Color = Color::opaque(70, 70, 70);

#[derive(PartialEq, Eq, Debug)]
pub enum SelectedEntity {
    Transition(Handle<TransitionDefinition>),
    State(Handle<StateDefinition>),
}

#[derive(Default)]
pub struct AbsmDataModel {
    path: PathBuf,
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
}

pub struct AbsmEditor {
    #[allow(dead_code)] // TODO
    window: Handle<UiNode>,
    canvas_context_menu: CanvasContextMenu,
    node_context_menu: NodeContextMenu,
    command_stack: AbsmCommandStack,
    data_model: Option<AbsmDataModel>,
    menu: Menu,
    message_sender: MessageSender,
    message_receiver: Receiver<AbsmMessage>,
    inspector: Inspector,
    document: Document,
    save_dialog: Handle<UiNode>,
    load_dialog: Handle<UiNode>,
}

impl AbsmEditor {
    pub fn new(ui: &mut UserInterface) -> Self {
        let (tx, rx) = channel();

        let ctx = &mut ui.build_ctx();
        let mut node_context_menu = NodeContextMenu::new(ctx);
        let mut canvas_context_menu = CanvasContextMenu::new(ctx);
        let menu = Menu::new(ctx);

        let inspector = Inspector::new(ctx);
        let document = Document::new(canvas_context_menu.menu, ctx);

        let docking_manager = DockingManagerBuilder::new(
            WidgetBuilder::new().on_row(1).with_child(
                TileBuilder::new(WidgetBuilder::new())
                    .with_content(TileContent::HorizontalTiles {
                        splitter: 0.7,
                        tiles: [
                            TileBuilder::new(WidgetBuilder::new())
                                .with_content(TileContent::Window(document.window))
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

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(700.0).with_height(400.0))
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

        canvas_context_menu.canvas = document.canvas;
        canvas_context_menu.node_context_menu = node_context_menu.menu;
        node_context_menu.canvas = document.canvas;

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
            canvas_context_menu,
            node_context_menu,
            message_sender: MessageSender::new(tx),
            message_receiver: rx,
            command_stack: AbsmCommandStack::new(false),
            data_model: None,
            menu,
            document,
            inspector,
            save_dialog,
            load_dialog,
        }
    }

    fn sync_to_model(&mut self, ui: &mut UserInterface) {
        if let Some(data_model) = self.data_model.as_ref() {
            let definition = &data_model.absm_definition;

            let canvas = ui
                .node(self.document.canvas)
                .cast::<AbsmCanvas>()
                .expect("Must be AbsmCanvas!");

            let mut states = canvas
                .children()
                .iter()
                .cloned()
                .filter(|c| ui.node(*c).has_component::<AbsmStateNode>())
                .collect::<Vec<_>>();

            let mut transitions = canvas
                .children()
                .iter()
                .cloned()
                .filter(|c| ui.node(*c).has_component::<Transition>())
                .collect::<Vec<_>>();

            match states
                .len()
                .cmp(&(definition.states.alive_count() as usize))
            {
                Ordering::Less => {
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
                            .with_name(state.name.clone())
                            .build(state_handle, &mut ui.build_ctx());

                            states.push(state_view_handle);

                            send_sync_message(
                                ui,
                                WidgetMessage::link(
                                    state_view_handle,
                                    MessageDirection::ToWidget,
                                    self.document.canvas,
                                ),
                            );
                        }
                    }
                }
                Ordering::Greater => {
                    // A state was removed.
                    for (state_view_handle, state_model_handle) in
                        states.clone().iter().cloned().map(|state_view| {
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
                            send_sync_message(
                                ui,
                                WidgetMessage::remove(
                                    state_view_handle,
                                    MessageDirection::ToWidget,
                                ),
                            );

                            if let Some(position) =
                                states.iter().position(|s| *s == state_view_handle)
                            {
                                states.remove(position);
                            }
                        }
                    }
                }
                _ => (),
            }

            // Sync state nodes.
            for state in states.iter() {
                let state_node = ui.node(*state).query_component::<AbsmStateNode>().unwrap();
                let state_model_ref = &definition.states[state_node.model_handle];

                if state_model_ref.name != state_node.name {
                    send_sync_message(
                        ui,
                        AbsmStateNodeMessage::name(
                            *state,
                            MessageDirection::ToWidget,
                            state_node.name.clone(),
                        ),
                    );
                }

                send_sync_message(
                    ui,
                    WidgetMessage::desired_position(
                        *state,
                        MessageDirection::ToWidget,
                        state_model_ref.position,
                    ),
                );
            }

            // Force update layout to be able to fetch positions of nodes for transitions.
            ui.update(ui.screen_size(), 0.0);

            // Sync transitions.
            match transitions
                .len()
                .cmp(&(definition.transitions.alive_count() as usize))
            {
                Ordering::Less => {
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

                            send_sync_message(
                                ui,
                                WidgetMessage::link(
                                    transition_view,
                                    MessageDirection::ToWidget,
                                    self.document.canvas,
                                ),
                            );

                            send_sync_message(
                                ui,
                                WidgetMessage::lowermost(
                                    transition_view,
                                    MessageDirection::ToWidget,
                                ),
                            );

                            transitions.push(transition_view);
                        }
                    }
                }

                Ordering::Greater => {
                    // A transition was removed.
                    for (transition_view_handle, transition_model_handle) in
                        transitions.clone().iter().cloned().map(|transition_view| {
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
                            send_sync_message(
                                ui,
                                WidgetMessage::remove(
                                    transition_view_handle,
                                    MessageDirection::ToWidget,
                                ),
                            );

                            if let Some(position) = transitions
                                .iter()
                                .position(|s| *s == transition_view_handle)
                            {
                                transitions.remove(position);
                            }
                        }
                    }
                }
                Ordering::Equal => {}
            }

            // Sync selection.
            let new_selection = data_model
                .selection
                .iter()
                .map(|entry| match entry {
                    SelectedEntity::Transition(transition) => transitions
                        .iter()
                        .cloned()
                        .find(|t| {
                            ui.node(*t)
                                .query_component::<Transition>()
                                .unwrap()
                                .model_handle
                                == *transition
                        })
                        .unwrap(),
                    SelectedEntity::State(state) => states
                        .iter()
                        .cloned()
                        .find(|s| {
                            ui.node(*s)
                                .query_component::<AbsmStateNode>()
                                .unwrap()
                                .model_handle
                                == *state
                        })
                        .unwrap(),
                })
                .collect::<Vec<_>>();
            send_sync_message(
                ui,
                AbsmCanvasMessage::selection_changed(
                    self.document.canvas,
                    MessageDirection::ToWidget,
                    new_selection,
                ),
            );
        }
    }

    fn do_command(&mut self, mut command: AbsmCommand) -> bool {
        if let Some(data_model) = self.data_model.as_mut() {
            dbg!(command.name(&data_model.ctx()));

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
        ui.send_message(FileSelectorMessage::root(
            self.save_dialog,
            MessageDirection::ToWidget,
            Some(std::env::current_dir().unwrap()),
        ));

        ui.send_message(WindowMessage::open_modal(
            self.save_dialog,
            MessageDirection::ToWidget,
            true,
        ));
    }

    fn open_load_dialog(&self, ui: &UserInterface) {
        ui.send_message(FileSelectorMessage::root(
            self.load_dialog,
            MessageDirection::ToWidget,
            Some(std::env::current_dir().unwrap()),
        ));

        ui.send_message(WindowMessage::open_modal(
            self.load_dialog,
            MessageDirection::ToWidget,
            true,
        ));
    }

    fn save_current_absm(&mut self, path: PathBuf) {
        if let Some(data_model) = self.data_model.as_mut() {
            data_model.path = path.clone();

            let mut visitor = Visitor::new();
            Log::verify(data_model.absm_definition.visit("Machine", &mut visitor));
            Log::verify(visitor.save_binary(path));
        }
    }

    fn load_absm(&mut self, path: &Path) {
        match block_on(Visitor::load_binary(path)) {
            Ok(mut visitor) => {
                let mut absm = MachineDefinition::default();
                if let Err(e) = absm.visit("Machine", &mut visitor) {
                    Log::err(format!(
                        "Unable to read ABSM from {}. Reason: {}",
                        path.display(),
                        e
                    ));
                } else {
                    self.data_model = Some(AbsmDataModel {
                        path: path.to_path_buf(),
                        absm_definition: absm,
                        selection: Default::default(),
                    });

                    self.message_sender.sync();
                }
            }
            Err(e) => Log::err(format!(
                "Unable to load ABSM from {}. Reason: {}",
                path.display(),
                e
            )),
        };
    }

    pub fn update(&mut self, engine: &mut Engine) {
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
            }
        }

        if need_sync {
            self.sync_to_model(&mut engine.user_interface);
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, ui: &mut UserInterface) {
        self.menu.handle_ui_message(&self.message_sender, message);
        self.node_context_menu.handle_ui_message(message, ui);
        self.canvas_context_menu
            .handle_ui_message(&self.message_sender, message, ui);

        if let Some(data_model) = self.data_model.as_ref() {
            self.document
                .handle_ui_message(message, ui, &self.message_sender, data_model);
        }

        if let Some(FileSelectorMessage::Commit(path)) = message.data() {
            if message.destination() == self.save_dialog {
                self.save_current_absm(path.clone())
            } else if message.destination() == self.load_dialog {
                self.load_absm(path);
            }
        }
    }
}
