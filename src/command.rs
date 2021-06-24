use crate::{
    gui::{BuildContext, Ui, UiMessage, UiNode},
    load_image, send_sync_message, Message,
};
use rg3d::{
    core::{color::Color, pool::Handle, scope_profile},
    gui::{
        brush::Brush,
        button::ButtonBuilder,
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        list_view::ListViewBuilder,
        message::{ButtonMessage, ListViewMessage, MessageDirection, UiMessageData},
        scroll_viewer::ScrollViewerBuilder,
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        Orientation, Thickness,
    },
};
use std::{fmt::Debug, sync::mpsc::Sender};

pub trait Command<'a> {
    type Context;

    fn name(&mut self, context: &Self::Context) -> String;
    fn execute(&mut self, context: &mut Self::Context);
    fn revert(&mut self, context: &mut Self::Context);
    fn finalize(&mut self, _: &mut Self::Context) {}
}

pub struct CommandStack<C> {
    commands: Vec<C>,
    top: Option<usize>,
    debug: bool,
}

impl<C> CommandStack<C> {
    pub fn new(debug: bool) -> Self {
        Self {
            commands: Default::default(),
            top: None,
            debug,
        }
    }

    pub fn do_command<'a, Ctx>(&mut self, mut command: C, mut context: Ctx)
    where
        C: Command<'a, Context = Ctx> + Debug,
    {
        if self.commands.is_empty() {
            self.top = Some(0);
        } else {
            // Advance top
            match self.top.as_mut() {
                None => self.top = Some(0),
                Some(top) => *top += 1,
            }
            // Drop everything after top.
            let top = self.top.unwrap_or(0);
            if top < self.commands.len() {
                for mut dropped_command in self.commands.drain(top..) {
                    if self.debug {
                        println!("Finalizing command {:?}", dropped_command);
                    }
                    dropped_command.finalize(&mut context);
                }
            }
        }

        if self.debug {
            println!("Executing command {:?}", command);
        }

        command.execute(&mut context);

        self.commands.push(command);
    }

    pub fn undo<'a, Ctx>(&mut self, mut context: Ctx)
    where
        C: Command<'a, Context = Ctx> + Debug,
    {
        if !self.commands.is_empty() {
            if let Some(top) = self.top.as_mut() {
                if let Some(command) = self.commands.get_mut(*top) {
                    if self.debug {
                        println!("Undo command {:?}", command);
                    }
                    command.revert(&mut context)
                }
                if *top == 0 {
                    self.top = None;
                } else {
                    *top -= 1;
                }
            }
        }
    }

    pub fn redo<'a, Ctx>(&mut self, mut context: Ctx)
    where
        C: Command<'a, Context = Ctx> + Debug,
    {
        if !self.commands.is_empty() {
            let command = match self.top.as_mut() {
                None => {
                    self.top = Some(0);
                    self.commands.first_mut()
                }
                Some(top) => {
                    let last = self.commands.len() - 1;
                    if *top < last {
                        *top += 1;
                        self.commands.get_mut(*top)
                    } else {
                        None
                    }
                }
            };

            if let Some(command) = command {
                if self.debug {
                    println!("Redo command {:?}", command);
                }
                command.execute(&mut context)
            }
        }
    }

    pub fn clear<'a, Ctx>(&mut self, mut context: Ctx)
    where
        C: Command<'a, Context = Ctx> + Debug,
    {
        for mut dropped_command in self.commands.drain(..) {
            if self.debug {
                println!("Finalizing command {:?}", dropped_command);
            }
            dropped_command.finalize(&mut context);
        }
    }
}

pub struct CommandStackViewer {
    pub window: Handle<UiNode>,
    list: Handle<UiNode>,
    sender: Sender<Message>,
    undo: Handle<UiNode>,
    redo: Handle<UiNode>,
    clear: Handle<UiNode>,
}

