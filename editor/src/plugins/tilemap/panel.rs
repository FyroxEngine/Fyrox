use crate::{
    fyrox::{
        asset::manager::ResourceManager,
        core::pool::Handle,
        gui::{
            grid::GridBuilder,
            message::{MessageDirection, UiMessage},
            widget::{WidgetBuilder, WidgetMessage},
            window::{WindowBuilder, WindowMessage, WindowTitle},
            BuildContext, HorizontalAlignment, Thickness, UiNode, UserInterface, VerticalAlignment,
        },
        scene::tilemap::TileMap,
    },
    plugins::{tilemap::palette::PaletteMessage, tilemap::palette::PaletteWidgetBuilder},
};

pub struct TileMapPanel {
    pub window: Handle<UiNode>,
    pub palette: Handle<UiNode>,
}

impl TileMapPanel {
    pub fn new(
        ctx: &mut BuildContext,
        scene_frame: Handle<UiNode>,
        resource_manager: ResourceManager,
        tile_map: &TileMap,
    ) -> Self {
        let palette = PaletteWidgetBuilder::new(WidgetBuilder::new())
            .with_brush(tile_map.active_brush())
            .with_tile_set(tile_map.tile_set().cloned())
            .build(resource_manager, ctx);

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

    pub fn sync_to_model(&self, ui: &UserInterface, tile_map: &TileMap) {
        ui.send_message(PaletteMessage::brush_resource(
            self.palette,
            MessageDirection::ToWidget,
            tile_map.active_brush(),
        ));

        ui.send_message(PaletteMessage::tile_set(
            self.palette,
            MessageDirection::ToWidget,
            tile_map.tile_set().cloned(),
        ));
    }
}
