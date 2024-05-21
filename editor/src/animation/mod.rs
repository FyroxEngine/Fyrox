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
    command::{Command, CommandGroup},
    fyrox::{
        asset::manager::ResourceManager,
        core::{
            algebra::Vector2, log::Log, math::Rect, pool::ErasedHandle, pool::Handle, uuid::Uuid,
            variable::InheritableVariable,
        },
        fxhash::FxHashSet,
        generic_animation::{signal::AnimationSignal, AnimationContainer},
        graph::{BaseSceneGraph, PrefabData, SceneGraph, SceneGraphNode},
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
        resource::model::AnimationSource,
    },
    message::MessageSender,
    scene::{commands::ChangeSelectionCommand, Selection},
    send_sync_message, Message,
};
use fyrox::core::color::Color;
use fyrox::core::math::curve::Curve;
use fyrox::gui::brush::Brush;
use std::any::{Any, TypeId};

pub mod command;
mod ruler;
pub mod selection;
mod thumb;
mod toolbar;
mod track;

pub trait PreviewData {
    fn enter(&mut self);
}

struct PreviewModeData<N: 'static> {
    nodes: Vec<(Handle<N>, N)>,
}

pub struct AnimationEditor {
    pub window: Handle<UiNode>,
    animation_player: ErasedHandle,
    animation: ErasedHandle,
    track_list: TrackList,
    curve_editor: Handle<UiNode>,
    toolbar: Toolbar,
    content: Handle<UiNode>,
    ruler: Handle<UiNode>,
    preview_mode_data: Option<Box<dyn Any>>,
    thumb: Handle<UiNode>,
}

fn fetch_selection<G, N>(
    editor: &mut AnimationEditor,
    graph: &G,
    editor_selection: &Selection,
) -> AnimationSelection<N>
where
    G: SceneGraph<Node = N>,
    N: SceneGraphNode<SceneGraph = G>,
{
    let mut sel = inner_fetch_selection(editor_selection);
    if animation_container_ref(graph, sel.animation_player).is_none() {
        sel.animation_player = Handle::NONE;
    }
    if sel.animation_player.is_none() {
        sel.animation_player = editor.animation_player.into();
        sel.animation = editor.animation.into();
    } else if ErasedHandle::from(sel.animation_player) == editor.animation_player {
        if sel.animation.is_none() {
            sel.animation = editor.animation.into();
        } else {
            editor.animation = sel.animation.into();
        }
    } else {
        editor.animation_player = sel.animation_player.into();
        editor.animation = sel.animation.into();
    }
    if !graph.is_valid_handle(sel.animation_player) {
        sel.animation_player = Handle::NONE;
        sel.animation = Handle::NONE;
    }
    sel
}

