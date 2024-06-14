use crate::{
    command::{Command, CommandGroup},
    fyrox::{
        core::pool::Handle,
        engine::Engine,
        graph::SceneGraph,
        gui::{
            button::{ButtonBuilder, ButtonMessage},
            message::{MessageDirection, UiMessage},
            stack_panel::StackPanelBuilder,
            utils::make_simple_tooltip,
            widget::WidgetBuilder,
            window::{WindowBuilder, WindowMessage, WindowTitle},
            BuildContext, HorizontalAlignment, Thickness, UiNode, VerticalAlignment,
        },
        scene::{
            base::BaseBuilder,
            collider::{ColliderBuilder, ColliderShape, ConvexPolyhedronShape, GeometrySource},
            mesh::{
                surface::{SurfaceBuilder, SurfaceResource},
                Mesh, MeshBuilder,
            },
            node::Node,
            rigidbody::{RigidBody, RigidBodyBuilder, RigidBodyType},
            Scene,
        },
    },
    message::MessageSender,
    preview::PreviewPanel,
    scene::{
        commands::graph::{AddNodeCommand, LinkNodesCommand},
        GameScene, Selection,
    },
    world::graph::selection::GraphSelection,
    Message,
};

pub struct MeshControlPanel {
    scene_viewer_frame: Handle<UiNode>,
    pub window: Handle<UiNode>,
    create_trimesh_collider: Handle<UiNode>,
    create_convex_collider: Handle<UiNode>,
    create_trimesh_rigid_body: Handle<UiNode>,
    add_convex_collider: Handle<UiNode>,
    add_trimesh_collider: Handle<UiNode>,
}

fn make_button(text: &str, tooltip: &str, ctx: &mut BuildContext) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::uniform(1.0))
            .with_tooltip(make_simple_tooltip(ctx, tooltip)),
    )
    .with_text(text)
    .build(ctx)
}

fn meshes_iter<'a>(
    selection: &'a GraphSelection,
    scene: &'a Scene,
) -> impl Iterator<Item = (Handle<Node>, &'a Mesh)> + 'a {
    selection.nodes.iter().filter_map(|handle| {
        scene
            .graph
            .try_get_of_type::<Mesh>(*handle)
            .map(|mesh| (*handle, mesh))
    })
}

impl MeshControlPanel {
    pub fn new(scene_viewer_frame: Handle<UiNode>, ctx: &mut BuildContext) -> Self {
        let create_trimesh_collider = make_button(
            "Create Trimesh Collider",
            "Creates a new trimesh collider and attaches it to the selected mesh(es)",
            ctx,
        );
        let create_convex_collider = make_button(
            "Create Convex Collider",
            "Creates a new convex (polyhedron) collider and attaches it to the selected mesh(es).",
            ctx,
        );
        let create_trimesh_rigid_body = make_button(
            "Create Trimesh Rigid Body",
            "Creates a new static rigid body with trimesh collider and attaches the selected \
            mesh(es) to it.",
            ctx,
        );
        let add_convex_collider = make_button(
            "Add Convex Collider",
            "Creates a new convex (polyhedron) collider and attaches it to an ancestor rigid \
            body. This option could be useful if you have multiple meshes and want to put them into \
            a single rigid body.",
            ctx,
        );
        let add_trimesh_collider = make_button(
            "Add Trimesh Collider",
            "Creates a new trimesh collider and attaches it to an ancestor rigid body. This \
            option could be useful if you have multiple meshes and want to put them into a single \
            rigid body.",
            ctx,
        );
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(210.0).with_height(200.0))
            .open(false)
            .with_title(WindowTitle::text("Mesh Control Panel"))
            .with_content(
                StackPanelBuilder::new(
                    WidgetBuilder::new()
                        .with_child(create_trimesh_collider)
                        .with_child(create_convex_collider)
                        .with_child(create_trimesh_rigid_body)
                        .with_child(add_convex_collider)
                        .with_child(add_trimesh_collider),
                )
                .build(ctx),
            )
            .build(ctx);

        Self {
            scene_viewer_frame,
            window,
            create_trimesh_collider,
            create_convex_collider,
            create_trimesh_rigid_body,
            add_convex_collider,
            add_trimesh_collider,
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_selection: &Selection,
        game_scene: &mut GameScene,
        engine: &mut Engine,
        sender: &MessageSender,
    ) {
        let Some(selection) = editor_selection.as_graph() else {
            return;
        };

        let scene = &engine.scenes[game_scene.scene];

        let mut commands = Vec::new();

        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.create_trimesh_collider {
                for (mesh_handle, _) in meshes_iter(selection, scene) {
                    let collider =
                        ColliderBuilder::new(BaseBuilder::new().with_name("TrimeshCollider"))
                            .with_shape(ColliderShape::trimesh(vec![GeometrySource(mesh_handle)]))
                            .build_node();
                    commands.push(Command::new(AddNodeCommand::new(
                        collider,
                        mesh_handle,
                        false,
                    )))
                }
            } else if message.destination() == self.create_convex_collider {
                for (mesh_handle, _) in meshes_iter(selection, scene) {
                    let collider =
                        ColliderBuilder::new(BaseBuilder::new().with_name("ConvexCollider"))
                            .with_shape(ColliderShape::Polyhedron(ConvexPolyhedronShape {
                                geometry_source: GeometrySource(mesh_handle),
                            }))
                            .build_node();
                    commands.push(Command::new(AddNodeCommand::new(
                        collider,
                        mesh_handle,
                        false,
                    )))
                }
            } else if message.destination() == self.create_trimesh_rigid_body {
                let handles = scene
                    .graph
                    .generate_free_handles(2 * meshes_iter(selection, scene).count());

                for (rb_collider_handles, (mesh_handle, mesh)) in
                    handles.chunks(2).zip(meshes_iter(selection, scene))
                {
                    let rigid_body_handle = rb_collider_handles[0];
                    let collider_handle = rb_collider_handles[1];

                    let rigid_body =
                        RigidBodyBuilder::new(BaseBuilder::new().with_name("RigidBody"))
                            .with_body_type(RigidBodyType::Static)
                            .build_node();
                    let collider =
                        ColliderBuilder::new(BaseBuilder::new().with_name("TrimeshCollider"))
                            .with_shape(ColliderShape::trimesh(vec![GeometrySource(mesh_handle)]))
                            .build_node();
                    commands.extend([
                        Command::new(AddNodeCommand::new(rigid_body, mesh_handle, false)),
                        Command::new(AddNodeCommand::new(collider, rigid_body_handle, false)),
                        Command::new(LinkNodesCommand::new(rigid_body_handle, mesh.parent())),
                        Command::new(LinkNodesCommand::new(mesh_handle, rigid_body_handle)),
                        Command::new(LinkNodesCommand::new(collider_handle, rigid_body_handle)),
                    ]);
                }
            } else if message.destination() == self.add_convex_collider {
                for (mesh_handle, _) in meshes_iter(selection, scene) {
                    if let Some((ancestor_rigid_body, _)) =
                        scene.graph.find_component_up::<RigidBody>(mesh_handle)
                    {
                        let collider =
                            ColliderBuilder::new(BaseBuilder::new().with_name("ConvexCollider"))
                                .with_shape(ColliderShape::Polyhedron(ConvexPolyhedronShape {
                                    geometry_source: GeometrySource(mesh_handle),
                                }))
                                .build_node();
                        commands.push(Command::new(AddNodeCommand::new(
                            collider,
                            ancestor_rigid_body,
                            false,
                        )))
                    }
                }
            } else if message.destination() == self.add_trimesh_collider {
                for (mesh_handle, _) in meshes_iter(selection, scene) {
                    if let Some((ancestor_rigid_body, _)) =
                        scene.graph.find_component_up::<RigidBody>(mesh_handle)
                    {
                        let collider =
                            ColliderBuilder::new(BaseBuilder::new().with_name("TrimeshCollider"))
                                .with_shape(ColliderShape::trimesh(vec![GeometrySource(
                                    mesh_handle,
                                )]))
                                .build_node();
                        commands.push(Command::new(AddNodeCommand::new(
                            collider,
                            ancestor_rigid_body,
                            false,
                        )))
                    }
                }
            }
        }

        if !commands.is_empty() {
            sender.do_command(CommandGroup::from(commands));
        }
    }

    pub fn handle_message(
        &mut self,
        message: &Message,
        editor_selection: &Selection,
        game_scene: &mut GameScene,
        engine: &mut Engine,
    ) {
        let Message::SelectionChanged { .. } = message else {
            return;
        };

        let scene = &engine.scenes[game_scene.scene];
        let Some(selection) = editor_selection.as_graph() else {
            return;
        };

        let any_mesh = selection
            .nodes
            .iter()
            .any(|n| scene.graph.try_get_of_type::<Mesh>(*n).is_some());
        if any_mesh {
            engine
                .user_interfaces
                .first_mut()
                .send_message(WindowMessage::open_and_align(
                    self.window,
                    MessageDirection::ToWidget,
                    self.scene_viewer_frame,
                    HorizontalAlignment::Right,
                    VerticalAlignment::Top,
                    Thickness::top_right(5.0),
                    false,
                    false,
                ));
        } else {
            engine
                .user_interfaces
                .first_mut()
                .send_message(WindowMessage::close(
                    self.window,
                    MessageDirection::ToWidget,
                ));
        }
    }
}

