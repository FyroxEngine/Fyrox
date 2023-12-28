use crate::{
    absm::{
        blendspace::BlendSpaceEditor,
        command::blend::{AddBlendSpacePointCommand, AddInputCommand, AddPoseSourceCommand},
        node::{AbsmNode, AbsmNodeMessage},
        parameter::ParameterPanel,
        selection::AbsmSelection,
        state_graph::StateGraphViewer,
        state_viewer::StateViewer,
        toolbar::{Toolbar, ToolbarAction},
    },
    message::MessageSender,
    scene::{GameScene, Selection},
    Message,
};
use fyrox::{
    core::{color::Color, pool::Handle},
    engine::Engine,
    fxhash::FxHashSet,
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
        animation::{absm::prelude::*, prelude::*},
        node::Node,
        Scene,
    },
};

mod blendspace;
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
    machine: Machine,
    nodes: Vec<(Handle<Node>, Node)>,
}

fn fetch_selection(editor_selection: &Selection) -> AbsmSelection {
    if let Selection::Absm(ref selection) = editor_selection {
        // Some selection in an animation.
        AbsmSelection {
            absm_node_handle: selection.absm_node_handle,
            layer: selection.layer,
            entities: selection.entities.clone(),
        }
    } else if let Selection::Graph(ref selection) = editor_selection {
        // Only some AnimationPlayer is selected.
        AbsmSelection {
            absm_node_handle: selection.nodes.first().cloned().unwrap_or_default(),
            layer: None,
            entities: vec![],
        }
    } else {
        // Stub in other cases.
        AbsmSelection {
            absm_node_handle: Default::default(),
            layer: None,
            entities: vec![],
        }
    }
}

pub struct AbsmEditor {
    pub window: Handle<UiNode>,
    state_graph_viewer: StateGraphViewer,
    state_viewer: StateViewer,
    parameter_panel: ParameterPanel,
    prev_absm: Handle<Node>,
    toolbar: Toolbar,
    preview_mode_data: Option<PreviewModeData>,
    blend_space_editor: BlendSpaceEditor,
}

impl AbsmEditor {
    pub fn new(ctx: &mut BuildContext, sender: MessageSender) -> Self {
        let state_graph_viewer = StateGraphViewer::new(ctx);
        let state_viewer = StateViewer::new(ctx);
        let parameter_panel = ParameterPanel::new(ctx, sender);
        let blend_space_editor = BlendSpaceEditor::new(ctx);

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
        .with_floating_windows(vec![blend_space_editor.window])
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

        let window = WindowBuilder::new(
            WidgetBuilder::new()
                .with_name("AsbmEditor")
                .with_width(800.0)
                .with_height(500.0),
        )
        .open(false)
        .with_content(content)
        .with_title(WindowTitle::text("ABSM Editor"))
        .build(ctx);

        Self {
            window,
            state_graph_viewer,
            state_viewer,
            parameter_panel,
            prev_absm: Default::default(),
            toolbar,
            preview_mode_data: None,
            blend_space_editor,
        }
    }

    fn enter_preview_mode(
        &mut self,
        machine: Machine,
        animation_targets: FxHashSet<Handle<Node>>,
        scene: &Scene,
        ui: &UserInterface,
        node_overrides: &mut FxHashSet<Handle<Node>>,
    ) {
        assert!(self.preview_mode_data.is_none());

        ui.send_message(CheckBoxMessage::checked(
            self.toolbar.preview,
            MessageDirection::ToWidget,
            Some(true),
        ));

        // Allow the engine to update the nodes affected by animations.
        for &target in &animation_targets {
            assert!(node_overrides.insert(target));
        }

        // Save state of affected nodes.
        self.preview_mode_data = Some(PreviewModeData {
            machine,
            nodes: animation_targets
                .into_iter()
                .map(|t| (t, scene.graph[t].clone_box()))
                .collect(),
        });
    }

    fn leave_preview_mode(
        &mut self,
        scene: &mut Scene,
        ui: &mut UserInterface,
        absm: Handle<Node>,
        node_overrides: &mut FxHashSet<Handle<Node>>,
    ) {
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
            assert!(node_overrides.remove(&handle));
            scene.graph[handle] = node;
        }

        let absm_node = scene.graph[absm]
            .query_component_mut::<AnimationBlendingStateMachine>()
            .unwrap();

        *absm_node.machine_mut().get_value_mut_silent() = preview_data.machine;

