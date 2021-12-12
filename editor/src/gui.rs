use rg3d::{
    core::pool::Handle,
    gui::{
        border::BorderBuilder,
        decorator::DecoratorBuilder,
        define_constructor,
        message::{MessageDirection, UiMessage},
        text::TextBuilder,
        widget::WidgetBuilder,
        BuildContext, HorizontalAlignment, UiNode, VerticalAlignment,
    },
};

#[derive(Debug, Clone, PartialEq)]
pub enum AssetItemMessage {
    Select(bool),
}

pub fn make_dropdown_list_option(ctx: &mut BuildContext, name: &str) -> Handle<UiNode> {
    DecoratorBuilder::new(BorderBuilder::new(
        WidgetBuilder::new().with_height(26.0).with_child(
            TextBuilder::new(WidgetBuilder::new())
                .with_vertical_text_alignment(VerticalAlignment::Center)
                .with_horizontal_text_alignment(HorizontalAlignment::Center)
                .with_text(name)
                .build(ctx),
        ),
    ))
    .build(ctx)
}

impl AssetItemMessage {
    define_constructor!(AssetItemMessage:Select => fn select(bool), layout: false);
}
