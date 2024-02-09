use crate::message::MessageSender;
use crate::{
    animation::{
        command::{
            AddAnimationSignal, MoveAnimationSignal, RemoveAnimationSignal,
            ReplaceTrackCurveCommand,
        },
        ruler::{RulerBuilder, RulerMessage, SignalView},
        selection::{AnimationSelection, SelectedEntity},
        thumb::{ThumbBuilder, ThumbMessage},
        toolbar::{Toolbar, ToolbarAction},
        track::TrackList,
    },
    scene::{commands::ChangeSelectionCommand, GameScene, Selection},
    send_sync_message, Message,
};
use fyrox::graph::SceneGraph;
use fyrox::{
    core::{algebra::Vector2, math::Rect, pool::Handle, uuid::Uuid},
    engine::Engine,
    fxhash::FxHashSet,
    gui::{
        border::BorderBuilder,
        check_box::CheckBoxMessage,
        curve::{CurveEditorBuilder, CurveEditorMessage, HighlightZone},
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessage},
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, UiNode, UserInterface, BRUSH_DARK, BRUSH_PRIMARY,
    },
    scene::{animation::prelude::*, node::Node, Scene},
};

pub mod command;
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
    if let Some(selection) = editor_selection.as_animation() {
        // Some selection in an animation.
        AnimationSelection {
            animation_player: selection.animation_player,
            animation: selection.animation,
            entities: selection.entities.clone(),
        }
    } else if let Some(selection) = editor_selection.as_graph() {
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
                                                WidgetBuilder::new()
                                                    .with_background(BRUSH_DARK)
                                                    .on_row(1),
                                            )
                                            .with_show_x_values(false)
                                            .build(ctx);
                                            curve_editor
                                        }),
                                )
                                .add_row(Row::strict(22.0))
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

        let window = WindowBuilder::new(
            WidgetBuilder::new()
                .with_name("AnimationEditor")
                .with_width(600.0)
                .with_height(500.0),
        )
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
        editor_selection: &Selection,
        game_scene: &mut GameScene,
        engine: &mut Engine,
        sender: &MessageSender,
    ) {
        let selection = fetch_selection(editor_selection);

        let scene = &mut engine.scenes[game_scene.scene];

        if let Some(animation_player) = scene
            .graph
            .try_get_of_type::<AnimationPlayer>(selection.animation_player)
        {
            let toolbar_action = self.toolbar.handle_ui_message(
                message,
                sender,
                scene,
                &mut engine.user_interface,
                selection.animation_player,
                animation_player,
                editor_selection,
                game_scene,
                &selection,
            );

            let animation_player = scene
                .graph
                .try_get_mut_of_type::<AnimationPlayer>(selection.animation_player)
                .unwrap();

            if let Some(msg) = message.data::<CurveEditorMessage>() {
                if message.destination() == self.curve_editor
                    && message.direction() == MessageDirection::FromWidget
                {
                    let ui = &engine.user_interface;
                    match msg {
                        CurveEditorMessage::Sync(curve) => {
                            sender.do_scene_command(ReplaceTrackCurveCommand {
                                animation_player: selection.animation_player,
                                animation: selection.animation,
                                curve: curve.clone(),
                            });
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
            } else if let Some(msg) = message.data::<RulerMessage>() {
                if message.destination() == self.ruler
                    && message.direction() == MessageDirection::FromWidget
                    && animation_player
                        .animations()
                        .try_get(selection.animation)
                        .is_some()
                {
                    match msg {
                        RulerMessage::Value(value) => {
                            if let Some(animation) = animation_player
                                .animations_mut()
                                .try_get_mut(selection.animation)
                            {
                                animation.set_time_position(*value);
                            }
                        }
                        RulerMessage::AddSignal(time) => {
                            sender.do_scene_command(AddAnimationSignal {
                                animation_player_handle: selection.animation_player,
                                animation_handle: selection.animation,
                                signal: Some(AnimationSignal {
                                    id: Uuid::new_v4(),
                                    name: "Unnamed".to_string(),
                                    time: *time,
                                    enabled: true,
                                }),
                            });
                        }
                        RulerMessage::RemoveSignal(id) => {
                            if let Some(animation) =
                                animation_player.animations().try_get(selection.animation)
                            {
                                sender.do_scene_command(RemoveAnimationSignal {
                                    animation_player_handle: selection.animation_player,
                                    animation_handle: selection.animation,
                                    signal_index: animation
                                        .signals()
                                        .iter()
                                        .position(|s| s.id == *id)
                                        .unwrap(),
                                    signal: None,
                                })
                            }
                        }
                        RulerMessage::MoveSignal { id, new_position } => {
                            sender.do_scene_command(MoveAnimationSignal {
                                animation_player_handle: selection.animation_player,
                                animation_handle: selection.animation,
                                signal: *id,
                                time: *new_position,
                            });
                        }
                        RulerMessage::SelectSignal(id) => {
                            sender.do_scene_command(ChangeSelectionCommand::new(
                                Selection::new(AnimationSelection {
                                    animation_player: selection.animation_player,
                                    animation: selection.animation,
                                    entities: vec![SelectedEntity::Signal(*id)],
                                }),
                                editor_selection.clone(),
                            ));
                        }
                        _ => (),
                    }
                }
            }

            match toolbar_action {
                ToolbarAction::None => {}
                ToolbarAction::EnterPreviewMode => {
                    let node_overrides = game_scene.graph_switches.node_overrides.as_mut().unwrap();
                    assert!(node_overrides.insert(selection.animation_player));

                    let animation_player_node =
                        scene.graph.try_get_mut(selection.animation_player).unwrap();

                    // Save state of animation player first.
                    let initial_animation_player_handle = selection.animation_player;
                    let initial_animation_player = animation_player_node.clone_box();

                    // Now we can freely modify the state of the animation player in the scene - all
                    // changes will be reverted at the exit of the preview mode.
                    let animation_player = animation_player_node
                        .query_component_mut::<AnimationPlayer>()
                        .unwrap();

                    animation_player.set_auto_apply(true);

                    let animations = animation_player.animations_mut();

                    // Disable every animation, except preview one.
                    for (handle, animation) in animations.pair_iter_mut() {
                        animation.set_enabled(handle == selection.animation);
                    }

                    if let Some(animation) = animations.try_get_mut(selection.animation) {
                        animation.rewind();

                        let animation_targets = animation
                            .tracks()
                            .iter()
                            .map(|t| t.target())
                            .collect::<FxHashSet<_>>();

                        self.enter_preview_mode(
                            initial_animation_player_handle,
                            initial_animation_player,
                            animation_targets,
                            scene,
                            &engine.user_interface,
                            node_overrides,
                        );
                    }
                }
                ToolbarAction::LeavePreviewMode => {
                    if self.preview_mode_data.is_some() {
                        self.leave_preview_mode(
                            scene,
                            &engine.user_interface,
                            game_scene.graph_switches.node_overrides.as_mut().unwrap(),
                        );
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
                            Vector2::new(0.5 * (size.x - animation_ref.length()), -0.5 * size.y),
                        ));
                }
                ToolbarAction::PlayPause => {
                    if self.preview_mode_data.is_some() {
                        if let Some(animation) = animation_player
                            .animations_mut()
                            .try_get_mut(selection.animation)
                        {
                            animation.set_enabled(!animation.is_enabled());
                        }
                    }
                }
                ToolbarAction::Stop => {
                    if self.preview_mode_data.is_some() {
                        if let Some(animation) = animation_player
                            .animations_mut()
                            .try_get_mut(selection.animation)
                        {
                            animation.rewind();
                            animation.set_enabled(false);
                        }
                    }
                }
            }

            self.track_list.handle_ui_message(
                message,
                editor_selection,
                game_scene,
                sender,
                selection.animation_player,
                selection.animation,
                &mut engine.user_interface,
                scene,
            );
        }

        self.toolbar.post_handle_ui_message(
            message,
            sender,
            &engine.user_interface,
            selection.animation_player,
            scene,
            editor_selection,
            game_scene,
            &engine.resource_manager,
        );
    }

    fn enter_preview_mode(
        &mut self,
        initial_animation_player_handle: Handle<Node>,
        initial_animation_player: Node,
        animation_targets: FxHashSet<Handle<Node>>,
        scene: &Scene,
        ui: &UserInterface,
        node_overrides: &mut FxHashSet<Handle<Node>>,
    ) {
        assert!(self.preview_mode_data.is_none());

        self.toolbar.on_preview_mode_changed(ui, true);

        for &target in &animation_targets {
            assert!(node_overrides.insert(target));
        }

        let mut data = PreviewModeData {
            nodes: animation_targets
                .into_iter()
                .map(|t| (t, scene.graph[t].clone_box()))
                .collect(),
        };

        data.nodes
            .push((initial_animation_player_handle, initial_animation_player));

        // Save state of affected nodes.
        self.preview_mode_data = Some(data);
    }

    fn leave_preview_mode(
        &mut self,
        scene: &mut Scene,
        ui: &UserInterface,
        node_overrides: &mut FxHashSet<Handle<Node>>,
    ) {
        self.toolbar.on_preview_mode_changed(ui, false);

        let preview_data = self
            .preview_mode_data
            .take()
            .expect("Unable to leave animation preview mode!");

        // Revert state of nodes.
        for (handle, node) in preview_data.nodes {
            assert!(node_overrides.remove(&handle));
            scene.graph[handle] = node;
        }
    }

    pub fn try_leave_preview_mode(&mut self, game_scene: &mut GameScene, engine: &mut Engine) {
        if self.preview_mode_data.is_some() {
            let scene = &mut engine.scenes[game_scene.scene];

            self.leave_preview_mode(
                scene,
                &engine.user_interface,
                game_scene.graph_switches.node_overrides.as_mut().unwrap(),
            );
        }
    }

    pub fn is_in_preview_mode(&self) -> bool {
        self.preview_mode_data.is_some()
    }

    pub fn handle_message(
        &mut self,
        message: &Message,
        game_scene: &mut GameScene,
        engine: &mut Engine,
    ) {
        // Leave preview mode before execution of any scene command.
        if let Message::DoGameSceneCommand(_)
        | Message::UndoCurrentSceneCommand
        | Message::RedoCurrentSceneCommand = message
        {
            self.try_leave_preview_mode(game_scene, engine);
        }
    }

    pub fn clear(&mut self, ui: &UserInterface) {
        self.toolbar.clear(ui);
        self.track_list.clear(ui);
    }

    pub fn update(
        &mut self,
        editor_selection: &Selection,
        game_scene: &GameScene,
        engine: &Engine,
    ) {
        let selection = fetch_selection(editor_selection);

        let scene = &engine.scenes[game_scene.scene];

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

    pub fn sync_to_model(
        &mut self,
        editor_selection: &Selection,
        game_scene: &GameScene,
        engine: &mut Engine,
    ) {
        let selection = fetch_selection(editor_selection);

        let scene = &engine.scenes[game_scene.scene];

        let mut is_animation_player_selected = false;
        let mut is_animation_selected = false;
        let mut is_curve_selected = false;

        if let Some(animation_player) = scene
            .graph
            .try_get(selection.animation_player)
            .and_then(|n| n.query_component_ref::<AnimationPlayer>())
        {
            self.toolbar.sync_to_model(
                animation_player,
                &selection,
                scene,
                &mut engine.user_interface,
                self.preview_mode_data.is_some(),
            );

            if let Some(animation) = animation_player.animations().try_get(selection.animation) {
                self.track_list.sync_to_model(
                    animation,
                    selection.animation,
                    &scene.graph,
                    editor_selection,
                    &mut engine.user_interface,
                );

                send_sync_message(
                    &engine.user_interface,
                    CurveEditorMessage::hightlight_zones(
                        self.curve_editor,
                        MessageDirection::ToWidget,
                        vec![HighlightZone {
                            rect: Rect::new(
                                animation.time_slice().start,
                                -100000.0,
                                animation.time_slice().end - animation.time_slice().start,
                                200000.0,
                            ),
                            brush: BRUSH_PRIMARY,
                        }],
                    ),
                );

                send_sync_message(
                    &engine.user_interface,
                    RulerMessage::sync_signals(
                        self.ruler,
                        MessageDirection::ToWidget,
                        animation
                            .signals()
                            .iter()
                            .map(|s| SignalView {
                                id: s.id,
                                time: s.time,
                                selected: false,
                            })
                            .collect(),
                    ),
                );

                // TODO: Support multi-selection.
                if let Some(SelectedEntity::Curve(selected_curve_id)) = selection.entities.first() {
                    if let Some(selected_curve) = animation.tracks().iter().find_map(|t| {
                        t.data_container()
                            .curves_ref()
                            .iter()
                            .find(|c| &c.id() == selected_curve_id)
                    }) {
                        send_sync_message(
                            &engine.user_interface,
                            CurveEditorMessage::sync(
                                self.curve_editor,
                                MessageDirection::ToWidget,
                                selected_curve.clone(),
                            ),
                        );
                    }
                    is_curve_selected = true;
                }
                is_animation_selected = true;
            }
            is_animation_player_selected = true;
        }

        let ui = &engine.user_interface;

        if !is_animation_selected || !is_animation_player_selected {
            self.track_list.clear(ui);

            send_sync_message(
                ui,
                CurveEditorMessage::zoom(
                    self.curve_editor,
                    MessageDirection::ToWidget,
                    Vector2::new(1.0, 1.0),
                ),
            );
            send_sync_message(
                ui,
                CurveEditorMessage::view_position(
                    self.curve_editor,
                    MessageDirection::ToWidget,
                    Vector2::default(),
                ),
            );
        }

        if !is_animation_selected || !is_animation_player_selected || !is_curve_selected {
            send_sync_message(
                ui,
                CurveEditorMessage::sync(
                    self.curve_editor,
                    MessageDirection::ToWidget,
                    Default::default(),
                ),
            );
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
        send_sync_message(
            ui,
            WidgetMessage::enabled(
                self.curve_editor,
                MessageDirection::ToWidget,
                is_curve_selected,
            ),
        );
    }
}
