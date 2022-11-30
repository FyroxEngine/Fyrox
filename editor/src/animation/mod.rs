use crate::{
    animation::{
        command::ReplaceTrackCurveCommand,
        ruler::{RulerBuilder, RulerMessage},
        selection::{AnimationSelection, SelectedEntity},
        thumb::{ThumbBuilder, ThumbMessage},
        toolbar::{Toolbar, ToolbarAction},
        track::TrackList,
    },
    scene::{EditorScene, Selection},
    send_sync_message, Message,
};
use fyrox::{
    core::{algebra::Vector2, pool::Handle},
    engine::Engine,
    gui::{
        border::BorderBuilder,
        check_box::CheckBoxMessage,
        curve::{CurveEditorBuilder, CurveEditorMessage},
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessage},
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, UiNode, UserInterface,
    },
    scene::{animation::AnimationPlayer, node::Node, Scene},
};
use std::sync::mpsc::Sender;

mod command;
mod ruler;
pub mod selection;
mod thumb;
mod toolbar;
mod track;

struct PreviewModeData {
    nodes: Vec<(Handle<Node>, Node)>,
}

pub struct AnimationEditor {
    pub window: Handle<UiNode>,
    track_list: TrackList,
    curve_editor: Handle<UiNode>,
    toolbar: Toolbar,
    content: Handle<UiNode>,
    ruler: Handle<UiNode>,
    preview_mode_data: Option<PreviewModeData>,
    thumb: Handle<UiNode>,
}

fn fetch_selection(editor_selection: &Selection) -> AnimationSelection {
    if let Selection::Animation(ref selection) = editor_selection {
        // Some selection in an animation.
        AnimationSelection {
            animation_player: selection.animation_player,
            animation: selection.animation,
            entities: selection.entities.clone(),
        }
    } else if let Selection::Graph(ref selection) = editor_selection {
        // Only some AnimationPlayer is selected.
        AnimationSelection {
            animation_player: selection.nodes.first().cloned().unwrap_or_default(),
            animation: Default::default(),
            entities: vec![],
        }
    } else {
        // Stub in other cases.
        AnimationSelection {
            animation_player: Default::default(),
            animation: Default::default(),
            entities: vec![],
        }
    }
}

impl AnimationEditor {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let curve_editor;
        let ruler;
        let thumb;

        let track_list = TrackList::new(ctx);
        let toolbar = Toolbar::new(ctx);

