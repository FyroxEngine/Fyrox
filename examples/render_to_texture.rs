use fyrox::{
    core::{futures::executor::block_on, pool::Handle},
    engine::executor::Executor,
    event_loop::ControlFlow,
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        grid::{Column, GridBuilder, Row},
        image::{ImageBuilder, ImageMessage},
        message::{MessageDirection, UiMessage},
        text::TextBuilder,
        widget::{WidgetBuilder, WidgetMessage},
        UiNode, VerticalAlignment,
    },
    plugin::{Plugin, PluginConstructor, PluginContext},
    resource::texture::{Texture, TextureKind},
    scene::{Scene, SceneLoader},
    utils,
};

struct Game {
    render_target: Texture,
    scene_handle: Handle<Scene>,
    scene_image: Handle<UiNode>,
    exit: Handle<UiNode>,
    grid: Handle<UiNode>,
}

impl Plugin for Game {
    fn update(&mut self, context: &mut PluginContext, _control_flow: &mut ControlFlow) {
        // Sync render target size with actual Image widget size.
        let render_target = self.render_target.data_ref();
        if let TextureKind::Rectangle { width, height } = render_target.kind() {
            let image_size = context
                .user_interface
                .node(self.scene_image)
                .actual_global_size();
            if width != image_size.x as u32 || height != image_size.y as u32 {
                // Re-create render target with new size.
                drop(render_target);
                self.render_target =
                    Texture::new_render_target(image_size.x as u32, image_size.y as u32);
                context.scenes[self.scene_handle].render_target = Some(self.render_target.clone());
                context.user_interface.send_message(ImageMessage::texture(
                    self.scene_image,
                    MessageDirection::ToWidget,
                    Some(utils::into_gui_texture(self.render_target.clone())),
                ));
            }
        }

        // Keep grid's size equal to window inner size.
        let window_size = context.window.inner_size();
        context.user_interface.send_message(WidgetMessage::width(
            self.grid,
            MessageDirection::ToWidget,
            window_size.width as f32,
        ));
        context.user_interface.send_message(WidgetMessage::height(
            self.grid,
            MessageDirection::ToWidget,
            window_size.height as f32,
        ));
    }

    fn on_ui_message(
        &mut self,
        _context: &mut PluginContext,
        message: &UiMessage,
        control_flow: &mut ControlFlow,
    ) {
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.exit {
                *control_flow = ControlFlow::Exit;
            }
        }
    }
}

struct GameConstructor;

fn load_scene(context: &PluginContext) -> Scene {
    let loader = block_on(SceneLoader::from_file(
        "examples/data/rt_scene.rgs",
        context.serialization_context.clone(),
    ))
    .unwrap();
    block_on(loader.finish(context.resource_manager.clone()))
}

impl PluginConstructor for GameConstructor {
    fn create_instance(
        &self,
        _override_scene: Handle<Scene>,
        context: PluginContext,
    ) -> Box<dyn Plugin> {
        // Load scene first.
        let mut scene = load_scene(&context);

        // Create render target and force the scene to render into it.
        let window_size = context.window.inner_size();
        let render_target = Texture::new_render_target(window_size.width, window_size.height);
        scene.render_target = Some(render_target.clone());

        // Add the loaded scene to the engine.
        let scene_handle = context.scenes.add(scene);

        let ctx = &mut context.user_interface.build_ctx();

        // Create an Image widget which will use the render target to render the scene.
        let scene_image = ImageBuilder::new(WidgetBuilder::new().on_row(0).on_column(0))
            .with_flip(true)
            .with_texture(utils::into_gui_texture(render_target.clone()))
            .build(ctx);

        // Create "exit game" button.
        let exit = ButtonBuilder::new(
            WidgetBuilder::new()
                .with_width(100.0)
                .with_height(30.0)
                .on_row(0)
                .on_column(1)
                .with_vertical_alignment(VerticalAlignment::Top),
        )
        .with_text("Exit")
        .build(ctx);

        // Create the grid.
        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_width(window_size.width as f32)
                .with_height(window_size.height as f32)
                .with_child(scene_image)
                .with_child(exit),
        )
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .build(ctx);

        Box::new(Game {
            grid,
            render_target,
            scene_handle,
            scene_image,
            exit,
        })
    }
}

fn main() {
    let mut executor = Executor::new();
    executor.get_window().set_title("Example - Render Target");
    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}
