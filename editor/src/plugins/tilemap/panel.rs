use crate::{
    fyrox::{
        core::{algebra::Vector2, pool::Handle},
        gui::{
            grid::GridBuilder,
            message::{MessageDirection, UiMessage},
            widget::{WidgetBuilder, WidgetMessage},
            window::{WindowBuilder, WindowMessage, WindowTitle},
            BuildContext, HorizontalAlignment, Thickness, UiNode, UserInterface, VerticalAlignment,
        },
        scene::tilemap::tileset::TileSetResource,
    },
    plugins::tilemap::palette::{PaletteWidgetBuilder, TileViewBuilder},
};

pub struct TileMapPanel {
    pub window: Handle<UiNode>,
    pub palette: Handle<UiNode>,
}

impl TileMapPanel {
    pub fn new(
        ctx: &mut BuildContext,
        scene_frame: Handle<UiNode>,
        tile_set: Option<TileSetResource>,
    ) -> Self {
        let tiles = tile_set
            .map(|tile_set_resource| {
                let tile_set = tile_set_resource.data_ref();
                tile_set
                    .tiles
                    .iter()
                    .enumerate()
                    .map(|(index, _tile)| {
                        let side_size = 10;

                        TileViewBuilder::new(tile_set_resource.clone(), WidgetBuilder::new())
                            .with_tile_index(index)
                            .with_position(Vector2::new(
                                index as i32 % side_size,
                                index as i32 / side_size,
                            ))
                            .build(ctx)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let palette = PaletteWidgetBuilder::new(WidgetBuilder::new())
            .with_tiles(tiles)
            .build(ctx);

        let content = GridBuilder::new(WidgetBuilder::new().with_child(palette)).build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(250.0).with_height(350.0))
            .open(false)
            .with_title(WindowTitle::text("Tile Map Control Panel"))
            .with_content(content)
            .build(ctx);

        ctx.sender()
            .send(WindowMessage::open_and_align(
                window,
                MessageDirection::ToWidget,
                scene_frame,
                HorizontalAlignment::Right,
                VerticalAlignment::Top,
                Thickness::uniform(2.0),
                false,
                true,
            ))
            .unwrap();

        Self { window, palette }
    }

    pub fn destroy(self, ui: &UserInterface) {
        ui.send_message(WidgetMessage::remove(
            self.window,
            MessageDirection::ToWidget,
        ));
    }

    pub fn handle_ui_message(self, message: &UiMessage, ui: &UserInterface) -> Option<Self> {
        if let Some(WindowMessage::Close) = message.data() {
            if message.destination() == self.window {
                self.destroy(ui);
                return None;
            }
        }

        Some(self)
    }
}
