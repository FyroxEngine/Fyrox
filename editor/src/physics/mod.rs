use crate::command::{Command, CommandGroup, SetPropertyCommand};
use crate::fyrox::graph::{BaseSceneGraph, SceneGraph};
use crate::fyrox::{
    core::{algebra::Vector3, pool::Handle, reflect::Reflect},
    engine::Engine,
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
        collider::{Collider, ColliderShape},
        node::Node,
    },
};
use crate::scene::commands::GameSceneContext;
use crate::{
    message::MessageSender,
    scene::{GameScene, Selection},
    Message,
};

pub struct ColliderControlPanel {
    pub window: Handle<UiNode>,
    fit: Handle<UiNode>,
    scene_frame: Handle<UiNode>,
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
    pub fn new(scene_frame: Handle<UiNode>, ctx: &mut BuildContext) -> Self {
        let tooltip = "Tries to calculate the new collider shape parameters (half extents,\
        radius, etc.) using bounding boxes of descendant nodes of the parent rigid body. This \
        operation performed in world-space coordinates.";

        let fit;
        let window = WindowBuilder::new(
            WidgetBuilder::new()
                .with_width(250.0)
                .with_height(50.0)
                .with_name("ColliderControlPanel"),
        )
        .with_title(WindowTitle::text("Collider Control Panel"))
        .with_content(
            StackPanelBuilder::new(
                WidgetBuilder::new()
                    .with_horizontal_alignment(HorizontalAlignment::Right)
                    .with_child({
                        fit = ButtonBuilder::new(
                            WidgetBuilder::new()
                                .with_width(80.0)
                                .with_height(24.0)
                                .with_tooltip(make_simple_tooltip(ctx, tooltip)),
                        )
                        .with_text("Try Fit")
                        .build(ctx);
                        fit
                    }),
            )
            .build(ctx),
        )
        .build(ctx);

        Self {
            window,
            fit,
            scene_frame,
        }
    }

    pub fn handle_message(
        &self,
        message: &Message,
        engine: &Engine,
        game_scene: &GameScene,
        selection: &Selection,
    ) {
        if let Message::SelectionChanged { .. } = message {
            let mut collider_selected = false;

            if let Some(selection) = selection.as_graph() {
                for selected in selection.nodes() {
                    let scene = &engine.scenes[game_scene.scene];

                    collider_selected =
                        scene.graph.try_get_of_type::<Collider>(*selected).is_some();
                }
            }

            if collider_selected {
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
        }
    }
}
