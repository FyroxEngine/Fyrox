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

use crate::settings::build::BuildSettings;
use crate::{
    fyrox::{
        asset::manager::ResourceManager,
        core::{
            log::{Log, LogMessage, MessageKind},
            platform::TargetPlatform,
            pool::Handle,
        },
        graph::SceneGraph,
        gui::{
            border::BorderBuilder,
            button::{Button, ButtonBuilder, ButtonMessage},
            decorator::DecoratorBuilder,
            dropdown_list::{DropdownList, DropdownListBuilder, DropdownListMessage},
            formatted_text::WrapMode,
            grid::{Column, GridBuilder, Row},
            inspector::{
                editors::PropertyEditorDefinitionContainer, Inspector, InspectorBuilder,
                InspectorContext, InspectorContextArgs, InspectorMessage, PropertyAction,
            },
            list_view::{ListView, ListViewBuilder, ListViewMessage},
            message::UiMessage,
            scroll_viewer::{ScrollViewer, ScrollViewerBuilder, ScrollViewerMessage},
            stack_panel::{StackPanel, StackPanelBuilder},
            style::{resource::StyleResourceExt, Style},
            text::{Text, TextBuilder},
            utils::make_dropdown_list_option,
            widget::{WidgetBuilder, WidgetMessage},
            window::{Window, WindowAlignment, WindowBuilder, WindowMessage, WindowTitle},
            wrap_panel::WrapPanelBuilder,
            BuildContext, HorizontalAlignment, Orientation, Thickness, UserInterface,
            VerticalAlignment,
        },
    },
    message::MessageSender,
    Message,
};
use fyrox_build_tools::export::{BuildResult, ExportOptions};
use std::sync::{
    atomic::AtomicBool,
    mpsc::{self, Receiver},
    Arc,
};
use strum::VariantNames;

pub struct ExportWindow {
    pub window: Handle<Window>,
    log: Handle<StackPanel>,
    export: Handle<Button>,
    cancel: Handle<Button>,
    log_scroll_viewer: Handle<ScrollViewer>,
    cancel_flag: Arc<AtomicBool>,
    log_message_receiver: Option<Receiver<LogMessage>>,
    build_result_receiver: Option<Receiver<BuildResult>>,
    target_platform_list: Handle<ListView>,
    export_options: ExportOptions,
    inspector: Handle<Inspector>,
    build_targets_selector: Handle<DropdownList>,
    child_processes: Vec<std::process::Child>,
    build_targets: Vec<String>,
}

fn make_title_text(text: &str, row: usize, ctx: &mut BuildContext) -> Handle<Text> {
    TextBuilder::new(
        WidgetBuilder::new()
            .on_row(row)
            .with_foreground(ctx.style.property(ExportWindow::TITLE_BRUSH))
            .with_margin(Thickness::uniform(2.0)),
    )
    .with_text(text)
    .build(ctx)
}

impl ExportWindow {
    pub const TITLE_BRUSH: &'static str = "ExportWindow.TitleBrush";