        self.parameter_panel.sync_to_model(ui, absm_node);
    }

    pub fn try_leave_preview_mode(
        &mut self,
        editor_selection: &Selection,
        game_scene: &mut GameScene,
        engine: &mut Engine,
    ) {
        if self.preview_mode_data.is_some() {
            let selection = fetch_selection(editor_selection);

            let scene = &mut engine.scenes[game_scene.scene];

            if let Some(absm) = scene
                .graph
                .try_get_mut(selection.absm_node_handle)
                .and_then(|n| n.query_component_mut::<AnimationBlendingStateMachine>())
            {
                let node_overrides = game_scene.graph_switches.node_overrides.as_mut().unwrap();
                assert!(node_overrides.remove(&selection.absm_node_handle));
                assert!(node_overrides.remove(&absm.animation_player()));

                self.leave_preview_mode(
                    scene,
                    &mut engine.user_interface,
                    selection.absm_node_handle,
                    node_overrides,
                );
            }
        }
    }

    pub fn is_in_preview_mode(&self) -> bool {
        self.preview_mode_data.is_some()
    }

    pub fn handle_message(
        &mut self,
        message: &Message,
        editor_selection: &Selection,
        game_scene: &mut GameScene,
        engine: &mut Engine,
    ) {
        // Leave preview mode before execution of any scene command.
        if let Message::DoGameSceneCommand(_)
        | Message::UndoCurrentSceneCommand
        | Message::RedoCurrentSceneCommand = message
        {
            self.try_leave_preview_mode(editor_selection, game_scene, engine);
        }
    }

    pub fn sync_to_model(
        &mut self,
        editor_selection: &Selection,
        game_scene: &GameScene,
        engine: &mut Engine,
    ) {
        let prev_absm = self.prev_absm;

        let selection = fetch_selection(editor_selection);

        let ui = &mut engine.user_interface;
        let scene = &engine.scenes[game_scene.scene];

        let absm_node = scene
            .graph
            .try_get(selection.absm_node_handle)
            .and_then(|n| n.query_component_ref::<AnimationBlendingStateMachine>());

        if selection.absm_node_handle != prev_absm {
            self.parameter_panel.on_selection_changed(ui, absm_node);
            self.prev_absm = selection.absm_node_handle;
        }

        if let Some(absm_node) = absm_node {
            self.parameter_panel.sync_to_model(ui, absm_node);
            self.toolbar.sync_to_model(absm_node, ui, &selection);
            if let Some(layer_index) = selection.layer {
                let machine = absm_node.machine();
                if let Some(layer) = machine.layers().get(layer_index) {
                    self.state_graph_viewer
                        .sync_to_model(layer, ui, editor_selection);
                    self.state_viewer.sync_to_model(
                        ui,
                        layer,
                        editor_selection,
                        absm_node,
                        &scene.graph,
                    );
                    self.blend_space_editor.sync_to_model(
                        machine.parameters(),
                        layer,
                        &selection,
                        ui,
                    );
                }
            }
        } else {
            self.clear(ui);
        }
    }

    pub fn clear(&mut self, ui: &UserInterface) {
        self.parameter_panel.reset(ui);
        self.state_graph_viewer.clear(ui);
        self.state_viewer.clear(ui);
    }

    pub fn open(&self, ui: &UserInterface) {
        ui.send_message(WindowMessage::open(
            self.window,
            MessageDirection::ToWidget,
            true,
        ));
    }

    pub fn update(
        &mut self,
        editor_selection: &Selection,
        game_scene: &GameScene,
        engine: &mut Engine,
    ) {
        self.handle_machine_events(editor_selection, game_scene, engine);
    }

    pub fn handle_machine_events(
        &self,
        editor_selection: &Selection,
        game_scene: &GameScene,
        engine: &mut Engine,
    ) {
        let scene = &mut engine.scenes[game_scene.scene];
        let selection = fetch_selection(editor_selection);

        if let Some(absm) = scene
            .graph
            .try_get_mut(selection.absm_node_handle)
            .and_then(|n| n.query_component_mut::<AnimationBlendingStateMachine>())
        {
            let machine = absm.machine_mut().get_value_mut_silent();

            if let Some(layer_index) = selection.layer {
                if let Some(layer) = machine.layers_mut().get_mut(layer_index) {
                    while let Some(event) = layer.pop_event() {
                        match event {
                            MachineEvent::ActiveStateChanged { new: state, .. } => {
                                self.state_graph_viewer
                                    .activate_state(&engine.user_interface, state);
                            }
                            MachineEvent::ActiveTransitionChanged(transition) => {
                                self.state_graph_viewer
                                    .activate_transition(&engine.user_interface, transition);
                            }
                            _ => (),
                        }
                    }
                }
            }
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        engine: &mut Engine,
        sender: &MessageSender,
        editor_selection: &Selection,
        game_scene: &mut GameScene,
    ) {
        let scene = &mut engine.scenes[game_scene.scene];
        let ui = &mut engine.user_interface;
        let selection = fetch_selection(editor_selection);

        if let Some(absm_node) = scene
            .graph
            .try_get_mut(selection.absm_node_handle)
            .and_then(|n| n.query_component_mut::<AnimationBlendingStateMachine>())
        {
            if let Some(layer_index) = selection.layer {
                self.state_viewer.handle_ui_message(
                    message,
                    ui,
                    sender,
                    selection.absm_node_handle,
                    absm_node,
                    layer_index,
                    editor_selection,
                );
                self.state_graph_viewer.handle_ui_message(
                    message,
                    ui,
                    sender,
                    selection.absm_node_handle,
                    absm_node,
                    layer_index,
                    editor_selection,
                );
                self.blend_space_editor.handle_ui_message(
                    &selection,
                    message,
                    sender,
                    absm_node.machine_mut(),
                    self.preview_mode_data.is_some(),
                );
            }

            self.parameter_panel.handle_ui_message(
                message,
                sender,
                selection.absm_node_handle,
                absm_node,
                self.preview_mode_data.is_some(),
            );

            let action = self.toolbar.handle_ui_message(
                message,
                editor_selection,
                game_scene,
                sender,
                &scene.graph,
                ui,
            );

            let absm_node = scene
                .graph
                .try_get_mut(selection.absm_node_handle)
                .and_then(|n| n.query_component_mut::<AnimationBlendingStateMachine>())
                .unwrap();

            match action {
                ToolbarAction::None => {}
                ToolbarAction::EnterPreviewMode => {
                    let node_overrides = game_scene.graph_switches.node_overrides.as_mut().unwrap();
                    assert!(node_overrides.insert(selection.absm_node_handle));
                    assert!(node_overrides.insert(absm_node.animation_player()));

                    let machine = (**absm_node.machine()).clone();

                    let animation_player = absm_node.animation_player();

                    if let Some(animation_player) = scene
                        .graph
                        .try_get_mut(animation_player)
                        .and_then(|n| n.query_component_mut::<AnimationPlayer>())
                    {
                        let mut animation_targets = FxHashSet::default();
                        for animation in animation_player.animations_mut().iter_mut() {
                            for track in animation.tracks() {
                                animation_targets.insert(track.target());
                            }
                        }

                        self.enter_preview_mode(
                            machine,
                            animation_targets,
                            scene,
                            ui,
                            node_overrides,
                        );
                    }
                }
                ToolbarAction::LeavePreviewMode => {
                    if self.preview_mode_data.is_some() {
                        let node_overrides =
                            game_scene.graph_switches.node_overrides.as_mut().unwrap();
                        assert!(node_overrides.remove(&selection.absm_node_handle));
                        assert!(node_overrides.remove(&absm_node.animation_player()));

                        self.leave_preview_mode(
                            scene,
                            ui,
                            selection.absm_node_handle,
                            node_overrides,
                        );
                    }
                }
            }
        }

        if let Some(msg) = message.data::<AbsmNodeMessage>() {
            if let Some(absm_node) = scene
                .graph
                .try_get_mut(selection.absm_node_handle)
                .and_then(|n| n.query_component_mut::<AnimationBlendingStateMachine>())
            {
                match msg {
                    AbsmNodeMessage::Enter => {
                        if let Some(node) = ui
                            .node(message.destination())
                            .query_component::<AbsmNode<State>>()
                        {
                            if let Some(layer_index) = selection.layer {
                                self.state_viewer.set_state(
                                    node.model_handle,
                                    absm_node,
                                    layer_index,
                                    ui,
                                );
                                sender.send(Message::ForceSync);
                            }
                        }
                    }
                    AbsmNodeMessage::Edit => {
                        if let Some(node) = ui
                            .node(message.destination())
                            .query_component::<AbsmNode<PoseNode>>()
                        {
                            if let Some(layer_index) = selection.layer {
                                let model_ref = &absm_node.machine().layers()[layer_index].nodes()
                                    [node.model_handle];

                                if let PoseNode::BlendSpace(_) = model_ref {
                                    self.blend_space_editor.open(ui);
                                }
                            }
                        }
                    }
                    AbsmNodeMessage::AddInput => {
                        if let Some(node) = ui
                            .node(message.destination())
                            .query_component::<AbsmNode<PoseNode>>()
                        {
                            if let Some(layer_index) = selection.layer {
                                let model_ref = &absm_node.machine().layers()[layer_index].nodes()
                                    [node.model_handle];

                                match model_ref {
                                    PoseNode::PlayAnimation(_) => {
                                        // No input sockets
                                    }
                                    PoseNode::BlendAnimations(_) => {
                                        sender.do_scene_command(AddPoseSourceCommand::new(
                                            selection.absm_node_handle,
                                            node.model_handle,
                                            layer_index,
                                            BlendPose::default(),
                                        ));
                                    }
                                    PoseNode::BlendAnimationsByIndex(_) => {
                                        sender.do_scene_command(AddInputCommand::new(
                                            selection.absm_node_handle,
                                            node.model_handle,
                                            layer_index,
                                            IndexedBlendInput::default(),
                                        ));
                                    }
                                    PoseNode::BlendSpace(_) => {
                                        sender.do_scene_command(AddBlendSpacePointCommand::new(
                                            selection.absm_node_handle,
                                            node.model_handle,
                                            layer_index,
                                            BlendSpacePoint::default(),
                                        ));
                                    }
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
