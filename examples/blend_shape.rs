pub mod shared;

use fyrox::{
    core::{
        algebra::{UnitQuaternion, Vector2, Vector3},
        color::Color,
        futures::executor::block_on,
        pool::Handle,
    },
    engine::{
        executor::Executor, resource_manager::ResourceManager, GraphicsContext,
        GraphicsContextParams,
    },
    event::{ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::ControlFlow,
    gui::{
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessage},
        scroll_bar::{ScrollBarBuilder, ScrollBarMessage},
        scroll_viewer::ScrollViewerBuilder,
        text::{TextBuilder, TextMessage},
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        UiNode, UserInterface,
    },
    plugin::{Plugin, PluginConstructor, PluginContext},
    scene::{
        base::BaseBuilder,
        camera::CameraBuilder,
        light::{point::PointLightBuilder, BaseLightBuilder},
        node::Node,
        transform::TransformBuilder,
        Scene,
    },
    window::WindowAttributes,
};
use std::collections::BTreeSet;

struct GameSceneLoader {
    scene: Scene,
    model_handle: Handle<Node>,

    sliders: Vec<(String, Handle<UiNode>)>,
}

impl GameSceneLoader {
    fn load_with(resource_manager: ResourceManager, ui: &mut UserInterface) -> Self {
        let mut scene = Scene::new();

        scene.ambient_lighting_color = Color::opaque(150, 150, 150);

        CameraBuilder::new(
            BaseBuilder::new().with_local_transform(
                TransformBuilder::new()
                    .with_local_position(Vector3::new(0.0, 2.0, -8.0))
                    .build(),
            ),
        )
        .build(&mut scene.graph);

        PointLightBuilder::new(BaseLightBuilder::new(
            BaseBuilder::new().with_local_transform(
                TransformBuilder::new()
                    .with_local_position(Vector3::new(0.0, 12.0, -6.0))
                    .build(),
            ),
        ))
        .with_radius(20.0)
        .build(&mut scene.graph);

        let model_resource = block_on(
            resource_manager.request_model("examples/data/blend_shape/Gunan_animated2.fbx"),
        )
        .unwrap();

        let model_handle = model_resource.instantiate(&mut scene);

        scene.graph[model_handle]
            .local_transform_mut()
            .set_scale(Vector3::new(0.05, 0.05, 0.05));

        let sphere = scene.graph.find_by_name_from_root("Head_Mesh").unwrap().0;
        let blend_shape = scene.graph[sphere].as_mesh_mut();

        let mut blend_shape_names = BTreeSet::new();
        for surface in blend_shape.surfaces_mut() {
            let data = surface.data();
            let data = data.lock();
            if let Some(container) = data.blend_shapes_container.as_ref() {
                for blend_shape in container.blend_shapes.iter() {
                    blend_shape_names.insert(blend_shape.name.clone());
                }
            }
        }

        let ctx = &mut ui.build_ctx();

        let mut children = Vec::new();
        let mut sliders = Vec::new();

        for (row, blend_shape_name) in blend_shape_names.iter().enumerate() {
            let short_name = blend_shape_name
                .strip_prefix("ExpressionBlendshapes.")
                .map(|n| n.to_owned())
                .unwrap_or_else(|| blend_shape_name.clone());

            let name = TextBuilder::new(WidgetBuilder::new().on_row(row))
                .with_text(short_name)
                .build(ctx);
            let slider = ScrollBarBuilder::new(WidgetBuilder::new().on_row(row).on_column(1))
                .with_min(0.0)
                .with_max(100.0)
                .with_step(1.0)
                .build(ctx);
            children.push(name);
            children.push(slider);
            sliders.push((blend_shape_name.clone(), slider));
        }

        WindowBuilder::new(
            WidgetBuilder::new()
                .with_width(250.0)
                .with_height(400.0)
                .with_desired_position(Vector2::new(5.0, 50.0)),
        )
        .with_title(WindowTitle::text("Blend Shapes"))
        .with_content(
            ScrollViewerBuilder::new(WidgetBuilder::new())
                .with_content(
                    GridBuilder::new(WidgetBuilder::new().with_children(children))
                        .add_column(Column::auto())
                        .add_column(Column::stretch())
                        .add_rows(
                            blend_shape_names
                                .iter()
                                .map(|_| Row::strict(20.0))
                                .collect(),
                        )
                        .build(ctx),
                )
                .build(ctx),
        )
        .build(ctx);

        Self {
            scene,
            sliders,
            model_handle,
        }
    }
}

