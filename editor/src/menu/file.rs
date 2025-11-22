// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::{
    asset::preview::cache::IconRequest,
    export::ExportWindow,
    fyrox::{
        core::pool::Handle,
        gui::{
            file_browser::{FileSelectorBuilder, FileSelectorMessage},
            menu,
            menu::MenuItemMessage,
            message::UiMessage,
            messagebox::{MessageBoxBuilder, MessageBoxButtons, MessageBoxMessage},
            widget::{WidgetBuilder, WidgetMessage},
            window::{WindowBuilder, WindowMessage, WindowTitle},
            BuildContext, UiNode, UserInterface,
        },
    },
    make_save_file_selector, make_scene_file_filter,
    menu::{create_menu_item, create_menu_item_shortcut, create_root_menu_item},
    message::MessageSender,
    scene::{container::EditorSceneEntry, GameScene},
    settings::{recent::RecentFiles, Settings},
    Engine, Message, Mode, Panels, SaveSceneConfirmationDialogAction,
};
use fyrox::asset::manager::ResourceManager;
use fyrox::core::{uuid, Uuid};
use std::{path::PathBuf, sync::mpsc::Sender};

pub struct FileMenu {
    pub menu: Handle<UiNode>,
    pub new_scene: Handle<UiNode>,
    pub new_ui_scene: Handle<UiNode>,
    pub save: Handle<UiNode>,
    pub save_as: Handle<UiNode>,
    pub save_all: Handle<UiNode>,
    pub load: Handle<UiNode>,
    pub close_scene: Handle<UiNode>,
    pub exit: Handle<UiNode>,
    pub configure: Handle<UiNode>,
    pub save_file_selector: Handle<UiNode>,
    pub load_file_selector: Handle<UiNode>,
    pub configure_message: Handle<UiNode>,
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
        .map(|f| create_menu_item(f.to_string_lossy().as_ref(), Uuid::new_v4(), vec![], ctx))
        .collect::<Vec<_>>()
}

impl FileMenu {
    pub const FILE: Uuid = uuid!("0e55e166-f3bd-44a9-b89d-083ce2cef255");
    pub const NEW_SCENE: Uuid = uuid!("c9e8025d-6492-4c13-a979-81ddc82dcadb");
    pub const NEW_UI_SCENE: Uuid = uuid!("1acf882c-cc4c-4745-9cce-d23bdc5b5ced");
    pub const LOAD_SCENE: Uuid = uuid!("8be5e160-7f60-4678-afcb-5a03d2914fe4");
    pub const OPEN_RECENT: Uuid = uuid!("112fc0ab-0b0e-4bdf-b6d0-b91ded556f01");
    pub const SAVE_SCENE: Uuid = uuid!("c8e575db-7de1-4150-a450-df034ddf3431");
    pub const SAVE_SCENE_AS: Uuid = uuid!("83dd70bc-d269-4904-97cd-489fa67efff2");
    pub const SAVE_ALL: Uuid = uuid!("48bd5fa6-0e64-4554-b834-68f7b322f842");
    pub const CLOSE_SCENE: Uuid = uuid!("f90803c3-4ff0-4d68-8ebd-c3c24eaa6693");
    pub const SCENE_SETTINGS: Uuid = uuid!("aa6743d0-f965-4589-9937-e0b79a350282");
    pub const CONFIGURE: Uuid = uuid!("220274af-bc1d-47d2-8a1d-7ee4b2d409f4");
    pub const EXPORT_PROJECT: Uuid = uuid!("699b39ae-a9c2-4af4-9ade-c36a14f96aa2");
    pub const EXIT: Uuid = uuid!("cf62b116-37fd-4620-8594-9ca04735d45b");
    pub const SAVE_FILE_SELECTOR: Uuid = uuid!("ddb9df20-ec54-4ebd-9493-fe06b9ac4ab2");

