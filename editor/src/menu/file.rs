use crate::export::ExportWindow;
use crate::fyrox::{
    core::pool::Handle,
    gui::{
        file_browser::{FileSelectorBuilder, FileSelectorMessage},
        menu::MenuItemMessage,
        message::{MessageDirection, UiMessage},
        messagebox::{MessageBoxBuilder, MessageBoxButtons, MessageBoxMessage},
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, UiNode, UserInterface,
    },
};
use crate::{
    make_save_file_selector, make_scene_file_filter,
    menu::{create_menu_item, create_menu_item_shortcut, create_root_menu_item},
    message::MessageSender,
    scene::container::EditorSceneEntry,
    settings::{recent::RecentFiles, Settings, SettingsWindow},
    Engine, Message, Mode, Panels, SaveSceneConfirmationDialogAction,
};
use std::path::PathBuf;

pub struct FileMenu {
    pub menu: Handle<UiNode>,
    new_scene: Handle<UiNode>,
    new_ui_scene: Handle<UiNode>,
    pub save: Handle<UiNode>,
    pub save_as: Handle<UiNode>,
    load: Handle<UiNode>,
    pub close_scene: Handle<UiNode>,
    exit: Handle<UiNode>,
    pub open_settings: Handle<UiNode>,
    configure: Handle<UiNode>,
    pub save_file_selector: Handle<UiNode>,
    pub load_file_selector: Handle<UiNode>,
    configure_message: Handle<UiNode>,
    pub settings: SettingsWindow,
    pub recent_files_container: Handle<UiNode>,
    pub recent_files: Vec<Handle<UiNode>>,
    pub open_scene_settings: Handle<UiNode>,
    pub export_project: Handle<UiNode>,
}

fn make_recent_files_items(
    ctx: &mut BuildContext,
    recent_files: &RecentFiles,
) -> Vec<Handle<UiNode>> {
    recent_files
        .scenes
        .iter()
        .map(|f| create_menu_item(f.to_string_lossy().as_ref(), vec![], ctx))
        .collect::<Vec<_>>()
}

impl FileMenu {
    pub fn new(engine: &mut Engine, settings: &Settings) -> Self {
        let new_scene;
        let new_ui_scene;
        let save;
        let save_as;
        let close_scene;
        let load;
        let open_settings;
        let open_scene_settings;
        let configure;
        let exit;
        let recent_files_container;
        let export_project;

        let ctx = &mut engine.user_interfaces.first_mut().build_ctx();

        let configure_message = MessageBoxBuilder::new(
            WindowBuilder::new(WidgetBuilder::new().with_width(250.0).with_height(150.0))
                .open(false)
                .with_title(WindowTitle::text("Warning")),
        )
        .with_text("Cannot reconfigure editor while scene is open! Close scene first and retry.")
        .with_buttons(MessageBoxButtons::Ok)
        .build(ctx);

        let recent_files = make_recent_files_items(ctx, &settings.recent);

        let menu = create_root_menu_item(
            "File",
            vec![
                {
                    new_scene = create_menu_item_shortcut("New Scene", "Ctrl+N", vec![], ctx);
                    new_scene
                },
                {
                    new_ui_scene = create_menu_item("New UI Scene", vec![], ctx);
                    new_ui_scene
                },
                {
                    save = create_menu_item_shortcut("Save Scene", "Ctrl+S", vec![], ctx);
                    save
                },
                {
                    save_as =
                        create_menu_item_shortcut("Save Scene As...", "Ctrl+Shift+S", vec![], ctx);
                    save_as
                },
                {
                    load = create_menu_item_shortcut("Load Scene...", "Ctrl+L", vec![], ctx);
                    load
                },
                {
                    close_scene = create_menu_item_shortcut("Close Scene", "Ctrl+Q", vec![], ctx);
                    close_scene
                },
                {
                    open_settings = create_menu_item("Editor Settings...", vec![], ctx);
                    open_settings
                },
                {
                    open_scene_settings = create_menu_item("Scene Settings...", vec![], ctx);
                    open_scene_settings
                },
                {
                    configure = create_menu_item("Configure...", vec![], ctx);
                    configure
                },
                {
                    export_project = create_menu_item("Export Project...", vec![], ctx);
                    export_project
                },
                {
                    recent_files_container =
                        create_menu_item("Recent Files", recent_files.clone(), ctx);
                    recent_files_container
                },
                {
                    exit = create_menu_item_shortcut("Exit", "Alt+F4", vec![], ctx);
                    exit
                },
            ],
            ctx,
        );

        let load_file_selector = FileSelectorBuilder::new(
            WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
                .open(false)
                .with_title(WindowTitle::text("Select a Scene To Load")),
        )
        .with_filter(make_scene_file_filter())
        .build(ctx);

        Self {
            save_file_selector: Handle::NONE,
            load_file_selector,
            menu,
            new_scene,
            new_ui_scene,
            save,
            save_as,
            close_scene,
            load,
            exit,
            open_settings,
            configure,
            configure_message,
            settings: SettingsWindow::new(engine),
            recent_files_container,
            recent_files,
            open_scene_settings,
            export_project,
        }
    }

