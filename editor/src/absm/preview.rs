use crate::preview::PreviewPanel;
use fyrox::gui::message::UiMessage;
use fyrox::{
    core::pool::Handle,
    engine::Engine,
    gui::{
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        UiNode,
    },
};

pub struct Previewer {
    pub window: Handle<UiNode>,
    pub panel: PreviewPanel,
}

impl Previewer {
    pub fn new(engine: &mut Engine) -> Self {
        let panel = PreviewPanel::new(engine, 300, 300);

        let ctx = &mut engine.user_interface.build_ctx();
        let window = WindowBuilder::new(WidgetBuilder::new())
            .with_title(WindowTitle::text("Previewer"))
            .with_content(panel.root)
            .build(ctx);

        Self { window, panel }
    }

    pub fn handle_message(&mut self, message: &UiMessage, engine: &mut Engine) {
        self.panel.handle_message(message, engine)
    }

    pub fn update(&mut self, engine: &mut Engine) {
        self.panel.update(engine)
    }
}