struct InputController {
    rotate_left: bool,
    rotate_right: bool,
}

struct Game {
    scene: Handle<Scene>,
    model_handle: Handle<Node>,
    input_controller: InputController,
    debug_text: Handle<UiNode>,
    model_angle: f32,
    sliders: Vec<(String, Handle<UiNode>)>,
}

impl Plugin for Game {
    fn update(&mut self, context: &mut PluginContext, _control_flow: &mut ControlFlow) {
        let scene = &mut context.scenes[self.scene];

        // Rotate model according to input controller state
        if self.input_controller.rotate_left {
            self.model_angle -= 5.0f32.to_radians();
        } else if self.input_controller.rotate_right {
            self.model_angle += 5.0f32.to_radians();
        }

        scene.graph[self.model_handle]
            .local_transform_mut()
            .set_rotation(UnitQuaternion::from_axis_angle(
                &Vector3::y_axis(),
                self.model_angle,
            ));

        if let GraphicsContext::Initialized(ref graphics_context) = context.graphics_context {
            context.user_interface.send_message(TextMessage::text(
                self.debug_text,
                MessageDirection::ToWidget,
                format!(
                    "Example - Blend Shapes\nUse [A][D] keys to rotate the model and sliders to select facial expression.\nFPS: {}",
                    graphics_context.renderer.get_statistics().frames_per_second
                ),
            ));
        }
    }

    fn on_ui_message(
        &mut self,
        context: &mut PluginContext,
        message: &UiMessage,
        _control_flow: &mut ControlFlow,
    ) {
        if let Some(ScrollBarMessage::Value(value)) = message.data() {
            if message.direction() == MessageDirection::FromWidget {
                for (name, slider) in self.sliders.iter() {
                    if message.destination() == *slider {
                        let scene = &mut context.scenes[self.scene];
                        let sphere = scene.graph.find_by_name_from_root("Head_Mesh").unwrap().0;
                        let blend_shape = scene.graph[sphere].as_mesh_mut();

                        for surface in blend_shape.surfaces_mut() {
                            let data = surface.data();
                            let mut data = data.lock();
                            let mut changed = false;
                            if let Some(container) = data.blend_shapes_container.as_mut() {
                                for blend_shape in container.blend_shapes.iter_mut() {
                                    if &blend_shape.name == name {
                                        blend_shape.weight = *value;
                                        changed = true;
                                    }
                                }
                            }
                            if changed {
                                data.apply_blend_shapes().unwrap();
                            }
                        }
                    }
                }
            }
        }
    }

    fn on_os_event(
        &mut self,
        event: &Event<()>,
        _context: PluginContext,
        _control_flow: &mut ControlFlow,
    ) {
        if let Event::WindowEvent {
            event: WindowEvent::KeyboardInput { input, .. },
            ..
        } = event
        {
            if let Some(key_code) = input.virtual_keycode {
                match key_code {
                    VirtualKeyCode::A => {
                        self.input_controller.rotate_left = input.state == ElementState::Pressed
                    }
                    VirtualKeyCode::D => {
                        self.input_controller.rotate_right = input.state == ElementState::Pressed
                    }
                    _ => (),
                }
            }
        }
    }
}

struct GameConstructor;

impl PluginConstructor for GameConstructor {
    fn create_instance(
        &self,
        _override_scene: Handle<Scene>,
        context: PluginContext,
    ) -> Box<dyn Plugin> {
        let scene =
            GameSceneLoader::load_with(context.resource_manager.clone(), context.user_interface);

        Box::new(Game {
            debug_text: TextBuilder::new(WidgetBuilder::new())
                .build(&mut context.user_interface.build_ctx()),
            scene: context.scenes.add(scene.scene),
            model_handle: scene.model_handle,
            // Create input controller - it will hold information about needed actions.
            input_controller: InputController {
                rotate_left: false,
                rotate_right: false,
            },
            // We will rotate model using keyboard input.
            model_angle: 180.0f32.to_radians(),
            sliders: scene.sliders,
        })
    }
}

fn main() {
    let mut executor = Executor::from_params(
        Default::default(),
        GraphicsContextParams {
            window_attributes: WindowAttributes {
                title: "Example - Blend Shapes".to_string(),
                ..Default::default()
            },
            vsync: true,
        },
    );
    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}
