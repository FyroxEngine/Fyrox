use crate::{
    fyrox::{
        core::pool::Handle,
        gui::{
            grid::GridBuilder,
            message::{MessageDirection, UiMessage},
            widget::{WidgetBuilder, WidgetMessage},
            window::{WindowBuilder, WindowMessage, WindowTitle},
            BuildContext, HorizontalAlignment, Thickness, UiNode, UserInterface, VerticalAlignment,
        },
    },
    plugins::tilemap::palette::PaletteWidgetBuilder,
};

pub struct TileMapPanel {
    pub window: Handle<UiNode>,
}

impl TileMapPanel {
    pub fn new(ctx: &mut BuildContext, scene_frame: Handle<UiNode>) -> Self {
        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(PaletteWidgetBuilder::new(WidgetBuilder::new()).build(ctx)),
        )
        .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(250.0).with_height(150.0))
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

        Self { window }
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
