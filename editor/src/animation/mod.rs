use crate::{
    animation::{
        command::{
            AnimationCommand, AnimationCommandStack, AnimationEditorContext,
            ReplaceTrackCurveCommand,
        },
        data::{DataModel, SelectedEntity},
        menu::Menu,
        message::Message,
        track::TrackList,
    },
    scene::EditorScene,
};
use fyrox::{
    asset::{Resource, ResourceState},
    core::{futures::executor::block_on, pool::Handle},
    engine::Engine,
    gui::{
        curve::{CurveEditorBuilder, CurveEditorMessage},
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessage},
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, Thickness, UiNode, UserInterface,
    },
    resource::animation::{AnimationResource, AnimationResourceState},
};
use std::sync::mpsc::{self, Receiver, Sender};

mod command;
mod data;
mod menu;
mod message;
mod track;

pub struct AnimationEditor {
    pub window: Handle<UiNode>,
    track_list: TrackList,
    curve_editor: Handle<UiNode>,
    data_model: Option<DataModel>,
    menu: Menu,
    command_stack: AnimationCommandStack,
    message_sender: Sender<Message>,
    message_receiver: Receiver<Message>,
    current_selection: Vec<SelectedEntity>,
}

impl AnimationEditor {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let curve_editor;

        let menu = Menu::new(ctx);
        let track_list = TrackList::new(ctx);

        let payload = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .on_column(0)
                .with_child(track_list.panel)
                .with_child({
                    curve_editor = CurveEditorBuilder::new(
                        WidgetBuilder::new()
                            .with_enabled(false)
                            .on_row(0)
                            .on_column(1)
                            .with_margin(Thickness::uniform(1.0)),
                    )
                    .build(ctx);
                    curve_editor
                }),
        )
        .add_row(Row::stretch())
        .add_column(Column::strict(250.0))
        .add_column(Column::stretch())
        .build(ctx);

        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(menu.menu)
                .with_child(payload),
        )
        .add_row(Row::strict(22.0))
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(600.0).with_height(500.0))
            .with_content(content)
            .open(false)
            .with_title(WindowTitle::text("Animation Editor"))
            .build(ctx);

        let (message_sender, message_receiver) = mpsc::channel();

        Self {
            window,
            track_list,
            curve_editor,
            data_model: None,
            menu,
            command_stack: AnimationCommandStack::new(false),
            message_sender,
            message_receiver,
            current_selection: Default::default(),
        }
    }

    pub fn open(&self, ui: &UserInterface) {
        ui.send_message(WindowMessage::open(
            self.window,
            MessageDirection::ToWidget,
            true,
        ));
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_scene: Option<&EditorScene>,
        engine: &mut Engine,
    ) {
        self.track_list
            .handle_ui_message(message, editor_scene, engine, &self.message_sender);
        self.menu.handle_ui_message(
            message,
            &engine.user_interface,
            &self.message_sender,
            self.data_model.as_ref(),
        );

        if let Some(CurveEditorMessage::Sync(curve)) = message.data() {
            if message.destination() == self.curve_editor
                && message.direction() == MessageDirection::FromWidget
            {
                self.message_sender
                    .send(Message::DoCommand(AnimationCommand::new(
                        ReplaceTrackCurveCommand {
                            curve: curve.clone(),
                        },
                    )))
                    .unwrap();
            }
        }
    }

    pub fn update(&mut self, engine: &mut Engine) {
        let mut need_sync = false;
        while let Ok(message) = self.message_receiver.try_recv() {
            match message {
                Message::DoCommand(command) => {
                    if let Some(data_model) = self.data_model.as_mut() {
                        let resource = data_model.resource.data_ref();
                        self.command_stack.do_command(
                            command.0,
                            AnimationEditorContext {
                                selection: &mut data_model.selection,
                                resource,
                            },
                        );
                        data_model.saved = false;
                        need_sync = true;
                    }
                }
                Message::Redo => {
                    if let Some(data_model) = self.data_model.as_mut() {
                        let resource = data_model.resource.data_ref();
                        self.command_stack.redo(AnimationEditorContext {
                            selection: &mut data_model.selection,
                            resource,
                        });
                        data_model.saved = false;
                        need_sync = true;
                    }
                }
                Message::Undo => {
                    if let Some(data_model) = self.data_model.as_mut() {
                        let resource = data_model.resource.data_ref();
                        self.command_stack.undo(AnimationEditorContext {
                            selection: &mut data_model.selection,
                            resource,
                        });
                        data_model.saved = false;
                        need_sync = true;
                    }
                }
                Message::ClearCommandStack => {
                    if let Some(data_model) = self.data_model.as_mut() {
                        let resource = data_model.resource.data_ref();
                        self.command_stack.clear(AnimationEditorContext {
                            selection: &mut data_model.selection,
                            resource,
                        });
                    }
                }
                Message::Exit => {
                    engine.user_interface.send_message(WindowMessage::close(
                        self.window,
                        MessageDirection::ToWidget,
                    ));
                }
                Message::NewAnimation => {
                    self.data_model = Some(DataModel {
                        resource: AnimationResource(Resource::new(ResourceState::Ok(
                            AnimationResourceState::default(),
                        ))),
                        saved: false,
                        selection: Default::default(),
                    });
                    need_sync = true;
                }
                Message::Save(path) => {
                    if let Some(data_model) = self.data_model.as_mut() {
                        data_model.save(path);
                    }
                }
                Message::Load(path) => {
                    if let Ok(animation) = block_on(engine.resource_manager.request_animation(path))
                    {
                        self.data_model = Some(DataModel {
                            saved: true,
                            selection: Default::default(),
                            resource: animation,
                        });
                        need_sync = true;
                    }
                }
            }
        }

        if need_sync {
            self.sync_to_model(engine)
        }
    }

    fn sync_to_model(&mut self, engine: &mut Engine) {
        engine.user_interface.send_message(WidgetMessage::enabled(
            self.curve_editor,
            MessageDirection::ToWidget,
            self.data_model.is_some(),
        ));

        self.menu
            .sync_to_model(&engine.user_interface, self.data_model.as_ref());
        self.track_list
            .sync_to_model(engine, self.data_model.as_ref());

        if let Some(data_model) = self.data_model.as_ref() {
            if self.current_selection != data_model.selection {
                self.current_selection = data_model.selection.clone();

                // TODO: Add support for multi-selection
                if let Some(SelectedEntity::Curve(selected_curve_id)) =
                    self.current_selection.first()
                {
                    let resource_ref = data_model.resource.data_ref();
                    if let Some(selected_curve) = resource_ref
                        .animation_definition
                        .tracks()
                        .iter()
                        .find_map(|t| {
                            t.frames_container()
                                .curves_ref()
                                .iter()
                                .find(|c| &c.id() == selected_curve_id)
                        })
                    {
                        engine.user_interface.send_message(CurveEditorMessage::sync(
                            self.curve_editor,
                            MessageDirection::ToWidget,
                            selected_curve.clone(),
                        ));
                    }
                }
            }
        }
    }
}
