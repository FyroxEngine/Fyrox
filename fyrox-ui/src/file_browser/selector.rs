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

use crate::button::Button;
use crate::{
    border::BorderBuilder,
    button::{ButtonBuilder, ButtonMessage},
    core::{
        algebra::Vector2, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        uuid_provider, visitor::prelude::*,
    },
    draw::DrawingContext,
    dropdown_list::{DropdownListBuilder, DropdownListMessage},
    file_browser::{FileBrowserBuilder, FileBrowserMessage, PathFilter},
    grid::{Column, GridBuilder, Row},
    message::{MessageData, OsEvent, UiMessage},
    messagebox::{MessageBoxBuilder, MessageBoxButtons, MessageBoxMessage, MessageBoxResult},
    stack_panel::StackPanelBuilder,
    style::{resource::StyleResourceExt, Style},
    text::{TextBuilder, TextMessage},
    text_box::{TextBoxBuilder, TextCommitMode},
    utils::make_dropdown_list_option,
    widget::{Widget, WidgetBuilder, WidgetMessage},
    window::{Window, WindowBuilder, WindowMessage, WindowTitle},
    BuildContext, Control, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
    VerticalAlignment,
};
use fyrox_graph::constructor::{ConstructorProvider, GraphNodeConstructor};
use std::{
    cell::Cell,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

#[derive(Default, Clone, PartialEq, Eq, Hash, Debug, Visit, Reflect)]
pub enum FileSelectorMode {
    #[default]
    Open,
    Save {
        default_file_name: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileSelectorMessage {
    Root(Option<PathBuf>),
    Path(PathBuf),
    Commit(PathBuf),
    FocusCurrentPath,
    Cancel,
    FileTypes(PathFilter),
}
impl MessageData for FileSelectorMessage {}

/// File selector is a modal window that allows you to select a file (or directory) and commit or
/// cancel selection.
#[derive(Default, Clone, Debug, Visit, Reflect, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct FileSelector {
    #[component(include)]
    pub window: Window,
    pub browser: Handle<UiNode>,
    pub ok: Handle<Button>,
    pub cancel: Handle<Button>,
    pub selected_folder: PathBuf,
    pub mode: FileSelectorMode,
    pub file_name: Handle<UiNode>,
    pub file_name_value: PathBuf,
    pub filter: PathFilter,
    pub file_type_selector: Handle<UiNode>,
    pub selected_file_type: Option<usize>,
    pub overwrite_message_box: Cell<Handle<UiNode>>,
}

impl ConstructorProvider<UiNode, UserInterface> for FileSelector {
    fn constructor() -> GraphNodeConstructor<UiNode, UserInterface> {
        GraphNodeConstructor::new::<Self>()
            .with_variant("File Selector", |ui| {
                FileSelectorBuilder::new(WindowBuilder::new(
                    WidgetBuilder::new().with_name("File Selector"),
                ))
                .build(&mut ui.build_ctx())
                .into()
            })
            .with_group("File System")
    }
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

fn extract_folder_path(path: &Path) -> Option<&Path> {
    if path.is_file() {
        path.parent()
    } else if path.is_dir() {
        Some(path)
    } else {
        None
    }
}

fn extract_folder_path_buf(path: &Path) -> Option<PathBuf> {
    extract_folder_path(path).map(|p| p.to_path_buf())
}

impl FileSelector {
    fn on_ok_clicked(&self, ui: &mut UserInterface) {
        let final_path = self.final_path();

        if final_path.exists() && matches!(self.mode, FileSelectorMode::Save { .. }) {
            self.overwrite_message_box.set(
                MessageBoxBuilder::new(
                    WindowBuilder::new(WidgetBuilder::new().with_width(350.0).with_height(100.0))
                        .with_title(WindowTitle::text("Confirm Action"))
                        .open(false),
                )
                .with_text(
                    format!(
                        "The file {} already exist. Do you want to overwrite it?",
                        final_path.display()
                    )
                    .as_str(),
                )
                .with_buttons(MessageBoxButtons::YesNo)
                .build(&mut ui.build_ctx()),
            );

            ui.send(
                self.overwrite_message_box.get(),
                MessageBoxMessage::Open {
                    title: None,
                    text: None,
                },
            );
        } else {
            ui.send(self.handle, FileSelectorMessage::Commit(self.final_path()));
        }
    }

    fn on_path_selected(&mut self, path: &Path, ui: &UserInterface) {
        if path.is_file() {
            ui.send(
                self.file_name,
                TextMessage::Text(
                    path.file_name()
                        .map(|f| f.to_string_lossy().to_string())
                        .unwrap_or_default(),
                ),
            );
            self.selected_folder = extract_folder_path_buf(path).unwrap_or_default();
        } else {
            self.selected_folder = path.to_path_buf();
        }

        self.validate_selection(ui);
    }

    fn on_file_selector_message(&mut self, msg: &FileSelectorMessage, ui: &UserInterface) {
        match msg {
            FileSelectorMessage::Commit(_) | FileSelectorMessage::Cancel => {
                ui.send(self.handle, WindowMessage::Close)
            }
            FileSelectorMessage::Path(path) => {
                ui.send(self.browser, FileBrowserMessage::Path(path.clone()))
            }
            FileSelectorMessage::Root(root) => {
                ui.send(self.browser, FileBrowserMessage::Root(root.clone()));
            }
            FileSelectorMessage::FileTypes(filter) => {
                ui.send(self.browser, FileBrowserMessage::Filter(filter.clone()));
            }
            FileSelectorMessage::FocusCurrentPath => {
                ui.send(self.browser, FileBrowserMessage::FocusCurrentPath);
            }
        }
    }

    fn final_path(&self) -> PathBuf {
        let mut final_path = self.selected_folder.join(&self.file_name_value);
        if let Some(file_type) = self.selected_file_type.and_then(|i| self.filter.get(i)) {
            final_path.set_extension(&file_type.extension);
        }
        final_path
    }

    fn validate_selection(&self, ui: &UserInterface) {
        let final_path = self.final_path();
        let passed = self
            .filter
            .supports_specific_type(&final_path, self.selected_file_type)
            && match self.mode {
                FileSelectorMode::Open => final_path.exists(),
                FileSelectorMode::Save { .. } => true,
            };
        ui.send(self.ok, WidgetMessage::Enabled(passed))
    }

    fn on_file_type_selected(&mut self, selection: Option<usize>, ui: &UserInterface) {
        // Minus one here because there's "All supported" option in the beginning of the file
        // type selector.
        let selection = selection.and_then(|i| i.checked_sub(1));
        self.selected_file_type = selection;
        self.validate_selection(ui);
    }

    fn on_file_name_changed(&mut self, file_name: &str, ui: &UserInterface) {
        self.file_name_value = file_name.into();
        self.validate_selection(ui);
    }
}

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
                self.on_ok_clicked(ui)
            } else if message.destination() == self.cancel {
                ui.send(self.handle, FileSelectorMessage::Cancel)
            }
        } else if let Some(msg) = message.data_for::<FileSelectorMessage>(self.handle) {
            self.on_file_selector_message(msg, ui)
        } else if let Some(FileBrowserMessage::Path(path)) = message.data_from(self.browser) {
            self.on_path_selected(path, ui)
        } else if let Some(TextMessage::Text(file_name)) = message.data_from(self.file_name) {
            self.on_file_name_changed(file_name, ui)
        } else if let Some(DropdownListMessage::Selection(selection)) =
            message.data_from(self.file_type_selector)
        {
            self.on_file_type_selected(*selection, ui)
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        self.window.preview_message(ui, message);

        if let Some(MessageBoxMessage::Close(result)) = message.data() {
            if message.destination() == self.overwrite_message_box.get() {
                if let MessageBoxResult::Yes = *result {
                    ui.send(self.handle, FileSelectorMessage::Commit(self.final_path()));
                }

                ui.send(self.overwrite_message_box.get(), WidgetMessage::Remove);

                self.overwrite_message_box.set(Handle::NONE);
            }
        }
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
    filter: PathFilter,
    mode: FileSelectorMode,
    path: PathBuf,
    root: Option<PathBuf>,
    selected_file_type: Option<usize>,
}

impl FileSelectorBuilder {
    pub fn new(window_builder: WindowBuilder) -> Self {
        Self {
            window_builder,
            mode: FileSelectorMode::Open,
            path: "./".into(),
            root: None,
            filter: Default::default(),
            selected_file_type: None,
        }
    }

    pub fn with_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        path.as_ref().clone_into(&mut self.path);
        self
    }

    pub fn with_mode(mut self, mode: FileSelectorMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_root(mut self, root: PathBuf) -> Self {
        self.root = Some(root);
        self
    }

    pub fn with_filter(mut self, file_types: PathFilter) -> Self {
        self.filter = file_types;
        self
    }

    pub fn with_selected_file_type(mut self, selected: usize) -> Self {
        self.selected_file_type = Some(selected);
        self
    }

    pub fn build(mut self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let browser;
        let ok;
        let cancel;

        if self.window_builder.title.is_none() {
            self.window_builder.title = Some(WindowTitle::text("Select File"));
        }

        let file_name;
        let name_grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_visibility(!self.filter.folders_only)
                .with_margin(Thickness::uniform(1.0))
                .on_row(1)
                .on_column(0)
                .with_child(
                    TextBuilder::new(
                        WidgetBuilder::new()
                            .on_row(0)
                            .on_column(0)
                            .with_vertical_alignment(VerticalAlignment::Center),
                    )
                    .with_text("File Name:")
                    .build(ctx),
                )
                .with_child({
                    file_name = TextBoxBuilder::new(
                        WidgetBuilder::new()
                            .on_row(0)
                            .on_column(1)
                            .with_height(25.0)
                            .with_margin(Thickness::uniform(1.0)),
                    )
                    .with_text_commit_mode(TextCommitMode::Immediate)
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .with_text(match self.mode {
                        FileSelectorMode::Open => Default::default(),
                        FileSelectorMode::Save {
                            default_file_name: ref default_file_name_no_extension,
                        } => default_file_name_no_extension.to_string_lossy().to_string(),
                    })
                    .build(ctx);
                    file_name
                }),
        )
        .add_row(Row::auto())
        .add_column(Column::strict(80.0))
        .add_column(Column::stretch())
        .build(ctx);

        let mut filter_items = self
            .filter
            .iter()
            .map(|file_type| make_dropdown_list_option(ctx, &file_type.to_string()))
            .collect::<Vec<_>>();

        filter_items.insert(0, make_dropdown_list_option(ctx, "All Supported"));

        let extension_selector;
        let extension_grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_visibility(!self.filter.folders_only)
                .with_margin(Thickness::uniform(1.0))
                .on_row(2)
                .on_column(0)
                .with_child(
                    TextBuilder::new(
                        WidgetBuilder::new()
                            .on_row(0)
                            .on_column(0)
                            .with_vertical_alignment(VerticalAlignment::Center),
                    )
                    .with_text("File Type:")
                    .build(ctx),
                )
                .with_child({
                    extension_selector = DropdownListBuilder::new(
                        WidgetBuilder::new()
                            .with_height(25.0)
                            .on_column(1)
                            .with_margin(Thickness::uniform(1.0)),
                    )
                    .with_items(filter_items)
                    .with_close_on_selection(true)
                    .with_selected(0)
                    .build(ctx);
                    extension_selector
                }),
        )
        .add_row(Row::auto())
        .add_column(Column::strict(80.0))
        .add_column(Column::stretch())
        .build(ctx);

        let browser_container = BorderBuilder::new(
            WidgetBuilder::new()
                .on_row(0)
                .on_column(0)
                .with_background(ctx.style.property(Style::BRUSH_LIGHT))
                .with_child({
                    browser = FileBrowserBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .with_tab_index(Some(0)),
                    )
                    .with_filter(self.filter.clone())
                    .with_path(self.path.clone())
                    .with_opt_root(self.root)
                    .build(ctx);
                    browser
                }),
        )
        .build(ctx);

        let ok_enabled = match self.mode {
            FileSelectorMode::Open => {
                let passed = self
                    .filter
                    .supports_specific_type(&self.path, self.selected_file_type);
                self.path.exists() && passed
            }
            FileSelectorMode::Save { .. } => true,
        };

        let buttons = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .with_horizontal_alignment(HorizontalAlignment::Right)
                .on_row(3)
                .on_column(0)
                .with_child({
                    ok = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .with_tab_index(Some(1))
                            .with_margin(Thickness::uniform(1.0))
                            .with_width(100.0)
                            .with_height(25.0)
                            .with_enabled(ok_enabled),
                    )
                    .with_ok_back(ctx)
                    .with_text(match &self.mode {
                        FileSelectorMode::Open => "Open",
                        FileSelectorMode::Save { .. } => "Save",
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
                            .with_height(25.0),
                    )
                    .with_cancel_back(ctx)
                    .with_text("Cancel")
                    .build(ctx);
                    cancel
                }),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        self.window_builder.widget_builder.preview_messages = true;

        let window = self
            .window_builder
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(browser_container)
                        .with_child(buttons)
                        .with_child(name_grid)
                        .with_child(extension_grid),
                )
                .add_column(Column::stretch())
                .add_row(Row::stretch())
                .add_row(Row::auto())
                .add_row(Row::auto())
                .add_row(Row::auto())
                .build(ctx),
            )
            .build_window(ctx);

        let file_selector = FileSelector {
            window,
            browser,
            ok,
            cancel,
            selected_folder: extract_folder_path_buf(&self.path).unwrap_or_default(),
            file_name_value: match self.mode {
                FileSelectorMode::Open => Default::default(),
                FileSelectorMode::Save {
                    ref default_file_name,
                } => default_file_name.clone(),
            },
            filter: self.filter,
            file_type_selector: extension_selector,
            mode: self.mode,
            file_name,
            selected_file_type: self.selected_file_type,
            overwrite_message_box: Default::default(),
        };

        ctx.add_node(UiNode::new(file_selector))
    }
}

#[cfg(test)]
mod test {
    use crate::file_browser::FileSelectorBuilder;
    use crate::window::WindowBuilder;
    use crate::{test::test_widget_deletion, widget::WidgetBuilder};

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| {
            FileSelectorBuilder::new(WindowBuilder::new(WidgetBuilder::new())).build(ctx)
        });
    }
}
