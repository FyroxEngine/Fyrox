use fyrox::{
    core::pool::Handle,
    gui::{
        check_box::{CheckBoxBuilder, CheckBoxMessage},
        message::{MessageDirection, UiMessage},
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        widget::WidgetBuilder,
        BuildContext, Orientation, Thickness, UiNode, VerticalAlignment,
    },
};

pub struct Toolbar {
    pub panel: Handle<UiNode>,
    pub preview: Handle<UiNode>,
}

pub enum ToolbarAction {
    None,
    EnterPreviewMode,
    LeavePreviewMode,
}

impl Toolbar {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let preview;
        let panel = StackPanelBuilder::new(WidgetBuilder::new().with_child({
            preview =
                CheckBoxBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
                    .with_content(
                        TextBuilder::new(
                            WidgetBuilder::new().with_vertical_alignment(VerticalAlignment::Center),
                        )
                        .with_text("Preview")
                        .build(ctx),
                    )
                    .build(ctx);
            preview
        }))
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        Self { panel, preview }
    }

    pub fn handle_ui_message(&self, message: &UiMessage) -> ToolbarAction {
        if let Some(CheckBoxMessage::Check(Some(value))) = message.data() {
            if message.destination() == self.preview
                && message.direction() == MessageDirection::FromWidget
            {
                return if *value {
                    ToolbarAction::EnterPreviewMode
                } else {
                    ToolbarAction::LeavePreviewMode
                };
            }
        }

        ToolbarAction::None
    }
}
