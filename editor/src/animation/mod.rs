use fyrox::{
    core::pool::Handle,
    gui::{
        message::MessageDirection,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, UiNode, UserInterface,
    },
};

pub struct AnimationEditor {
    pub window: Handle<UiNode>,
}

impl AnimationEditor {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(600.0).with_height(500.0))
            .open(false)
            .with_title(WindowTitle::text("Animation Editor"))
            .build(ctx);

        Self { window }
    }

    pub fn open(&self, ui: &UserInterface) {
        ui.send_message(WindowMessage::open(
            self.window,
            MessageDirection::ToWidget,
            true,
        ));
    }
}
