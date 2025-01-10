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

//! Special utility that allows you to fix paths to resources. It is very useful if you've
//! moved a resource in a file system, but a scene has old path.

use crate::{
    fyrox::{
        asset::{manager::ResourceManager, untyped::ResourceKind, untyped::UntypedResource},
        core::{
            futures::executor::block_on, pool::Handle, replace_slashes, some_or_return,
            visitor::Visitor,
        },
        engine::SerializationContext,
        graph::{BaseSceneGraph, SceneGraph},
        gui::{
            border::BorderBuilder,
            button::{ButtonBuilder, ButtonMessage},
            decorator::DecoratorBuilder,
            file_browser::{FileSelectorBuilder, FileSelectorMessage},
            formatted_text::WrapMode,
            grid::{Column, GridBuilder, Row},
            list_view::{ListView, ListViewBuilder, ListViewMessage},
            menu::MenuItemMessage,
            message::{MessageDirection, UiMessage},
            stack_panel::StackPanelBuilder,
            text::{Text, TextBuilder, TextMessage},
            widget::{WidgetBuilder, WidgetMessage},
            window::{WindowBuilder, WindowMessage, WindowTitle},
            BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
            VerticalAlignment,
        },
        scene::{Scene, SceneLoader},
    },
    make_scene_file_filter,
    menu::create_menu_item,
    plugin::EditorPlugin,
    Editor, Message,
};
use fyrox::gui::style::resource::StyleResourceExt;
use fyrox::gui::style::Style;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct PathFixer {
    pub window: Handle<UiNode>,
    scene_path_value: PathBuf,
    scene_path: Handle<UiNode>,
    scene_selector: Handle<UiNode>,
    load_scene: Handle<UiNode>,
    scene: Option<Scene>,
    orphaned_scene_resources: Vec<UntypedResource>,
    resources_list: Handle<UiNode>,
    cancel: Handle<UiNode>,
    ok: Handle<UiNode>,
    selection: Option<usize>,
    fix: Handle<UiNode>,
    resource_path: Handle<UiNode>,
    new_path_selector: Handle<UiNode>,
    auto_fix: Handle<UiNode>,
}

fn find_file(name: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for dir in fyrox::walkdir::WalkDir::new("../../..")
        .into_iter()
        .flatten()
    {
        let path = dir.path();
        if let Some(file_name) = path.file_name() {
            if file_name == name {
                files.push(path.to_owned());
            }
        }
    }
    files
}

impl PathFixer {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let scene_selector = FileSelectorBuilder::new(
            WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
                .open(false)
                .with_title(WindowTitle::text("Select a scene for diagnostics")),
        )
        .with_filter(make_scene_file_filter())
        .build(ctx);

        let new_path_selector = FileSelectorBuilder::new(
            WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
                .open(false)
                .with_title(WindowTitle::text("Select a new path to the resource")),
        )
        .build(ctx);

