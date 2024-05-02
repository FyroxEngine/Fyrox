//! A simple widget that opens a popup when clicked. It could be used to create drop down menus that
//! consolidates content of a group.

use crate::{
    core::{pool::Handle, reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*},
    message::{MessageDirection, MouseButton, UiMessage},
    popup::{Placement, PopupBuilder, PopupMessage},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, UiNode, UserInterface,
};
use std::ops::{Deref, DerefMut};

/// A simple widget that opens a popup when clicked. It could be used to create drop down menus that
/// consolidates content of a group.
#[derive(Default, Clone, Visit, Reflect, Debug, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "c0a4c51b-f041-453b-a89d-7ceb5394e321")]
pub struct DropdownMenu {
    /// Base widget of the dropdown menu.
    pub widget: Widget,
    /// A handle of the inner popup, that stores the content of the menu.
    pub popup: Handle<UiNode>,
}

crate::define_widget_deref!(DropdownMenu);

impl Control for DropdownMenu {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(WidgetMessage::MouseDown { button, .. }) = message.data() {
            if *button == MouseButton::Left {
                ui.send_message(PopupMessage::placement(
                    self.popup,
                    MessageDirection::ToWidget,
                    Placement::LeftBottom(self.handle),
                ));
                ui.send_message(PopupMessage::open(self.popup, MessageDirection::ToWidget));
            }
        }
    }
}

/// Canvas builder creates new [`DropdownMenu`] widget instances and adds them to the user interface.
pub struct DropdownMenuBuilder {
    widget_builder: WidgetBuilder,
    header: Handle<UiNode>,
    content: Handle<UiNode>,
}

impl DropdownMenuBuilder {
    /// Creates new builder instance.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            header: Handle::NONE,
            content: Handle::NONE,
        }
    }

    /// Sets the desired header.
    pub fn with_header(mut self, header: Handle<UiNode>) -> Self {
        self.header = header;
        self
    }

    /// Sets the content of the menu.
    pub fn with_content(mut self, content: Handle<UiNode>) -> Self {
        self.content = content;
        self
    }

    /// Finishes dropdown menu widget building and adds the instance to the user interface and
    /// returns its handle.
    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let popup = PopupBuilder::new(WidgetBuilder::new())
            .stays_open(false)
            .with_content(self.content)
            .build(ctx);

        let dropdown_menu = DropdownMenu {
            widget: self.widget_builder.with_child(self.header).build(),
            popup,
        };
        ctx.add_node(UiNode::new(dropdown_menu))
    }
}
