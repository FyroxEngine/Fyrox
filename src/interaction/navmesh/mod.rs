use crate::interaction::gizmo::move_gizmo::MoveGizmo;
use crate::interaction::plane::PlaneKind;
use crate::scene::commands::SceneCommand;
use crate::settings::Settings;
use crate::{
    interaction::{
        calculate_gizmo_distance_scaling,
        navmesh::{
            data_model::{Navmesh, NavmeshEdge, NavmeshEntity, NavmeshVertex},
            selection::NavmeshSelection,
        },
        InteractionMode,
    },
    scene::{
        commands::{
            navmesh::{
                AddNavmeshCommand, AddNavmeshEdgeCommand, ConnectNavmeshEdgesCommand,
                DeleteNavmeshCommand, DeleteNavmeshVertexCommand, MoveNavmeshVertexCommand,
            },
            ChangeSelectionCommand, CommandGroup,
        },
        EditorScene, Selection,
    },
    send_sync_message, GameEngine, Message, MSG_SYNC_FLAG,
};
use rg3d::gui::list_view::ListView;
use rg3d::gui::message::UiMessage;
use rg3d::gui::{BuildContext, UiNode};
use rg3d::{
    core::{
        algebra::{Vector2, Vector3},
        color::Color,
        math::ray::CylinderKind,
        pool::Handle,
        scope_profile,
    },
    gui::{
        border::BorderBuilder,
        button::ButtonBuilder,
        check_box::CheckBoxBuilder,
        decorator::DecoratorBuilder,
        grid::{Column, GridBuilder, Row},
        list_view::ListViewBuilder,
        message::{
            ButtonMessage, KeyCode, ListViewMessage, MessageDirection, UiMessageData, WidgetMessage,
        },
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        Orientation, Thickness, VerticalAlignment,
    },
    scene::{camera::Camera, node::Node},
};
use std::{collections::HashMap, rc::Rc, sync::mpsc::Sender};

pub mod data_model;
pub mod selection;

const VERTEX_RADIUS: f32 = 0.2;

pub struct NavmeshPanel {
    pub window: Handle<UiNode>,
    navmeshes: Handle<UiNode>,
    add: Handle<UiNode>,
    connect: Handle<UiNode>,
    remove: Handle<UiNode>,
    sender: Sender<Message>,
    selected: Handle<Navmesh>,
}

