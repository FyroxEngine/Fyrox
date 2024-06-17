use crate::command::{Command, CommandGroup};
use crate::fyrox::graph::SceneGraph;
use crate::fyrox::{
    core::{
        algebra::{Vector2, Vector3},
        color::Color,
        math::{ray::CylinderKind, TriangleEdge},
        pool::Handle,
        scope_profile,
        uuid::{uuid, Uuid},
        TypeUuidProvider,
    },
    engine::Engine,
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        grid::{Column, GridBuilder, Row},
        message::{KeyCode, MessageDirection, UiMessage},
        stack_panel::StackPanelBuilder,
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, Orientation, Thickness, UiNode, UserInterface,
    },
    gui::{HorizontalAlignment, VerticalAlignment},
    scene::{camera::Camera, navmesh::NavigationalMesh},
};
use crate::scene::SelectionContainer;
use crate::{
    camera::PickingOptions,
    interaction::{
        calculate_gizmo_distance_scaling,
        gizmo::move_gizmo::MoveGizmo,
        make_interaction_mode_button,
        navmesh::selection::{NavmeshEntity, NavmeshSelection},
        plane::PlaneKind,
        InteractionMode,
    },
    message::MessageSender,
    scene::{
        commands::{
            navmesh::{
                AddNavmeshEdgeCommand, ConnectNavmeshEdgesCommand, DeleteNavmeshVertexCommand,
                MoveNavmeshVertexCommand,
            },
            ChangeSelectionCommand,
        },
        controller::SceneController,
        GameScene, Selection,
    },
    settings::Settings,
    utils::window_content,
    Mode,
};
use std::collections::HashMap;

pub mod selection;

pub struct NavmeshPanel {
    pub window: Handle<UiNode>,
    connect_edges: Handle<UiNode>,
    sender: MessageSender,
    scene_frame: Handle<UiNode>,
}

fn fetch_selection(editor_selection: &Selection) -> Option<NavmeshSelection> {
    if let Some(selection) = editor_selection.as_navmesh() {
        Some(selection.clone())
    } else {
        editor_selection.as_graph().map(|selection| {
            NavmeshSelection::new(selection.nodes.first().cloned().unwrap_or_default(), vec![])
        })
    }
}