    pub fn new(engine: &mut Engine, settings: &Settings) -> Self {
        let new_scene;
        let new_ui_scene;
        let save;
        let save_as;
        let save_all;
        let close_scene;
        let load;
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
            Self::FILE,
            vec![
                {
                    new_scene = create_menu_item_shortcut(
                        "New Scene",
                        Self::NEW_SCENE,
                        "Ctrl+N",
                        vec![],
                        ctx,
                    );
                    new_scene
                },
                {
                    new_ui_scene =
                        create_menu_item("New UI Scene", Self::NEW_UI_SCENE, vec![], ctx);
                    new_ui_scene
                },
                {
                    load = create_menu_item_shortcut(
                        "Load Scene...",
                        Self::LOAD_SCENE,
                        "Ctrl+L",
                        vec![],
                        ctx,
                    );
                    load
                },
                {
                    recent_files_container = create_menu_item(
                        "Open Recent Scene",
                        Self::OPEN_RECENT,
                        recent_files.clone(),
                        ctx,
                    );
                    recent_files_container
                },
                menu::make_menu_splitter(ctx),
                {
                    save = create_menu_item_shortcut(
                        "Save Scene",
                        Self::SAVE_SCENE,
                        "Ctrl+S",
                        vec![],
                        ctx,
                    );
                    save
                },
                {
                    save_as = create_menu_item_shortcut(
                        "Save Scene As...",
                        Self::SAVE_SCENE_AS,
                        "Ctrl+Shift+S",
                        vec![],
                        ctx,
                    );
                    save_as
                },
                {
                    save_all = create_menu_item_shortcut(
                        "Save All",
                        Self::SAVE_ALL,
                        "Ctrl+Alt+S",
                        vec![],
                        ctx,
                    );
                    save_all
                },
                menu::make_menu_splitter(ctx),
                {
                    close_scene = create_menu_item_shortcut(
                        "Close Current Scene",
                        Self::CLOSE_SCENE,
                        "Ctrl+Q",
                        vec![],
                        ctx,
                    );
                    close_scene
                },
                {
                    open_scene_settings = create_menu_item(
                        "Current Scene Settings...",
                        Self::SCENE_SETTINGS,
                        vec![],
                        ctx,
                    );
                    open_scene_settings
                },
                menu::make_menu_splitter(ctx),
                {
                    configure =
                        create_menu_item("Configure Editor...", Self::CONFIGURE, vec![], ctx);
                    configure
                },
                {
                    export_project =
                        create_menu_item("Export Project...", Self::EXPORT_PROJECT, vec![], ctx);
                    export_project
                },
                {
                    exit = create_menu_item_shortcut("Exit", Self::EXIT, "Alt+F4", vec![], ctx);
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
            configure,
            configure_message,
            recent_files_container,
            recent_files,
            open_scene_settings,
            export_project,
            save_all,
        }
    }

    pub fn update_recent_files_list(&mut self, ui: &mut UserInterface, settings: &Settings) {
        self.recent_files = make_recent_files_items(&mut ui.build_ctx(), &settings.recent);
        ui.send(
            self.recent_files_container,
            MenuItemMessage::Items(self.recent_files.clone()),
        );
    }

    pub fn open_load_file_selector(&self, ui: &mut UserInterface) {
        ui.send(
            self.load_file_selector,
            WindowMessage::OpenModal {
                center: true,
                focus_content: true,
            },
        );
        ui.send(
            self.load_file_selector,
            FileSelectorMessage::Root(Some(std::env::current_dir().unwrap())),
        );
    }

    pub fn open_save_file_selector(
        &mut self,
        ui: &mut UserInterface,
        resource_manager: &ResourceManager,
        default_file_name: PathBuf,
    ) {
        self.save_file_selector = make_save_file_selector(
            &mut ui.build_ctx(),
            default_file_name,
            Self::SAVE_FILE_SELECTOR,
        );

        ui.send(
            self.save_file_selector,
            WindowMessage::OpenModal {
                center: true,
                focus_content: true,
            },
        );
        let registry_dir = resource_manager
            .state()
            .resource_registry
            .lock()
            .directory()
            .unwrap()
            .to_path_buf();
        ui.send_many(
            self.save_file_selector,
            [
                FileSelectorMessage::Path(registry_dir.clone()),
                FileSelectorMessage::Root(Some(registry_dir)),
            ],
        );
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        sender: &MessageSender,
        entry: Option<&mut EditorSceneEntry>,
        engine: &mut Engine,
        settings: &mut Settings,
        panels: &mut Panels,
        icon_request_sender: Sender<IconRequest>,
    ) {
        if let Some(FileSelectorMessage::Commit(path)) = message.data::<FileSelectorMessage>() {
            if message.destination() == self.save_file_selector {
                if let Some(game_scene) = entry {
                    sender.send(Message::SaveScene {
                        id: game_scene.id,
                        path: path.to_owned(),
                    });
                }
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
                            &engine.resource_manager,
                            entry.default_file_name(),
                        );
                    }
                }
            } else if message.destination() == self.save_as {
                if let Some(entry) = entry {
                    self.open_save_file_selector(
                        engine.user_interfaces.first_mut(),
                        &engine.resource_manager,
                        entry.default_file_name(),
                    );
                }
            } else if message.destination() == self.save_all {
                sender.send(Message::SaveAllScenes);
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
                    engine.user_interfaces.first().send(
                        panels.configurator_window,
                        WindowMessage::OpenModal {
                            center: true,
                            focus_content: true,
                        },
                    );
                } else {
                    engine.user_interfaces.first_mut().send(
                        self.configure_message,
                        MessageBoxMessage::Open {
                            title: None,
                            text: None,
                        },
                    );
                }
            } else if message.destination() == self.export_project {
                let export_window =
                    ExportWindow::new(&mut engine.user_interfaces.first_mut().build_ctx());
                export_window.open(engine.user_interfaces.first());
                *panels.export_window = Some(export_window);
            } else if message.destination() == self.open_scene_settings {
                if let Some(game_scene) = entry {
                    if let Some(game_scene) = game_scene.controller.downcast_ref::<GameScene>() {
                        panels.scene_settings.open(
                            game_scene,
                            engine,
                            sender.clone(),
                            icon_request_sender,
                        );
                    }
                }
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
        ui.send(self.menu, WidgetMessage::Enabled(mode.is_edit()));
    }
}