impl NavmeshPanel {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let add;
        let remove;
        let navmeshes;
        let connect;
        let window = WindowBuilder::new(WidgetBuilder::new())
            .with_title(WindowTitle::text("Navmesh"))
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(
                            StackPanelBuilder::new(
                                WidgetBuilder::new()
                                    .with_child(
                                        CheckBoxBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0))
                                                .on_row(0),
                                        )
                                        .with_content(
                                            TextBuilder::new(
                                                WidgetBuilder::new()
                                                    .with_margin(Thickness::uniform(1.0))
                                                    .with_vertical_alignment(
                                                        VerticalAlignment::Center,
                                                    ),
                                            )
                                            .with_text("Show")
                                            .build(ctx),
                                        )
                                        .checked(Some(true))
                                        .build(ctx),
                                    )
                                    .with_child({
                                        connect = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_text("Connect")
                                        .build(ctx);
                                        connect
                                    }),
                            )
                            .with_orientation(Orientation::Horizontal)
                            .build(ctx),
                        )
                        .with_child({
                            navmeshes =
                                ListViewBuilder::new(WidgetBuilder::new().on_row(1)).build(ctx);
                            navmeshes
                        })
                        .with_child(
                            GridBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(2)
                                    .with_child({
                                        add = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0))
                                                .on_column(0),
                                        )
                                        .with_text("Add")
                                        .build(ctx);
                                        add
                                    })
                                    .with_child({
                                        remove = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0))
                                                .on_column(1),
                                        )
                                        .with_text("Remove")
                                        .build(ctx);
                                        remove
                                    }),
                            )
                            .add_row(Row::stretch())
                            .add_column(Column::stretch())
                            .add_column(Column::stretch())
                            .build(ctx),
                        ),
                )
                .add_column(Column::stretch())
                .add_row(Row::strict(20.0))
                .add_row(Row::stretch())
                .add_row(Row::strict(24.0))
                .build(ctx),
            )
            .build(ctx);

        Self {
            window,
            sender,
            add,
            remove,
            navmeshes,
            connect,
            selected: Default::default(),
        }
    }

    pub fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        scope_profile!();

        let ctx = &mut engine.user_interface.build_ctx();

        let items = editor_scene
            .navmeshes
            .pair_iter()
            .enumerate()
            .map(|(i, (handle, _))| {
                DecoratorBuilder::new(BorderBuilder::new(
                    WidgetBuilder::new()
                        .with_height(22.0)
                        .with_user_data(Rc::new(handle))
                        .with_child(
                            TextBuilder::new(WidgetBuilder::new())
                                .with_text(format!("Navmesh {}", i))
                                .build(ctx),
                        ),
                ))
                .build(ctx)
            })
            .collect::<Vec<_>>();

        let ui = &mut engine.user_interface;

        let new_selection = if let Selection::Navmesh(selection) = &editor_scene.selection {
            let selected_vertex_count = selection
                .entities()
                .iter()
                .filter(|entity| matches!(entity, NavmeshEntity::Edge(_)))
                .count();

            send_sync_message(
                ui,
                WidgetMessage::enabled(
                    self.connect,
                    MessageDirection::ToWidget,
                    selected_vertex_count == 2,
                ),
            );

            editor_scene
                .navmeshes
                .pair_iter()
                .position(|(i, _)| i == selection.navmesh())
        } else {
            send_sync_message(
                ui,
                WidgetMessage::enabled(self.connect, MessageDirection::ToWidget, false),
            );

            self.selected = Handle::NONE;

            None
        };

        let mut message =
            ListViewMessage::selection(self.navmeshes, MessageDirection::ToWidget, new_selection);

        message.flags = MSG_SYNC_FLAG;
        engine.user_interface.send_message(message);

        engine.user_interface.send_message(ListViewMessage::items(
            self.navmeshes,
            MessageDirection::ToWidget,
            items,
        ));

        engine.user_interface.send_message(WidgetMessage::enabled(
            self.remove,
            MessageDirection::ToWidget,
            editor_scene.navmeshes.is_valid_handle(self.selected),
        ));
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        editor_scene: &EditorScene,
        engine: &GameEngine,
        edit_mode: &mut EditNavmeshMode,
    ) {
        scope_profile!();

        match message.data() {
            UiMessageData::Button(ButtonMessage::Click) => {
                if message.destination() == self.add {
                    self.sender
                        .send(Message::do_scene_command(AddNavmeshCommand::new(
                            Navmesh::new(),
                        )))
                        .unwrap();
                } else if message.destination() == self.remove {
                    if editor_scene.navmeshes.is_valid_handle(self.selected) {
                        self.sender
                            .send(Message::do_scene_command(DeleteNavmeshCommand::new(
                                self.selected,
                            )))
                            .unwrap();
                    }
                } else if message.destination() == self.connect {
                    if let Selection::Navmesh(selection) = &editor_scene.selection {
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

                        self.sender
                            .send(Message::do_scene_command(ConnectNavmeshEdgesCommand::new(
                                self.selected,
                                [vertices[0], vertices[1]],
                            )))
                            .unwrap();
                    }
                }
            }
            UiMessageData::ListView(ListViewMessage::SelectionChanged(selection)) => {
                if message.destination() == self.navmeshes
                    && message.direction() == MessageDirection::FromWidget
                {
                    let new_selection = if let Some(selection) = *selection {
                        let navmeshes = engine.user_interface.node(self.navmeshes);
                        let item = navmeshes.cast::<ListView>().unwrap().items()[selection];
                        *engine
                            .user_interface
                            .node(item)
                            .user_data_ref::<Handle<Navmesh>>()
                            .unwrap()
                    } else {
                        Default::default()
                    };

                    if self.selected != new_selection {
                        self.selected = new_selection;
                        edit_mode.navmesh = self.selected;

                        engine.user_interface.send_message(WidgetMessage::enabled(
                            self.remove,
                            MessageDirection::ToWidget,
                            editor_scene.navmeshes.is_valid_handle(self.selected),
                        ));

                        if !message.has_flags(MSG_SYNC_FLAG) {
                            let new_selection =
                                Selection::Navmesh(NavmeshSelection::empty(self.selected));

                            if new_selection != editor_scene.selection {
                                self.sender
                                    .send(Message::do_scene_command(ChangeSelectionCommand::new(
                                        new_selection,
                                        editor_scene.selection.clone(),
                                    )))
                                    .unwrap();
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

enum DragContext {
    MoveSelection {
        initial_positions: HashMap<Handle<NavmeshVertex>, Vector3<f32>>,
    },
    EdgeDuplication {
        vertices: [NavmeshVertex; 2],
        opposite_edge: NavmeshEdge,
    },
}

impl DragContext {
    pub fn is_edge_duplication(&self) -> bool {
        matches!(self, DragContext::EdgeDuplication { .. })
    }
}

pub struct EditNavmeshMode {
    navmesh: Handle<Navmesh>,
    move_gizmo: MoveGizmo,
    message_sender: Sender<Message>,
    drag_context: Option<DragContext>,
    plane_kind: PlaneKind,
}

impl EditNavmeshMode {
    pub fn new(
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
        message_sender: Sender<Message>,
    ) -> Self {
        Self {
            navmesh: Default::default(),
            move_gizmo: MoveGizmo::new(editor_scene, engine),
            message_sender,
            drag_context: None,
            plane_kind: PlaneKind::X,
        }
    }
}

impl InteractionMode for EditNavmeshMode {
    fn on_left_mouse_button_down(
        &mut self,
        editor_scene: &mut EditorScene,
        engine: &mut GameEngine,
        mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
    ) {
        if editor_scene.navmeshes.is_valid_handle(self.navmesh) {
            let navmesh = &editor_scene.navmeshes[self.navmesh];
            let scene = &mut engine.scenes[editor_scene.scene];
            let camera: &Camera = scene.graph[editor_scene.camera_controller.camera].as_camera();
            let ray = camera.make_ray(mouse_pos, frame_size);

            let camera = editor_scene.camera_controller.camera;
            let camera_pivot = editor_scene.camera_controller.pivot;
            let gizmo_origin = self.move_gizmo.origin;
            let editor_node = editor_scene
                .camera_controller
                .pick(
                    mouse_pos,
                    &scene.graph,
                    editor_scene.root,
                    frame_size,
                    true,
                    |handle, _| {
                        handle != camera && handle != camera_pivot && handle != gizmo_origin
                    },
                )
                .map(|r| r.node)
                .unwrap_or_default();

            let graph = &mut engine.scenes[editor_scene.scene].graph;
            if let Some(plane_kind) = self.move_gizmo.handle_pick(editor_node, graph) {
                let mut initial_positions = HashMap::new();
                for (handle, vertex) in navmesh.vertices.pair_iter() {
                    initial_positions.insert(handle, vertex.position);
                }
                self.plane_kind = plane_kind;
                self.drag_context = Some(DragContext::MoveSelection { initial_positions });
            } else {
                let mut new_selection = if engine.user_interface.keyboard_modifiers().shift {
                    if let Selection::Navmesh(navmesh_selection) = &editor_scene.selection {
                        navmesh_selection.clone()
                    } else {
                        NavmeshSelection::empty(self.navmesh)
                    }
                } else {
                    NavmeshSelection::empty(self.navmesh)
                };

                let mut picked = false;
                for (handle, vertex) in navmesh.vertices.pair_iter() {
                    if ray
                        .sphere_intersection(&vertex.position, VERTEX_RADIUS)
                        .is_some()
                    {
                        new_selection.add(NavmeshEntity::Vertex(handle));
                        picked = true;
                        break;
                    }
                }

                if !picked {
                    for triangle in navmesh.triangles.iter() {
                        for edge in &triangle.edges() {
                            let begin = navmesh.vertices[edge.begin].position;
                            let end = navmesh.vertices[edge.end].position;
                            if ray
                                .cylinder_intersection(
                                    &begin,
                                    &end,
                                    VERTEX_RADIUS,
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

                let new_selection = Selection::Navmesh(new_selection);

                if new_selection != editor_scene.selection {
                    self.message_sender
                        .send(Message::do_scene_command(ChangeSelectionCommand::new(
                            new_selection,
                            editor_scene.selection.clone(),
                        )))
                        .unwrap();
                }
            }
        }
    }

    fn on_left_mouse_button_up(
        &mut self,
        editor_scene: &mut EditorScene,
        _engine: &mut GameEngine,
        _mouse_pos: Vector2<f32>,
        _frame_size: Vector2<f32>,
    ) {
        if editor_scene.navmeshes.is_valid_handle(self.navmesh) {
            let navmesh = &mut editor_scene.navmeshes[self.navmesh];
            if let Some(drag_context) = self.drag_context.take() {
                let mut commands = Vec::new();

                match drag_context {
                    DragContext::MoveSelection { initial_positions } => {
                        if let Selection::Navmesh(navmesh_selection) = &mut editor_scene.selection {
                            for vertex in navmesh_selection.unique_vertices().iter() {
                                commands.push(SceneCommand::new(MoveNavmeshVertexCommand::new(
                                    self.navmesh,
                                    *vertex,
                                    *initial_positions.get(vertex).unwrap(),
                                    navmesh.vertices[*vertex].position,
                                )));
                            }
                        }
                    }
                    DragContext::EdgeDuplication {
                        vertices,
                        opposite_edge,
                    } => {
                        let va = vertices[0].clone();
                        let vb = vertices[1].clone();

                        commands.push(SceneCommand::new(AddNavmeshEdgeCommand::new(
                            self.navmesh,
                            (va, vb),
                            opposite_edge,
                            true,
                        )));
                    }
                }

                self.message_sender
                    .send(Message::do_scene_command(CommandGroup::from(commands)))
                    .unwrap();
            }
        }
    }

    fn on_mouse_move(
        &mut self,
        mouse_offset: Vector2<f32>,
        mouse_position: Vector2<f32>,
        camera: Handle<Node>,
        editor_scene: &mut EditorScene,
        engine: &mut GameEngine,
        frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        if editor_scene.navmeshes.is_valid_handle(self.navmesh) && self.drag_context.is_some() {
            let offset = self.move_gizmo.calculate_offset(
                editor_scene,
                camera,
                mouse_offset,
                mouse_position,
                engine,
                frame_size,
                self.plane_kind,
            );

            let navmesh = &mut editor_scene.navmeshes[self.navmesh];

            // If we're dragging single edge it is possible to enter edge duplication mode by
            // holding Shift key. This is the main navmesh construction mode.
            if let Selection::Navmesh(navmesh_selection) = &editor_scene.selection {
                if navmesh_selection.entities().len() == 1 {
                    if let NavmeshEntity::Edge(edge) = navmesh_selection.entities().first().unwrap()
                    {
                        if engine.user_interface.keyboard_modifiers().shift
                            && !self.drag_context.as_ref().unwrap().is_edge_duplication()
                        {
                            let new_begin = navmesh.vertices[edge.begin].clone();
                            let new_end = navmesh.vertices[edge.end].clone();

                            self.drag_context = Some(DragContext::EdgeDuplication {
                                vertices: [new_begin, new_end],
                                opposite_edge: *edge,
                            });

                            // Discard selection.
                            self.message_sender
                                .send(Message::do_scene_command(ChangeSelectionCommand::new(
                                    Selection::Navmesh(NavmeshSelection::empty(self.navmesh)),
                                    editor_scene.selection.clone(),
                                )))
                                .unwrap();
                        }
                    }
                }
            }

            if let Some(drag_context) = self.drag_context.as_mut() {
                match drag_context {
                    DragContext::MoveSelection { .. } => {
                        if let Selection::Navmesh(navmesh_selection) = &mut editor_scene.selection {
                            for &vertex in navmesh_selection.unique_vertices() {
                                navmesh.vertices[vertex].position += offset;
                            }
                        }
                    }
                    DragContext::EdgeDuplication { vertices, .. } => {
                        for vertex in vertices.iter_mut() {
                            vertex.position += offset;
                        }
                    }
                }
            }
        }
    }

    fn update(
        &mut self,
        editor_scene: &mut EditorScene,
        camera: Handle<Node>,
        engine: &mut GameEngine,
    ) {
        let scene = &mut engine.scenes[editor_scene.scene];
        self.move_gizmo.set_visible(&mut scene.graph, false);

        let scale = calculate_gizmo_distance_scaling(&scene.graph, camera, self.move_gizmo.origin);

        if editor_scene.navmeshes.is_valid_handle(self.navmesh) {
            let navmesh = &editor_scene.navmeshes[self.navmesh];

            if let Selection::Navmesh(navmesh_selection) = &mut editor_scene.selection {
                for (handle, vertex) in navmesh.vertices.pair_iter() {
                    scene.drawing_context.draw_sphere(
                        vertex.position,
                        10,
                        10,
                        VERTEX_RADIUS,
                        if navmesh_selection.unique_vertices().contains(&handle) {
                            Color::RED
                        } else {
                            Color::GREEN
                        },
                    );
                }

                for triangle in navmesh.triangles.iter() {
                    for edge in &triangle.edges() {
                        scene.drawing_context.add_line(rg3d::scene::debug::Line {
                            begin: navmesh.vertices[edge.begin].position,
                            end: navmesh.vertices[edge.end].position,
                            color: if navmesh_selection.contains_edge(*edge) {
                                Color::RED
                            } else {
                                Color::GREEN
                            },
                        });
                    }
                }
            }

            if let Some(DragContext::EdgeDuplication {
                vertices,
                opposite_edge,
            }) = self.drag_context.as_ref()
            {
                for vertex in vertices.iter() {
                    scene.drawing_context.draw_sphere(
                        vertex.position,
                        10,
                        10,
                        VERTEX_RADIUS,
                        Color::RED,
                    );
                }

                let ob = navmesh.vertices[opposite_edge.begin].position;
                let nb = vertices[0].position;
                let oe = navmesh.vertices[opposite_edge.end].position;
                let ne = vertices[1].position;

                scene.drawing_context.add_line(rg3d::scene::debug::Line {
                    begin: nb,
                    end: ne,
                    color: Color::RED,
                });

                for &(begin, end) in &[(ob, oe), (ob, nb), (nb, oe), (oe, ne)] {
                    scene.drawing_context.add_line(rg3d::scene::debug::Line {
                        begin,
                        end,
                        color: Color::GREEN,
                    });
                }

                self.move_gizmo.set_visible(&mut scene.graph, true);
                self.move_gizmo
                    .transform(&mut scene.graph)
                    .set_scale(scale)
                    .set_position((nb + ne).scale(0.5));
            }

            if let Selection::Navmesh(navmesh_selection) = &editor_scene.selection {
                if let Some(first) = navmesh_selection.first() {
                    self.move_gizmo.set_visible(&mut scene.graph, true);

                    let gizmo_position = match *first {
                        NavmeshEntity::Vertex(v) => navmesh.vertices[v].position,
                        NavmeshEntity::Edge(edge) => {
                            let a = navmesh.vertices[edge.begin].position;
                            let b = navmesh.vertices[edge.end].position;
                            (a + b).scale(0.5)
                        }
                    };

                    self.move_gizmo
                        .transform(&mut scene.graph)
                        .set_scale(scale)
                        .set_position(gizmo_position);
                }
            }
        }
    }

    fn deactivate(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        let scene = &mut engine.scenes[editor_scene.scene];
        self.move_gizmo.set_visible(&mut scene.graph, false);
    }

    fn on_key_down(
        &mut self,
        key: KeyCode,
        editor_scene: &mut EditorScene,
        engine: &mut GameEngine,
    ) {
        match key {
            KeyCode::Delete => {
                if editor_scene.navmeshes.is_valid_handle(self.navmesh) {
                    if let Selection::Navmesh(navmesh_selection) = &mut editor_scene.selection {
                        if !navmesh_selection.is_empty() {
                            let mut commands = Vec::new();

                            for &vertex in navmesh_selection.unique_vertices() {
                                commands.push(SceneCommand::new(DeleteNavmeshVertexCommand::new(
                                    self.navmesh,
                                    vertex,
                                )));
                            }

                            commands.push(SceneCommand::new(ChangeSelectionCommand::new(
                                Selection::Navmesh(NavmeshSelection::empty(self.navmesh)),
                                editor_scene.selection.clone(),
                            )));

                            self.message_sender
                                .send(Message::do_scene_command(CommandGroup::from(commands)))
                                .unwrap();
                        }
                    }
                }
            }
            KeyCode::A if engine.user_interface.keyboard_modifiers().control => {
                if editor_scene.navmeshes.is_valid_handle(self.navmesh) {
                    let navmesh = &editor_scene.navmeshes[self.navmesh];

                    let selection = NavmeshSelection::new(
                        self.navmesh,
                        navmesh
                            .vertices
                            .pair_iter()
                            .map(|(handle, _)| NavmeshEntity::Vertex(handle))
                            .collect(),
                    );

                    self.message_sender
                        .send(Message::do_scene_command(ChangeSelectionCommand::new(
                            Selection::Navmesh(selection),
                            editor_scene.selection.clone(),
                        )))
                        .unwrap();
                }
            }
            _ => {}
        }
    }
}
