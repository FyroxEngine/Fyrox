use crate::{
    make_save_file_selector, make_scene_file_filter,
    menu::{create_menu_item, create_menu_item_shortcut, create_root_menu_item},
    scene::EditorScene,
    settings::{Settings, SettingsWindow},
    GameEngine, Message,
};
use rg3d::{
    core::pool::Handle,
    gui::{
        file_browser::FileSelectorBuilder,
        message::{
            FileSelectorMessage, MenuItemMessage, MessageBoxMessage, MessageDirection, UiMessage,
            UiMessageData, WindowMessage,
        },
        messagebox::{MessageBoxBuilder, MessageBoxButtons},
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        UiNode, UserInterface,
    },
};
use std::sync::mpsc::Sender;

pub struct FileMenu {
    pub menu: Handle<UiNode>,
    new_scene: Handle<UiNode>,
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
}

impl FileMenu {
    pub fn new(
        engine: &mut GameEngine,
        message_sender: &Sender<Message>,
        settings: &Settings,
    ) -> Self {
        let new_scene;
        let save;
        let save_as;
        let close_scene;
        let load;
        let open_settings;
        let configure;
        let exit;

        let ctx = &mut engine.user_interface.build_ctx();

        let configure_message = MessageBoxBuilder::new(
            WindowBuilder::new(WidgetBuilder::new().with_width(250.0).with_height(150.0))
                .open(false)
                .with_title(WindowTitle::Text("Warning".to_owned())),
        )
        .with_text("Cannot reconfigure editor while scene is open! Close scene first and retry.")
        .with_buttons(MessageBoxButtons::Ok)
        .build(ctx);

        let menu = create_root_menu_item(
            "File",
            vec![
                {
                    new_scene = create_menu_item_shortcut("New Scene", "Ctrl+N", vec![], ctx);
                    new_scene
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
                    open_settings = create_menu_item("Settings...", vec![], ctx);
                    open_settings
                },
                {
                    configure = create_menu_item("Configure...", vec![], ctx);
                    configure
                },
                {
                    exit = create_menu_item_shortcut("Exit", "Alt+F4", vec![], ctx);
                    exit
                },
            ],
            ctx,
        );

        let save_file_selector = make_save_file_selector(ctx);

        let load_file_selector = FileSelectorBuilder::new(
            WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
                .open(false)
                .with_title(WindowTitle::Text("Select a Scene To Load".into())),
        )
        .with_filter(make_scene_file_filter())
        .build(ctx);

        Self {
            save_file_selector,
            load_file_selector,
            menu,
            new_scene,
            save,
            save_as,
            close_scene,
            load,
            exit,
            open_settings,
            configure,
            configure_message,
            settings: SettingsWindow::new(engine, message_sender.clone(), settings),
        }
    }

    pub fn open_load_file_selector(&self, ui: &mut UserInterface) {
        ui.send_message(WindowMessage::open_modal(
            self.load_file_selector,
            MessageDirection::ToWidget,
            true,
        ));
        ui.send_message(FileSelectorMessage::root(
            self.load_file_selector,
            MessageDirection::ToWidget,
            Some(std::env::current_dir().unwrap()),
        ));
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        sender: &Sender<Message>,
        editor_scene: &Option<&mut EditorScene>,
        engine: &mut GameEngine,
        settings: &mut Settings,
        configurator_window: Handle<UiNode>,
    ) {
        if let Some(scene) = editor_scene.as_ref() {
            self.settings
                .handle_message(message, scene, engine, settings);
        }

        match message.data() {
            UiMessageData::FileSelector(FileSelectorMessage::Commit(path)) => {
                if message.destination() == self.save_file_selector {
                    sender.send(Message::SaveScene(path.to_owned())).unwrap();
                } else if message.destination() == self.load_file_selector {
                    sender.send(Message::LoadScene(path.to_owned())).unwrap();
                }
            }
            UiMessageData::MenuItem(MenuItemMessage::Click) => {
                if message.destination() == self.save {
                    if let Some(scene_path) =
                        editor_scene.as_ref().map(|s| s.path.as_ref()).flatten()
                    {
                        sender.send(Message::SaveScene(scene_path.clone())).unwrap();
                    } else {
                        // If scene wasn't saved yet - open Save As window.
                        engine
                            .user_interface
                            .send_message(WindowMessage::open_modal(
                                self.save_file_selector,
                                MessageDirection::ToWidget,
                                true,
                            ));
                        engine
                            .user_interface
                            .send_message(FileSelectorMessage::path(
                                self.save_file_selector,
                                MessageDirection::ToWidget,
                                std::env::current_dir().unwrap(),
                            ));
                    }
                } else if message.destination() == self.save_as {
                    engine
                        .user_interface
                        .send_message(WindowMessage::open_modal(
                            self.save_file_selector,
                            MessageDirection::ToWidget,
                            true,
                        ));
                    engine
                        .user_interface
                        .send_message(FileSelectorMessage::path(
                            self.save_file_selector,
                            MessageDirection::ToWidget,
                            std::env::current_dir().unwrap(),
                        ));
                } else if message.destination() == self.load {
                    self.open_load_file_selector(&mut engine.user_interface);
                } else if message.destination() == self.close_scene {
                    sender.send(Message::CloseScene).unwrap();
                } else if message.destination() == self.exit {
                    sender.send(Message::Exit { force: false }).unwrap();
                } else if message.destination() == self.new_scene {
                    sender.send(Message::NewScene).unwrap();
                } else if message.destination() == self.configure {
                    if editor_scene.is_none() {
                        engine
                            .user_interface
                            .send_message(WindowMessage::open_modal(
                                configurator_window,
                                MessageDirection::ToWidget,
                                true,
                            ));
                    } else {
                        engine.user_interface.send_message(MessageBoxMessage::open(
                            self.configure_message,
                            MessageDirection::ToWidget,
                            None,
                            None,
                        ));
                    }
                } else if message.destination() == self.open_settings {
                    self.settings.open(&engine.user_interface, settings, None);
                }
            }
            _ => {}
        }
    }
}
