use crate::fyrox::{
    core::{pool::Handle, reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*},
    gui::{
        define_widget_deref,
        message::UiMessage,
        widget::{Widget, WidgetBuilder},
        BuildContext, Control, UiNode, UserInterface,
    },
};
use std::ops::{Deref, DerefMut};

#[derive(Clone, Debug, Visit, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "5356a864-c026-4bd7-a4b1-30bacf77d8fa")]
pub struct PaletteWidget {
    widget: Widget,
    tiles: Vec<Handle<UiNode>>,
}

define_widget_deref!(PaletteWidget);

impl Control for PaletteWidget {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message)
    }
}

pub struct PaletteWidgetBuilder {
    widget_builder: WidgetBuilder,
}

impl PaletteWidgetBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self { widget_builder }
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        ctx.add_node(UiNode::new(PaletteWidget {
            widget: self.widget_builder.build(),
        }))
    }
}