fn inner_fetch_selection<N>(editor_selection: &Selection) -> AnimationSelection<N> {
    if let Some(selection) = editor_selection.as_animation() {
        // Some selection in an animation.
        AnimationSelection {
            animation_player: selection.animation_player,
            animation: selection.animation,
            entities: selection.entities.clone(),
        }
    } else if let Some(selection) = editor_selection.as_graph() {
        // Only some AnimationPlayer in Game Scene is selected.
        AnimationSelection {
            animation_player: ErasedHandle::from(
                selection.nodes.first().cloned().unwrap_or_default(),
            )
            .into(),
            animation: Default::default(),
            entities: vec![],
        }
    } else if let Some(selection) = editor_selection.as_ui() {
        // Only some AnimationPlayer in UI Scene is selected.
        AnimationSelection {
            animation_player: ErasedHandle::from(
                selection.widgets.first().cloned().unwrap_or_default(),
            )
            .into(),
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

fn animation_container<G, N>(
    graph: &mut G,
    handle: Handle<N>,
) -> Option<&mut AnimationContainer<Handle<N>>>
where
    G: SceneGraph<Node = N>,
    N: SceneGraphNode<SceneGraph = G>,
{
    graph
        .try_get_mut(handle)
        .and_then(|n| n.component_mut::<InheritableVariable<AnimationContainer<Handle<N>>>>())
        .map(|v| v.get_value_mut_silent())
}

fn animation_container_ref<G, N>(
    graph: &G,
    handle: Handle<N>,
) -> Option<&AnimationContainer<Handle<N>>>
where
    G: SceneGraph<Node = N>,
    N: SceneGraphNode<SceneGraph = G>,
{
    graph
        .try_get(handle)
        .and_then(|n| {
            n.query_component_ref(TypeId::of::<
                InheritableVariable<AnimationContainer<Handle<N>>>,
            >())
        })
        .and_then(|a| a.downcast_ref::<InheritableVariable<AnimationContainer<Handle<N>>>>())
        .map(|v| v.get_value_ref())
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
            animation_player: ErasedHandle::none(),
            animation: ErasedHandle::none(),
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
            true,
        ));
    }

    pub fn handle_ui_message<P, G, N>(
        &mut self,
        message: &UiMessage,
        editor_selection: &Selection,
        graph: &mut G,
        root: Handle<N>,
        ui: &mut UserInterface,
        resource_manager: &ResourceManager,
        sender: &MessageSender,
        node_overrides: &mut FxHashSet<Handle<N>>,
    ) where
        P: PrefabData<Graph = G> + AnimationSource<Node = N, SceneGraph = G, Prefab = P>,
        G: SceneGraph<Node = N, Prefab = P>,
        N: SceneGraphNode<SceneGraph = G, ResourceData = P>,
    {
        let selection = fetch_selection(self, graph, editor_selection);

        if let Some(container) = animation_container_ref(graph, selection.animation_player) {
            let toolbar_action = self.toolbar.handle_ui_message(
                message,
                sender,
                graph,
                ui,
                selection.animation_player,
                container,
                root,
                &selection,
            );

            let animations = animation_container(graph, selection.animation_player).unwrap();

            if let Some(msg) = message.data::<CurveEditorMessage>() {
                if message.destination() == self.curve_editor
                    && message.direction() == MessageDirection::FromWidget
                {
                    match msg {
                        CurveEditorMessage::Sync(curves) => {
                            let group = CommandGroup::from(
                                curves
                                    .iter()
                                    .cloned()
                                    .map(|curve| {
                                        Command::new(ReplaceTrackCurveCommand {
                                            animation_player: selection.animation_player,
                                            animation: selection.animation,
                                            curve,
                                        })
                                    })
                                    .collect::<Vec<_>>(),
                            );

                            sender.do_command(group);
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
                    && animations.try_get(selection.animation).is_some()
                {
                    match msg {
                        RulerMessage::Value(value) => {
                            if let Some(animation) = animations.try_get_mut(selection.animation) {
                                animation.set_time_position(*value);
                            }
                        }
                        RulerMessage::AddSignal(time) => {
                            sender.do_command(AddAnimationSignal {
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
                            if let Some(animation) = animations.try_get(selection.animation) {
                                sender.do_command(RemoveAnimationSignal {
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
                            sender.do_command(MoveAnimationSignal {
                                animation_player_handle: selection.animation_player,
                                animation_handle: selection.animation,
                                signal: *id,
                                time: *new_position,
                            });
                        }
                        RulerMessage::SelectSignal(id) => {
                            sender.do_command(ChangeSelectionCommand::new(Selection::new(
                                AnimationSelection {
                                    animation_player: selection.animation_player,
                                    animation: selection.animation,
                                    entities: vec![SelectedEntity::Signal(*id)],
                                },
                            )));
                        }
                        _ => (),
                    }
                }
            }

            match toolbar_action {
                ToolbarAction::None => {}
                ToolbarAction::EnterPreviewMode => {
                    assert!(node_overrides.insert(selection.animation_player));

                    let animation_player_node =
                        graph.try_get_mut(selection.animation_player).unwrap();

                    // HACK. This is unreliable to just use `bool` here. It should be wrapped into
                    // newtype or something.
                    if let Some(auto_apply) = animation_player_node.component_mut::<bool>() {
                        *auto_apply = true;
                    } else {
                        Log::warn("No `auto_apply` component in animation player!")
                    }

                    // Save state of animation player first.
                    let initial_animation_player_handle = selection.animation_player;
                    let initial_animation_player = animation_player_node.clone();

                    // Now we can freely modify the state of the animation player in the scene - all
                    // changes will be reverted at the exit of the preview mode.
                    let animations =
                        animation_container(graph, selection.animation_player).unwrap();

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
                            graph,
                            ui,
                            node_overrides,
                        );
                    }
                }
                ToolbarAction::LeavePreviewMode => {
                    if self.preview_mode_data.is_some() {
                        self.leave_preview_mode(graph, ui, node_overrides);
                    }
                }
                ToolbarAction::SelectAnimation(animation) => {
                    let animation_ref = &animations[animation.into()];

                    let size = ui.node(self.curve_editor).actual_local_size();
                    let length = animation_ref.length().max(1.0);
                    let zoom = size.x / length;

                    ui.send_message(CurveEditorMessage::zoom(
                        self.curve_editor,
                        MessageDirection::ToWidget,
                        Vector2::new(zoom, zoom),
                    ));

                    ui.send_message(CurveEditorMessage::view_position(
                        self.curve_editor,
                        MessageDirection::ToWidget,
                        Vector2::new(0.5 * animation_ref.length(), 0.0),
                    ));
                }
                ToolbarAction::PlayPause => {
                    if self.preview_mode_data.is_some() {
                        if let Some(animation) = animations.try_get_mut(selection.animation) {
                            animation.set_enabled(!animation.is_enabled());
                        }
                    }
                }
                ToolbarAction::Stop => {
                    if self.preview_mode_data.is_some() {
                        if let Some(animation) = animations.try_get_mut(selection.animation) {
                            animation.rewind();
                            animation.set_enabled(false);
                        }
                    }
                }
            }

            self.track_list
                .handle_ui_message(message, &selection, root, sender, ui, graph);
        }

        self.toolbar.post_handle_ui_message(
            message,
            sender,
            ui,
            selection.animation_player,
            graph,
            root,
            editor_selection,
            resource_manager,
        );
    }

    fn enter_preview_mode<G, N>(
        &mut self,
        initial_animation_player_handle: Handle<N>,
        initial_animation_player: N,
        animation_targets: FxHashSet<Handle<N>>,
        graph: &G,
        ui: &UserInterface,
        node_overrides: &mut FxHashSet<Handle<N>>,
    ) where
        G: SceneGraph<Node = N>,
        N: SceneGraphNode,
    {
        assert!(self.preview_mode_data.is_none());

        self.toolbar.on_preview_mode_changed(ui, true);

        for &target in &animation_targets {
            assert!(node_overrides.insert(target));
        }

        let mut data = PreviewModeData {
            nodes: animation_targets
                .into_iter()
                .map(|t| (t, graph.node(t).clone()))
                .collect(),
        };

        data.nodes
            .push((initial_animation_player_handle, initial_animation_player));

        // Save state of affected nodes.
        self.preview_mode_data = Some(Box::new(data));
    }

    fn leave_preview_mode<G, N>(
        &mut self,
        graph: &mut G,
        ui: &UserInterface,
        node_overrides: &mut FxHashSet<Handle<N>>,
    ) where
        G: SceneGraph<Node = N>,
        N: SceneGraphNode<SceneGraph = G>,
    {
        self.toolbar.on_preview_mode_changed(ui, false);

        let preview_data = self
            .preview_mode_data
            .take()
            .expect("Unable to leave animation preview mode!");

        // Revert state of nodes.
        for (handle, node) in preview_data.downcast::<PreviewModeData<N>>().unwrap().nodes {
            node_overrides.remove(&handle);
            *graph.node_mut(handle) = node;
        }
    }

    pub fn try_leave_preview_mode<G, N>(
        &mut self,
        graph: &mut G,
        ui: &UserInterface,
        node_overrides: &mut FxHashSet<Handle<N>>,
    ) where
        G: SceneGraph<Node = N>,
        N: SceneGraphNode<SceneGraph = G>,
    {
        if self.preview_mode_data.is_some() {
            self.leave_preview_mode(graph, ui, node_overrides);
        }
    }

    pub fn is_in_preview_mode(&self) -> bool {
        self.preview_mode_data.is_some()
    }

    pub fn handle_message<G, N>(
        &mut self,
        message: &Message,
        graph: &mut G,
        ui: &UserInterface,
        node_overrides: &mut FxHashSet<Handle<N>>,
    ) where
        G: SceneGraph<Node = N>,
        N: SceneGraphNode<SceneGraph = G>,
    {
        // Leave preview mode before execution of any scene command.
        if let Message::DoCommand(_)
        | Message::UndoCurrentSceneCommand
        | Message::RedoCurrentSceneCommand = message
        {
            self.try_leave_preview_mode(graph, ui, node_overrides);
        }
    }

    pub fn clear(&mut self, ui: &UserInterface) {
        self.toolbar.clear(ui);
        self.track_list.clear(ui);
    }

    pub fn update<G, N>(&mut self, editor_selection: &Selection, ui: &UserInterface, graph: &G)
    where
        G: SceneGraph<Node = N>,
        N: SceneGraphNode<SceneGraph = G>,
    {
        if !self.is_in_preview_mode() {
            return;
        }

        let selection = fetch_selection(self, graph, editor_selection);

        if let Some(container) = animation_container_ref(graph, selection.animation_player) {
            if let Some(animation) = container.try_get(selection.animation) {
                ui.send_message(ThumbMessage::position(
                    self.thumb,
                    MessageDirection::ToWidget,
                    animation.time_position(),
                ));
            }
        }
    }

    pub fn sync_to_model<G, N>(
        &mut self,
        editor_selection: &Selection,
        ui: &mut UserInterface,
        graph: &G,
    ) where
        G: SceneGraph<Node = N>,
        N: SceneGraphNode<SceneGraph = G>,
    {
        let selection = fetch_selection(self, graph, editor_selection);

        let mut is_animation_player_selected = false;
        let mut is_animation_selected = false;
        let mut is_curve_selected = false;

        if let Some(animations) = animation_container_ref(graph, selection.animation_player) {
            self.toolbar.sync_to_model(
                animations,
                &selection,
                graph,
                ui,
                self.preview_mode_data.is_some(),
            );

            if let Some(animation) = animations.try_get(selection.animation) {
                self.track_list
                    .sync_to_model(animation, graph, &selection, ui);

                send_sync_message(
                    ui,
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
                    ui,
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

                let mut selected_curves = Vec::<(usize, Curve)>::new();
                for entity in selection.entities.iter() {
                    match entity {
                        SelectedEntity::Track(track_id) => {
                            // If a track is selected, show all its curves at once. This way it will
                            // be easier to edit complex values, such as Vector2/3/4.
                            if let Some(track) = animation
                                .tracks()
                                .iter()
                                .find(|track| &track.id() == track_id)
                            {
                                for (index, track_curve) in
                                    track.data_container().curves_ref().iter().enumerate()
                                {
                                    if !selected_curves
                                        .iter()
                                        .any(|(_, curve)| curve.id == track_curve.id)
                                    {
                                        selected_curves.push((index, track_curve.clone()));
                                    }
                                }
                            }
                        }
                        SelectedEntity::Curve(curve_id) => {
                            if let Some((index, selected_curve)) =
                                animation.tracks().iter().find_map(|t| {
                                    t.data_container().curves_ref().iter().enumerate().find_map(
                                        |(i, c)| {
                                            if &c.id() == curve_id {
                                                Some((i, c))
                                            } else {
                                                None
                                            }
                                        },
                                    )
                                })
                            {
                                if !selected_curves
                                    .iter()
                                    .any(|(_, curve)| curve.id == selected_curve.id)
                                {
                                    selected_curves.push((index, selected_curve.clone()));
                                }
                            }
                        }
                        _ => (),
                    }
                }
                let mut background_curves = Vec::<Curve>::new();
                for track in animation.tracks() {
                    for curve in track.data_container().curves_ref() {
                        if !selected_curves.iter().any(|(_, c)| c.id == curve.id) {
                            background_curves.push(curve.clone());
                        }
                    }
                }

                send_sync_message(
                    ui,
                    CurveEditorMessage::sync_background(
                        self.curve_editor,
                        MessageDirection::ToWidget,
                        background_curves,
                    ),
                );

                if !selected_curves.is_empty() {
                    let color_map = selected_curves
                        .iter()
                        .map(|(index, curve)| (curve.id, Brush::Solid(Color::COLORS[3 + *index])))
                        .collect::<Vec<_>>();

                    send_sync_message(
                        ui,
                        CurveEditorMessage::sync(
                            self.curve_editor,
                            MessageDirection::ToWidget,
                            selected_curves
                                .into_iter()
                                .map(|(_, curve)| curve)
                                .collect(),
                        ),
                    );

                    send_sync_message(
                        ui,
                        CurveEditorMessage::colorize(
                            self.curve_editor,
                            MessageDirection::ToWidget,
                            color_map,
                        ),
                    );

                    is_curve_selected = true;
                }
                is_animation_selected = true;
            }
            is_animation_player_selected = true;
        }

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
                is_animation_selected,
            ),
        );
    }
}