        let load_scene;
        let scene_path;
        let resources_list;
        let cancel;
        let ok;
        let auto_fix;
        let fix;
        let resource_path;
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(400.0).with_height(500.0))
            .with_title(WindowTitle::text("Path Fixer"))
            .open(false)
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            scene_path = TextBuilder::new(WidgetBuilder::new().on_row(0))
                                .with_text("Scene: No scene loaded!")
                                .with_wrap(WrapMode::Word)
                                .build(ctx);
                            scene_path
                        })
                        .with_child(
                            GridBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(1)
                                    .with_child({
                                        resource_path =
                                            TextBuilder::new(WidgetBuilder::new().on_column(0))
                                                .with_vertical_text_alignment(
                                                    VerticalAlignment::Center,
                                                )
                                                .build(ctx);
                                        resource_path
                                    })
                                    .with_child({
                                        fix = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_width(40.0)
                                                .on_column(1)
                                                .with_enabled(false)
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_text("Fix...")
                                        .build(ctx);
                                        fix
                                    }),
                            )
                            .add_column(Column::stretch())
                            .add_column(Column::auto())
                            .add_row(Row::stretch())
                            .build(ctx),
                        )
                        .with_child({
                            resources_list =
                                ListViewBuilder::new(WidgetBuilder::new().on_row(2)).build(ctx);
                            resources_list
                        })
                        .with_child(
                            StackPanelBuilder::new(
                                WidgetBuilder::new()
                                    .with_horizontal_alignment(HorizontalAlignment::Right)
                                    .on_row(3)
                                    .with_child({
                                        load_scene = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_width(100.0)
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_text("Load Scene...")
                                        .build(ctx);
                                        load_scene
                                    })
                                    .with_child({
                                        auto_fix = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_width(100.0)
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_text("Auto Fix")
                                        .build(ctx);
                                        auto_fix
                                    })
                                    .with_child({
                                        ok = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_width(100.0)
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_text("OK")
                                        .build(ctx);
                                        ok
                                    })
                                    .with_child({
                                        cancel = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_width(100.0)
                                                .with_margin(Thickness::uniform(1.0)),
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
                .add_row(Row::strict(28.0))
                .add_row(Row::stretch())
                .add_row(Row::strict(28.0))
                .add_column(Column::stretch())
                .build(ctx),
            )
            .with_remove_on_close(true)
            .build(ctx);

        Self {
            window,
            scene_selector,
            load_scene,
            scene_path,
            scene: None,
            orphaned_scene_resources: Default::default(),
            resources_list,
            ok,
            cancel,
            resource_path,
            fix,
            selection: None,
            new_path_selector,
            auto_fix,
            scene_path_value: Default::default(),
        }
    }

    fn fix_path(&mut self, index: usize, new_path: PathBuf, ui: &UserInterface) {
        let text = new_path.to_string_lossy().to_string();

        self.orphaned_scene_resources[index].set_kind(ResourceKind::External(new_path));

        let item = ui
            .node(self.resources_list)
            .cast::<ListView>()
            .unwrap()
            .items()[index];
        let item_text = ui.find_handle(item, &mut |n| n.cast::<Text>().is_some());

        assert!(item_text.is_some());

        ui.send_message(WidgetMessage::foreground(
            item_text,
            MessageDirection::ToWidget,
            ui.style.property(Style::BRUSH_OK),
        ));
        ui.send_message(TextMessage::text(
            item_text,
            MessageDirection::ToWidget,
            text.clone(),
        ));

        ui.send_message(TextMessage::text(
            self.resource_path,
            MessageDirection::ToWidget,
            text,
        ));
    }

    fn destroy(self, ui: &UserInterface) {
        ui.send_message(WidgetMessage::remove(
            self.new_path_selector,
            MessageDirection::ToWidget,
        ));
        ui.send_message(WidgetMessage::remove(
            self.scene_selector,
            MessageDirection::ToWidget,
        ));
    }

    pub fn handle_ui_message(
        mut self,
        message: &UiMessage,
        ui: &mut UserInterface,
        serialization_context: Arc<SerializationContext>,
        resource_manager: ResourceManager,
    ) -> Option<Self> {
        if let Some(FileSelectorMessage::Commit(path)) = message.data::<FileSelectorMessage>() {
            if message.destination() == self.scene_selector {
                let message;
                match block_on(Visitor::load_binary(path)) {
                    Ok(mut visitor) => {
                        match SceneLoader::load(
                            "Scene",
                            serialization_context,
                            resource_manager.clone(),
                            &mut visitor,
                            Some(path.clone()),
                        ) {
                            Err(e) => {
                                message = format!(
                                    "Failed to load a scene {}\nReason: {}",
                                    path.display(),
                                    e
                                );
                            }
                            Ok(loader) => {
                                let scene = block_on(loader.finish());

                                // Gather resources.
                                let scene_resources = scene.collect_used_resources();

                                // Turn hash map into vec to be able to index it.
                                self.orphaned_scene_resources = scene_resources
                                    .into_iter()
                                    .filter(|r| !r.kind().path().is_some_and(|p| p.exists()))
                                    .collect::<Vec<_>>();

                                let ctx = &mut ui.build_ctx();
                                let items = self
                                    .orphaned_scene_resources
                                    .iter()
                                    .map(|r| {
                                        DecoratorBuilder::new(BorderBuilder::new(
                                            WidgetBuilder::new().with_height(22.0).with_child(
                                                TextBuilder::new(
                                                    WidgetBuilder::new()
                                                        .with_margin(Thickness::uniform(1.0))
                                                        .with_foreground(
                                                            ctx.style.property(Style::BRUSH_ERROR),
                                                        ),
                                                )
                                                .with_vertical_text_alignment(
                                                    VerticalAlignment::Center,
                                                )
                                                .with_text(r.kind().to_string())
                                                .build(ctx),
                                            ),
                                        ))
                                        .build(ctx)
                                    })
                                    .collect::<Vec<_>>();

                                ui.send_message(ListViewMessage::items(
                                    self.resources_list,
                                    MessageDirection::ToWidget,
                                    items,
                                ));
                                ui.send_message(ListViewMessage::selection(
                                    self.resources_list,
                                    MessageDirection::ToWidget,
                                    Default::default(),
                                ));

                                self.scene = Some(scene);
                                self.scene_path_value.clone_from(path);

                                message = format!("Scene: {}", path.display());
                            }
                        }
                    }
                    Err(e) => {
                        message =
                            format!("Failed to load a scene {}\nReason: {}", path.display(), e);
                    }
                }

                ui.send_message(TextMessage::text(
                    self.scene_path,
                    MessageDirection::ToWidget,
                    message,
                ));
            } else if message.destination() == self.new_path_selector {
                if let Some(selection) = self.selection {
                    self.fix_path(selection, replace_slashes(path), ui);
                }
            }
        } else if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == self.load_scene {
                ui.send_message(WindowMessage::open_modal(
                    self.scene_selector,
                    MessageDirection::ToWidget,
                    true,
                    true,
                ));
            } else if message.destination() == self.cancel {
                ui.send_message(WindowMessage::close(
                    self.window,
                    MessageDirection::ToWidget,
                ));
            } else if message.destination() == self.ok {
                ui.send_message(WindowMessage::close(
                    self.window,
                    MessageDirection::ToWidget,
                ));

                if let Some(mut scene) = self.scene.take() {
                    let mut visitor = Visitor::new();
                    scene
                        .save("Scene", &mut visitor)
                        .expect("Unable to visit a scene!");
                    visitor
                        .save_binary(&self.scene_path_value)
                        .expect("Unable to save a scene!");
                }

                ui.send_message(TextMessage::text(
                    self.scene_path,
                    MessageDirection::ToWidget,
                    "No scene loaded!".to_owned(),
                ));
                ui.send_message(ListViewMessage::items(
                    self.resources_list,
                    MessageDirection::ToWidget,
                    Default::default(),
                ));
                ui.send_message(TextMessage::text(
                    self.resource_path,
                    MessageDirection::ToWidget,
                    Default::default(),
                ));
                ui.send_message(WidgetMessage::enabled(
                    self.fix,
                    MessageDirection::ToWidget,
                    false,
                ));
            } else if message.destination() == self.fix {
                if let Some(selection) = self.selection {
                    // Try to find a resource by its file name.
                    if let Some(mut resource_path) =
                        self.orphaned_scene_resources[selection].kind().into_path()
                    {
                        if let Some(file_name) = resource_path.file_name() {
                            let candidates = find_file(file_name.as_ref());
                            // Skip ambiguous file paths.
                            if candidates.len() == 1 {
                                resource_path.clone_from(candidates.first().unwrap());
                            }
                        }

                        // Pop parts of the path one by one until existing found.
                        while !resource_path.exists() {
                            resource_path.pop();
                        }

                        // Set it as a path for the selector to reduce amount of clicks needed.
                        ui.send_message(FileSelectorMessage::path(
                            self.new_path_selector,
                            MessageDirection::ToWidget,
                            resource_path,
                        ));

                        ui.send_message(WindowMessage::open_modal(
                            self.new_path_selector,
                            MessageDirection::ToWidget,
                            true,
                            true,
                        ));
                    }
                }
            } else if message.destination() == self.auto_fix {
                for (i, orphaned_resource) in
                    self.orphaned_scene_resources.clone().iter().enumerate()
                {
                    if let Some(file_name) =
                        orphaned_resource.kind().path().and_then(|p| p.file_name())
                    {
                        let candidates = find_file(file_name.as_ref());
                        // Skip ambiguous file paths.
                        if candidates.len() == 1 {
                            let new_path = candidates.first().unwrap().clone();
                            self.fix_path(i, replace_slashes(new_path), ui);
                        }
                    }
                }
            }
        } else if let Some(ListViewMessage::SelectionChanged(selection)) =
            message.data::<ListViewMessage>()
        {
            if message.destination() == self.resources_list {
                self.selection = selection.first().cloned();

                if let Some(selection) = self.selection {
                    ui.send_message(TextMessage::text(
                        self.resource_path,
                        MessageDirection::ToWidget,
                        format!(
                            "Resource: {}",
                            self.orphaned_scene_resources[selection].kind()
                        ),
                    ))
                } else {
                    ui.send_message(TextMessage::text(
                        self.resource_path,
                        MessageDirection::ToWidget,
                        "No resource selected".to_owned(),
                    ));
                }

                ui.send_message(WidgetMessage::enabled(
                    self.fix,
                    MessageDirection::ToWidget,
                    self.selection.is_some(),
                ));
            }
        } else if let Some(WindowMessage::Close) = message.data() {
            if message.destination() == self.window {
                self.destroy(ui);
                return None;
            }
        }

        Some(self)
    }

    pub fn handle_message(&mut self, message: &Message, ui: &UserInterface) {
        if let Message::Configure { working_directory } = message {
            ui.send_message(FileSelectorMessage::root(
                self.new_path_selector,
                MessageDirection::ToWidget,
                Some(working_directory.to_owned()),
            ));
            ui.send_message(FileSelectorMessage::root(
                self.scene_selector,
                MessageDirection::ToWidget,
                Some(working_directory.to_owned()),
            ));
        }
    }

    fn open(&self, ui: &UserInterface) {
        ui.send_message(WindowMessage::open_modal(
            self.window,
            MessageDirection::ToWidget,
            true,
            true,
        ));
    }
}

