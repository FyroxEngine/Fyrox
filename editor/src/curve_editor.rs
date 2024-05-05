use crate::fyrox::{
    asset::{untyped::ResourceKind, Resource},
    core::{
        color::Color, futures::executor::block_on, math::curve::Curve, pool::Handle,
        type_traits::prelude::*, visitor::prelude::*,
    },
    engine::Engine,
    gui::{
        border::BorderBuilder,
        brush::Brush,
        button::{ButtonBuilder, ButtonMessage},
        curve::{CurveEditorBuilder, CurveEditorMessage},
        file_browser::{FileBrowserMode, FileSelectorMessage},
        grid::{Column, GridBuilder, Row},
        menu::{MenuBuilder, MenuItemBuilder, MenuItemContent, MenuItemMessage},
        message::{MessageDirection, UiMessage},
        messagebox::{MessageBoxBuilder, MessageBoxResult},
        stack_panel::StackPanelBuilder,
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
    },
    resource::curve::{CurveResource, CurveResourceState},
};
use crate::{
    command::{Command, CommandContext, CommandStack, CommandTrait},
    send_sync_message,
    utils::create_file_selector,
    MessageBoxButtons, MessageBoxMessage, MSG_SYNC_FLAG,
};
use std::{fmt::Debug, path::PathBuf};

#[derive(Debug, ComponentProvider)]
pub struct CurveEditorContext {}

impl CommandContext for CurveEditorContext {}

#[derive(Debug)]
struct ModifyCurveCommand {
    curve_resource: CurveResource,
    curve: Curve,
}

impl ModifyCurveCommand {
    fn swap(&mut self) {
        std::mem::swap(&mut self.curve_resource.data_ref().curve, &mut self.curve);
    }
}

impl CommandTrait for ModifyCurveCommand {
    fn name(&mut self, _: &dyn CommandContext) -> String {
        "Modify Curve".to_owned()
    }

    fn execute(&mut self, _: &mut dyn CommandContext) {
        self.swap();
    }

    fn revert(&mut self, _: &mut dyn CommandContext) {
        self.swap();
    }
}

struct FileMenu {
    new: Handle<UiNode>,
    save: Handle<UiNode>,
    load: Handle<UiNode>,
}

struct EditMenu {
    undo: Handle<UiNode>,
    redo: Handle<UiNode>,
}

struct Menu {
    file: FileMenu,
    edit: EditMenu,
}

pub struct CurveEditorWindow {
    window: Handle<UiNode>,
    curve_editor: Handle<UiNode>,
    ok: Handle<UiNode>,
    cancel: Handle<UiNode>,
    curve_resource: Option<CurveResource>,
    command_stack: CommandStack,
    menu: Menu,
    load_file_selector: Handle<UiNode>,
    save_file_selector: Handle<UiNode>,
    path: PathBuf,
    save_changes_message_box: Handle<UiNode>,
    cancel_message_box: Handle<UiNode>,
    modified: bool,
    backup: Curve,
}

impl CurveEditorWindow {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let load_file_selector = create_file_selector(ctx, "crv", FileBrowserMode::Open);
        let save_file_selector = create_file_selector(
            ctx,
            "crv",
            FileBrowserMode::Save {
                default_file_name: PathBuf::from("unnamed.crv"),
            },
        );

        let save_changes_message_box = MessageBoxBuilder::new(
            WindowBuilder::new(WidgetBuilder::new())
                .open(false)
                .with_title(WindowTitle::text("Unsaved Changes")),
        )
        .with_text(
            "You have unsaved changes, do you want to save it before closing the curve editor?",
        )
        .with_buttons(MessageBoxButtons::YesNoCancel)
        .build(ctx);

        let cancel_message_box = MessageBoxBuilder::new(
            WindowBuilder::new(WidgetBuilder::new())
                .open(false)
                .with_title(WindowTitle::text("Unsaved Changes")),
        )
        .with_text("You have unsaved changes, do you want to quit the curve editor without saving?")
        .with_buttons(MessageBoxButtons::YesNo)
        .build(ctx);

