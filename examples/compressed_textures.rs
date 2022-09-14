//! Example - Texture compression
//!
//! Just shows two textures with compression. Engine compresses textures automatically,
//! based on compression options.

use fyrox::{
    core::{algebra::Vector2, color::Color, pool::Handle},
    engine::executor::Executor,
    gui::{image::ImageBuilder, widget::WidgetBuilder},
    plugin::{Plugin, PluginConstructor, PluginContext},
    resource::texture::{CompressionOptions, TextureImportOptions},
    scene::Scene,
    utils::into_gui_texture,
};

struct Game;

impl Plugin for Game {}

struct GameConstructor;

impl PluginConstructor for GameConstructor {
    fn create_instance(
        &self,
        _override_scene: Handle<Scene>,
        context: PluginContext,
    ) -> Box<dyn Plugin> {
        // Explicitly set compression options - here we use Quality which in most cases will use
        // DXT5 compression with compression ratio 4:1
        context
            .resource_manager
            .state()
            .containers_mut()
            .textures
            .set_default_import_options(
                TextureImportOptions::default().with_compression(CompressionOptions::Quality),
            );

        context
            .renderer
            .set_backbuffer_clear_color(Color::opaque(120, 120, 120));

        ImageBuilder::new(
            WidgetBuilder::new()
                .with_desired_position(Vector2::new(0.0, 0.0))
                .with_width(512.0)
                .with_height(512.0),
        )
        .with_texture(into_gui_texture(
            context
                .resource_manager
                .request_texture("examples/data/MetalMesh_Base_Color.png"),
        ))
        .build(&mut context.user_interface.build_ctx());

        ImageBuilder::new(
            WidgetBuilder::new()
                .with_desired_position(Vector2::new(512.0, 0.0))
                .with_width(512.0)
                .with_height(512.0),
        )
        .with_texture(into_gui_texture(
            context
                .resource_manager
                .request_texture("examples/data/R8Texture.png"),
        ))
        .build(&mut context.user_interface.build_ctx());

        Box::new(Game)
    }
}

fn main() {
    let mut executor = Executor::new();
    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}