        let payload = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .on_column(0)
                .with_child(track_list.panel)
                .with_child(
                    BorderBuilder::new(
                        WidgetBuilder::new()
                            .on_row(0)
                            .on_column(1)
                            .with_child(
                                GridBuilder::new(
                                    WidgetBuilder::new()
                                        .with_child({
                                            ruler =
                                                RulerBuilder::new(WidgetBuilder::new().on_row(0))
                                                    .with_value(0.0)
                                                    .build(ctx);
                                            ruler
                                        })
                                        .with_child({
                                            curve_editor = CurveEditorBuilder::new(
                                                WidgetBuilder::new().on_row(1),
                                            )
                                            .with_show_x_values(false)
                                            .build(ctx);
                                            curve_editor
                                        }),
                                )
                                .add_row(Row::strict(25.0))
                                .add_row(Row::stretch())
                                .add_column(Column::stretch())
                                .build(ctx),
                            )
                            .with_child({
                                thumb = ThumbBuilder::new(WidgetBuilder::new()).build(ctx);
                                thumb
                            }),
                    )
                    .build(ctx),
                ),
        )
        .add_row(Row::stretch())
        .add_column(Column::strict(250.0))
        .add_column(Column::stretch())
        .build(ctx);

        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(toolbar.panel)
                .with_child(payload),
        )
        .add_row(Row::strict(26.0))
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(600.0).with_height(500.0))
            .with_content(content)
            .open(false)
            .with_title(WindowTitle::text("Animation Editor"))
            .build(ctx);

        Self {
            window,
            track_list,
            curve_editor,
            toolbar,
            content,
            ruler,
            preview_mode_data: None,
            thumb,
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
        sender: &Sender<Message>,
    ) {
        if let Some(editor_scene) = editor_scene {
            let selection = fetch_selection(&editor_scene.selection);

            let scene = &mut engine.scenes[editor_scene.scene];

            if let Some(animation_player) = scene
                .graph
                .try_get_mut(selection.animation_player)
                .and_then(|n| n.query_component_mut::<AnimationPlayer>())
            {
                let toolbar_action = self.toolbar.handle_ui_message(
                    message,
                    sender,
                    &engine.user_interface,
                    selection.animation_player,
                    animation_player,
                    editor_scene,
                    &selection,
                );

                if let Some(msg) = message.data::<CurveEditorMessage>() {
                    if message.destination() == self.curve_editor
                        && message.direction() == MessageDirection::FromWidget
                    {
                        let ui = &engine.user_interface;
                        match msg {
                            CurveEditorMessage::Sync(curve) => {
                                sender
                                    .send(Message::do_scene_command(ReplaceTrackCurveCommand {
                                        animation_player: selection.animation_player,
                                        animation: selection.animation,
                                        curve: curve.clone(),
                                    }))
                                    .unwrap();
                            }
                            CurveEditorMessage::ViewPosition(position) => {
                                ui.send_message(RulerMessage::view_position(
                                    self.ruler,
                                    MessageDirection::ToWidget,
                                    position.x,
                                ));
                                ui.send_message(ThumbMessage::view_position(
                                    self.thumb,
                                    MessageDirection::ToWidget,
                                    position.x,
                                ));
                            }
                            CurveEditorMessage::Zoom(zoom) => {
                                ui.send_message(RulerMessage::zoom(
                                    self.ruler,
                                    MessageDirection::ToWidget,
                                    zoom.x,
                                ));
                                ui.send_message(ThumbMessage::zoom(
                                    self.thumb,
                                    MessageDirection::ToWidget,
                                    zoom.x,
                                ))
                            }
                            _ => (),
                        }
                    }
                } else if let Some(RulerMessage::Value(value)) = message.data() {
                    if message.destination() == self.ruler
                        && message.direction() == MessageDirection::FromWidget
                    {
                        if let Some(animation) = animation_player
                            .animations_mut()
                            .try_get_mut(selection.animation)
                        {
                            animation.set_time_position(*value);
                        }
                    }
                }

                match toolbar_action {
                    ToolbarAction::None => {}
                    ToolbarAction::EnterPreviewMode => {
                        animation_player.set_auto_apply(true);
                        if let Some(animation) = animation_player
                            .animations_mut()
                            .try_get_mut(selection.animation)
                        {
                            animation.rewind();

                            let animation_targets =
                                animation.tracks().iter().map(|t| t.target()).collect();

                            self.enter_preview_mode(
                                animation_targets,
                                scene,
                                &engine.user_interface,
                            );
                        }
                    }
                    ToolbarAction::LeavePreviewMode => {
                        animation_player.set_auto_apply(false);
                        if let Some(animation) = animation_player
                            .animations_mut()
                            .try_get_mut(selection.animation)
                        {
                            animation.set_enabled(false);
                            self.leave_preview_mode(scene, &engine.user_interface);
                        }
                    }
                    ToolbarAction::SelectAnimation(animation) => {
                        let animation_ref = &animation_player.animations()[animation];

                        let size = engine
                            .user_interface
                            .node(self.curve_editor)
                            .actual_local_size();
                        let zoom = size.x / animation_ref.length().max(f32::EPSILON);

                        engine.user_interface.send_message(CurveEditorMessage::zoom(
                            self.curve_editor,
                            MessageDirection::ToWidget,
                            Vector2::new(zoom, zoom),
                        ));

                        engine
                            .user_interface
                            .send_message(CurveEditorMessage::view_position(
                                self.curve_editor,
                                MessageDirection::ToWidget,
                                Vector2::new(
                                    0.5 * (size.x - animation_ref.length()),
                                    -0.5 * size.y,
                                ),
                            ));
                    }
                    ToolbarAction::PlayPause => {
                        if let Some(animation) = animation_player
                            .animations_mut()
                            .try_get_mut(selection.animation)
                        {
                            animation.set_enabled(!animation.is_enabled());
                        }
                    }
                    ToolbarAction::Stop => {
                        if let Some(animation) = animation_player
                            .animations_mut()
                            .try_get_mut(selection.animation)
                        {
                            animation.rewind();
                            animation.set_enabled(false);
                        }
                    }
                }

                self.track_list.handle_ui_message(
                    message,
                    editor_scene,
                    sender,
                    selection.animation_player,
                    selection.animation,
                    &mut engine.user_interface,
                    scene,
                );
            }
        }
    }

    fn enter_preview_mode(
        &mut self,
        animation_targets: Vec<Handle<Node>>,
        scene: &Scene,
        ui: &UserInterface,
    ) {
        assert!(self.preview_mode_data.is_none());

        self.toolbar.on_preview_mode_changed(ui, true);

        // Save state of affected nodes.
        self.preview_mode_data = Some(PreviewModeData {
            nodes: animation_targets
                .into_iter()
                .map(|t| (t, scene.graph[t].clone_box()))
                .collect(),
        });
    }

    fn leave_preview_mode(&mut self, scene: &mut Scene, ui: &UserInterface) {
        self.toolbar.on_preview_mode_changed(ui, false);

        let preview_data = self
            .preview_mode_data
            .take()
            .expect("Unable to leave animation preview mode!");

        // Revert state of nodes.
        for (handle, node) in preview_data.nodes {
            scene.graph[handle] = node;
        }
    }

    pub fn handle_message(
        &mut self,
        message: &Message,
        editor_scene: &EditorScene,
        engine: &mut Engine,
    ) {
        // Leave preview mode before execution of any scene command.
        if let Message::DoSceneCommand(_) | Message::UndoSceneCommand | Message::RedoSceneCommand =
            message
        {
            let selection = fetch_selection(&editor_scene.selection);

            let scene = &mut engine.scenes[editor_scene.scene];

            if let Some(animation_player) = scene
                .graph
                .try_get_mut(selection.animation_player)
                .and_then(|n| n.query_component_mut::<AnimationPlayer>())
            {
                if let Some(animation) = animation_player
                    .animations_mut()
                    .try_get_mut(selection.animation)
                {
                    if self.preview_mode_data.is_some() {
                        animation.set_enabled(false);

                        self.leave_preview_mode(scene, &engine.user_interface);
                    }
                }
            }
        }
    }

    pub fn update(&mut self, editor_scene: &EditorScene, engine: &Engine) {
        let selection = fetch_selection(&editor_scene.selection);

        let scene = &engine.scenes[editor_scene.scene];

        if let Some(animation_player) = scene
            .graph
            .try_get(selection.animation_player)
            .and_then(|n| n.query_component_ref::<AnimationPlayer>())
        {
            if let Some(animation) = animation_player.animations().try_get(selection.animation) {
                engine.user_interface.send_message(ThumbMessage::position(
                    self.thumb,
                    MessageDirection::ToWidget,
                    animation.time_position(),
                ));
            }
        }
    }

    pub fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut Engine) {
        let selection = fetch_selection(&editor_scene.selection);

        let scene = &engine.scenes[editor_scene.scene];

        let mut is_animation_player_selected = false;
        let mut is_animation_selected = false;

        if let Some(animation_player) = scene
            .graph
            .try_get(selection.animation_player)
            .and_then(|n| n.query_component_ref::<AnimationPlayer>())
        {
            self.toolbar.sync_to_model(
                animation_player,
                &selection,
                &mut engine.user_interface,
                self.preview_mode_data.is_some(),
            );

            if let Some(animation) = animation_player.animations().try_get(selection.animation) {
                self.track_list
                    .sync_to_model(animation, &scene.graph, &mut engine.user_interface);

                // TODO: Support multi-selection.
                if let Some(SelectedEntity::Curve(selected_curve_id)) = selection.entities.first() {
                    if let Some(selected_curve) = animation.tracks().iter().find_map(|t| {
                        t.frames_container()
                            .curves_ref()
                            .iter()
                            .find(|c| &c.id() == selected_curve_id)
                    }) {
                        engine.user_interface.send_message(CurveEditorMessage::sync(
                            self.curve_editor,
                            MessageDirection::ToWidget,
                            selected_curve.clone(),
                        ));
                    }
                }
                is_animation_selected = true;
            }
            is_animation_player_selected = true;
        }

        let ui = &engine.user_interface;

        if !is_animation_selected || !is_animation_player_selected {
            self.track_list.clear(ui);

            ui.send_message(CurveEditorMessage::sync(
                self.curve_editor,
                MessageDirection::ToWidget,
                Default::default(),
            ));

            ui.send_message(CurveEditorMessage::zoom(
                self.curve_editor,
                MessageDirection::ToWidget,
                Vector2::new(1.0, 1.0),
            ));
            ui.send_message(CurveEditorMessage::view_position(
                self.curve_editor,
                MessageDirection::ToWidget,
                Vector2::default(),
            ))
        }

        if !is_animation_player_selected {
            self.toolbar.clear(ui);
        }

        send_sync_message(
            ui,
            WidgetMessage::visibility(
                self.content,
                MessageDirection::ToWidget,
                is_animation_player_selected,
            ),
        );
        send_sync_message(
            ui,
            WidgetMessage::enabled(
                self.track_list.panel,
                MessageDirection::ToWidget,
                is_animation_selected,
            ),
        );
        send_sync_message(
            ui,
            CheckBoxMessage::checked(
                self.toolbar.preview,
                MessageDirection::ToWidget,
                Some(self.preview_mode_data.is_some()),
            ),
        );
    }
}