impl CommandStackViewer {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let list;
        let undo;
        let redo;
        let clear;
        let window = WindowBuilder::new(WidgetBuilder::new())
            .with_title(WindowTitle::Text("Command Stack".to_owned()))
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(
                            StackPanelBuilder::new(
                                WidgetBuilder::new()
                                    .with_child({
                                        undo = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_content(
                                            ImageBuilder::new(
                                                WidgetBuilder::new()
                                                    .with_margin(Thickness::uniform(1.0))
                                                    .with_height(28.0)
                                                    .with_width(28.0),
                                            )
                                            .with_opt_texture(load_image(include_bytes!(
                                                "../resources/undo.png"
                                            )))
                                            .build(ctx),
                                        )
                                        .build(ctx);
                                        undo
                                    })
                                    .with_child({
                                        redo = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_content(
                                            ImageBuilder::new(
                                                WidgetBuilder::new()
                                                    .with_margin(Thickness::uniform(1.0))
                                                    .with_height(28.0)
                                                    .with_width(28.0),
                                            )
                                            .with_opt_texture(load_image(include_bytes!(
                                                "../resources/redo.png"
                                            )))
                                            .build(ctx),
                                        )
                                        .build(ctx);
                                        redo
                                    })
                                    .with_child({
                                        clear = ButtonBuilder::new(WidgetBuilder::new())
                                            .with_content(
                                                ImageBuilder::new(
                                                    WidgetBuilder::new()
                                                        .with_margin(Thickness::uniform(1.0))
                                                        .with_height(28.0)
                                                        .with_width(28.0),
                                                )
                                                .with_opt_texture(load_image(include_bytes!(
                                                    "../resources/clear.png"
                                                )))
                                                .build(ctx),
                                            )
                                            .build(ctx);
                                        clear
                                    }),
                            )
                            .with_orientation(Orientation::Horizontal)
                            .build(ctx),
                        )
                        .with_child(
                            ScrollViewerBuilder::new(
                                WidgetBuilder::new()
                                    .with_margin(Thickness::uniform(1.0))
                                    .on_row(1),
                            )
                            .with_content({
                                list = ListViewBuilder::new(WidgetBuilder::new()).build(ctx);
                                list
                            })
                            .build(ctx),
                        ),
                )
                .add_column(Column::stretch())
                .add_row(Row::strict(30.0))
                .add_row(Row::stretch())
                .build(ctx),
            )
            .build(ctx);

        Self {
            window,
            list,
            sender,
            undo,
            redo,
            clear,
        }
    }

    pub fn handle_ui_message(&self, message: &UiMessage) {
        scope_profile!();

        if let UiMessageData::Button(ButtonMessage::Click) = message.data() {
            if message.destination() == self.undo {
                self.sender.send(Message::UndoSceneCommand).unwrap();
            } else if message.destination() == self.redo {
                self.sender.send(Message::RedoSceneCommand).unwrap();
            } else if message.destination() == self.clear {
                self.sender.send(Message::ClearSceneCommandStack).unwrap();
            }
        }
    }

    pub fn sync_to_model<'a, C, Ctx>(
        &mut self,
        command_stack: &mut CommandStack<C>,
        ctx: &Ctx,
        ui: &mut Ui,
    ) where
        C: Command<'a, Context = Ctx> + Debug,
    {
        scope_profile!();

        let top = command_stack.top;
        let items = command_stack
            .commands
            .iter_mut()
            .enumerate()
            .rev() // First command in list is last on stack.
            .map(|(i, cmd)| {
                let brush = if let Some(top) = top {
                    if (0..=top).contains(&i) {
                        Brush::Solid(Color::opaque(255, 255, 255))
                    } else {
                        Brush::Solid(Color::opaque(100, 100, 100))
                    }
                } else {
                    Brush::Solid(Color::opaque(100, 100, 100))
                };

                TextBuilder::new(WidgetBuilder::new().with_foreground(brush))
                    .with_text(cmd.name(ctx))
                    .build(&mut ui.build_ctx())
            })
            .collect();

        send_sync_message(
            ui,
            ListViewMessage::items(self.list, MessageDirection::ToWidget, items),
        );
    }
}
