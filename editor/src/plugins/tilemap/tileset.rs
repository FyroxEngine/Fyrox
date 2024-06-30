use crate::fyrox::{
    core::pool::Handle,
    gui::{
        button::ButtonBuilder,
        grid::{Column, GridBuilder, Row},
        list_view::ListViewBuilder,
        message::{MessageDirection, UiMessage},
        stack_panel::StackPanelBuilder,
        utils::make_simple_tooltip,
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        wrap_panel::WrapPanelBuilder,
        BuildContext, Orientation, Thickness, UiNode, UserInterface,
    },
    scene::tilemap::tileset::TileSetResource,
};

#[allow(dead_code)]
pub struct TileSetEditor {
    window: Handle<UiNode>,
    tiles: Handle<UiNode>,
    tile_set: TileSetResource,
}

impl TileSetEditor {
    pub fn new(tile_set: TileSetResource, ctx: &mut BuildContext) -> Self {
        let import;
        let buttons = StackPanelBuilder::new(WidgetBuilder::new().on_row(0).with_child({
            import = ButtonBuilder::new(
                WidgetBuilder::new()
                    .with_width(100.0)
                    .with_height(24.0)
                    .with_margin(Thickness::uniform(1.0))
                    .with_tooltip(make_simple_tooltip(
                        ctx,
                        "Import tile set from a sprite sheet.",
                    )),
            )
            .with_text("Import...")
            .build(ctx);
            import
        }))
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        let tiles = ListViewBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .with_margin(Thickness::uniform(1.0)),
        )
        .with_items_panel(
            WrapPanelBuilder::new(WidgetBuilder::new())
                .with_orientation(Orientation::Horizontal)
                .build(ctx),
        )
        .build(ctx);

        let content = GridBuilder::new(WidgetBuilder::new().with_child(buttons).with_child(tiles))
            .add_row(Row::auto())
            .add_row(Row::stretch())
            .add_column(Column::stretch())
            .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
            .open(false)
            .with_title(WindowTitle::text("Tile Set Editor"))
            .with_content(content)
            .build(ctx);

        ctx.sender()
            .send(WindowMessage::open(
                window,
                MessageDirection::ToWidget,
                true,
                true,
            ))
            .unwrap();

        Self {
            window,
            tiles,
            tile_set,
        }
    }

    fn destroy(self, ui: &UserInterface) {
        ui.send_message(WidgetMessage::remove(
            self.window,
            MessageDirection::ToWidget,
        ));
    }

    pub fn handle_ui_message(self, message: &UiMessage, ui: &UserInterface) -> Option<Self> {
        if let Some(WindowMessage::Close) = message.data() {
            if message.destination() == self.window
                && message.direction() == MessageDirection::FromWidget
            {
                self.destroy(ui);
                return None;
            }
        }

        Some(self)
    }
}
