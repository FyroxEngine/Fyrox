use crate::{
    asset::preview::{render_ui_to_texture, AssetPreviewGenerator, AssetPreviewTexture},
    fyrox::{
        asset::{manager::ResourceManager, untyped::UntypedResource},
        core::{algebra::Vector2, pool::Handle},
        engine::Engine,
        gui::{
            image::ImageBuilder, screen::ScreenBuilder, widget::WidgetBuilder,
            wrap_panel::WrapPanelBuilder, Orientation, Thickness, UserInterface,
        },
        scene::{node::Node, tilemap::tileset::TileSet, Scene},
    },
    load_image,
};

pub struct TileSetPreview;

impl AssetPreviewGenerator for TileSetPreview {
    fn generate_scene(
        &mut self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
        _scene: &mut Scene,
    ) -> Handle<Node> {
        Handle::NONE
    }

    fn generate_preview(
        &mut self,
        resource: &UntypedResource,
        engine: &mut Engine,
    ) -> Option<AssetPreviewTexture> {
        let tile_set_resource = resource.try_cast::<TileSet>()?;
        let tile_set_data = tile_set_resource.data_ref();
        let tile_set = tile_set_data.as_loaded_ref()?;
        let mut ui = UserInterface::new(Vector2::new(256.0, 256.0));
        let ctx = &mut ui.build_ctx();
        ScreenBuilder::new(
            WidgetBuilder::new().with_child(
                WrapPanelBuilder::new(WidgetBuilder::new().with_children(
                    tile_set.tiles.iter().map(|tile| {
                        let texture =
                            tile.material
                                .data_ref()
                                .as_loaded_ref()
                                .and_then(|material| {
                                    material
                                        .texture("diffuseTexture")
                                        .map(|texture| texture.into_untyped())
                                });

                        ImageBuilder::new(
                            WidgetBuilder::new()
                                .with_width(32.0)
                                .with_height(32.0)
                                .with_margin(Thickness::uniform(1.0)),
                        )
                        .with_uv_rect(tile.uv_rect)
                        .with_opt_texture(texture)
                        .build(ctx)
                    }),
                ))
                .with_orientation(Orientation::Horizontal)
                .build(ctx),
            ),
        )
        .build(ctx);
        render_ui_to_texture(&mut ui, engine)
    }

    fn simple_icon(
        &self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
    ) -> Option<UntypedResource> {
        load_image(include_bytes!("../../../resources/tile_set.png"))
    }
}
