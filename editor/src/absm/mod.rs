use crate::fyrox::{
    core::{color::Color, pool::ErasedHandle, pool::Handle, variable::InheritableVariable},
    fxhash::FxHashSet,
    generic_animation::{
        machine::{
            event::Event, node::blendspace::BlendSpacePoint, BlendPose, IndexedBlendInput, Machine,
            PoseNode, State,
        },
        AnimationContainer,
    },
    graph::{BaseSceneGraph, PrefabData, SceneGraph, SceneGraphNode},
    gui::{
        check_box::CheckBoxMessage,
        dock::{DockingManagerBuilder, TileBuilder, TileContent},
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessage},
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, UiNode, UserInterface,
    },
};
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
    scene::Selection,
    Message,
};
use std::{any::Any, fmt::Debug};

mod blendspace;
mod canvas;
pub mod command;
mod connection;
mod node;
mod parameter;
mod segment;
pub mod selectable;
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

struct PreviewModeData<N: 'static> {
    machine: Machine<Handle<N>>,
    nodes: Vec<(Handle<N>, N)>,
}

fn fetch_selection<N>(editor_selection: &Selection) -> AbsmSelection<N>
where
    N: Debug,
{
    if let Some(selection) = editor_selection.as_absm() {
        // Some selection in an animation.
        AbsmSelection {
            absm_node_handle: selection.absm_node_handle,
            layer: selection.layer,
            entities: selection.entities.clone(),
        }
    } else if let Some(selection) = editor_selection.as_graph() {
        // Only some AnimationPlayer in a graph is selected.
        AbsmSelection {
            absm_node_handle: ErasedHandle::from(
                selection.nodes.first().cloned().unwrap_or_default(),
            )
            .into(),
            layer: None,
            entities: vec![],
        }
    } else if let Some(selection) = editor_selection.as_ui() {
        // Only some AnimationPlayer in a UI is selected.
        AbsmSelection {
            absm_node_handle: ErasedHandle::from(
                selection.widgets.first().cloned().unwrap_or_default(),
            )
            .into(),
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

fn machine_container<G, N>(graph: &mut G, handle: Handle<N>) -> Option<&mut Machine<Handle<N>>>
where
    G: SceneGraph<Node = N>,
    N: SceneGraphNode<SceneGraph = G>,
{
    graph
        .try_get_mut(handle)
        .and_then(|n| n.component_mut::<InheritableVariable<Machine<Handle<N>>>>())
        .map(|v| v.get_value_mut_silent())
}

fn animation_container<G, N>(
    graph: &mut G,
    handle: Handle<N>,
) -> Option<(Handle<N>, &mut AnimationContainer<Handle<N>>)>
where
    G: SceneGraph<Node = N>,
    N: SceneGraphNode<SceneGraph = G>,
{
    let animation_player_handle = *graph
        .try_get(handle)
        .and_then(|n| n.component_ref::<InheritableVariable<Handle<N>>>())
        .cloned()?;

    graph
        .try_get_mut(animation_player_handle)
        .and_then(|n| n.component_mut::<InheritableVariable<AnimationContainer<Handle<N>>>>())
        .map(|ac| (animation_player_handle, ac.get_value_mut_silent()))
}

fn machine_container_ref<G, N>(graph: &G, handle: Handle<N>) -> Option<&Machine<Handle<N>>>
where
    G: SceneGraph<Node = N>,
    N: SceneGraphNode<SceneGraph = G>,
{
    graph
        .try_get(handle)
        .and_then(|n| n.component_ref::<InheritableVariable<Machine<Handle<N>>>>())
        .map(|v| v.get_value_ref())
}

pub fn animation_container_ref<G, N>(
    graph: &G,
    handle: Handle<N>,
) -> Option<(Handle<N>, &AnimationContainer<Handle<N>>)>
where
    G: SceneGraph<Node = N>,
    N: SceneGraphNode<SceneGraph = G>,
{
    graph
        .try_get(handle)
        .and_then(|n| n.component_ref::<InheritableVariable<Handle<N>>>())
        .and_then(|ap| {
            graph
                .try_get(**ap)
                .and_then(|n| {
                    n.component_ref::<InheritableVariable<AnimationContainer<Handle<N>>>>()
                })
                .map(|ac| (**ap, &**ac))
        })
}

pub struct AbsmEditor {
    pub window: Handle<UiNode>,
    state_graph_viewer: StateGraphViewer,
    state_viewer: StateViewer,
    parameter_panel: ParameterPanel,
    prev_absm: ErasedHandle,
    toolbar: Toolbar,
    preview_mode_data: Option<Box<dyn Any>>,
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

    fn enter_preview_mode<P, G, N>(
        &mut self,
        machine: Machine<Handle<N>>,
        animation_targets: FxHashSet<Handle<N>>,
        graph: &G,
        ui: &UserInterface,
        node_overrides: &mut FxHashSet<Handle<N>>,
    ) where
        P: PrefabData<Graph = G>,
        G: SceneGraph<Node = N, Prefab = P>,
        N: SceneGraphNode<SceneGraph = G, ResourceData = P>,
    {
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
        self.preview_mode_data = Some(Box::new(PreviewModeData {
            machine,
            nodes: animation_targets
                .into_iter()
                .map(|t| (t, graph.node(t).clone()))
                .collect(),
        }));
    }

    fn leave_preview_mode<P, G, N>(
        &mut self,
        graph: &mut G,
        ui: &mut UserInterface,
        absm: Handle<N>,
        node_overrides: &mut FxHashSet<Handle<N>>,
    ) where
        P: PrefabData<Graph = G>,
        G: SceneGraph<Node = N, Prefab = P>,
        N: SceneGraphNode<SceneGraph = G, ResourceData = P>,
    {
        ui.send_message(CheckBoxMessage::checked(
            self.toolbar.preview,
            MessageDirection::ToWidget,
            Some(false),
        ));

        let preview_data = self
            .preview_mode_data
            .take()
            .expect("Unable to leave ABSM preview mode!")
            .downcast::<PreviewModeData<N>>()
            .expect("Types must match!");

        // Revert state of nodes.
        for (handle, node) in preview_data.nodes {
            assert!(node_overrides.remove(&handle));
            *graph.node_mut(handle) = node;
        }

        let machine = machine_container(graph, absm).unwrap();

        *machine = preview_data.machine;

        self.parameter_panel.sync_to_model(ui, machine.parameters());
    }

    pub fn try_leave_preview_mode<P, G, N>(
        &mut self,
        editor_selection: &Selection,
        graph: &mut G,
        ui: &mut UserInterface,
        node_overrides: &mut FxHashSet<Handle<N>>,
    ) where
        P: PrefabData<Graph = G>,
        G: SceneGraph<Node = N, Prefab = P>,
        N: SceneGraphNode<SceneGraph = G, ResourceData = P>,
    {
        if self.preview_mode_data.is_some() {
            let selection = fetch_selection(editor_selection);

            let animation_player = animation_container(graph, selection.absm_node_handle)
                .map(|pair| pair.0)
                .unwrap_or_default();

            assert!(node_overrides.remove(&selection.absm_node_handle));
            assert!(node_overrides.remove(&animation_player));

            self.leave_preview_mode(graph, ui, selection.absm_node_handle, node_overrides);
        }
    }

    pub fn is_in_preview_mode(&self) -> bool {
        self.preview_mode_data.is_some()
    }

    pub fn handle_message<P, G, N>(
        &mut self,
        message: &Message,
        editor_selection: &Selection,
        graph: &mut G,
        ui: &mut UserInterface,
        node_overrides: &mut FxHashSet<Handle<N>>,
    ) where
        P: PrefabData<Graph = G>,
        G: SceneGraph<Node = N, Prefab = P>,
        N: SceneGraphNode<SceneGraph = G, ResourceData = P>,
    {
        // Leave preview mode before execution of any scene command.
        if let Message::DoCommand(_)
        | Message::UndoCurrentSceneCommand
        | Message::RedoCurrentSceneCommand = message
        {
            self.try_leave_preview_mode(editor_selection, graph, ui, node_overrides);
        }
    }

    pub fn sync_to_model<P, G, N>(
        &mut self,
        editor_selection: &Selection,
        graph: &G,
        ui: &mut UserInterface,
    ) where
        P: PrefabData<Graph = G>,
        G: SceneGraph<Node = N, Prefab = P>,
        N: SceneGraphNode<SceneGraph = G, ResourceData = P>,
    {
        let prev_absm = self.prev_absm;

        let selection = fetch_selection(editor_selection);

        let machine = machine_container_ref(graph, selection.absm_node_handle);

        if prev_absm != selection.absm_node_handle.into() {
            self.parameter_panel
                .on_selection_changed(ui, machine.as_ref().map(|m| m.parameters()));
            self.prev_absm = selection.absm_node_handle.into();
        }

        if let Some(machine) = machine {
            self.parameter_panel.sync_to_model(ui, machine.parameters());
            self.toolbar.sync_to_model(machine, ui, &selection);
            if let Some(layer_index) = selection.layer {
                if let Some(layer) = machine.layers().get(layer_index) {
                    self.state_graph_viewer
                        .sync_to_model(layer, ui, editor_selection);
                    self.state_viewer.sync_to_model(
                        ui,
                        layer,
                        editor_selection,
                        animation_container_ref(graph, selection.absm_node_handle).map(|(_, c)| c),
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
            true,
        ));
    }

    pub fn update<P, G, N>(
        &mut self,
        editor_selection: &Selection,
        graph: &mut G,
        ui: &mut UserInterface,
    ) where
        P: PrefabData<Graph = G>,
        G: SceneGraph<Node = N, Prefab = P>,
        N: SceneGraphNode<SceneGraph = G, ResourceData = P>,
    {
        self.handle_machine_events(editor_selection, graph, ui);
    }

    pub fn handle_machine_events<P, G, N>(
        &self,
        editor_selection: &Selection,
        graph: &mut G,
        ui: &mut UserInterface,
    ) where
        P: PrefabData<Graph = G>,
        G: SceneGraph<Node = N, Prefab = P>,
        N: SceneGraphNode<SceneGraph = G, ResourceData = P>,
    {
        let selection = fetch_selection(editor_selection);

        if let Some(machine) = machine_container(graph, selection.absm_node_handle) {
            if let Some(layer_index) = selection.layer {
                if let Some(layer) = machine.layers_mut().get_mut(layer_index) {
                    while let Some(event) = layer.pop_event() {
                        match event {
                            Event::ActiveStateChanged { new: state, .. } => {
                                self.state_graph_viewer.activate_state(ui, state);
                            }
                            Event::ActiveTransitionChanged(transition) => {
                                self.state_graph_viewer.activate_transition(ui, transition);
                            }
                            _ => (),
                        }
                    }
                }
            }
        }
    }

    pub fn handle_ui_message<P, G, N>(
        &mut self,
        message: &UiMessage,
        sender: &MessageSender,
        editor_selection: &Selection,
        graph: &mut G,
        ui: &mut UserInterface,
        node_overrides: &mut FxHashSet<Handle<N>>,
    ) where
        P: PrefabData<Graph = G>,
        G: SceneGraph<Node = N, Prefab = P>,
        N: SceneGraphNode<SceneGraph = G, ResourceData = P>,
    {
        let selection = fetch_selection(editor_selection);

        if let Some(machine) = machine_container(graph, selection.absm_node_handle) {
            if let Some(layer_index) = selection.layer {
                self.state_viewer.handle_ui_message(
                    message,
                    ui,
                    sender,
                    selection.absm_node_handle,
                    machine,
                    layer_index,
                    editor_selection,
                );
                self.state_graph_viewer.handle_ui_message(
                    message,
                    ui,
                    sender,
                    selection.absm_node_handle,
                    machine,
                    layer_index,
                    editor_selection,
                );
                self.blend_space_editor.handle_ui_message(
                    &selection,
                    message,
                    sender,
                    machine,
                    self.preview_mode_data.is_some(),
                );
            }

            self.parameter_panel.handle_ui_message(
                message,
                sender,
                selection.absm_node_handle,
                machine.parameters_mut(),
                self.preview_mode_data.is_some(),
            );

            let action =
                self.toolbar
                    .handle_ui_message(message, editor_selection, sender, graph, ui);

            let machine = machine_container(graph, selection.absm_node_handle).unwrap();

            match action {
                ToolbarAction::None => {}
                ToolbarAction::EnterPreviewMode => {
                    assert!(node_overrides.insert(selection.absm_node_handle));

                    let machine_clone = machine.clone();

                    if let Some((animation_container_handle, animations)) =
                        animation_container(graph, selection.absm_node_handle)
                    {
                        assert!(node_overrides.insert(animation_container_handle));

                        let mut animation_targets = FxHashSet::default();
                        for animation in animations.iter_mut() {
                            for track in animation.tracks() {
                                animation_targets.insert(track.target());
                            }
                        }

                        self.enter_preview_mode(
                            machine_clone,
                            animation_targets,
                            graph,
                            ui,
                            node_overrides,
                        );
                    }
                }
                ToolbarAction::LeavePreviewMode => {
                    if self.preview_mode_data.is_some() {
                        let animation_player =
                            animation_container(graph, selection.absm_node_handle)
                                .map(|pair| pair.0)
                                .unwrap_or_default();
                        assert!(node_overrides.remove(&selection.absm_node_handle));
                        assert!(node_overrides.remove(&animation_player));

                        self.leave_preview_mode(
                            graph,
                            ui,
                            selection.absm_node_handle,
                            node_overrides,
                        );
                    }
                }
            }
        }

        if let Some(msg) = message.data::<AbsmNodeMessage>() {
            if let Some(machine) = machine_container(graph, selection.absm_node_handle) {
                match msg {
                    AbsmNodeMessage::Enter => {
                        if let Some(node) = ui
                            .node(message.destination())
                            .query_component::<AbsmNode<State<Handle<N>>>>()
                        {
                            if let Some(layer_index) = selection.layer {
                                self.state_viewer.set_state(
                                    node.model_handle,
                                    machine,
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
                            .query_component::<AbsmNode<PoseNode<Handle<N>>>>()
                        {
                            if let Some(layer_index) = selection.layer {
                                let model_ref =
                                    &machine.layers()[layer_index].nodes()[node.model_handle];

                                if let PoseNode::BlendSpace(_) = model_ref {
                                    self.blend_space_editor.open(ui);
                                }
                            }
                        }
                    }
                    AbsmNodeMessage::AddInput => {
                        if let Some(node) = ui
                            .node(message.destination())
                            .query_component::<AbsmNode<PoseNode<Handle<N>>>>()
                        {
                            if let Some(layer_index) = selection.layer {
                                let model_ref =
                                    &machine.layers()[layer_index].nodes()[node.model_handle];

                                match model_ref {
                                    PoseNode::PlayAnimation(_) => {
                                        // No input sockets
                                    }
                                    PoseNode::BlendAnimations(_) => {
                                        sender.do_command(AddPoseSourceCommand::new(
                                            selection.absm_node_handle,
                                            node.model_handle,
                                            layer_index,
                                            BlendPose::default(),
                                        ));
                                    }
                                    PoseNode::BlendAnimationsByIndex(_) => {
                                        sender.do_command(AddInputCommand::new(
                                            selection.absm_node_handle,
                                            node.model_handle,
                                            layer_index,
                                            IndexedBlendInput::default(),
                                        ));
                                    }
                                    PoseNode::BlendSpace(_) => {
                                        sender.do_command(AddBlendSpacePointCommand::new(
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