        let curve_editor;
        let ok;
        let cancel;
        let new;
        let save;
        let load;
        let undo;
        let redo;
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(400.0).with_height(300.0))
            .open(false)
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(
                            MenuBuilder::new(WidgetBuilder::new())
                                .with_items(vec![
                                    MenuItemBuilder::new(WidgetBuilder::new())
                                        .with_content(MenuItemContent::text("File"))
                                        .with_items(vec![
                                            {
                                                new = MenuItemBuilder::new(WidgetBuilder::new())
                                                    .with_content(
                                                        MenuItemContent::text_with_shortcut(
                                                            "New", "Ctrl+N",
                                                        ),
                                                    )
                                                    .build(ctx);
                                                new
                                            },
                                            {
                                                load = MenuItemBuilder::new(WidgetBuilder::new())
                                                    .with_content(
                                                        MenuItemContent::text_with_shortcut(
                                                            "Load", "Ctrl+L",
                                                        ),
                                                    )
                                                    .build(ctx);
                                                load
                                            },
                                            {
                                                save = MenuItemBuilder::new(WidgetBuilder::new())
                                                    .with_content(
                                                        MenuItemContent::text_with_shortcut(
                                                            "Save", "Ctrl+S",
                                                        ),
                                                    )
                                                    .build(ctx);
                                                save
                                            },
                                        ])
                                        .build(ctx),
                                    MenuItemBuilder::new(WidgetBuilder::new())
                                        .with_content(MenuItemContent::text("Edit"))
                                        .with_items(vec![
                                            {
                                                undo = MenuItemBuilder::new(WidgetBuilder::new())
                                                    .with_content(
                                                        MenuItemContent::text_with_shortcut(
                                                            "Undo", "Ctrl+Z",
                                                        ),
                                                    )
                                                    .build(ctx);
                                                undo
                                            },
                                            {
                                                redo = MenuItemBuilder::new(WidgetBuilder::new())
                                                    .with_content(
                                                        MenuItemContent::text_with_shortcut(
                                                            "Redo", "Ctrl+Y",
                                                        ),
                                                    )
                                                    .build(ctx);
                                                redo
                                            },
                                        ])
                                        .build(ctx),
                                ])
                                .build(ctx),
                        )
                        .with_child(
                            BorderBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(1)
                                    .on_column(0)
                                    .with_background(Brush::Solid(Color::opaque(20, 20, 20)))
                                    .with_child({
                                        curve_editor = CurveEditorBuilder::new(
                                            WidgetBuilder::new().with_enabled(false),
                                        )
                                        .build(ctx);
                                        curve_editor
                                    }),
                            )
                            .build(ctx),
                        )
                        .with_child(
                            StackPanelBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(2)
                                    .on_column(0)
                                    .with_horizontal_alignment(HorizontalAlignment::Right)
                                    .with_child({
                                        ok = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0))
                                                .with_width(100.0),
                                        )
                                        .with_text("OK")
                                        .build(ctx);
                                        ok
                                    })
                                    .with_child({
                                        cancel = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0))
                                                .with_width(100.0),
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
                .add_row(Row::strict(25.0))
                .add_row(Row::stretch())
                .add_row(Row::strict(25.0))
                .add_column(Column::stretch())
                .build(ctx),
            )
            .with_title(WindowTitle::text("Curve Editor"))
            .build(ctx);

        Self {
            window,
            curve_editor,
            ok,
            cancel,
            curve_resource: None,
            command_stack: CommandStack::new(false, 2048),
            menu: Menu {
                file: FileMenu { new, save, load },
                edit: EditMenu { undo, redo },
            },
            load_file_selector,
            save_file_selector,
            path: Default::default(),
            save_changes_message_box,
            modified: false,
            backup: Default::default(),
            cancel_message_box,
        }
    }

    fn close(&mut self, ui: &UserInterface) {
        self.clear(ui);

        ui.send_message(WindowMessage::close(
            self.window,
            MessageDirection::ToWidget,
        ));
    }

    pub fn open(&self, ui: &UserInterface) {
        ui.send_message(WindowMessage::open_modal(
            self.window,
            MessageDirection::ToWidget,
            true,
            true,
        ));
    }

    fn sync_to_model(&mut self, ui: &UserInterface) {
        if let Some(curve_resource) = self.curve_resource.as_ref() {
            send_sync_message(
                ui,
                CurveEditorMessage::sync(
                    self.curve_editor,
                    MessageDirection::ToWidget,
                    vec![curve_resource.data_ref().curve.clone()],
                ),
            );
        }
    }

    fn save(&self) {
        if let Some(curve_resource) = self.curve_resource.as_ref() {
            if let Some(state) = curve_resource.state().data() {
                let mut visitor = Visitor::new();
                state.curve.visit("Curve", &mut visitor).unwrap();
                visitor.save_binary(&self.path).unwrap();
            }
        }
    }

    fn set_curve(&mut self, curve: CurveResource, ui: &UserInterface) {
        self.backup = curve.data_ref().curve.clone();
        self.curve_resource = Some(curve);

        ui.send_message(WidgetMessage::enabled(
            self.curve_editor,
            MessageDirection::ToWidget,
            true,
        ));

        self.sync_to_model(ui);
        self.sync_title(ui);

        self.modified = false;

        self.command_stack.clear(&mut CurveEditorContext {});
    }

    fn sync_title(&self, ui: &UserInterface) {
        let title = if let Some(curve_resource) = self.curve_resource.as_ref() {
            let kind = curve_resource.header().kind.clone();

            match kind {
                ResourceKind::Embedded => "Curve Editor - Unnamed Curve".to_string(),
                ResourceKind::External(path) => {
                    format!("Curve Editor - {}", path.display())
                }
            }
        } else {
            "Curve Editor".to_string()
        };

        ui.send_message(WindowMessage::title(
            self.window,
            MessageDirection::ToWidget,
            WindowTitle::text(title),
        ));
    }

    fn clear(&mut self, ui: &UserInterface) {
        self.path = Default::default();
        self.backup = Default::default();
        self.command_stack.clear(&mut CurveEditorContext {});
        self.curve_resource = None;
        self.sync_title(ui);
        ui.send_message(WidgetMessage::enabled(
            self.curve_editor,
            MessageDirection::ToWidget,
            false,
        ));
        send_sync_message(
            ui,
            CurveEditorMessage::sync(
                self.curve_editor,
                MessageDirection::ToWidget,
                Default::default(),
            ),
        );
    }

    fn revert(&self) {
        if let Some(curve_resource) = self.curve_resource.as_ref() {
            curve_resource.data_ref().curve = self.backup.clone();
        }
    }

    fn open_save_file_dialog(&self, ui: &UserInterface) {
        ui.send_message(FileSelectorMessage::root(
            self.save_file_selector,
            MessageDirection::ToWidget,
            Some(std::env::current_dir().unwrap()),
        ));

        ui.send_message(WindowMessage::open_modal(
            self.save_file_selector,
            MessageDirection::ToWidget,
            true,
            true,
        ));
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, engine: &mut Engine) {
        let ui = &engine.user_interfaces.first_mut();

        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.cancel {
                if self.modified && self.curve_resource.is_some() {
                    ui.send_message(MessageBoxMessage::open(
                        self.cancel_message_box,
                        MessageDirection::ToWidget,
                        None,
                        None,
                    ));
                } else {
                    self.close(ui);
                }
            } else if message.destination() == self.ok {
                if self.modified && self.curve_resource.is_some() {
                    if self.path == PathBuf::default() {
                        ui.send_message(MessageBoxMessage::open(
                            self.save_changes_message_box,
                            MessageDirection::ToWidget,
                            None,
                            None,
                        ));
                    } else {
                        self.save();
                        self.close(ui);
                    }
                } else {
                    self.close(ui);
                }
            }
        } else if let Some(CurveEditorMessage::Sync(curve)) = message.data() {
            if message.destination() == self.curve_editor
                && message.direction() == MessageDirection::FromWidget
                && message.flags != MSG_SYNC_FLAG
            {
                if let Some(curve_resource) = self.curve_resource.as_ref() {
                    self.command_stack.do_command(
                        Command::new(ModifyCurveCommand {
                            curve_resource: curve_resource.clone(),
                            curve: curve.first().cloned().unwrap(),
                        }),
                        &mut CurveEditorContext {},
                    );

                    self.modified = true;
                }
            }
        } else if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.menu.edit.undo {
                self.command_stack.undo(&mut CurveEditorContext {});

                self.sync_to_model(ui);
            } else if message.destination() == self.menu.edit.redo {
                self.command_stack.redo(&mut CurveEditorContext {});

                self.sync_to_model(ui);
            } else if message.destination() == self.menu.file.load {
                ui.send_message(FileSelectorMessage::root(
                    self.load_file_selector,
                    MessageDirection::ToWidget,
                    Some(std::env::current_dir().unwrap()),
                ));

                ui.send_message(WindowMessage::open_modal(
                    self.load_file_selector,
                    MessageDirection::ToWidget,
                    true,
                    true,
                ));
            } else if message.destination() == self.menu.file.new {
                self.path = Default::default();

                self.set_curve(
                    Resource::new_ok(Default::default(), CurveResourceState::default()),
                    ui,
                );
            } else if message.destination() == self.menu.file.save {
                if self.path == PathBuf::default() {
                    self.open_save_file_dialog(ui);
                } else {
                    self.save();
                }
            }
        } else if let Some(FileSelectorMessage::Commit(path)) = message.data() {
            if message.destination() == self.load_file_selector {
                if let Ok(curve) =
                    block_on(engine.resource_manager.request::<CurveResourceState>(path))
                {
                    self.path.clone_from(path);
                    self.set_curve(curve, ui);
                }
            } else if message.destination() == self.save_file_selector {
                self.path.clone_from(path);
                self.save();
            }
        } else if let Some(MessageBoxMessage::Close(result)) = message.data() {
            if message.destination() == self.save_changes_message_box {
                match result {
                    MessageBoxResult::No => {
                        self.revert();
                        self.close(ui);
                    }
                    MessageBoxResult::Yes => {
                        if self.path == PathBuf::default() {
                            self.open_save_file_dialog(ui);
                        } else {
                            self.save();
                            self.close(ui);
                        }
                    }
                    _ => (),
                }
            } else if message.destination() == self.cancel_message_box {
                if let MessageBoxResult::Yes = result {
                    self.revert();
                    self.close(ui);
                }
            }
        }
    }
}
