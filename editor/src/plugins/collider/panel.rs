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
    command::{Command, CommandGroup, SetPropertyCommand},
    fyrox::{
        core::{algebra::Vector3, pool::Handle, reflect::Reflect, TypeUuidProvider},
        engine::Engine,
        graph::{BaseSceneGraph, SceneGraph},
        gui::{
            button::{ButtonBuilder, ButtonMessage},
            message::{MessageDirection, UiMessage},
            stack_panel::StackPanelBuilder,
            utils::make_simple_tooltip,
            widget::{WidgetBuilder, WidgetMessage},
            BuildContext, HorizontalAlignment, Orientation, UiNode, UserInterface,
        },
        scene::{
            collider::{Collider, ColliderShape},
            node::Node,
        },
    },
    message::MessageSender,
    plugins::collider::ColliderShapeInteractionMode,
    scene::{commands::GameSceneContext, GameScene, Selection},
    Message,
};

pub struct ColliderControlPanel {
    pub root_widget: Handle<UiNode>,
    fit: Handle<UiNode>,
    edit: Handle<UiNode>,
}

fn set_property<T: Reflect>(
    name: &str,
    value: T,
    commands: &mut Vec<Command>,
    selected_collider: Handle<Node>,
) {
    commands.push(Command::new(SetPropertyCommand::new(
        name.into(),
        Box::new(value) as Box<dyn Reflect>,
        move |ctx| {
            ctx.get_mut::<GameSceneContext>()
                .scene
                .graph
                .node_mut(selected_collider)
        },
    )));
}

impl ColliderControlPanel {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let try_fit_tooltip = "Tries to calculate the new collider shape parameters (half extents,\
        radius, etc.) using bounding boxes of descendant nodes of the parent rigid body. This \
        operation performed in world-space coordinates.";

        let edit_tooltip = "Enables the shape editing interaction mode, that allows you to \
        edit the shape in-scene.";

        let fit;
        let edit;
        let root_widget = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_horizontal_alignment(HorizontalAlignment::Right)
                .with_child({
                    fit = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .with_width(80.0)
                            .with_height(24.0)
                            .with_tooltip(make_simple_tooltip(ctx, try_fit_tooltip)),
                    )
                    .with_text("Try Fit")
                    .build(ctx);
                    fit
                })
                .with_child({
                    edit = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .with_width(80.0)
                            .with_height(24.0)
                            .with_tooltip(make_simple_tooltip(ctx, edit_tooltip)),
                    )
                    .with_text("Edit")
                    .build(ctx);
                    edit
                }),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);
        Self {
            root_widget,
            fit,
            edit,
        }
    }

    pub fn destroy(self, ui: &UserInterface) {
        ui.send_message(WidgetMessage::remove(
            self.root_widget,
            MessageDirection::ToWidget,
        ));
    }

    pub fn handle_ui_message(
        &self,
        message: &UiMessage,
        engine: &Engine,
        game_scene: &GameScene,
        selection: &Selection,
        sender: &MessageSender,
    ) {
        if message.destination() == self.fit {
            let Some(ButtonMessage::Click) = message.data() else {
                return;
            };

            let Some(selection) = selection.as_graph() else {
                return;
            };

            let scene = &engine.scenes[game_scene.scene];

            let mut commands = Vec::new();

            for collider in selection.nodes() {
                let Some(collider_ref) = scene.graph.try_get_of_type::<Collider>(*collider) else {
                    continue;
                };

                let Some(aabb) = scene
                    .graph
                    .aabb_of_descendants(collider_ref.parent(), |h, _| {
                        // Ignore self bounds of the selected collider.
                        h != *collider
                    })
                else {
                    continue;
                };

                let half = aabb.half_extents();

                match collider_ref.shape() {
                    ColliderShape::Ball(_) => {
                        set_property("shape.Ball@0.radius", half.max(), &mut commands, *collider);
                    }
                    ColliderShape::Cylinder(_) => {
                        set_property(
                            "shape.Cylinder@0.radius",
                            half.x.max(half.z),
                            &mut commands,
                            *collider,
                        );

                        set_property(
                            "shape.Cylinder@0.half_height",
                            half.y,
                            &mut commands,
                            *collider,
                        );
                    }
                    ColliderShape::Cone(_) => {
                        set_property(
                            "shape.Cone@0.radius",
                            half.x.max(half.z),
                            &mut commands,
                            *collider,
                        );

                        set_property("shape.Cone@0.half_height", half.y, &mut commands, *collider);
                    }
                    ColliderShape::Cuboid(_) => {
                        set_property(
                            "shape.Cuboid@0.half_extents",
                            half,
                            &mut commands,
                            *collider,
                        );
                    }
                    ColliderShape::Capsule(_) => {
                        let local_center = scene
                            .graph
                            .try_get(collider_ref.parent())
                            .map(|p| p.global_transform())
                            .unwrap_or_default()
                            .try_inverse()
                            .unwrap_or_default()
                            .transform_point(&aabb.center().into());

                        let dy = Vector3::new(0.0, half.y, 0.0);

                        set_property(
                            "shape.Capsule@0.begin",
                            local_center.coords + dy,
                            &mut commands,
                            *collider,
                        );

                        set_property(
                            "shape.Capsule@0.end",
                            local_center.coords - dy,
                            &mut commands,
                            *collider,
                        );

                        set_property(
                            "shape.Capsule@0.radius",
                            half.x.max(half.z),
                            &mut commands,
                            *collider,
                        );
                    }
                    _ => (),
                }
            }
            if !commands.is_empty() {
                sender.do_command(CommandGroup::from(commands));
            }
        } else if message.destination() == self.edit {
            sender.send(Message::SetInteractionMode(
                ColliderShapeInteractionMode::type_uuid(),
            ));
        }
    }
}
