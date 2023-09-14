use crate::{
    button::{ButtonBuilder, ButtonMessage},
    core::pool::Handle,
    define_constructor,
    file_browser::{FileSelectorBuilder, FileSelectorMessage},
    grid::{Column, GridBuilder, Row},
    message::{MessageDirection, UiMessage},
    text::TextMessage,
    text_box::TextBoxBuilder,
    widget::{Widget, WidgetBuilder, WidgetMessage},
    window::{WindowBuilder, WindowMessage, WindowTitle},
    BuildContext, Control, Thickness, UiNode, UserInterface,
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
    path::PathBuf,
};

#[derive(Debug, Clone, PartialEq)]
pub enum PathEditorMessage {
    Path(PathBuf),
}

impl PathEditorMessage {
    define_constructor!(PathEditorMessage:Path => fn path(PathBuf), layout: false);
}

#[derive(Clone)]
pub struct PathEditor {
    pub widget: Widget,
    pub text_field: Handle<UiNode>,
    pub select: Handle<UiNode>,
    pub selector: Handle<UiNode>,
    pub path: PathBuf,
}

crate::define_widget_deref!(PathEditor);

impl Control for PathEditor {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.select {
                self.selector = FileSelectorBuilder::new(
                    WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(450.0))
                        .open(false)
                        .with_title(WindowTitle::text("Select a Path")),
                )
                .build(&mut ui.build_ctx());

                ui.send_message(FileSelectorMessage::path(
                    self.selector,
                    MessageDirection::ToWidget,
                    self.path.clone(),
                ));
                ui.send_message(WindowMessage::open_modal(
                    self.selector,
                    MessageDirection::ToWidget,
                    true,
                ));
            }
        } else if let Some(PathEditorMessage::Path(path)) = message.data() {
            if message.destination() == self.handle
                && message.direction() == MessageDirection::ToWidget
                && &self.path != path
            {
                self.path = path.clone();

                ui.send_message(TextMessage::text(
                    self.text_field,
                    MessageDirection::ToWidget,
                    path.to_string_lossy().to_string(),
                ));
                ui.send_message(message.reverse());
            }
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        if let Some(FileSelectorMessage::Commit(path)) = message.data() {
            if message.destination() == self.selector && &self.path != path {
                ui.send_message(WidgetMessage::remove(
                    self.selector,
                    MessageDirection::ToWidget,
                ));

                ui.send_message(PathEditorMessage::path(
                    self.handle,
                    MessageDirection::ToWidget,
                    path.clone(),
                ));
            }
        }
    }
}

pub struct PathEditorBuilder {
    widget_builder: WidgetBuilder,
    path: PathBuf,
}

impl PathEditorBuilder {
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
        let text_field;
        let select;
        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child({
                    text_field = TextBoxBuilder::new(
                        WidgetBuilder::new()
                            .on_column(0)
                            .with_margin(Thickness::uniform(1.0)),
                    )
                    .with_text(self.path.to_string_lossy())
                    .with_editable(false)
                    .build(ctx);
                    text_field
                })
                .with_child({
                    select = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .on_column(1)
                            .with_width(30.0)
                            .with_margin(Thickness::uniform(1.0)),
                    )
                    .with_text("...")
                    .build(ctx);
                    select
                }),
        )
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .build(ctx);

        let canvas = PathEditor {
            widget: self
                .widget_builder
                .with_child(grid)
                .with_preview_messages(true)
                .build(),
            text_field,
            select,
            selector: Default::default(),
            path: self.path,
        };
        ctx.add_node(UiNode::new(canvas))
    }
}