    pub fn new(ctx: &mut BuildContext) -> Self {
        let instructions =
            "Select the target directory in which you want to export the current project. You can \
            also specify the assets, that will be included in the final build. Previous content of \
            the build folder will be completely erased when you press Export.";

        let export;
        let cancel;
        let log;
        let log_scroll_viewer;
        let target_platform_list;
        let export_options = ExportOptions::default();
        let build_targets = vec![export_options.build_target.clone()];

        let platform_section = StackPanelBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .with_child(make_title_text("Target Platform", 0, ctx))
                .with_child({
                    target_platform_list = ListViewBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(2.0))
                            .with_height(60.0),
                    )
                    .with_items_panel(
                        WrapPanelBuilder::new(WidgetBuilder::new())
                            .with_orientation(Orientation::Horizontal)
                            .build(ctx),
                    )
                    .with_items(
                        TargetPlatform::VARIANTS
                            .iter()
                            .enumerate()
                            .map(|(i, p)| {
                                DecoratorBuilder::new(BorderBuilder::new(
                                    WidgetBuilder::new()
                                        .with_width(100.0)
                                        .with_height(50.0)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_child(
                                            TextBuilder::new(WidgetBuilder::new())
                                                .with_vertical_text_alignment(
                                                    VerticalAlignment::Center,
                                                )
                                                .with_horizontal_text_alignment(
                                                    HorizontalAlignment::Center,
                                                )
                                                .with_text(*p)
                                                .build(ctx),
                                        ),
                                ))
                                .with_selected(i == 0)
                                .build(ctx)
                                .to_base()
                            })
                            .collect::<Vec<_>>(),
                    )
                    .build(ctx);
                    target_platform_list
                }),
        )
        .build(ctx);

        let build_targets_selector;
        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(2)
                .with_child(
                    TextBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(2.0)))
                        .with_vertical_text_alignment(VerticalAlignment::Center)
                        .with_text("Build Target")
                        .build(ctx),
                )
                .with_child({
                    build_targets_selector =
                        DropdownListBuilder::new(WidgetBuilder::new().on_column(1))
                            .with_items(
                                build_targets
                                    .iter()
                                    .map(|opt| make_dropdown_list_option(ctx, opt))
                                    .collect::<Vec<_>>(),
                            )
                            .with_selected(0)
                            .build(ctx);
                    build_targets_selector
                }),
        )
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .add_row(Row::strict(22.0))
        .build(ctx);

        let inspector;
        let export_options_section = BorderBuilder::new(
            WidgetBuilder::new()
                .on_row(3)
                .with_margin(Thickness::uniform(2.0))
                .with_background(ctx.style.property(Style::BRUSH_LIGHT))
                .with_child(
                    ScrollViewerBuilder::new(
                        WidgetBuilder::new().with_margin(Thickness::uniform(2.0)),
                    )
                    .with_content({
                        let context = InspectorContext::from_object(InspectorContextArgs {
                            object: &export_options,
                            ctx,
                            definition_container: Arc::new(
                                PropertyEditorDefinitionContainer::with_default_editors(),
                            ),
                            environment: None,
                            layer_index: 0,
                            generate_property_string_values: true,
                            filter: Default::default(),
                            name_column_width: 150.0,
                            base_path: Default::default(),
                            has_parent_object: false,
                        });

                        inspector = InspectorBuilder::new(WidgetBuilder::new())
                            .with_context(context)
                            .build(ctx);
                        inspector
                    })
                    .build(ctx),
                ),
        )
        .build(ctx);

        let log_section = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(4)
                .with_child(make_title_text("Export Log", 0, ctx))
                .with_child(
                    BorderBuilder::new(
                        WidgetBuilder::new()
                            .on_row(1)
                            .with_background(ctx.style.property(Style::BRUSH_DARKER))
                            .with_margin(Thickness::uniform(2.0))
                            .with_child({
                                log_scroll_viewer = ScrollViewerBuilder::new(
                                    WidgetBuilder::new().with_margin(Thickness::uniform(2.0)),
                                )
                                .with_content({
                                    log = StackPanelBuilder::new(WidgetBuilder::new()).build(ctx);
                                    log
                                })
                                .build(ctx);
                                log_scroll_viewer
                            }),
                    )
                    .build(ctx),
                ),
        )
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        let buttons_section = StackPanelBuilder::new(
            WidgetBuilder::new()
                .on_row(5)
                .with_horizontal_alignment(HorizontalAlignment::Right)
                .with_child({
                    export = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .with_width(100.0)
                            .with_margin(Thickness::uniform(2.0)),
                    )
                    .with_text("Export")
                    .build(ctx);
                    export
                })
                .with_child({
                    cancel = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .with_width(100.0)
                            .with_margin(Thickness::uniform(2.0)),
                    )
                    .with_text("Cancel")
                    .build(ctx);
                    cancel
                }),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(500.0).with_height(650.0))
            .open(false)
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(
                            TextBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(0)
                                    .with_margin(Thickness::uniform(2.0)),
                            )
                            .with_wrap(WrapMode::Word)
                            .with_text(instructions)
                            .build(ctx),
                        )
                        .with_child(platform_section)
                        .with_child(grid)
                        .with_child(export_options_section)
                        .with_child(log_section)
                        .with_child(buttons_section),
                )
                .add_row(Row::auto())
                .add_row(Row::auto())
                .add_row(Row::auto())
                .add_row(Row::strict(200.0))
                .add_row(Row::stretch())
                .add_row(Row::strict(32.0))
                .add_column(Column::stretch())
                .build(ctx),
            )
            .with_title(WindowTitle::text("Export Project"))
            .build(ctx);

        Self {
            window,
            log,
            export,
            cancel,
            log_scroll_viewer,
            cancel_flag: Arc::new(AtomicBool::new(false)),
            log_message_receiver: None,
            build_result_receiver: None,
            target_platform_list,
            export_options,
            inspector,
            build_targets_selector,
            child_processes: Default::default(),
            build_targets,
        }
    }

    pub fn open(&self, ui: &UserInterface) {
        ui.send(
            self.window,
            WindowMessage::Open {
                alignment: WindowAlignment::Center,
                modal: true,
                focus_content: true,
            },
        );
    }

    fn kill_child_processes(&mut self) {
        for mut child_process in self.child_processes.drain(..) {
            let _ = child_process.kill();
        }
    }

    pub fn close_and_destroy(&mut self, ui: &UserInterface) {
        ui.send(self.window, WindowMessage::Close);
        ui.send(self.window, WidgetMessage::Remove);
        self.log_message_receiver = None;
        self.build_result_receiver = None;
        self.kill_child_processes();
    }

    fn clear_log(&self, ui: &UserInterface) {
        for child in ui[self.log].children() {
            ui.send(*child, WidgetMessage::Remove);
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        ui: &mut UserInterface,
        sender: &MessageSender,
        resource_manager: ResourceManager,
        settings: &BuildSettings,
    ) {
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.export {
                self.kill_child_processes();

                let (tx, rx) = mpsc::channel();
                Log::add_listener(tx);
                self.log_message_receiver = Some(rx);

                let (tx, rx) = mpsc::channel();
                self.build_result_receiver = Some(rx);

                ui.send(self.export, WidgetMessage::Enabled(false));

                self.clear_log(ui);

                let cancel_flag = self.cancel_flag.clone();
                let export_options = self.export_options.clone();

                Log::verify(
                    std::thread::Builder::new()
                        .name("ExportWorkerThread".to_string())
                        .spawn(move || {
                            tx.send(fyrox_build_tools::export::export(
                                export_options,
                                cancel_flag,
                                resource_manager,
                            ))
                            .expect("Channel must exist!")
                        }),
                );
            } else if message.destination() == self.cancel {
                self.close_and_destroy(ui);
            }
        } else if let Some(ListViewMessage::Selection(selection)) =
            message.data_from(self.target_platform_list)
        {
            if let Some(index) = selection.first().cloned() {
                match index {
                    0 => self.export_options.target_platform = TargetPlatform::PC,
                    1 => self.export_options.target_platform = TargetPlatform::WebAssembly,
                    2 => self.export_options.target_platform = TargetPlatform::Android,
                    _ => Log::err("Unhandled platform index!"),
                }

                let build_targets = match self.export_options.target_platform {
                    TargetPlatform::PC => &settings.pc_build_targets,
                    TargetPlatform::WebAssembly => &settings.wasm_build_targets,
                    TargetPlatform::Android => &settings.android_build_targets,
                };

                if let Some(first_build_target) = build_targets.first().cloned() {
                    self.export_options.build_target = first_build_target;
                }
                self.build_targets = build_targets.clone();

                let ui_items = self
                    .build_targets
                    .iter()
                    .map(|name| make_dropdown_list_option(&mut ui.build_ctx(), name))
                    .collect::<Vec<_>>();

                ui.send(
                    self.build_targets_selector,
                    DropdownListMessage::Items(ui_items),
                );
            }
        } else if let Some(InspectorMessage::PropertyChanged(args)) =
            message.data_from(self.inspector)
        {
            PropertyAction::from_field_kind(&args.action).apply(
                &args.path(),
                &mut self.export_options,
                &mut |result| {
                    Log::verify(result);
                },
            );
            sender.send(Message::ForceSync);
        } else if let Some(DropdownListMessage::Selection(Some(index))) =
            message.data_from(self.build_targets_selector)
        {
            self.export_options.build_target = self.build_targets[*index].clone();
        }
    }

    pub fn sync_to_model(&self, ui: &mut UserInterface) {
        if let Ok(inspector) = ui.try_get(self.inspector) {
            let ctx = inspector.context().clone();
            if let Err(sync_errors) = ctx.sync(
                &self.export_options,
                ui,
                0,
                true,
                Default::default(),
                Default::default(),
            ) {
                for error in sync_errors {
                    Log::err(format!("Failed to sync property. Reason: {error:?}"))
                }
            }
        }
    }

    pub fn update(&mut self, ui: &mut UserInterface) {
        if let Some(log_message_receiver) = self.log_message_receiver.as_mut() {
            while let Ok(message) = log_message_receiver.try_recv() {
                let ctx = &mut ui.build_ctx();
                let foreground = match message.kind {
                    MessageKind::Information => ctx.style.property(Style::BRUSH_INFORMATION),
                    MessageKind::Warning => ctx.style.property(Style::BRUSH_WARNING),
                    MessageKind::Error => ctx.style.property(Style::BRUSH_ERROR),
                };
                let entry = TextBuilder::new(
                    WidgetBuilder::new()
                        .with_margin(Thickness::uniform(1.0))
                        .with_foreground(foreground),
                )
                .with_wrap(WrapMode::Letter)
                .with_text(format!("> {}", message.content))
                .build(ctx);

                ui.send(entry, WidgetMessage::link_with(self.log));
                ui.send(self.log_scroll_viewer, ScrollViewerMessage::ScrollToEnd);
            }
        }

        if let Some(receiver) = self.build_result_receiver.as_ref() {
            if let Ok(result) = receiver.try_recv() {
                match result {
                    Ok(mut output) => {
                        Log::info("Build finished!");
                        self.child_processes.append(&mut output.child_processes);
                    }
                    Err(err) => Log::err(format!("Build failed! Reason: {err}")),
                }

                ui.send(self.export, WidgetMessage::Enabled(true));
            }
        }
    }
}
