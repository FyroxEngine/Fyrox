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
    command::CommandGroup,
    fyrox::{
        core::pool::Handle,
        engine::Engine,
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
            mesh::Mesh,
            node::Node,
            rigidbody::{RigidBody, RigidBodyBuilder, RigidBodyType},
        },
    },
    game_scene::{
        GameScene, Selection,
        commands::graph::{AddNodeCommand, LinkNodesCommand},
    },
    message::MessageSender,
    world::selection::GraphSelection,
};
use fyrox::core::some_or_continue;

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
    pub fn new(selection: &GraphSelection, scene: &Scene, ctx: &mut BuildContext) -> Self {
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

        if !Self::has_parent_rigid_body(selection, scene) {
            ctx[add_convex_collider].set_enabled(false);
            ctx[add_trimesh_collider].set_enabled(false);
        }

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

        let mut group = CommandGroup::default();

        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.create_trimesh_collider {
                Self::on_create_trimesh_collider_clicked(selection, scene, &mut group);
            } else if message.destination() == self.create_convex_collider {
                Self::on_create_convex_collider_clicked(selection, scene, &mut group);
            } else if message.destination() == self.create_trimesh_rigid_body {
                Self::on_create_trimesh_rigid_body_clicked(selection, scene, &mut group);
            } else if message.destination() == self.add_convex_collider {
                Self::on_add_convex_collider_clicked(selection, scene, &mut group);
            } else if message.destination() == self.add_trimesh_collider {
                Self::on_add_trimesh_collider_clicked(selection, scene, &mut group);
            }
        }

        if !group.is_empty() {
            sender.do_command(group);
        }
    }

    pub fn sync_to_model(&self, selection: &GraphSelection, scene: &Scene, ui: &UserInterface) {
        let has_parent_rigid_body = Self::has_parent_rigid_body(selection, scene);
        for button in [self.add_convex_collider, self.add_trimesh_collider] {
            ui.send(button, WidgetMessage::Enabled(has_parent_rigid_body));
        }
    }

    fn has_parent_rigid_body(selection: &GraphSelection, scene: &Scene) -> bool {
        meshes_iter(selection, scene).any(|(mesh_handle, _)| {
            scene
                .graph
                .find_self_or_field_up::<RigidBody>(mesh_handle)
                .is_some()
        })
    }

    fn on_add_trimesh_collider_clicked(
        selection: &GraphSelection,
        scene: &Scene,
        commands: &mut CommandGroup,
    ) {
        for (mesh_handle, _) in meshes_iter(selection, scene) {
            let (ancestor_rigid_body, _) =
                some_or_continue!(scene.graph.find_self_or_field_up::<RigidBody>(mesh_handle));
            let collider = ColliderBuilder::new(BaseBuilder::new().with_name("TrimeshCollider"))
                .with_shape(ColliderShape::trimesh(vec![GeometrySource(mesh_handle)]))
                .build_node();
            commands.push(AddNodeCommand::new(collider, ancestor_rigid_body, false))
        }
    }

    fn on_add_convex_collider_clicked(
        selection: &GraphSelection,
        scene: &Scene,
        commands: &mut CommandGroup,
    ) {
        for (mesh_handle, _) in meshes_iter(selection, scene) {
            let (ancestor_rigid_body, _) =
                some_or_continue!(scene.graph.find_self_or_field_up::<RigidBody>(mesh_handle));
            let collider = ColliderBuilder::new(BaseBuilder::new().with_name("ConvexCollider"))
                .with_shape(ColliderShape::Polyhedron(ConvexPolyhedronShape {
                    geometry_source: GeometrySource(mesh_handle),
                }))
                .build_node();
            commands.push(AddNodeCommand::new(collider, ancestor_rigid_body, false))
        }
    }

    fn on_create_trimesh_rigid_body_clicked(
        selection: &GraphSelection,
        scene: &Scene,
        commands: &mut CommandGroup,
    ) {
        let handles = scene
            .graph
            .generate_free_handles(2 * meshes_iter(selection, scene).count());

        for (rb_collider_handles, (mesh_handle, mesh)) in
            handles.chunks(2).zip(meshes_iter(selection, scene))
        {
            let rigid_body_handle = rb_collider_handles[0];
            let collider_handle = rb_collider_handles[1];

            let rigid_body = RigidBodyBuilder::new(BaseBuilder::new().with_name("RigidBody"))
                .with_body_type(RigidBodyType::Static)
                .build_node();
            let collider = ColliderBuilder::new(BaseBuilder::new().with_name("TrimeshCollider"))
                .with_shape(ColliderShape::trimesh(vec![GeometrySource(mesh_handle)]))
                .build_node();

            commands.push(AddNodeCommand::new(rigid_body, mesh_handle, false));
            commands.push(AddNodeCommand::new(collider, rigid_body_handle, false));
            commands.push(LinkNodesCommand::new(rigid_body_handle, mesh.parent()));
            commands.push(LinkNodesCommand::new(mesh_handle, rigid_body_handle));
            commands.push(LinkNodesCommand::new(collider_handle, rigid_body_handle));
        }
    }

    fn on_create_convex_collider_clicked(
        selection: &GraphSelection,
        scene: &Scene,
        commands: &mut CommandGroup,
    ) {
        for (mesh_handle, _) in meshes_iter(selection, scene) {
            let collider = ColliderBuilder::new(BaseBuilder::new().with_name("ConvexCollider"))
                .with_shape(ColliderShape::Polyhedron(ConvexPolyhedronShape {
                    geometry_source: GeometrySource(mesh_handle),
                }))
                .build_node();
            commands.push(AddNodeCommand::new(collider, mesh_handle, false))
        }
    }

    fn on_create_trimesh_collider_clicked(
        selection: &GraphSelection,
        scene: &Scene,
        commands: &mut CommandGroup,
    ) {
        for (mesh_handle, _) in meshes_iter(selection, scene) {
            let collider = ColliderBuilder::new(BaseBuilder::new().with_name("TrimeshCollider"))
                .with_shape(ColliderShape::trimesh(vec![GeometrySource(mesh_handle)]))
                .build_node();
            commands.push(AddNodeCommand::new(collider, mesh_handle, false))
        }
    }

    pub fn destroy(self, ui: &UserInterface) {
        ui.send(self.root_widget, WidgetMessage::Remove);
    }
}
