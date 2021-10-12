use rg3d::{
    core::{algebra::Vector2, pool::Handle},
    gui::{
        menu::{MenuItemBuilder, MenuItemContent},
        message::{PopupMessage, UiMessage, UiMessageData},
        popup::{Placement, PopupBuilder},
        stack_panel::StackPanelBuilder,
        widget::WidgetBuilder,
        BuildContext, UiNode,
    },
};

pub struct LinkContextMenu {
    pub menu: Handle<UiNode>,
    pub unlink: Handle<UiNode>,
    /// A link node above which the menu was opened.
    pub target: Handle<UiNode>,
}

impl LinkContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let unlink;
        let menu = PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
            .with_content(
                StackPanelBuilder::new(WidgetBuilder::new().with_child({
                    unlink = MenuItemBuilder::new(
                        WidgetBuilder::new().with_min_size(Vector2::new(120.0, 20.0)),
                    )
                    .with_content(MenuItemContent::Text {
                        text: "Unlink",
                        shortcut: "",
                        icon: Default::default(),
                    })
                    .build(ctx);
                    unlink
                }))
                .build(ctx),
            )
            .build(ctx);

        Self {
            menu,
            unlink,
            target: Default::default(),
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage) {
        if let UiMessageData::Popup(PopupMessage::Placement(Placement::Cursor(target))) =
            message.data()
        {
            if message.destination() == self.menu {
                self.target = *target;
            }
        }
    }
}
