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

use crate::message::MessageSender;
use crate::{
    asset::{self, item::AssetItem},
    fyrox::{
        asset::manager::ResourceManager,
        core::{
            algebra::Vector2, futures::executor::block_on, log::Log, pool::Handle,
            reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*,
        },
        engine::Engine,
        graph::SceneGraph,
        gui::{
            button::{ButtonBuilder, ButtonMessage},
            draw::DrawingContext,
            grid::{Column, GridBuilder, Row},
            menu::{ContextMenuBuilder, MenuItemBuilder, MenuItemContent, MenuItemMessage},
            message::{MessageDirection, OsEvent, UiMessage},
            messagebox::{
                MessageBoxBuilder, MessageBoxButtons, MessageBoxMessage, MessageBoxResult,
            },
            popup::{Placement, PopupBuilder, PopupMessage},
            stack_panel::StackPanelBuilder,
            text::{TextBuilder, TextMessage},
            text_box::{TextBoxBuilder, TextCommitMode},
            widget::{Widget, WidgetBuilder, WidgetMessage},
            window::{Window, WindowBuilder, WindowMessage, WindowTitle},
            BuildContext, Control, HorizontalAlignment, Orientation, RcUiNodeHandle, Thickness,
            UiNode, UserInterface,
        },
    },
    Message,
};
use fyrox::core::SafeLock;
use fyrox::gui::formatted_text::WrapMode;
use std::{
    fs::File,
    io::Write,
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::mpsc::Sender,
};

#[derive(Clone, Visit, Reflect, Debug, ComponentProvider, TypeUuidProvider)]
#[reflect(derived_type = "UiNode")]
#[type_uuid(id = "8c9934ad-b4e1-4c68-9876-f253e34c6667")]
struct AssetRenameDialog {
    window: Window,
    name_field: Handle<UiNode>,
    rename: Handle<UiNode>,
    cancel: Handle<UiNode>,
    old_file_name: String,
    new_file_name: String,
    extension: String,
    folder: String,
    old_path: PathBuf,
    #[visit(skip)]
    #[reflect(hidden)]
    resource_manager: ResourceManager,
}

impl Deref for AssetRenameDialog {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.window.widget
    }
}

impl DerefMut for AssetRenameDialog {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.window.widget
    }
}

impl Control for AssetRenameDialog {
    fn on_remove(&self, sender: &Sender<UiMessage>) {
        self.window.on_remove(sender);
    }

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

        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.rename {
                Log::verify(block_on(self.resource_manager.move_resource_by_path(
                    &self.old_path,
                    format!("{}/{}.{}", self.folder, self.new_file_name, self.extension),
                    false,
                )));
            }

            if message.destination() == self.cancel || message.destination() == self.rename {
                ui.send_message(WindowMessage::close(
                    self.handle,
                    MessageDirection::ToWidget,
                ));
            }
        } else if let Some(TextMessage::Text(name)) = message.data_from(self.name_field) {
            name.clone_into(&mut self.new_file_name);

            let can_be_moved = block_on(self.resource_manager.can_resource_be_moved(
                &self.old_path,
                format!("{}/{}.{}", self.folder, self.new_file_name, self.extension),
                false,
            ));

            ui.send_message(WidgetMessage::enabled(
                self.rename,
                MessageDirection::ToWidget,
                can_be_moved,
            ));
        } else if let Some(WindowMessage::OpenModal { .. }) = message.data() {
            if message.destination() == self.handle {
                ui.send_message(WidgetMessage::focus(
                    self.name_field,
                    MessageDirection::ToWidget,
                ));
            }
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        self.window.preview_message(ui, message)
    }

    fn handle_os_event(
        &mut self,
        self_handle: Handle<UiNode>,
        ui: &mut UserInterface,
        event: &OsEvent,
    ) {
        self.window.handle_os_event(self_handle, ui, event)
    }
}

struct AssetRenameDialogBuilder {
    window_builder: WindowBuilder,
}

impl AssetRenameDialogBuilder {
    fn new(window_builder: WindowBuilder) -> Self {
        Self { window_builder }
    }

