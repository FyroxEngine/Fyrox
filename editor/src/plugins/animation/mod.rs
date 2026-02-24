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

use crate::{
    command::{Command, CommandGroup},
    fyrox::{
        asset::manager::ResourceManager,
        core::{
            algebra::Vector2, color::Color, log::Log, math::curve::Curve, math::Rect,
            pool::ErasedHandle, pool::Handle, reflect::Reflect, some_or_return, uuid, uuid::Uuid,
            variable::InheritableVariable,
        },
        engine::ApplicationLoopController,
        fxhash::FxHashSet,
        generic_animation::{signal::AnimationSignal, AnimationContainer},
        graph::{PrefabData, SceneGraph, SceneGraphNode},
        gui::{
            border::BorderBuilder,
            brush::Brush,
            curve::{CurveEditor, CurveEditorBuilder, CurveEditorMessage, HighlightZone},
            dock::{DockingManager, DockingManagerMessage},
            grid::{Column, Grid, GridBuilder, Row},
            menu::{MenuItem, MenuItemMessage},
            message::UiMessage,
            style::{resource::StyleResourceExt, Style},
            toggle::ToggleButtonMessage,
            widget::{WidgetBuilder, WidgetMessage},
            window::{Window, WindowAlignment, WindowBuilder, WindowMessage, WindowTitle},
            BuildContext, UserInterface,
        },
        resource::model::AnimationSource,
    },
    menu::create_menu_item,
    message::MessageSender,
    plugin::EditorPlugin,
    plugins::animation::{
        command::{
            AddAnimationSignal, MoveAnimationSignal, RemoveAnimationSignal,
            ReplaceTrackCurveCommand,
        },
        ruler::{Ruler, RulerBuilder, RulerMessage, SignalView},
        selection::{AnimationSelection, SelectedEntity},
        thumb::{Thumb, ThumbBuilder, ThumbMessage},
        toolbar::{Toolbar, ToolbarAction},
        track::TrackList,
    },
    scene::{commands::ChangeSelectionCommand, GameScene, Selection},
    ui_scene::UiScene,
    Editor, Message,
};
use fyrox::gui::UiNode;
use fyrox::scene::node::Node;
use std::any::{Any, TypeId};

pub mod command;
mod ruler;
pub mod selection;
mod thumb;
mod toolbar;
mod track;

pub trait TransformProvider {
    fn position(&self) -> &[f32];
    fn rotation(&self) -> Option<&[f32]>;
    fn scale(&self) -> Option<&[f32]>;
}

impl TransformProvider for Node {
    fn position(&self) -> &[f32] {
        self.local_transform().position().as_slice()
    }

    fn rotation(&self) -> Option<&[f32]> {
        Some(self.local_transform().rotation().coords.as_slice())
    }

    fn scale(&self) -> Option<&[f32]> {
        Some(self.local_transform().scale().as_slice())
    }
}

impl TransformProvider for UiNode {
    fn position(&self) -> &[f32] {
        self.desired_local_position.as_slice()
    }

    fn rotation(&self) -> Option<&[f32]> {
        None
    }

    fn scale(&self) -> Option<&[f32]> {
        None
    }
}

pub trait PreviewData {
    fn enter(&mut self);
}

struct PreviewModeData<N: 'static> {
    nodes: Vec<(Handle<N>, N)>,
}