#[derive(Default)]
pub struct PathFixerPlugin {
    path_fixer: Option<PathFixer>,
    open_path_fixer: Handle<UiNode>,
}

impl PathFixerPlugin {
    fn on_open_curve_editor_clicked(&mut self, editor: &mut Editor) {
        let ui = editor.engine.user_interfaces.first_mut();
        let ctx = &mut ui.build_ctx();
        let path_fixer = self.path_fixer.get_or_insert_with(|| PathFixer::new(ctx));
        path_fixer.open(ui);
    }
}

impl EditorPlugin for PathFixerPlugin {
    fn on_start(&mut self, editor: &mut Editor) {
        let ui = editor.engine.user_interfaces.first_mut();
        let ctx = &mut ui.build_ctx();
        self.open_path_fixer = create_menu_item("Path Fixer", vec![], ctx);
        ui.send_message(MenuItemMessage::add_item(
            editor.menu.utils_menu.menu,
            MessageDirection::ToWidget,
            self.open_path_fixer,
        ));
    }

    fn on_ui_message(&mut self, message: &mut UiMessage, editor: &mut Editor) {
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.open_path_fixer {
                self.on_open_curve_editor_clicked(editor)
            }
        }
        let path_fixer = some_or_return!(self.path_fixer.take());
        let ui = editor.engine.user_interfaces.first_mut();
        self.path_fixer = path_fixer.handle_ui_message(
            message,
            ui,
            editor.engine.serialization_context.clone(),
            editor.engine.resource_manager.clone(),
        );
    }

    fn on_message(&mut self, message: &Message, editor: &mut Editor) {
        let path_fixer = some_or_return!(self.path_fixer.as_mut());
        let ui = editor.engine.user_interfaces.first_mut();
        path_fixer.handle_message(message, ui);
    }
}