pub struct SurfaceDataViewer {
    pub window: Handle<UiNode>,
    preview_panel: PreviewPanel,
}

impl SurfaceDataViewer {
    pub fn new(engine: &mut Engine) -> Self {
        let preview_panel = PreviewPanel::new(engine, 386, 386);

        let ctx = &mut engine.user_interfaces.first_mut().build_ctx();

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(400.0).with_height(400.0))
            .open(false)
            .with_content(preview_panel.root)
            .build(ctx);

        Self {
            window,
            preview_panel,
        }
    }

    pub fn open(&mut self, surface_data: SurfaceResource, engine: &mut Engine) {
        let ui = engine.user_interfaces.first();
        ui.send_message(WindowMessage::open_modal(
            self.window,
            MessageDirection::ToWidget,
            true,
            true,
        ));

        let guard = surface_data.data_ref();
        let title = WindowTitle::text(format!(
            "Surface Data - Vertices: {} Triangles: {}",
            guard.vertex_buffer.vertex_count(),
            guard.geometry_buffer.len(),
        ));
        ui.send_message(WindowMessage::title(
            self.window,
            MessageDirection::ToWidget,
            title,
        ));
        drop(guard);

        let graph = &mut engine.scenes[self.preview_panel.scene()].graph;
        let mesh = MeshBuilder::new(BaseBuilder::new())
            .with_surfaces(vec![SurfaceBuilder::new(surface_data).build()])
            .build(graph);

        self.preview_panel.set_model(mesh, engine);
    }

    pub fn handle_ui_message(mut self, message: &UiMessage, engine: &mut Engine) -> Option<Self> {
        self.preview_panel.handle_message(message, engine);

        if let Some(WindowMessage::Close) = message.data() {
            if message.destination() == self.window {
                self.preview_panel.destroy(engine);
                return None;
            }
        }

        Some(self)
    }

    pub fn update(&mut self, engine: &mut Engine) {
        self.preview_panel.update(engine)
    }
}
