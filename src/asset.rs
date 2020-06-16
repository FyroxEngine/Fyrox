use crate::{
    gui::{
        UiMessage,
        Ui,
        UiNode,
        CustomUiNode,
        CustomWidget
    },
    GameEngine,
};
use std::{
    rc::Rc,
    cell::RefCell,
    path::Path,
    ops::{Deref, DerefMut}
};
use rg3d::{
    resource::texture::TextureKind,
    core::pool::Handle,
    gui::{
        widget::{WidgetBuilder},
        Control,
        window::{
            WindowBuilder,
            WindowTitle,
        },
        grid::{GridBuilder, Column, Row},
        wrap_panel::WrapPanelBuilder,
        file_browser::FileBrowserBuilder,
        message::{
            FileBrowserMessage,
            UiMessageData,
            WidgetMessage,
        },
        scroll_viewer::ScrollViewerBuilder,
        image::ImageBuilder,
        Orientation,
        Thickness,
    },
    utils::into_any_arc,
};
use std::path::PathBuf;
use crate::gui::{CustomMessage, BuildContext};

#[derive(Debug)]
pub struct AssetItem {
    widget: CustomWidget
}

impl Deref for AssetItem {
    type Target = CustomWidget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl DerefMut for AssetItem {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl Clone for AssetItem {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.raw_copy()
        }
    }
}

impl Control<CustomMessage, CustomUiNode> for AssetItem {
    fn raw_copy(&self) -> UiNode {
        UiNode::User(CustomUiNode::AssetItem(self.clone()))
    }

    fn handle_routed_message(&mut self, ui: &mut Ui, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);
    }
}

pub struct AssetBrowser {
    pub window: Handle<UiNode>,
    content_panel: Handle<UiNode>,
    folder_browser: Handle<UiNode>,
    preview: Handle<UiNode>,
}

impl AssetBrowser {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let path = PathBuf::from("./data");
        let content_panel;
        let folder_browser;
        let window = WindowBuilder::new(WidgetBuilder::new())
            .with_title(WindowTitle::text("Asset Browser"))
            .with_content(GridBuilder::new(WidgetBuilder::new()
                .with_child({
                    folder_browser = FileBrowserBuilder::new(WidgetBuilder::new()
                        .on_column(0))
                        .with_path(&path) // TODO: Bind to project when it will be available.
                        .with_filter(Rc::new(RefCell::new(|p: &Path| p.is_dir())))
                        .build(ctx);
                    folder_browser
                })
                .with_child({
                    ScrollViewerBuilder::new(WidgetBuilder::new()
                        .on_column(1))
                        .with_content( {
                            content_panel = WrapPanelBuilder::new(WidgetBuilder::new())
                                .with_orientation(Orientation::Horizontal)
                                .build(ctx);
                            content_panel
                        })
                        .build(ctx)
                }))
                .add_column(Column::strict(250.0))
                .add_column(Column::stretch())
                .add_row(Row::stretch())
                .build(ctx))
            .build(ctx);

        /*
        ui.send_message(UiMessage {
            data: UiMessageData::FileBrowser(FileBrowserMessage::SelectionChanged(path)),
            handled: false,
            destination: folder_browser
        });*/

        Self {
            window,
            content_panel,
            folder_browser,
            preview: Default::default()
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, engine: &mut GameEngine) {
        let ui = &mut engine.user_interface;
        let resource_manager = &mut engine.resource_manager.lock().unwrap();
        if message.destination == self.folder_browser {
            if let UiMessageData::FileBrowser(msg) = &message.data {
                if let FileBrowserMessage::SelectionChanged(path) = msg {
                    // Clean content panel first.
                    for &child in ui.node(self.content_panel).children() {
                        ui.send_message(WidgetMessage::remove(child));
                    }
                    // Get all supported assets from folder and generate previews for them.
                    if let Ok(dir_iter) = std::fs::read_dir(path) {
                        for p in dir_iter {
                            if let Ok(entry) = p {
                                let entry_path = entry.path();
                                if !entry_path.is_dir() {
                                    let texture = entry_path.extension().map(|ext| {
                                        match ext.to_string_lossy().as_ref() {
                                            "jpg" | "tga" | "png" | "bmp" => {
                                                into_any_arc(resource_manager.request_texture(&entry_path, TextureKind::RGBA8))
                                            }
                                            _ => None
                                        }
                                    }).flatten();

                                    let content = ImageBuilder::new(WidgetBuilder::new()
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_width(64.0)
                                        .with_height(64.0))
                                        .with_opt_texture(texture)
                                        .build(&mut ui.build_ctx());
                                    ui.send_message(WidgetMessage::link(content, self.content_panel));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