    pub fn update_recent_files_list(&mut self, ui: &mut UserInterface, settings: &Settings) {
        self.recent_files = make_recent_files_items(&mut ui.build_ctx(), &settings.recent);
        ui.send_message(MenuItemMessage::items(
            self.recent_files_container,
            MessageDirection::ToWidget,
            self.recent_files.clone(),
        ));
    }

    pub fn open_load_file_selector(&self, ui: &mut UserInterface) {
        ui.send_message(WindowMessage::open_modal(
            self.load_file_selector,
            MessageDirection::ToWidget,
            true,
            true,
        ));
        ui.send_message(FileSelectorMessage::root(
            self.load_file_selector,
            MessageDirection::ToWidget,
            Some(std::env::current_dir().unwrap()),
        ));
    }

    pub fn open_save_file_selector(&mut self, ui: &mut UserInterface, default_file_name: PathBuf) {
        self.save_file_selector = make_save_file_selector(&mut ui.build_ctx(), default_file_name);

        ui.send_message(WindowMessage::open_modal(
            self.save_file_selector,
            MessageDirection::ToWidget,
            true,
            true,
        ));
        ui.send_message(FileSelectorMessage::path(
            self.save_file_selector,
            MessageDirection::ToWidget,
            std::env::current_dir().unwrap(),
        ));
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        sender: &MessageSender,
        entry: Option<&mut EditorSceneEntry>,
        engine: &mut Engine,
        settings: &mut Settings,
        panels: &mut Panels,
    ) {
        self.settings
            .handle_message(message, engine, settings, sender);

        if let Some(FileSelectorMessage::Commit(path)) = message.data::<FileSelectorMessage>() {
            if message.destination() == self.save_file_selector {
                if let Some(game_scene) = entry {
                    sender.send(Message::SaveScene {
                        id: game_scene.id,
                        path: path.to_owned(),
                    });
                }
                engine
                    .user_interfaces
                    .first_mut()
                    .send_message(WidgetMessage::remove(
                        self.save_file_selector,
                        MessageDirection::ToWidget,
                    ));
                self.save_file_selector = Handle::NONE;
            } else if message.destination() == self.load_file_selector {
                sender.send(Message::LoadScene(path.to_owned()));
            }
        } else if let Some(MenuItemMessage::Click) = message.data::<MenuItemMessage>() {
            if message.destination() == self.save {
                if let Some(entry) = entry {
                    if let Some(scene_path) = entry.path.as_ref() {
                        sender.send(Message::SaveScene {
                            id: entry.id,
                            path: scene_path.clone(),
                        });
                    } else {
                        // If scene wasn't saved yet - open Save As window.
                        self.open_save_file_selector(
                            engine.user_interfaces.first_mut(),
                            entry.default_file_name(),
                        );
                    }
                }
            } else if message.destination() == self.save_as {
                if let Some(entry) = entry {
                    self.open_save_file_selector(
                        engine.user_interfaces.first_mut(),
                        entry.default_file_name(),
                    );
                }
            } else if message.destination() == self.load {
                self.open_load_file_selector(engine.user_interfaces.first_mut());
            } else if message.destination() == self.close_scene {
                if let Some(entry) = entry.as_ref() {
                    if entry.need_save() {
                        sender.send(Message::OpenSaveSceneConfirmationDialog {
                            id: entry.id,
                            action: SaveSceneConfirmationDialogAction::CloseScene(entry.id),
                        });
                    } else {
                        sender.send(Message::CloseScene(entry.id));
                    }
                }
            } else if message.destination() == self.exit {
                sender.send(Message::Exit { force: false });
            } else if message.destination() == self.new_scene {
                sender.send(Message::NewScene);
            } else if message.destination() == self.new_ui_scene {
                sender.send(Message::NewUiScene);
            } else if message.destination() == self.configure {
                if entry.is_none() {
                    engine
                        .user_interfaces
                        .first_mut()
                        .send_message(WindowMessage::open_modal(
                            panels.configurator_window,
                            MessageDirection::ToWidget,
                            true,
                            true,
                        ));
                } else {
                    engine
                        .user_interfaces
                        .first_mut()
                        .send_message(MessageBoxMessage::open(
                            self.configure_message,
                            MessageDirection::ToWidget,
                            None,
                            None,
                        ));
                }
            } else if message.destination() == self.export_project {
                let export_window =
                    ExportWindow::new(&mut engine.user_interfaces.first_mut().build_ctx());
                export_window.open(engine.user_interfaces.first());
                *panels.export_window = Some(export_window);
            } else if message.destination() == self.open_settings {
                self.settings
                    .open(engine.user_interfaces.first_mut(), settings, sender);
            } else if message.destination() == self.open_scene_settings {
                panels.scene_settings.open(engine.user_interfaces.first());
            } else if let Some(recent_file) = self
                .recent_files
                .iter()
                .position(|i| *i == message.destination())
            {
                if let Some(recent_file_path) = settings.recent.scenes.get(recent_file) {
                    sender.send(Message::LoadScene(recent_file_path.clone()));
                }
            }
        }
    }

    pub fn on_mode_changed(&mut self, ui: &UserInterface, mode: &Mode) {
        ui.send_message(WidgetMessage::enabled(
            self.menu,
            MessageDirection::ToWidget,
            mode.is_edit(),
        ));
    }
}
