use crate::absm::canvas::AbsmCanvasBuilder;
use fyrox::{
    core::pool::Handle,
    gui::{
        border::BorderBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        BuildContext, UiNode,
    },
};

pub struct Document {
    pub window: Handle<UiNode>,
    pub canvas: Handle<UiNode>,
}

impl Document {
    pub fn new(context_menu: Handle<UiNode>, ctx: &mut BuildContext) -> Self {
        let canvas =
            AbsmCanvasBuilder::new(WidgetBuilder::new().with_context_menu(context_menu)).build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new())
            .with_title(WindowTitle::text("Document"))
            .with_content(BorderBuilder::new(WidgetBuilder::new().with_child(canvas)).build(ctx))
            .build(ctx);

        Self { window, canvas }
    }
}
