use crate::fyrox::{
    core::pool::Handle,
    gui::{
        formatted_text::WrapMode,
        message::MessageDirection,
        scroll_viewer::ScrollViewerBuilder,
        text::TextMessage,
        text_box::TextBoxBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, Thickness, UiNode, UserInterface,
    },
};

pub struct DocWindow {
    pub window: Handle<UiNode>,
    text: Handle<UiNode>,
}

impl DocWindow {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let text;
        let window = WindowBuilder::new(
            WidgetBuilder::new()
                .with_name("DocPanel")
                .with_width(400.0)
                .with_height(300.0),
        )
        .open(false)
        .with_content(
            ScrollViewerBuilder::new(WidgetBuilder::new())
                .with_content({
                    text = TextBoxBuilder::new(
                        WidgetBuilder::new().with_margin(Thickness::uniform(3.0)),
                    )
                    .with_editable(false)
                    .with_wrap(WrapMode::Word)
                    .build(ctx);
                    text
                })
                .build(ctx),
        )
        .with_title(WindowTitle::text("Documentation"))
        .build(ctx);
        Self { window, text }
    }

    pub fn open(&self, doc: String, ui: &UserInterface) {
        ui.send_message(TextMessage::text(
            self.text,
            MessageDirection::ToWidget,
            doc,
        ));
        ui.send_message(WindowMessage::open(
            self.window,
            MessageDirection::ToWidget,
            true,
            true,
        ));
    }
}
