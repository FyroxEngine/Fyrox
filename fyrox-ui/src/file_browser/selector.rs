use crate::{
    button::{ButtonBuilder, ButtonMessage},
    core::{
        algebra::Vector2, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        visitor::prelude::*,
    },
    define_constructor, define_widget_deref,
    draw::DrawingContext,
    file_browser::{FileBrowser, FileBrowserBuilder, FileBrowserMessage, FileBrowserMode, Filter},
    grid::{Column, GridBuilder, Row},
    message::{MessageDirection, OsEvent, UiMessage},
    stack_panel::StackPanelBuilder,
    text::TextMessage,
    text_box::TextBoxBuilder,
    widget::{Widget, WidgetBuilder, WidgetMessage},
    window::{Window, WindowBuilder, WindowMessage, WindowTitle},
    BuildContext, Control, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
    VerticalAlignment,
};
use fyrox_core::uuid_provider;
use fyrox_graph::BaseSceneGraph;
use std::{
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq)]
pub enum FileSelectorMessage {
    Root(Option<PathBuf>),
    Path(PathBuf),
    Commit(PathBuf),
    Cancel,
    Filter(Option<Filter>),
}

impl FileSelectorMessage {
    define_constructor!(FileSelectorMessage:Commit => fn commit(PathBuf), layout: false);
    define_constructor!(FileSelectorMessage:Root => fn root(Option<PathBuf>), layout: false);
    define_constructor!(FileSelectorMessage:Path => fn path(PathBuf), layout: false);
    define_constructor!(FileSelectorMessage:Cancel => fn cancel(), layout: false);
    define_constructor!(FileSelectorMessage:Filter => fn filter(Option<Filter>), layout: false);
}

/// File selector is a modal window that allows you to select a file (or directory) and commit or
/// cancel selection.
#[derive(Default, Clone, Debug, Visit, Reflect, ComponentProvider)]
pub struct FileSelector {
    #[component(include)]
    pub window: Window,
    pub browser: Handle<UiNode>,
    pub ok: Handle<UiNode>,
    pub cancel: Handle<UiNode>,
}

impl Deref for FileSelector {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

impl DerefMut for FileSelector {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.window
    }
}

uuid_provider!(FileSelector = "878b2220-03e6-4a50-a97d-3a8e5397b6cb");

// File selector extends Window widget so it delegates most of calls
// to inner window.
impl Control for FileSelector {
    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        self.window.measure_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        self.window.arrange_override(ui, final_size)
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        self.window.draw(drawing_context)
    }

    fn update(&mut self, dt: f32, ui: &mut UserInterface) {
        self.window.update(dt, ui);
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.window.handle_routed_message(ui, message);

        if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == self.ok {
                let path = ui
                    .node(self.browser)
                    .cast::<FileBrowser>()
                    .expect("self.browser must be FileBrowser")
                    .path
                    .clone();

                ui.send_message(FileSelectorMessage::commit(
                    self.handle,
                    MessageDirection::ToWidget,
                    path,
                ));
            } else if message.destination() == self.cancel {
                ui.send_message(FileSelectorMessage::cancel(
                    self.handle,
                    MessageDirection::ToWidget,
                ))
            }
        } else if let Some(msg) = message.data::<FileSelectorMessage>() {
            if message.destination() == self.handle {
                match msg {
                    FileSelectorMessage::Commit(_) | FileSelectorMessage::Cancel => ui
                        .send_message(WindowMessage::close(
                            self.handle,
                            MessageDirection::ToWidget,
                        )),
                    FileSelectorMessage::Path(path) => ui.send_message(FileBrowserMessage::path(
                        self.browser,
                        MessageDirection::ToWidget,
                        path.clone(),
                    )),
                    FileSelectorMessage::Root(root) => {
                        ui.send_message(FileBrowserMessage::root(
                            self.browser,
                            MessageDirection::ToWidget,
                            root.clone(),
                        ));
                    }
                    FileSelectorMessage::Filter(filter) => {
                        ui.send_message(FileBrowserMessage::filter(
                            self.browser,
                            MessageDirection::ToWidget,
                            filter.clone(),
                        ));
                    }
                }
            }
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        self.window.preview_message(ui, message);
    }

    fn handle_os_event(
        &mut self,
        self_handle: Handle<UiNode>,
        ui: &mut UserInterface,
        event: &OsEvent,
    ) {
        self.window.handle_os_event(self_handle, ui, event);
    }
}

pub struct FileSelectorBuilder {
    window_builder: WindowBuilder,
    filter: Option<Filter>,
    mode: FileBrowserMode,
    path: PathBuf,
    root: Option<PathBuf>,
}

impl FileSelectorBuilder {
    pub fn new(window_builder: WindowBuilder) -> Self {
        Self {
            window_builder,
            filter: None,
            mode: FileBrowserMode::Open,
            path: Default::default(),
            root: None,
        }
    }

