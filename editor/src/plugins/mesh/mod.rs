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
    Editor, Message,
    command::{Command, CommandGroup},
    fyrox::{
        core::{pool::Handle, some_or_return},
        engine::{ApplicationLoopController, Engine},
        graph::SceneGraph,
        gui::{
            BuildContext, Thickness, UserInterface,
            button::{Button, ButtonBuilder, ButtonMessage},
            message::UiMessage,
            stack_panel::{StackPanel, StackPanelBuilder},
            utils::make_simple_tooltip,
            widget::{WidgetBuilder, WidgetMessage},
        },
        scene::{
            Scene,
            base::BaseBuilder,
            collider::{ColliderBuilder, ColliderShape, ConvexPolyhedronShape, GeometrySource},
            mesh::{Mesh, surface::SurfaceResource},
            node::Node,
            rigidbody::{RigidBody, RigidBodyBuilder, RigidBodyType},
        },
    },
    message::MessageSender,
    plugin::EditorPlugin,
    plugins::{inspector::InspectorPlugin, mesh::surface::SurfaceDataPropertyEditorDefinition},
    scene::{
        GameScene, Selection,
        commands::graph::{AddNodeCommand, LinkNodesCommand},
    },
    world::selection::GraphSelection,
};
use data_viewer::SurfaceDataViewer;

pub mod data_viewer;
mod surface;

struct ViewSurfaceDataMessage(pub SurfaceResource);

pub struct MeshControlPanel {
    pub root_widget: Handle<StackPanel>,
    create_trimesh_collider: Handle<Button>,
    create_convex_collider: Handle<Button>,
    create_trimesh_rigid_body: Handle<Button>,
    add_convex_collider: Handle<Button>,
    add_trimesh_collider: Handle<Button>,
}

fn make_button(text: &str, tooltip: &str, ctx: &mut BuildContext) -> Handle<Button> {
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
            .ok()
            .map(|mesh| (*handle, mesh))
    })
}

impl MeshControlPanel {
    fn new(ctx: &mut BuildContext) -> Self {
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
        let root_widget = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_child(create_trimesh_collider)
                .with_child(create_convex_collider)
                .with_child(create_trimesh_rigid_body)
                .with_child(add_convex_collider)
                .with_child(add_trimesh_collider),
        )
        .build(ctx);

        Self {
            root_widget,
            create_trimesh_collider,
            create_convex_collider,
            create_trimesh_rigid_body,
            add_convex_collider,
            add_trimesh_collider,
        }
    }

    fn handle_ui_message(
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
                        scene.graph.find_self_or_field_up::<RigidBody>(mesh_handle)
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
                        scene.graph.find_self_or_field_up::<RigidBody>(mesh_handle)
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

    fn destroy(self, ui: &UserInterface) {
        ui.send(self.root_widget, WidgetMessage::Remove);
    }
}

#[derive(Default)]
pub struct MeshPlugin {
    panel: Option<MeshControlPanel>,
    surface_data_viewer: Option<SurfaceDataViewer>,
}

impl MeshPlugin {
    fn destroy_panel(&mut self, ui: &UserInterface) {
        if let Some(panel) = self.panel.take() {
            panel.destroy(ui);
        }
    }

    fn destroy_surface_viewer(&mut self, engine: &mut Engine) {
        if let Some(surface_data_viewer) = self.surface_data_viewer.take() {
            surface_data_viewer.destroy(engine)
        }
    }
}

impl EditorPlugin for MeshPlugin {
    fn on_start(&mut self, editor: &mut Editor) {
        editor
            .property_editors
            .insert(SurfaceDataPropertyEditorDefinition {
                sender: editor.message_sender.clone(),
            });
    }

    fn on_ui_message(&mut self, message: &mut UiMessage, editor: &mut Editor) {
        if let Some(surface_data_viewer) = self.surface_data_viewer.take() {
            self.surface_data_viewer =
                surface_data_viewer.handle_ui_message(message, &mut editor.engine);
        }

        let entry = editor.scenes.current_scene_entry_mut();
        let game_scene = some_or_return!(entry.controller.downcast_mut::<GameScene>());
        let panel = some_or_return!(self.panel.as_mut());
        panel.handle_ui_message(
            message,
            &entry.selection,
            game_scene,
            &mut editor.engine,
            &editor.message_sender,
        );
    }

    fn on_update(&mut self, editor: &mut Editor, _loop_controller: ApplicationLoopController) {
        if let Some(surface_data_viewer) = self.surface_data_viewer.as_mut() {
            surface_data_viewer.update(&mut editor.engine);
        }
    }

    fn on_message(&mut self, message: &Message, editor: &mut Editor) {
        let ui = editor.engine.user_interfaces.first_mut();
        let entry = editor.scenes.current_scene_entry_mut();
        let Some(game_scene) = entry.controller.downcast_mut::<GameScene>() else {
            self.destroy_panel(ui);
            self.destroy_surface_viewer(&mut editor.engine);
            return;
        };

        let scene = &mut editor.engine.scenes[game_scene.scene];

        if let Message::SelectionChanged { .. } = message {
            let is_mesh_selected = entry
                .selection
                .as_graph()
                .is_some_and(|s| s.contains_type::<Mesh>(&scene.graph));
            if is_mesh_selected {
                if self.panel.is_none() {
                    let inspector = editor.plugins.get::<InspectorPlugin>();
                    let panel = MeshControlPanel::new(&mut ui.build_ctx());
                    ui.send(panel.root_widget, WidgetMessage::link_with(inspector.head));
                    self.panel = Some(panel);
                }
            } else {
                self.destroy_panel(ui);
            }
        } else if let Some(data) = message.try_cast_any::<ViewSurfaceDataMessage>() {
            let mut viewer = SurfaceDataViewer::new(&mut editor.engine);
            viewer.open(data.0.clone(), &mut editor.engine);
            self.surface_data_viewer = Some(viewer);
        }
    }
}
