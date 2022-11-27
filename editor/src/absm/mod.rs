use crate::{
    absm::{
        command::blend::{AddInputCommand, AddPoseSourceCommand},
        node::{AbsmNode, AbsmNodeMessage},
        parameter::ParameterPanel,
        state_graph::StateGraphViewer,
        state_viewer::StateViewer,
        toolbar::{Toolbar, ToolbarAction},
    },
    scene::{EditorScene, Selection},
    Message,
};
use fyrox::{
    animation::machine::{BlendPose, Event, IndexedBlendInput, PoseNode, State},
    core::{color::Color, pool::Handle},
    engine::Engine,
    gui::{
        check_box::CheckBoxMessage,
        dock::{DockingManagerBuilder, TileBuilder, TileContent},
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessage},
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, UiNode, UserInterface,
    },
    scene::{
        animation::{absm::AnimationBlendingStateMachine, AnimationPlayer},
        node::Node,
        Scene,
    },
};
use std::sync::mpsc::Sender;

mod canvas;
pub mod command;
mod connection;
mod node;
mod parameter;
mod segment;
mod selectable;
pub mod selection;
mod socket;
mod state_graph;
mod state_viewer;
mod toolbar;
mod transition;

const NORMAL_BACKGROUND: Color = Color::opaque(60, 60, 60);
const SELECTED_BACKGROUND: Color = Color::opaque(80, 80, 80);
const BORDER_COLOR: Color = Color::opaque(70, 70, 70);
const NORMAL_ROOT_COLOR: Color = Color::opaque(40, 80, 0);
const SELECTED_ROOT_COLOR: Color = Color::opaque(60, 100, 0);

struct PreviewModeData {
    nodes: Vec<(Handle<Node>, Node)>,
}

pub struct AbsmEditor {
    pub window: Handle<UiNode>,
    state_graph_viewer: StateGraphViewer,
    state_viewer: StateViewer,
    parameter_panel: ParameterPanel,
    absm: Handle<Node>,
    toolbar: Toolbar,
    preview_mode_data: Option<PreviewModeData>,
}

impl AbsmEditor {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let state_graph_viewer = StateGraphViewer::new(ctx);
        let state_viewer = StateViewer::new(ctx);
        let parameter_panel = ParameterPanel::new(ctx, sender);

        let docking_manager = DockingManagerBuilder::new(
            WidgetBuilder::new().on_row(1).with_child(
                TileBuilder::new(WidgetBuilder::new())
                    .with_content(TileContent::HorizontalTiles {
                        splitter: 0.3,
                        tiles: [
                            TileBuilder::new(WidgetBuilder::new())
                                .with_content(TileContent::Window(parameter_panel.window))
                                .build(ctx),
                            TileBuilder::new(WidgetBuilder::new())
                                .with_content(TileContent::HorizontalTiles {
                                    splitter: 0.5,
                                    tiles: [
                                        TileBuilder::new(WidgetBuilder::new())
                                            .with_content(TileContent::Window(
                                                state_graph_viewer.window,
                                            ))
                                            .build(ctx),
                                        TileBuilder::new(WidgetBuilder::new())
                                            .with_content(TileContent::Window(state_viewer.window))
                                            .build(ctx),
                                    ],
                                })
                                .build(ctx),
                        ],
                    })
                    .build(ctx),
            ),
        )
        .build(ctx);

