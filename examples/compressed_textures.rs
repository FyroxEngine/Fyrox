//! Example - Texture compression
//!
//! Just shows two textures with compression. Engine compresses textures automatically,
//! based on compression options.

use fyrox::engine::Engine;
use fyrox::{
    core::{algebra::Vector2, color::Color},
    engine::{framework::prelude::*, resource_manager::TextureImportOptions},
    gui::{image::ImageBuilder, widget::WidgetBuilder},
    resource::texture::CompressionOptions,
    utils::into_gui_texture,
};

struct Game;

impl GameState for Game {
    fn init(engine: &mut Engine) -> Self
    where
        Self: Sized,
    {
        // Explicitly set compression options - here we use Quality which in most cases will use
        // DXT5 compression with compression ratio 4:1
        engine.resource_manager.state().set_textures_import_options(
            TextureImportOptions::default().with_compression(CompressionOptions::Quality),
        );

        engine
            .renderer
            .set_backbuffer_clear_color(Color::opaque(120, 120, 120));

        ImageBuilder::new(
            WidgetBuilder::new()
                .with_desired_position(Vector2::new(0.0, 0.0))
                .with_width(512.0)
                .with_height(512.0),
        )
        .with_texture(into_gui_texture(
            engine
                .resource_manager
                .request_texture("examples/data/MetalMesh_Base_Color.png", None),
        ))
        .build(&mut engine.user_interface.build_ctx());

        ImageBuilder::new(
            WidgetBuilder::new()
                .with_desired_position(Vector2::new(512.0, 0.0))
                .with_width(512.0)
                .with_height(512.0),
        )
        .with_texture(into_gui_texture(
            engine
                .resource_manager
                .request_texture("examples/data/R8Texture.png", None),
        ))
        .build(&mut engine.user_interface.build_ctx());

        Self
    }
}

fn main() {
    Framework::<Game>::new()
        .unwrap()
        .title("Example - Compressed Textures")
        .run();
}