impl NavmeshPanel {
    pub fn new(scene_frame: Handle<UiNode>, ctx: &mut BuildContext, sender: MessageSender) -> Self {
        let connect_edges;
        let window = WindowBuilder::new(WidgetBuilder::new().with_name("NavmeshPanel"))
            .open(false)
            .with_title(WindowTitle::text("Navmesh"))
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new().with_child(
                        StackPanelBuilder::new(WidgetBuilder::new().with_child({
                            connect_edges = ButtonBuilder::new(
                                WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                            )
                            .with_text("Connect Edges")
                            .build(ctx);
                            connect_edges
                        }))
                        .with_orientation(Orientation::Horizontal)
                        .build(ctx),
                    ),
                )
                .add_column(Column::stretch())
                .add_row(Row::strict(20.0))
                .build(ctx),
            )
            .build(ctx);

        Self {
            window,
            sender,
            connect_edges,
            scene_frame,
        }
    }

    pub fn handle_message(&mut self, message: &UiMessage, editor_selection: &Selection) {
        scope_profile!();

        if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == self.connect_edges {
                if let Some(selection) = fetch_selection(editor_selection) {
                    let vertices = selection
                        .entities()
                        .iter()
                        .filter_map(|entity| {
                            if let NavmeshEntity::Edge(v) = *entity {
                                Some(v)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>();

                    self.sender.do_command(ConnectNavmeshEdgesCommand::new(
                        selection.navmesh_node(),
                        [vertices[0], vertices[1]],
                    ));
                }
            }
        }
    }

    pub fn sync_to_model(
        &mut self,
        engine: &Engine,
        editor_selection: &Selection,
        game_scene: &GameScene,
    ) {
        let mut navmesh_selected = false;

        let graph = &engine.scenes[game_scene.scene].graph;
        if let Some(selection) = fetch_selection(editor_selection) {
            navmesh_selected = graph
                .try_get_of_type::<NavigationalMesh>(selection.navmesh_node())
                .is_some();
        }

        if navmesh_selected {
            engine
                .user_interfaces
                .first()
                .send_message(WindowMessage::open_and_align(
                    self.window,
                    MessageDirection::ToWidget,
                    self.scene_frame,
                    HorizontalAlignment::Right,
                    VerticalAlignment::Top,
                    Thickness::uniform(1.0),
                    false,
                    false,
                ));
        } else {
            engine
                .user_interfaces
                .first()
                .send_message(WindowMessage::close(
                    self.window,
                    MessageDirection::ToWidget,
                ));
        }
    }

    pub fn on_mode_changed(&mut self, ui: &UserInterface, mode: &Mode) {
        ui.send_message(WidgetMessage::enabled(
            window_content(self.window, ui),
            MessageDirection::ToWidget,
            mode.is_edit(),
        ));
    }
}

enum DragContext {
    MoveSelection {
        initial_positions: HashMap<usize, Vector3<f32>>,
    },
    EdgeDuplication {
        vertices: [Vector3<f32>; 2],
        opposite_edge: TriangleEdge,
    },
}

impl DragContext {
    pub fn is_edge_duplication(&self) -> bool {
        matches!(self, DragContext::EdgeDuplication { .. })
    }
}

pub struct EditNavmeshMode {
    move_gizmo: MoveGizmo,
    message_sender: MessageSender,
    drag_context: Option<DragContext>,
    plane_kind: PlaneKind,
}

impl EditNavmeshMode {
    pub fn new(game_scene: &GameScene, engine: &mut Engine, message_sender: MessageSender) -> Self {
        Self {
            move_gizmo: MoveGizmo::new(game_scene, engine),
            message_sender,
            drag_context: None,
            plane_kind: PlaneKind::X,
        }
    }
}

impl TypeUuidProvider for EditNavmeshMode {
    fn type_uuid() -> Uuid {
        uuid!("a8ed875d-0932-400d-b5b0-e0dcfb78c6c1")
    }
}

impl InteractionMode for EditNavmeshMode {
    fn on_left_mouse_button_down(
        &mut self,
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
        settings: &Settings,
    ) {
        let Some(game_scene) = controller.downcast_mut::<GameScene>() else {
            return;
        };

        let scene = &mut engine.scenes[game_scene.scene];
        let camera: &Camera = scene.graph[game_scene.camera_controller.camera].as_camera();
        let ray = camera.make_ray(mouse_pos, frame_size);

        let gizmo_origin = self.move_gizmo.origin;
        let editor_node = game_scene
            .camera_controller
            .pick(
                &scene.graph,
                PickingOptions {
                    cursor_pos: mouse_pos,
                    editor_only: true,
                    filter: Some(&mut |handle, _| handle != gizmo_origin),
                    ignore_back_faces: false,
                    use_picking_loop: true,
                    only_meshes: false,
                },
            )
            .map(|r| r.node)
            .unwrap_or_default();

        if let Some(selection) = fetch_selection(editor_selection) {
            let graph = &mut engine.scenes[game_scene.scene].graph;

            if let Some(plane_kind) = self.move_gizmo.handle_pick(editor_node, graph) {
                if let Some(navmesh) = graph
                    .try_get_of_type::<NavigationalMesh>(selection.navmesh_node())
                    .map(|n| n.navmesh_ref())
                {
                    let mut initial_positions = HashMap::new();
                    for (index, vertex) in navmesh.vertices().iter().enumerate() {
                        initial_positions.insert(index, *vertex);
                    }
                    self.plane_kind = plane_kind;
                    self.drag_context = Some(DragContext::MoveSelection { initial_positions });
                }
            } else if let Some(navmesh) = graph
                .try_get_of_type::<NavigationalMesh>(selection.navmesh_node())
                .map(|n| n.navmesh_ref())
            {
                let mut new_selection = if engine
                    .user_interfaces
                    .first_mut()
                    .keyboard_modifiers()
                    .shift
                {
                    selection
                } else {
                    NavmeshSelection::empty(selection.navmesh_node())
                };

                let mut picked = false;
                for (index, vertex) in navmesh.vertices().iter().enumerate() {
                    if ray
                        .sphere_intersection(vertex, settings.navmesh.vertex_radius)
                        .is_some()
                    {
                        new_selection.add(NavmeshEntity::Vertex(index));
                        picked = true;
                        break;
                    }
                }

                if !picked {
                    for triangle in navmesh.triangles().iter() {
                        for edge in &triangle.edges() {
                            let begin = navmesh.vertices()[edge.a as usize];
                            let end = navmesh.vertices()[edge.b as usize];
                            if ray
                                .cylinder_intersection(
                                    &begin,
                                    &end,
                                    settings.navmesh.vertex_radius,
                                    CylinderKind::Finite,
                                )
                                .is_some()
                            {
                                new_selection.add(NavmeshEntity::Edge(*edge));
                                break;
                            }
                        }
                    }
                }

                let new_selection = Selection::new(new_selection);

                if &new_selection != editor_selection {
                    self.message_sender
                        .do_command(ChangeSelectionCommand::new(new_selection));
                }
            }
        }
    }

    fn on_left_mouse_button_up(
        &mut self,
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        _mouse_pos: Vector2<f32>,
        _frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        let Some(game_scene) = controller.downcast_mut::<GameScene>() else {
            return;
        };

        let graph = &mut engine.scenes[game_scene.scene].graph;

        self.move_gizmo.reset_state(graph);

        if let Some(selection) = fetch_selection(editor_selection) {
            if let Some(navmesh) = graph
                .try_get_of_type::<NavigationalMesh>(selection.navmesh_node())
                .map(|n| n.navmesh_ref())
            {
                if let Some(drag_context) = self.drag_context.take() {
                    let mut commands = Vec::new();

                    match drag_context {
                        DragContext::MoveSelection { initial_positions } => {
                            for vertex in selection.unique_vertices().iter() {
                                commands.push(Command::new(MoveNavmeshVertexCommand::new(
                                    selection.navmesh_node(),
                                    *vertex,
                                    *initial_positions.get(vertex).unwrap(),
                                    navmesh.vertices()[*vertex],
                                )));
                            }
                        }
                        DragContext::EdgeDuplication {
                            vertices,
                            opposite_edge,
                        } => {
                            let va = vertices[0];
                            let vb = vertices[1];

                            commands.push(Command::new(AddNavmeshEdgeCommand::new(
                                selection.navmesh_node(),
                                (va, vb),
                                opposite_edge,
                                true,
                            )));
                        }
                    }

                    self.message_sender.do_command(CommandGroup::from(commands));
                }
            }
        }
    }

    fn on_mouse_move(
        &mut self,
        mouse_offset: Vector2<f32>,
        mouse_position: Vector2<f32>,
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        let Some(game_scene) = controller.downcast_mut::<GameScene>() else {
            return;
        };

        let graph = &mut engine.scenes[game_scene.scene].graph;

        if self.drag_context.is_none() {
            let gizmo_origin = self.move_gizmo.origin;
            let editor_node = game_scene
                .camera_controller
                .pick(
                    graph,
                    PickingOptions {
                        cursor_pos: mouse_position,
                        editor_only: true,
                        filter: Some(&mut |handle, _| handle != gizmo_origin),
                        ignore_back_faces: false,
                        use_picking_loop: true,
                        only_meshes: false,
                    },
                )
                .map(|r| r.node)
                .unwrap_or_default();
            self.move_gizmo.handle_pick(editor_node, graph);
        }

        if self.drag_context.is_none() {
            return;
        }

        let offset = self.move_gizmo.calculate_offset(
            graph,
            game_scene.camera_controller.camera,
            mouse_offset,
            mouse_position,
            frame_size,
            self.plane_kind,
        );

        if let Some(selection) = fetch_selection(editor_selection) {
            if let Some(mut navmesh) = graph
                .try_get_mut_of_type::<NavigationalMesh>(selection.navmesh_node())
                .map(|n| n.navmesh_mut())
            {
                // If we're dragging single edge it is possible to enter edge duplication mode by
                // holding Shift key. This is the main navmesh construction mode.
                if selection.entities().len() == 1 {
                    if let NavmeshEntity::Edge(edge) = selection.entities().first().unwrap() {
                        if engine
                            .user_interfaces
                            .first_mut()
                            .keyboard_modifiers()
                            .shift
                            && !self.drag_context.as_ref().unwrap().is_edge_duplication()
                        {
                            let new_begin = navmesh.vertices()[edge.a as usize];
                            let new_end = navmesh.vertices()[edge.b as usize];

                            self.drag_context = Some(DragContext::EdgeDuplication {
                                vertices: [new_begin, new_end],
                                opposite_edge: *edge,
                            });

                            // Discard selection.
                            self.message_sender.do_command(ChangeSelectionCommand::new(
                                Selection::new(NavmeshSelection::empty(selection.navmesh_node())),
                            ));
                        }
                    }
                }

                if let Some(drag_context) = self.drag_context.as_mut() {
                    match drag_context {
                        DragContext::MoveSelection { .. } => {
                            let mut ctx = navmesh.modify();
                            for &vertex in &*selection.unique_vertices() {
                                ctx.vertices_mut()[vertex] += offset;
                            }
                        }
                        DragContext::EdgeDuplication { vertices, .. } => {
                            for vertex in vertices.iter_mut() {
                                *vertex += offset;
                            }
                        }
                    }
                }
            }
        }
    }

    fn update(
        &mut self,
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        settings: &Settings,
    ) {
        let Some(game_scene) = controller.downcast_mut::<GameScene>() else {
            return;
        };

        let scene = &mut engine.scenes[game_scene.scene];
        self.move_gizmo.set_visible(&mut scene.graph, false);

        let scale = calculate_gizmo_distance_scaling(
            &scene.graph,
            game_scene.camera_controller.camera,
            self.move_gizmo.origin,
        );

        if let Some(selection) = fetch_selection(editor_selection) {
            let mut gizmo_visible = false;
            let mut gizmo_position = Default::default();

            if let Some(navmesh) = scene
                .graph
                .try_get_mut_of_type::<NavigationalMesh>(selection.navmesh_node())
                .map(|n| n.navmesh_mut())
            {
                if let Some(DragContext::EdgeDuplication {
                    vertices,
                    opposite_edge,
                }) = self.drag_context.as_ref()
                {
                    for vertex in vertices.iter() {
                        scene.drawing_context.draw_sphere(
                            *vertex,
                            10,
                            10,
                            settings.navmesh.vertex_radius,
                            Color::RED,
                        );
                    }

                    let ob = navmesh.vertices()[opposite_edge.a as usize];
                    let nb = vertices[0];
                    let oe = navmesh.vertices()[opposite_edge.b as usize];
                    let ne = vertices[1];

                    scene.drawing_context.add_line(fyrox::scene::debug::Line {
                        begin: nb,
                        end: ne,
                        color: Color::RED,
                    });

                    for &(begin, end) in &[(ob, oe), (ob, nb), (nb, oe), (oe, ne)] {
                        scene.drawing_context.add_line(fyrox::scene::debug::Line {
                            begin,
                            end,
                            color: Color::GREEN,
                        });
                    }

                    gizmo_visible = true;
                    gizmo_position = (nb + ne).scale(0.5);
                }

                if let Some(first) = selection.first() {
                    gizmo_visible = true;
                    gizmo_position = match *first {
                        NavmeshEntity::Vertex(v) => navmesh.vertices()[v],
                        NavmeshEntity::Edge(edge) => {
                            let a = navmesh.vertices()[edge.a as usize];
                            let b = navmesh.vertices()[edge.b as usize];
                            (a + b).scale(0.5)
                        }
                    };
                }
            }

            self.move_gizmo.set_visible(&mut scene.graph, gizmo_visible);
            self.move_gizmo
                .transform(&mut scene.graph)
                .set_scale(scale)
                .set_position(gizmo_position);
        }
    }

    fn deactivate(&mut self, controller: &dyn SceneController, engine: &mut Engine) {
        let Some(game_scene) = controller.downcast_ref::<GameScene>() else {
            return;
        };

        let scene = &mut engine.scenes[game_scene.scene];
        self.move_gizmo.set_visible(&mut scene.graph, false);
    }

    fn on_key_down(
        &mut self,
        key: KeyCode,
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
    ) -> bool {
        let Some(game_scene) = controller.downcast_mut::<GameScene>() else {
            return false;
        };

        let scene = &mut engine.scenes[game_scene.scene];

        if let Some(selection) = fetch_selection(editor_selection) {
            return match key {
                KeyCode::Delete => {
                    if scene
                        .graph
                        .try_get_of_type::<NavigationalMesh>(selection.navmesh_node())
                        .map(|n| n.navmesh_ref())
                        .is_some()
                        && !selection.is_empty()
                    {
                        let mut commands = Vec::new();

                        for vertex in selection.unique_vertices().iter().rev().cloned() {
                            commands.push(Command::new(DeleteNavmeshVertexCommand::new(
                                selection.navmesh_node(),
                                vertex,
                            )));
                        }

                        commands.push(Command::new(ChangeSelectionCommand::new(Selection::new(
                            NavmeshSelection::empty(selection.navmesh_node()),
                        ))));

                        self.message_sender.do_command(CommandGroup::from(commands));
                    }

                    true
                }
                KeyCode::KeyA
                    if engine
                        .user_interfaces
                        .first_mut()
                        .keyboard_modifiers()
                        .control =>
                {
                    if let Some(navmesh) = scene
                        .graph
                        .try_get_of_type::<NavigationalMesh>(selection.navmesh_node())
                        .map(|n| n.navmesh_ref())
                    {
                        let selection = NavmeshSelection::new(
                            selection.navmesh_node(),
                            navmesh
                                .vertices()
                                .iter()
                                .enumerate()
                                .map(|(handle, _)| NavmeshEntity::Vertex(handle))
                                .collect(),
                        );

                        self.message_sender
                            .do_command(ChangeSelectionCommand::new(Selection::new(selection)));
                    }

                    true
                }
                _ => false,
            };
        } else {
            false
        }
    }

    fn make_button(&mut self, ctx: &mut BuildContext, selected: bool) -> Handle<UiNode> {
        let navmesh_mode_tooltip =
            "Edit Navmesh\n\nNavmesh edit mode allows you to modify selected \
        navigational mesh.";

        make_interaction_mode_button(
            ctx,
            include_bytes!("../../../resources/navmesh.png"),
            navmesh_mode_tooltip,
            selected,
        )
    }

    fn uuid(&self) -> Uuid {
        Self::type_uuid()
    }
}