    pub fn with_filter(mut self, filter: Filter) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn with_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        path.as_ref().clone_into(&mut self.path);
        self
    }

    pub fn with_mode(mut self, mode: FileBrowserMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_root(mut self, root: PathBuf) -> Self {
        self.root = Some(root);
        self
    }

    pub fn build(mut self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let browser;
        let ok;
        let cancel;

        if self.window_builder.title.is_none() {
            self.window_builder.title = Some(WindowTitle::text("Select File"));
        }

        let window = self
            .window_builder
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(
                            StackPanelBuilder::new(
                                WidgetBuilder::new()
                                    .with_margin(Thickness::uniform(1.0))
                                    .with_horizontal_alignment(HorizontalAlignment::Right)
                                    .on_column(0)
                                    .on_row(1)
                                    .with_child({
                                        ok = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_tab_index(Some(1))
                                                .with_margin(Thickness::uniform(1.0))
                                                .with_width(100.0)
                                                .with_height(30.0),
                                        )
                                        .with_text(match &self.mode {
                                            FileBrowserMode::Open => "Open",
                                            FileBrowserMode::Save { .. } => "Save",
                                        })
                                        .build(ctx);
                                        ok
                                    })
                                    .with_child({
                                        cancel = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_tab_index(Some(2))
                                                .with_margin(Thickness::uniform(1.0))
                                                .with_width(100.0)
                                                .with_height(30.0),
                                        )
                                        .with_text("Cancel")
                                        .build(ctx);
                                        cancel
                                    }),
                            )
                            .with_orientation(Orientation::Horizontal)
                            .build(ctx),
                        )
                        .with_child({
                            browser = FileBrowserBuilder::new(
                                WidgetBuilder::new().on_column(0).with_tab_index(Some(0)),
                            )
                            .with_mode(self.mode)
                            .with_opt_filter(self.filter)
                            .with_path(self.path)
                            .with_opt_root(self.root)
                            .build(ctx);
                            browser
                        }),
                )
                .add_column(Column::stretch())
                .add_row(Row::stretch())
                .add_row(Row::auto())
                .build(ctx),
            )
            .build_window(ctx);

        let file_selector = FileSelector {
            window,
            browser,
            ok,
            cancel,
        };

        ctx.add_node(UiNode::new(file_selector))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileSelectorFieldMessage {
    Path(PathBuf),
}

impl FileSelectorFieldMessage {
    define_constructor!(FileSelectorFieldMessage:Path => fn path(PathBuf), layout: false);
}

#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider)]
pub struct FileSelectorField {
    widget: Widget,
    path: PathBuf,
    path_field: Handle<UiNode>,
    select: Handle<UiNode>,
    file_selector: Handle<UiNode>,
}

define_widget_deref!(FileSelectorField);

uuid_provider!(FileSelectorField = "2dbda730-8a60-4f62-aee8-2ff0ccd15bf2");

impl Control for FileSelectorField {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(TextMessage::Text(text)) = message.data() {
            if message.destination() == self.path_field
                && message.direction() == MessageDirection::FromWidget
                && Path::new(text.as_str()) != self.path
            {
                ui.send_message(FileSelectorFieldMessage::path(
                    self.handle,
                    MessageDirection::ToWidget,
                    text.into(),
                ));
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.select {
                let file_selector = FileSelectorBuilder::new(
                    WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
                        .open(false)
                        .can_minimize(false),
                )
                .with_path(self.path.clone())
                .with_root(std::env::current_dir().unwrap_or_default())
                .with_mode(FileBrowserMode::Open)
                .build(&mut ui.build_ctx());

                self.file_selector = file_selector;

                ui.send_message(WindowMessage::open_modal(
                    file_selector,
                    MessageDirection::ToWidget,
                    true,
                    true,
                ));
            }
        } else if let Some(FileSelectorFieldMessage::Path(new_path)) = message.data() {
            if message.destination() == self.handle
                && message.direction() == MessageDirection::ToWidget
                && &self.path != new_path
            {
                self.path.clone_from(new_path);
                ui.send_message(TextMessage::text(
                    self.path_field,
                    MessageDirection::ToWidget,
                    self.path.to_string_lossy().to_string(),
                ));

                ui.send_message(message.reverse());
            }
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        if let Some(FileSelectorMessage::Commit(new_path)) = message.data() {
            if message.destination() == self.file_selector {
                ui.send_message(FileSelectorFieldMessage::path(
                    self.handle,
                    MessageDirection::ToWidget,
                    new_path.clone(),
                ));
            }
        } else if let Some(WindowMessage::Close) = message.data() {
            if message.destination() == self.file_selector {
                ui.send_message(WidgetMessage::remove(
                    self.file_selector,
                    MessageDirection::ToWidget,
                ));
            }
        }
    }
}

pub struct FileSelectorFieldBuilder {
    widget_builder: WidgetBuilder,
    path: PathBuf,
}

impl FileSelectorFieldBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            path: Default::default(),
        }
    }

    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.path = path;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let select;
        let path_field;
        let field = FileSelectorField {
            widget: self
                .widget_builder
                .with_preview_messages(true)
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .with_child({
                                path_field = TextBoxBuilder::new(WidgetBuilder::new().on_column(0))
                                    .with_text(self.path.to_string_lossy())
                                    .with_vertical_text_alignment(VerticalAlignment::Center)
                                    .build(ctx);
                                path_field
                            })
                            .with_child({
                                select = ButtonBuilder::new(
                                    WidgetBuilder::new().on_column(1).with_width(25.0),
                                )
                                .with_text("...")
                                .build(ctx);
                                select
                            }),
                    )
                    .add_row(Row::stretch())
                    .add_column(Column::stretch())
                    .add_column(Column::auto())
                    .build(ctx),
                )
                .build(),
            path: self.path,
            path_field,
            select,
            file_selector: Default::default(),
        };

        ctx.add_node(UiNode::new(field))
    }
}