    fn build(
        self,
        old_file_name: String,
        extension: String,
        folder: String,
        old_path: PathBuf,
        resource_manager: ResourceManager,
        ctx: &mut BuildContext,
    ) -> Handle<UiNode> {
        let rename;
        let cancel;
        let name_field;
        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(
                    TextBuilder::new(
                        WidgetBuilder::new()
                            .on_row(0)
                            .with_margin(Thickness::uniform(2.0)),
                    )
                    .with_text(format!(
                        "Enter a new name for {old_file_name}.{extension} resource."
                    ))
                    .with_wrap(WrapMode::Word)
                    .build(ctx),
                )
                .with_child({
                    name_field = TextBoxBuilder::new(
                        WidgetBuilder::new()
                            .on_row(1)
                            .with_margin(Thickness::uniform(2.0))
                            .with_tab_index(Some(0)),
                    )
                    .with_text(&old_file_name)
                    .with_text_commit_mode(TextCommitMode::Immediate)
                    .build(ctx);
                    name_field
                })
                .with_child(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .on_row(3)
                            .with_horizontal_alignment(HorizontalAlignment::Right)
                            .with_child({
                                rename = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_width(100.0)
                                        .with_height(24.0)
                                        .with_tab_index(Some(1)),
                                )
                                .with_text("Rename")
                                .build(ctx);
                                rename
                            })
                            .with_child({
                                cancel = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_width(100.0)
                                        .with_height(24.0)
                                        .with_tab_index(Some(2)),
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
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .add_row(Row::auto())
        .add_column(Column::stretch())
        .build(ctx);

        let dialog = AssetRenameDialog {
            window: self.window_builder.with_content(content).build_window(ctx),
            name_field,
            rename,
            cancel,
            new_file_name: old_file_name.clone(),
            old_file_name,
            extension,
            folder,
            old_path,
            resource_manager,
        };

        ctx.add_node(UiNode::new(dialog))
    }
}

pub struct AssetItemContextMenu {
    pub menu: RcUiNodeHandle,
    pub open: Handle<UiNode>,
    pub duplicate: Handle<UiNode>,
    pub copy_path: Handle<UiNode>,
    pub copy_file_name: Handle<UiNode>,
    pub show_in_explorer: Handle<UiNode>,
    pub delete: Handle<UiNode>,
    pub rename: Handle<UiNode>,
    pub placement_target: Handle<UiNode>,
    pub dependencies: Handle<UiNode>,
    pub delete_confirmation_dialog: Handle<UiNode>,
    pub path_to_delete: PathBuf,
    pub reload: Handle<UiNode>,
}

impl AssetItemContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        fn item(text: &str, ctx: &mut BuildContext) -> Handle<UiNode> {
            MenuItemBuilder::new(WidgetBuilder::new())
                .with_content(MenuItemContent::text(text))
                .build(ctx)
        }

        let delete = item("Delete", ctx);
        let show_in_explorer = item("Show In Explorer", ctx);
        let open = item("Open", ctx);
        let duplicate = item("Duplicate", ctx);
        let copy_path = item("Copy Full Path", ctx);
        let copy_file_name = item("Copy File Name", ctx);
        let dependencies = item("Dependencies", ctx);
        let rename = item("Rename", ctx);
        let reload = item("Reload", ctx);

        let menu = ContextMenuBuilder::new(
            PopupBuilder::new(WidgetBuilder::new())
                .with_content(
                    StackPanelBuilder::new(WidgetBuilder::new().with_children(vec![
                        open,
                        duplicate,
                        copy_path,
                        copy_file_name,
                        delete,
                        show_in_explorer,
                        dependencies,
                        rename,
                        reload,
                    ]))
                    .build(ctx),
                )
                .with_restrict_picking(false),
        )
        .build(ctx);
        let menu = RcUiNodeHandle::new(menu, ctx.sender());

        Self {
            menu,
            open,
            duplicate,
            copy_path,
            delete,
            show_in_explorer,
            placement_target: Default::default(),
            copy_file_name,
            dependencies,
            delete_confirmation_dialog: Default::default(),
            path_to_delete: Default::default(),
            rename,
            reload,
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        sender: &MessageSender,
        engine: &mut Engine,
    ) -> bool {
        let ui = engine.user_interfaces.first_mut();
        if let Some(PopupMessage::Placement(Placement::Cursor(target))) = message.data() {
            if message.destination() == self.menu.handle() {
                self.placement_target = *target;
                if let Some(item) = ui.try_get_of_type::<AssetItem>(self.placement_target) {
                    for handle in [self.dependencies, self.duplicate] {
                        let is_built_in = engine
                            .resource_manager
                            .state()
                            .built_in_resources
                            .get(&item.path)
                            .is_some();
                        ui.send_message(WidgetMessage::enabled(
                            handle,
                            MessageDirection::ToWidget,
                            item.path.is_file() || is_built_in,
                        ));
                    }
                    if let Some(resource) = item.untyped_resource() {
                        ui.send_message(WidgetMessage::enabled(
                            self.delete,
                            MessageDirection::ToWidget,
                            !engine.resource_manager.is_built_in_resource(&resource),
                        ));
                    }
                }
            }
        } else if let Some(MenuItemMessage::Click) = message.data() {
            if let Some(item) = ui.try_get_mut_of_type::<AssetItem>(self.placement_target) {
                if message.destination() == self.delete {
                    let text = format!(
                        "Do you really want to delete {} asset? This \
                    is irreversible operation!",
                        item.path.display()
                    );

                    self.path_to_delete = item.path.clone();

                    self.delete_confirmation_dialog = MessageBoxBuilder::new(
                        WindowBuilder::new(
                            WidgetBuilder::new().with_width(250.0).with_height(100.0),
                        )
                        .open(false)
                        .with_remove_on_close(true)
                        .with_title(WindowTitle::text("Confirm Deletion")),
                    )
                    .with_text(&text)
                    .with_buttons(MessageBoxButtons::YesNo)
                    .build(&mut ui.build_ctx());

                    ui.send_message(WindowMessage::open_modal(
                        self.delete_confirmation_dialog,
                        MessageDirection::ToWidget,
                        true,
                        true,
                    ));

                    return true;
                } else if message.destination() == self.show_in_explorer {
                    if let Ok(canonical_path) = item.path.canonicalize() {
                        asset::show_in_explorer(canonical_path)
                    }
                } else if message.destination() == self.open {
                    item.open();
                } else if message.destination() == self.duplicate {
                    if let Some(resource) = item.untyped_resource() {
                        if let Some(path) = engine.resource_manager.resource_path(&resource) {
                            let built_in = engine
                                .resource_manager
                                .state()
                                .built_in_resources
                                .get(&path)
                                .cloned();
                            if let Some(built_in) = built_in {
                                if let Some(data_source) = built_in.data_source.as_ref() {
                                    let registry_path = engine
                                        .resource_manager
                                        .state()
                                        .resource_registry
                                        .safe_lock()
                                        .path()
                                        .parent()
                                        .map(|p| p.to_path_buf())
                                        .unwrap_or_default();

                                    let final_copy_path = asset::make_unique_path(
                                        &registry_path,
                                        path.to_str().unwrap(),
                                        &data_source.extension,
                                    );

                                    match File::create(&final_copy_path) {
                                        Ok(mut file) => {
                                            let result = file.write_all(&data_source.bytes);
                                            drop(file);
                                            if result.is_ok() {
                                                engine
                                                    .resource_manager
                                                    .request_untyped(&final_copy_path);
                                            }
                                            Log::verify(result);
                                            sender
                                                .send(Message::ShowInAssetBrowser(final_copy_path));
                                        }
                                        Err(err) => Log::err(format!(
                                            "Failed to create a file for resource at path {final_copy_path:?}. \
                                                Reason: {err}",
                                        )),
                                    }
                                }
                            } else if let Ok(canonical_path) = path.canonicalize() {
                                if let (Some(parent), Some(stem), Some(ext)) = (
                                    canonical_path.parent(),
                                    canonical_path.file_stem(),
                                    canonical_path.extension(),
                                ) {
                                    let stem = stem.to_string_lossy().to_string();
                                    let ext = ext.to_string_lossy().to_string();
                                    let final_copy_path =
                                        asset::make_unique_path(parent, &stem, &ext);
                                    let result = std::fs::copy(canonical_path, &final_copy_path);
                                    if result.is_ok() {
                                        engine.resource_manager.request_untyped(final_copy_path);
                                    }
                                    Log::verify(result);
                                }
                            }
                        }
                    }
                } else if message.destination() == self.copy_path {
                    if let Ok(canonical_path) = item.path.canonicalize() {
                        asset::put_path_to_clipboard(engine, canonical_path.as_os_str())
                    }
                } else if message.destination() == self.copy_file_name {
                    if let Some(file_name) = item.path.clone().file_name() {
                        asset::put_path_to_clipboard(engine, file_name)
                    }
                } else if message.destination() == self.rename {
                    if let (Some(file_stem), Some(extension), Some(parent)) = (
                        item.path.file_stem(),
                        item.path.extension(),
                        item.path.parent(),
                    ) {
                        let dialog = AssetRenameDialogBuilder::new(
                            WindowBuilder::new(
                                WidgetBuilder::new().with_width(350.0).with_height(100.0),
                            )
                            .with_title(WindowTitle::text("Rename a Resource"))
                            .with_remove_on_close(true)
                            .open(false),
                        )
                        .build(
                            file_stem.to_string_lossy().to_string(),
                            extension.to_string_lossy().to_string(),
                            parent.to_string_lossy().to_string(),
                            item.path.clone(),
                            engine.resource_manager.clone(),
                            &mut ui.build_ctx(),
                        );
                        ui.send_message(WindowMessage::open_modal(
                            dialog,
                            MessageDirection::ToWidget,
                            true,
                            true,
                        ));
                    }
                } else if message.destination() == self.reload {
                    if let Ok(resource) =
                        block_on(engine.resource_manager.request_untyped(&item.path))
                    {
                        engine.resource_manager.state().reload_resource(resource);
                    }
                }
            }
        } else if let Some(MessageBoxMessage::Close(result)) = message.data() {
            if message.destination() == self.delete_confirmation_dialog {
                if *result == MessageBoxResult::Yes {
                    Log::verify(std::fs::remove_file(&self.path_to_delete));
                }

                self.delete_confirmation_dialog = Handle::NONE;
            }
        }

        false
    }
}
