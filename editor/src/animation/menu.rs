use crate::animation::{message::Message, DataModel};
use fyrox::{
    asset::ResourceData,
    core::pool::Handle,
    gui::{
        file_browser::{FileBrowserMode, FileSelectorBuilder, FileSelectorMessage, Filter},
        menu::{MenuBuilder, MenuItemBuilder, MenuItemContent, MenuItemMessage},
        message::{MessageDirection, UiMessage},
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, UiNode, UserInterface,
    },
};
use std::{
    path::{Path, PathBuf},
    sync::mpsc::Sender,
};

pub struct Menu {
    pub menu: Handle<UiNode>,
    new: Handle<UiNode>,
    load: Handle<UiNode>,
    save: Handle<UiNode>,
    save_as: Handle<UiNode>,
    exit: Handle<UiNode>,
    undo: Handle<UiNode>,
    redo: Handle<UiNode>,
    clear_command_stack: Handle<UiNode>,
    save_file_dialog: Handle<UiNode>,
    load_file_dialog: Handle<UiNode>,
}

pub fn make_file_dialog(
    title: &str,
    mode: FileBrowserMode,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    FileSelectorBuilder::new(
        WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
            .with_title(WindowTitle::text(title))
            .open(false),
    )
    .with_mode(mode)
    .with_path("./")
    .with_filter(Filter::new(|p: &Path| {
        if let Some(ext) = p.extension() {
            ext.to_string_lossy().as_ref() == "anim"
        } else {
            p.is_dir()
        }
    }))
    .build(ctx)
}

impl Menu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let save_file_dialog = make_file_dialog(
            "Save Animation As",
            FileBrowserMode::Save {
                default_file_name: PathBuf::from("unnamed.anim"),
            },
            ctx,
        );
        let load_file_dialog = make_file_dialog("Load Animation", FileBrowserMode::Open, ctx);

        let new;
        let load;
        let save;
        let save_as;
        let exit;
        let undo;
        let redo;
        let clear_command_stack;
        let menu = MenuBuilder::new(WidgetBuilder::new().on_row(0).on_column(0))
            .with_items(vec![
                MenuItemBuilder::new(WidgetBuilder::new())
                    .with_content(MenuItemContent::text_no_arrow("File"))
                    .with_items(vec![
                        {
                            new = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text_no_arrow("New"))
                                .build(ctx);
                            new
                        },
                        {
                            load = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text_no_arrow("Load..."))
                                .build(ctx);
                            load
                        },
                        {
                            save = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text_no_arrow("Save"))
                                .build(ctx);
                            save
                        },
                        {
                            save_as = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text_no_arrow("Save As..."))
                                .build(ctx);
                            save_as
                        },
                        {
                            exit = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text_no_arrow("Exit"))
                                .build(ctx);
                            exit
                        },
                    ])
                    .build(ctx),
                MenuItemBuilder::new(WidgetBuilder::new())
                    .with_content(MenuItemContent::text_no_arrow("Edit"))
                    .with_items(vec![
                        {
                            undo = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text_no_arrow("Undo"))
                                .build(ctx);
                            undo
                        },
                        {
                            redo = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text_no_arrow("Redo"))
                                .build(ctx);
                            redo
                        },
                        {
                            clear_command_stack = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text_no_arrow("Redo"))
                                .build(ctx);
                            clear_command_stack
                        },
                    ])
                    .build(ctx),
            ])
            .build(ctx);

        Self {
            menu,
            new,
            load,
            save,
            save_as,
            exit,
            undo,
            redo,
            clear_command_stack,
            save_file_dialog,
            load_file_dialog,
        }
    }
}

impl Menu {
    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        ui: &UserInterface,
        sender: &Sender<Message>,
        data_model: Option<&DataModel>,
    ) {
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.new {
                sender.send(Message::NewAnimation).unwrap();
            } else if message.destination() == self.load {
                self.open_load_file_dialog(ui);
            } else if message.destination() == self.save {
                if let Some(data_model) = data_model {
                    if !data_model.saved
                        && data_model.resource.data_ref().path() == PathBuf::default()
                    {
                        self.open_save_file_dialog(ui);
                    } else {
                        sender
                            .send(Message::Save(
                                data_model.resource.data_ref().path().to_path_buf(),
                            ))
                            .unwrap();
                    }
                }
            } else if message.destination() == self.save_as {
                self.open_save_file_dialog(ui);
            } else if message.destination() == self.exit {
                sender.send(Message::Exit).unwrap();
            } else if message.destination() == self.undo {
                sender.send(Message::Undo).unwrap();
            } else if message.destination() == self.redo {
                sender.send(Message::Redo).unwrap();
            } else if message.destination() == self.clear_command_stack {
                sender.send(Message::ClearCommandStack).unwrap();
            }
        } else if let Some(FileSelectorMessage::Commit(path)) = message.data() {
            if message.destination() == self.save_file_dialog {
                sender.send(Message::Save(path.clone())).unwrap();
            } else if message.destination() == self.load_file_dialog {
                sender.send(Message::Load(path.clone())).unwrap();
            }
        }
    }

    pub fn open_load_file_dialog(&self, ui: &UserInterface) {
        ui.send_message(FileSelectorMessage::root(
            self.load_file_dialog,
            MessageDirection::ToWidget,
            std::env::current_dir().ok(),
        ));
        ui.send_message(WindowMessage::open_modal(
            self.load_file_dialog,
            MessageDirection::ToWidget,
            true,
        ));
    }

    pub fn open_save_file_dialog(&self, ui: &UserInterface) {
        ui.send_message(FileSelectorMessage::root(
            self.save_file_dialog,
            MessageDirection::ToWidget,
            std::env::current_dir().ok(),
        ));
        ui.send_message(WindowMessage::open_modal(
            self.save_file_dialog,
            MessageDirection::ToWidget,
            true,
        ));
    }
}
