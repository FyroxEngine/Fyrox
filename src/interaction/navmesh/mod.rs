use crate::{
    gui::{BuildContext, UiMessage, UiNode},
    interaction::{
        navmesh::{
            data_model::{Navmesh, NavmeshEntity, NavmeshVertex},
            selection::NavmeshSelection,
        },
        InteractionModeTrait, MoveGizmo,
    },
    scene::{
        AddNavmeshCommand, CommandGroup, DeleteNavmeshCommand, EditorScene,
        MoveNavmeshVertexCommand, SceneCommand,
    },
    GameEngine, Message,
};
use rg3d::sound::math::ray::CylinderKind;
use rg3d::{
    core::{algebra::Vector3, color::Color, pool::Handle},
    gui::{
        border::BorderBuilder,
        button::ButtonBuilder,
        check_box::CheckBoxBuilder,
        decorator::DecoratorBuilder,
        grid::{Column, GridBuilder, Row},
        list_view::ListViewBuilder,
        message::{ButtonMessage, ListViewMessage, MessageDirection, UiMessageData, WidgetMessage},
        text::TextBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        Thickness, VerticalAlignment,
    },
    physics::ncollide::na::Vector2,
    scene::{camera::Camera, node::Node},
};
use std::{collections::HashMap, rc::Rc, sync::mpsc::Sender};

pub mod data_model;
mod selection;

const VERTEX_RADIUS: f32 = 0.2;

pub struct NavmeshPanel {
    pub window: Handle<UiNode>,
    navmeshes: Handle<UiNode>,
    add: Handle<UiNode>,
    remove: Handle<UiNode>,
    sender: Sender<Message>,
    selected: Handle<Navmesh>,
}