        let toolbar = Toolbar::new(ctx);

        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(toolbar.panel)
                .with_child(docking_manager),
        )
        .add_row(Row::strict(22.0))
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(800.0).with_height(500.0))
            .open(false)
            .with_content(content)
            .with_title(WindowTitle::text("ABSM Editor"))
            .build(ctx);

        Self {
            window,
            state_graph_viewer,
            state_viewer,
            parameter_panel,
            absm: Default::default(),
            toolbar,
            preview_mode_data: None,
        }
    }

    fn enter_preview_mode(
        &mut self,
        animation_targets: Vec<Handle<Node>>,
        scene: &Scene,
        ui: &UserInterface,
    ) {
        assert!(self.preview_mode_data.is_none());

        ui.send_message(CheckBoxMessage::checked(
            self.toolbar.preview,
            MessageDirection::ToWidget,
            Some(true),
        ));

        // Save state of affected nodes.
        self.preview_mode_data = Some(PreviewModeData {
            nodes: animation_targets
                .into_iter()
                .map(|t| (t, scene.graph[t].clone_box()))
                .collect(),
        });
    }

    fn leave_preview_mode(&mut self, scene: &mut Scene, ui: &UserInterface) {
        ui.send_message(CheckBoxMessage::checked(
            self.toolbar.preview,
            MessageDirection::ToWidget,
            Some(false),
        ));

        let preview_data = self
            .preview_mode_data
            .take()
            .expect("Unable to leave ABSM preview mode!");

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
            if let Selection::Absm(ref selection) = editor_scene.selection {
                let scene = &mut engine.scenes[editor_scene.scene];

                if let Some(absm) = scene
                    .graph
                    .try_get_mut(selection.absm_node_handle)
                    .and_then(|n| n.query_component_mut::<AnimationBlendingStateMachine>())
                {
                    absm.set_enabled(false);

                    let animation_player_handle = absm.animation_player();

                    if let Some(animation_player) = scene
                        .graph
                        .try_get_mut(animation_player_handle)
                        .and_then(|n| n.query_component_mut::<AnimationPlayer>())
                    {
                        if self.preview_mode_data.is_some() {
                            for animation in animation_player.animations_mut().iter_mut() {
                                animation.set_enabled(false);
                            }

                            self.leave_preview_mode(scene, &engine.user_interface);
                        }
                    }
                }
            }
        }
    }

    pub fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut Engine) {
        let prev_absm = self.absm;

        self.absm = match editor_scene.selection {
            Selection::Absm(ref selection) => selection.absm_node_handle,
            Selection::Graph(ref selection) => selection.nodes.first().cloned().unwrap_or_default(),
            _ => Default::default(),
        };

        let ui = &mut engine.user_interface;
        let scene = &mut engine.scenes[editor_scene.scene];

        let absm_node = scene
            .graph
            .try_get(self.absm)
            .and_then(|n| n.query_component_ref::<AnimationBlendingStateMachine>());

        if self.absm != prev_absm {
            self.parameter_panel.on_selection_changed(ui, absm_node);
        }

        if let Some(absm_node) = absm_node {
            self.parameter_panel.sync_to_model(ui, absm_node);
            self.state_graph_viewer
                .sync_to_model(absm_node, ui, editor_scene);
            self.state_viewer.sync_to_model(ui, absm_node, editor_scene);
        } else {
            self.parameter_panel.reset(ui);
            self.state_graph_viewer.clear(ui);
            self.state_viewer.clear(ui);
        }
    }

    pub fn open(&self, ui: &UserInterface) {
        ui.send_message(WindowMessage::open(
            self.window,
            MessageDirection::ToWidget,
            true,
        ));
    }

    pub fn update(&mut self, editor_scene: &EditorScene, engine: &mut Engine) {
        self.handle_machine_events(editor_scene, engine);
    }

    pub fn handle_machine_events(&self, editor_scene: &EditorScene, engine: &mut Engine) {
        let scene = &mut engine.scenes[editor_scene.scene];

        if let Some(absm) = scene
            .graph
            .try_get_mut(self.absm)
            .and_then(|n| n.query_component_mut::<AnimationBlendingStateMachine>())
        {
            let machine = absm.machine_mut();

            while let Some(event) = machine.pop_event() {
                match event {
                    Event::ActiveStateChanged(state) => {
                        self.state_graph_viewer
                            .activate_state(&engine.user_interface, state);
                    }
                    Event::ActiveTransitionChanged(transition) => {
                        self.state_graph_viewer
                            .activate_transition(&engine.user_interface, transition);
                    }
                    _ => (),
                }
            }
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        engine: &mut Engine,
        sender: &Sender<Message>,
        editor_scene: &EditorScene,
    ) {
        let scene = &mut engine.scenes[editor_scene.scene];
        let ui = &mut engine.user_interface;

        if let Some(absm_node) = scene
            .graph
            .try_get_mut(self.absm)
            .and_then(|n| n.query_component_mut::<AnimationBlendingStateMachine>())
        {
            self.state_viewer.handle_ui_message(
                message,
                ui,
                sender,
                self.absm,
                absm_node,
                editor_scene,
            );
            self.state_graph_viewer.handle_ui_message(
                message,
                ui,
                sender,
                self.absm,
                absm_node,
                editor_scene,
            );
            self.parameter_panel
                .handle_ui_message(message, sender, self.absm);

            let action = self.toolbar.handle_ui_message(message);

            match action {
                ToolbarAction::None => {}
                ToolbarAction::EnterPreviewMode => {
                    absm_node.set_enabled(true);

                    // Enable all animations in the player.
                    let animation_player = absm_node.animation_player();

                    if let Some(animation_player) = scene
                        .graph
                        .try_get_mut(animation_player)
                        .and_then(|n| n.query_component_mut::<AnimationPlayer>())
                    {
                        let mut animation_targets = Vec::new();
                        for animation in animation_player.animations_mut().iter_mut() {
                            animation.set_enabled(true);

                            for track in animation.tracks() {
                                animation_targets.push(track.target());
                            }
                        }

                        self.enter_preview_mode(animation_targets, scene, ui);
                    }
                }
                ToolbarAction::LeavePreviewMode => {
                    absm_node.set_enabled(false);

                    // Disable all animations in the player.
                    let animation_player = absm_node.animation_player();
                    if let Some(animation_player) = scene
                        .graph
                        .try_get_mut(animation_player)
                        .and_then(|n| n.query_component_mut::<AnimationPlayer>())
                    {
                        for animation in animation_player.animations_mut().iter_mut() {
                            animation.set_enabled(false);
                        }

                        self.leave_preview_mode(scene, ui);
                    }
                }
            }
        }

        if let Some(msg) = message.data::<AbsmNodeMessage>() {
            if let Some(absm_node) = scene
                .graph
                .try_get_mut(self.absm)
                .and_then(|n| n.query_component_mut::<AnimationBlendingStateMachine>())
            {
                match msg {
                    AbsmNodeMessage::Enter => {
                        if let Some(node) = ui
                            .node(message.destination())
                            .query_component::<AbsmNode<State>>()
                        {
                            self.state_viewer
                                .set_state(node.model_handle, absm_node, ui);
                            sender.send(Message::ForceSync).unwrap();
                        }
                    }
                    AbsmNodeMessage::AddInput => {
                        if let Some(node) = ui
                            .node(message.destination())
                            .query_component::<AbsmNode<PoseNode>>()
                        {
                            let model_ref = &absm_node.machine().nodes()[node.model_handle];

                            match model_ref {
                                PoseNode::PlayAnimation(_) => {
                                    // No input sockets
                                }
                                PoseNode::BlendAnimations(_) => {
                                    sender
                                        .send(Message::do_scene_command(AddPoseSourceCommand::new(
                                            self.absm,
                                            node.model_handle,
                                            BlendPose::default(),
                                        )))
                                        .unwrap();
                                }
                                PoseNode::BlendAnimationsByIndex(_) => {
                                    sender
                                        .send(Message::do_scene_command(AddInputCommand::new(
                                            self.absm,
                                            node.model_handle,
                                            IndexedBlendInput::default(),
                                        )))
                                        .unwrap();
                                }
                            }
                        }
                    }
                    _ => (),
                }
            }
        }
    }
}
