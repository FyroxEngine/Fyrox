//! Special utility that allows you to fix paths to resources. It is very useful if you've
//! moved a resource in a file system, but a scene has old path.

use crate::{
    gui::{BuildContext, Ui, UiMessage, UiNode},
    make_scene_file_filter,
};
use rg3d::{
    core::{
        futures::executor::block_on,
        pool::Handle,
        visitor::{Visit, Visitor},
    },
    gui::{
        button::ButtonBuilder,
        file_browser::FileSelectorBuilder,
        formatted_text::WrapMode,
        grid::{Column, GridBuilder, Row},
        list_view::ListViewBuilder,
        message::TextMessage,
        message::{
            ButtonMessage, FileSelectorMessage, MessageDirection, UiMessageData, WindowMessage,
        },
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        HorizontalAlignment, Orientation, Thickness,
    },
    scene::Scene,
};

pub struct PathFixer {
    pub window: Handle<UiNode>,
    scene_path: Handle<UiNode>,
    scene_selector: Handle<UiNode>,
    load_scene: Handle<UiNode>,
    scene: Option<Scene>,
}

impl PathFixer {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let scene_selector = FileSelectorBuilder::new(
            WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
                .open(false)
                .with_title(WindowTitle::Text("Select a scene for diagnostics".into())),
        )
        .with_filter(make_scene_file_filter())
        .build(ctx);

        let load_scene;
        let scene_path;
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(500.0))
            .with_title(WindowTitle::text("Path Fixer"))
            .open(false)
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            scene_path = TextBuilder::new(WidgetBuilder::new().on_row(0))
                                .with_text("No scene loaded!")
                                .with_wrap(WrapMode::Word)
                                .build(ctx);
                            scene_path
                        })
                        .with_child(ListViewBuilder::new(WidgetBuilder::new().on_row(1)).build(ctx))
                        .with_child(
                            StackPanelBuilder::new(
                                WidgetBuilder::new()
                                    .with_horizontal_alignment(HorizontalAlignment::Right)
                                    .on_row(2)
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
                                    .with_child(
                                        ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_width(100.0)
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_text("OK")
                                        .build(ctx),
                                    )
                                    .with_child(
                                        ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_width(100.0)
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_text("Cancel")
                                        .build(ctx),
                                    ),
                            )
                            .with_orientation(Orientation::Horizontal)
                            .build(ctx),
                        ),
                )
                .add_row(Row::auto())
                .add_row(Row::stretch())
                .add_row(Row::strict(28.0))
                .add_column(Column::stretch())
                .build(ctx),
            )
            .build(ctx);

        Self {
            window,
            scene_selector,
            load_scene,
            scene_path,
            scene: None,
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, ui: &Ui) {
        match message.data() {
            UiMessageData::FileSelector(FileSelectorMessage::Commit(path)) => {
                if message.destination() == self.scene_selector {
                    let mut scene = Scene::default();
                    let message;
                    match block_on(Visitor::load_binary(path)) {
                        Ok(mut visitor) => {
                            if let Err(e) = scene.visit("Scene", &mut visitor) {
                                message = format!(
                                    "Failed to load a scene {}\nReason: {}",
                                    path.display(),
                                    e
                                );
                            } else {
                                self.scene = Some(scene);

                                message = path.to_string_lossy().to_string();
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
                }
            }
            UiMessageData::Button(ButtonMessage::Click) => {
                if message.destination() == self.load_scene {
                    ui.send_message(WindowMessage::open_modal(
                        self.scene_selector,
                        MessageDirection::ToWidget,
                        true,
                    ));
                }
            }
            _ => {}
        }
    }
}