impl NavmeshPanel {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let add;
        let remove;
        let navmeshes;
        let window = WindowBuilder::new(WidgetBuilder::new())
            .with_title(WindowTitle::text("Navmesh"))
            .with_content(
                GridBuilder::new(
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
                                        .with_vertical_alignment(VerticalAlignment::Center),
                                )
                                .with_text("Show")
                                .build(ctx),
                            )
                            .checked(Some(true))
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
            selected: Default::default(),
        }
    }

    pub fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
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
        match message.data() {
            UiMessageData::Button(msg) => {
                if let ButtonMessage::Click = msg {
                    if message.destination() == self.add {
                        self.sender
                            .send(Message::DoSceneCommand(SceneCommand::AddNavmesh(
                                AddNavmeshCommand::new(Navmesh::new()),
                            )))
                            .unwrap();
                    } else if message.destination() == self.remove {
                        if editor_scene.navmeshes.is_valid_handle(self.selected) {
                            self.sender
                                .send(Message::DoSceneCommand(SceneCommand::DeleteNavmesh(
                                    DeleteNavmeshCommand::new(self.selected),
                                )))
                                .unwrap();
                        }
                    }
                }
            }
            UiMessageData::ListView(msg) => {
                if let ListViewMessage::SelectionChanged(selection) = msg {
                    if message.destination() == self.navmeshes {
                        if let &Some(selection) = selection {
                            let navmeshes = engine.user_interface.node(self.navmeshes);
                            let item = navmeshes.as_list_view().items()[selection];
                            self.selected = *engine
                                .user_interface
                                .node(item)
                                .user_data_ref::<Handle<Navmesh>>();
                            edit_mode.navmesh = self.selected;

                            engine.user_interface.send_message(WidgetMessage::enabled(
                                self.remove,
                                MessageDirection::ToWidget,
                                editor_scene.navmeshes.is_valid_handle(self.selected),
                            ));
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

pub struct EditNavmeshMode {
    navmesh: Handle<Navmesh>,
    move_gizmo: MoveGizmo,
    interacting: bool,
    message_sender: Sender<Message>,
    selection: NavmeshSelection,
    initial_positions: HashMap<Handle<NavmeshVertex>, Vector3<f32>>,
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
            interacting: false,
            message_sender,
            selection: Default::default(),
            initial_positions: Default::default(),
        }
    }
}

impl InteractionModeTrait for EditNavmeshMode {
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
            let camera: &Camera = &scene.graph[editor_scene.camera_controller.camera].as_camera();
            let ray = camera.make_ray(mouse_pos, frame_size);

            let camera = editor_scene.camera_controller.camera;
            let camera_pivot = editor_scene.camera_controller.pivot;
            let editor_node = editor_scene.camera_controller.pick(
                mouse_pos,
                &mut scene.graph,
                editor_scene.root,
                frame_size,
                true,
                |handle, _| handle != camera && handle != camera_pivot,
            );

            self.interacting = self
                .move_gizmo
                .handle_pick(editor_node, editor_scene, engine);

            if !self.interacting {
                if !engine.user_interface.keyboard_modifiers().shift {
                    self.selection.clear();
                }
                for (handle, vertex) in navmesh.vertices.pair_iter() {
                    if ray
                        .sphere_intersection(&vertex.position, VERTEX_RADIUS)
                        .is_some()
                    {
                        self.selection.add(NavmeshEntity::Vertex(handle));
                    }
                }

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
                            self.selection.add(NavmeshEntity::Edge(*edge));
                        }
                    }
                }
            } else {
                self.initial_positions.clear();
                for (handle, vertex) in navmesh.vertices.pair_iter() {
                    self.initial_positions.insert(handle, vertex.position);
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
            let navmesh = &editor_scene.navmeshes[self.navmesh];
            if self.interacting {
                let mut commands = Vec::new();
                for vertex in self.selection.unique_vertices().iter() {
                    commands.push(SceneCommand::MoveNavmeshVertex(
                        MoveNavmeshVertexCommand::new(
                            self.navmesh,
                            *vertex,
                            *self.initial_positions.get(vertex).unwrap(),
                            navmesh.vertices[*vertex].position,
                        ),
                    ));
                }

                self.message_sender
                    .send(Message::DoSceneCommand(SceneCommand::CommandGroup(
                        CommandGroup::from(commands),
                    )))
                    .unwrap();

                self.interacting = false;
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
    ) {
        if editor_scene.navmeshes.is_valid_handle(self.navmesh) {
            if self.interacting {
                let offset = self.move_gizmo.calculate_offset(
                    editor_scene,
                    camera,
                    mouse_offset,
                    mouse_position,
                    engine,
                    frame_size,
                );

                let navmesh = &mut editor_scene.navmeshes[self.navmesh];
                for &vertex in self.selection.unique_vertices() {
                    navmesh.vertices[vertex].position += offset;
                }
            }
        }
    }

    fn update(
        &mut self,
        editor_scene: &EditorScene,
        _camera: Handle<Node>,
        engine: &mut GameEngine,
    ) {
        let scene = &mut engine.scenes[editor_scene.scene];

        if editor_scene.navmeshes.is_valid_handle(self.navmesh) {
            let navmesh = &editor_scene.navmeshes[self.navmesh];

            for (handle, vertex) in navmesh.vertices.pair_iter() {
                scene.drawing_context.draw_sphere(
                    vertex.position,
                    10,
                    10,
                    VERTEX_RADIUS,
                    if self.selection.unique_vertices().contains(&handle) {
                        Color::RED
                    } else {
                        Color::GREEN
                    },
                );
            }

            for triangle in navmesh.triangles.iter() {
                for edge in &triangle.edges() {
                    scene.drawing_context.add_line(rg3d::scene::Line {
                        begin: navmesh.vertices[edge.begin].position,
                        end: navmesh.vertices[edge.end].position,
                        color: if self.selection.contains_edge(*edge) {
                            Color::RED
                        } else {
                            Color::GREEN
                        },
                    });
                }
            }

            if let Some(first) = self.selection.first() {
                self.move_gizmo.set_visible(&mut scene.graph, true);

                let gizmo_position = match first {
                    &NavmeshEntity::Vertex(v) => navmesh.vertices[v].position,
                    &NavmeshEntity::Edge(edge) => {
                        let a = navmesh.vertices[edge.begin].position;
                        let b = navmesh.vertices[edge.end].position;
                        (a + b).scale(0.5)
                    }
                };

                self.move_gizmo
                    .transform(&mut scene.graph)
                    .set_position(gizmo_position);
            }
        } else {
            self.move_gizmo.set_visible(&mut scene.graph, false);
        }
    }

    fn deactivate(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        let scene = &mut engine.scenes[editor_scene.scene];
        self.move_gizmo.set_visible(&mut scene.graph, false);
    }
}