pub struct AnimationEditor {
    pub window: Handle<Window>,
    animation_player: ErasedHandle,
    animation: ErasedHandle,
    track_list: TrackList,
    curve_editor: Handle<CurveEditor>,
    toolbar: Toolbar,
    content: Handle<Grid>,
    ruler: Handle<Ruler>,
    preview_mode_data: Option<Box<dyn Any>>,
    thumb: Handle<Thumb>,
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

fn inner_fetch_selection<N: Reflect>(editor_selection: &Selection) -> AnimationSelection<N> {
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
        .try_get_node_mut(handle)
        .ok()
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
        .try_get_node(handle)
        .ok()
        .and_then(|n| {
            n.query_component_ref(TypeId::of::<
                InheritableVariable<AnimationContainer<Handle<N>>>,
            >())
        })
        .and_then(|a| a.downcast_ref::<InheritableVariable<AnimationContainer<Handle<N>>>>())
        .map(|v| v.get_value_ref())
}

impl AnimationEditor {
    const WINDOW_NAME: &'static str = "AnimationEditor";

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
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .on_row(0)
                            .on_column(1)
                            .with_child(
                                BorderBuilder::new(
                                    WidgetBuilder::new()
                                        .with_child(
                                            GridBuilder::new(
                                                WidgetBuilder::new()
                                                    .with_child({
                                                        ruler = RulerBuilder::new(
                                                            WidgetBuilder::new().on_row(0),
                                                        )
                                                        .with_value(0.0)
                                                        .build(ctx);
                                                        ruler
                                                    })
                                                    .with_child({
                                                        curve_editor = CurveEditorBuilder::new(
                                                            WidgetBuilder::new()
                                                                .with_background(
                                                                    ctx.style.property(
                                                                        Style::BRUSH_DARK,
                                                                    ),
                                                                )
                                                                .on_row(1),
                                                        )
                                                        .with_show_x_values(false)
                                                        .build(ctx);
                                                        curve_editor
                                                    }),
                                            )
                                            .add_row(Row::strict(22.0))
                                            .add_row(Row::stretch())
                                            .add_row(Row::auto())
                                            .add_column(Column::stretch())
                                            .build(ctx),
                                        )
                                        .with_child({
                                            thumb =
                                                ThumbBuilder::new(WidgetBuilder::new()).build(ctx);
                                            thumb
                                        }),
                                )
                                .build(ctx),
                            )
                            .with_child(toolbar.bottom_panel),
                    )
                    .add_row(Row::stretch())
                    .add_row(Row::auto())
                    .add_column(Column::stretch())
                    .build(ctx),
                ),
        )
        .add_row(Row::stretch())
        .add_column(Column::strict(250.0))
        .add_column(Column::stretch())
        .build(ctx);

        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(toolbar.top_panel)
                .with_child(payload),
        )
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        let window = WindowBuilder::new(
            WidgetBuilder::new()
                .with_name(Self::WINDOW_NAME)
                .with_width(600.0)
                .with_height(500.0),
        )
        .with_content(content)
        .open(false)
        .with_title(WindowTitle::text("Animation Editor"))
        .with_tab_label("Animation")
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
        ui.send(
            self.window,
            WindowMessage::Open {
                alignment: WindowAlignment::Center,
                modal: false,
                focus_content: true,
            },
        );
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
        N: SceneGraphNode<SceneGraph = G, ResourceData = P> + TransformProvider,
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

            if let Some(msg) = message.data_from::<CurveEditorMessage>(self.curve_editor) {
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
                        ui.send(self.ruler, RulerMessage::ViewPosition(position.x));
                        ui.send(self.thumb, ThumbMessage::ViewPosition(position.x));
                    }
                    CurveEditorMessage::Zoom(zoom) => {
                        ui.send(self.ruler, RulerMessage::Zoom(zoom.x));
                        ui.send(self.thumb, ThumbMessage::Zoom(zoom.x))
                    }
                    _ => (),
                }
            } else if let Some(msg) = message.data_from::<RulerMessage>(self.ruler) {
                if animations.try_get(selection.animation).is_ok() {
                    match msg {
                        RulerMessage::Value(value) => {
                            if let Ok(animation) = animations.try_get_mut(selection.animation) {
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
                            if let Ok(animation) = animations.try_get(selection.animation) {
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
                        graph.try_get_node_mut(selection.animation_player).unwrap();

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

                    if let Ok(animation) = animations.try_get_mut(selection.animation) {
                        animation.rewind();

                        let animation_targets = animation
                            .track_bindings()
                            .values()
                            .map(|binding| binding.target())
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

                    let size = ui[self.curve_editor].actual_local_size();
                    let length = animation_ref.length().max(1.0);
                    let zoom = size.x / length;

                    ui.send(
                        self.curve_editor,
                        CurveEditorMessage::Zoom(Vector2::new(zoom, zoom)),
                    );

                    ui.send(
                        self.curve_editor,
                        CurveEditorMessage::ViewPosition(Vector2::new(
                            0.5 * animation_ref.length(),
                            0.0,
                        )),
                    );
                }
                ToolbarAction::PlayPause => {
                    if self.preview_mode_data.is_some() {
                        if let Ok(animation) = animations.try_get_mut(selection.animation) {
                            animation.set_enabled(!animation.is_enabled());
                        }
                    }
                }
                ToolbarAction::Stop => {
                    if self.preview_mode_data.is_some() {
                        if let Ok(animation) = animations.try_get_mut(selection.animation) {
                            animation.rewind();
                            animation.set_enabled(false);
                        }
                    }
                }
                ToolbarAction::NewAnimation => {
                    let size = ui[self.curve_editor].actual_local_size();
                    let length = 1.0;
                    let zoom = size.x / length * 0.9;
                    ui.send_many(
                        self.curve_editor,
                        [
                            CurveEditorMessage::Zoom(Vector2::new(zoom, zoom)),
                            CurveEditorMessage::ViewPosition(Vector2::new(0.5 * length, 0.0)),
                        ],
                    );
                }
            }

            self.track_list.handle_ui_message(
                message,
                editor_selection,
                &selection,
                root,
                sender,
                ui,
                graph,
            );
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

    pub fn destroy(self, ui: &UserInterface, docking_manager: Handle<DockingManager>) {
        self.toolbar.destroy(ui);
        ui.send(
            docking_manager,
            DockingManagerMessage::RemoveFloatingWindow(self.window),
        );
        ui.send(self.window, WidgetMessage::Remove);
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
        let selection = fetch_selection(self, graph, editor_selection);

        if let Some(container) = animation_container_ref(graph, selection.animation_player) {
            if let Ok(animation) = container.try_get(selection.animation) {
                ui.send(
                    self.thumb,
                    ThumbMessage::Position(animation.time_position()),
                );
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

            if let Ok(animation) = animations.try_get(selection.animation) {
                self.track_list
                    .sync_to_model(editor_selection, animation, graph, &selection, ui);

                let animation_tracks_data_state = animation.tracks_data().state();
                let Some(animation_tracks_data) = animation_tracks_data_state.data_ref() else {
                    return;
                };

                ui.send_sync(
                    self.curve_editor,
                    CurveEditorMessage::HighlightZones(vec![HighlightZone {
                        rect: Rect::new(
                            animation.time_slice().start,
                            -100000.0,
                            animation.time_slice().end - animation.time_slice().start,
                            200000.0,
                        ),
                        brush: ui.style.get_or_default(Style::BRUSH_PRIMARY),
                    }]),
                );

                ui.send_sync(
                    self.ruler,
                    RulerMessage::SyncSignals(
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

                            if let Some(track) = animation_tracks_data
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
                                animation_tracks_data.tracks().iter().find_map(|t| {
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
                for track in animation_tracks_data.tracks() {
                    for curve in track.data_container().curves_ref() {
                        if !selected_curves.iter().any(|(_, c)| c.id == curve.id) {
                            background_curves.push(curve.clone());
                        }
                    }
                }

                ui.send_sync(
                    self.curve_editor,
                    CurveEditorMessage::SyncBackground(background_curves),
                );

                if !selected_curves.is_empty() {
                    let color_map = selected_curves
                        .iter()
                        .map(|(index, curve)| (curve.id, Brush::Solid(Color::COLORS[3 + *index])))
                        .collect::<Vec<_>>();

                    ui.send_sync(
                        self.curve_editor,
                        CurveEditorMessage::Sync(
                            selected_curves
                                .into_iter()
                                .map(|(_, curve)| curve)
                                .collect(),
                        ),
                    );
                    ui.send_sync(self.curve_editor, CurveEditorMessage::Colorize(color_map));

                    is_curve_selected = true;
                }
                is_animation_selected = true;
            }
            is_animation_player_selected = true;
        }

        if !is_animation_selected || !is_animation_player_selected {
            self.track_list.clear(ui);

            ui.send_sync(
                self.curve_editor,
                CurveEditorMessage::Zoom(Vector2::new(1.0, 1.0)),
            );
            ui.send_sync(
                self.curve_editor,
                CurveEditorMessage::ViewPosition(Vector2::default()),
            );
        }

        if !is_animation_selected || !is_animation_player_selected || !is_curve_selected {
            ui.send_sync(
                self.curve_editor,
                CurveEditorMessage::Sync(Default::default()),
            );
        }

        if !is_animation_player_selected {
            self.toolbar.clear(ui);
        }

        ui.send_sync(
            self.content,
            WidgetMessage::Visibility(is_animation_player_selected),
        );
        ui.send_sync(
            self.track_list.panel,
            WidgetMessage::Enabled(is_animation_selected),
        );
        ui.send_sync(
            self.toolbar.preview,
            ToggleButtonMessage::Toggled(self.preview_mode_data.is_some()),
        );
        ui.send_sync(
            self.curve_editor,
            WidgetMessage::Enabled(is_animation_selected),
        );
        let name = if let Ok(player) = graph.try_get(selection.animation_player) {
            player.name().to_string()
        } else {
            "No Player".to_string()
        };
        ui.send_sync(
            self.window,
            WindowMessage::Title(WindowTitle::text(format!(
                "Animation Editor - {}({}:{})",
                name,
                self.animation_player.index(),
                self.animation_player.generation()
            ))),
        )
    }
}

#[derive(Default)]
pub struct AnimationEditorPlugin {
    animation_editor: Option<AnimationEditor>,
    open_animation_editor: Handle<MenuItem>,
}

impl AnimationEditorPlugin {
    pub const ANIMATION_EDITOR: Uuid = uuid!("139e314b-89a0-4494-ae82-22487f77335d");

    fn get_or_create_animation_editor(&mut self, ui: &mut UserInterface) -> &mut AnimationEditor {
        self.animation_editor
            .get_or_insert_with(|| AnimationEditor::new(&mut ui.build_ctx()))
    }
}

impl EditorPlugin for AnimationEditorPlugin {
    fn on_start(&mut self, editor: &mut Editor) {
        let ui = editor.engine.user_interfaces.first_mut();

        if let Some(layout) = editor.settings.windows.layout.as_ref() {
            if layout.has_window(AnimationEditor::WINDOW_NAME) {
                self.get_or_create_animation_editor(ui);
            }
        }

        let ctx = &mut ui.build_ctx();
        self.open_animation_editor =
            create_menu_item("Animation Editor", Self::ANIMATION_EDITOR, vec![], ctx);
        ui.send(
            editor.menu.utils_menu.menu,
            MenuItemMessage::AddItem(self.open_animation_editor),
        );
    }

    fn on_sync_to_model(&mut self, editor: &mut Editor) {
        let entry = editor.scenes.current_scene_entry_mut();
        let animation_editor = some_or_return!(self.animation_editor.as_mut());
        let ui = editor.engine.user_interfaces.first_mut();
        if let Some(game_scene) = entry.controller.downcast_mut::<GameScene>() {
            let graph = &editor.engine.scenes[game_scene.scene].graph;
            animation_editor.sync_to_model(&entry.selection, ui, graph);
        } else if let Some(ui_scene) = entry.controller.downcast_mut::<UiScene>() {
            animation_editor.sync_to_model(&entry.selection, ui, &ui_scene.ui);
        }
    }

    fn on_scene_changed(&mut self, editor: &mut Editor) {
        let animation_editor = some_or_return!(self.animation_editor.as_mut());
        let ui = editor.engine.user_interfaces.first_mut();
        animation_editor.clear(ui);
    }

    fn on_ui_message(&mut self, message: &mut UiMessage, editor: &mut Editor) {
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.open_animation_editor {
                editor.message_sender.send(Message::OpenAnimationEditor);
            }
        }

        let mut animation_editor = some_or_return!(self.animation_editor.take());

        if let Some(WindowMessage::Close) = message.data() {
            if message.destination() == animation_editor.window {
                self.on_leave_preview_mode(editor);

                animation_editor.destroy(
                    editor.engine.user_interfaces.first(),
                    editor.docking_manager,
                );

                return;
            }
        }

        let entry = editor.scenes.current_scene_entry_mut();
        let ui = editor.engine.user_interfaces.first_mut();
        if let Some(game_scene) = entry.controller.downcast_mut::<GameScene>() {
            let graph = &mut editor.engine.scenes[game_scene.scene].graph;
            animation_editor.handle_ui_message(
                message,
                &entry.selection,
                graph,
                game_scene.scene_content_root,
                ui,
                &editor.engine.resource_manager,
                &editor.message_sender,
                game_scene.graph_switches.node_overrides.as_mut().unwrap(),
            );
        } else if let Some(ui_scene) = entry.controller.downcast_mut::<UiScene>() {
            let ui_root = ui_scene.ui.root();
            animation_editor.handle_ui_message(
                message,
                &entry.selection,
                &mut ui_scene.ui,
                ui_root,
                ui,
                &editor.engine.resource_manager,
                &editor.message_sender,
                ui_scene.ui_update_switches.node_overrides.as_mut().unwrap(),
            );
        }

        self.animation_editor = Some(animation_editor);
    }

    fn on_leave_preview_mode(&mut self, editor: &mut Editor) {
        let entry = editor.scenes.current_scene_entry_mut();
        let animation_editor = some_or_return!(self.animation_editor.as_mut());
        if let Some(game_scene) = entry.controller.downcast_mut::<GameScene>() {
            let engine = &mut editor.engine;
            animation_editor.try_leave_preview_mode(
                &mut engine.scenes[game_scene.scene].graph,
                engine.user_interfaces.first(),
                game_scene.graph_switches.node_overrides.as_mut().unwrap(),
            );
        } else if let Some(ui_scene) = entry.controller.downcast_mut::<UiScene>() {
            animation_editor.try_leave_preview_mode(
                &mut ui_scene.ui,
                editor.engine.user_interfaces.first(),
                ui_scene.ui_update_switches.node_overrides.as_mut().unwrap(),
            );
        }
    }

    fn is_in_preview_mode(&self, _editor: &Editor) -> bool {
        let animation_editor = some_or_return!(self.animation_editor.as_ref(), false);
        animation_editor.is_in_preview_mode()
    }

    fn on_update(&mut self, editor: &mut Editor, _loop_controller: ApplicationLoopController) {
        let entry = editor.scenes.current_scene_entry_mut();
        let animation_editor = some_or_return!(self.animation_editor.as_mut());
        if let Some(game_scene) = entry.controller.downcast_ref::<GameScene>() {
            animation_editor.update(
                &entry.selection,
                editor.engine.user_interfaces.first(),
                &editor.engine.scenes[game_scene.scene].graph,
            );
        } else if let Some(ui_scene) = entry.controller.downcast_ref::<UiScene>() {
            animation_editor.update(
                &entry.selection,
                editor.engine.user_interfaces.first(),
                &ui_scene.ui,
            );
        }
    }

    fn on_message(&mut self, message: &Message, editor: &mut Editor) {
        if let Message::OpenAnimationEditor = message {
            let ui = editor.engine.user_interfaces.first_mut();
            let animation_editor = self.get_or_create_animation_editor(ui);

            animation_editor.open(ui);

            ui.send(
                editor.docking_manager,
                DockingManagerMessage::AddFloatingWindow(animation_editor.window),
            );

            self.on_sync_to_model(editor);
        }

        let entry = editor.scenes.current_scene_entry_mut();
        let animation_editor = some_or_return!(self.animation_editor.as_mut());
        if let Some(game_scene) = entry.controller.downcast_mut::<GameScene>() {
            animation_editor.handle_message(
                message,
                &mut editor.engine.scenes[game_scene.scene].graph,
                editor.engine.user_interfaces.first(),
                game_scene.graph_switches.node_overrides.as_mut().unwrap(),
            );
        } else if let Some(ui_scene) = entry.controller.downcast_mut::<UiScene>() {
            animation_editor.handle_message(
                message,
                &mut ui_scene.ui,
                editor.engine.user_interfaces.first(),
                ui_scene.ui_update_switches.node_overrides.as_mut().unwrap(),
            );
        }
    }
}

#[cfg(test)]
mod test {
    use crate::plugins::animation::AnimationEditor;
    use fyrox::core::algebra::Vector2;
    use fyrox::core::pool::Handle;
    use fyrox::gui::UserInterface;

    #[test]
    fn test_deletion() {
        let screen_size = Vector2::new(100.0, 100.0);
        let mut ui = UserInterface::new(screen_size);
        let editor = AnimationEditor::new(&mut ui.build_ctx());
        editor.destroy(&ui, Handle::NONE);
        ui.update(screen_size, 1.0 / 60.0, &Default::default());
        while ui.poll_message().is_some() {}
        // Only root node must be alive.
        assert_eq!(ui.nodes().alive_count(), 1);
    }
}
