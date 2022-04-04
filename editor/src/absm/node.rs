use crate::absm::{
    selectable::{Selectable, SelectableMessage},
    BORDER_COLOR, NORMAL_BACKGROUND, SELECTED_BACKGROUND,
};
use fyrox::{
    animation::machine::state::StateDefinition,
    core::{algebra::Vector2, pool::Handle},
    gui::{
        border::BorderBuilder,
        brush::Brush,
        define_constructor, define_widget_deref,
        message::{MessageDirection, UiMessage},
        text::TextBuilder,
        widget::{Widget, WidgetBuilder, WidgetMessage},
        BuildContext, Control, HorizontalAlignment, Thickness, UiNode, UserInterface,
        VerticalAlignment,
    },
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

#[derive(Clone)]
pub struct AbsmStateNode {
    widget: Widget,
    background: Handle<UiNode>,
    selectable: Selectable,
    pub name: String,
    pub model_handle: Handle<StateDefinition>,
}

define_widget_deref!(AbsmStateNode);

#[derive(Debug, Clone, PartialEq)]
pub enum AbsmStateNodeMessage {
    Name(String),
}

impl AbsmStateNodeMessage {
    define_constructor!(AbsmStateNodeMessage:Name => fn name(String), layout: false);
}

impl Control for AbsmStateNode {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else if type_id == TypeId::of::<Selectable>() {
            Some(&self.selectable)
        } else {
            None
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);
        self.selectable
            .handle_routed_message(self.handle(), ui, message);

        if let Some(SelectableMessage::Select(selected)) = message.data() {
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::FromWidget
            {
                ui.send_message(WidgetMessage::background(
                    self.background,
                    MessageDirection::ToWidget,
                    Brush::Solid(if *selected {
                        SELECTED_BACKGROUND
                    } else {
                        NORMAL_BACKGROUND
                    }),
                ));

                if *selected {
                    ui.send_message(WidgetMessage::topmost(
                        self.handle(),
                        MessageDirection::ToWidget,
                    ));
                }
            }
        }
    }
}

pub struct AbsmStateNodeBuilder {
    widget_builder: WidgetBuilder,
    name: String,
}

impl AbsmStateNodeBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            name: "New State".to_string(),
        }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    pub fn build(
        self,
        model_handle: Handle<StateDefinition>,
        ctx: &mut BuildContext,
    ) -> Handle<UiNode> {
        let background = BorderBuilder::new(
            WidgetBuilder::new()
                .with_foreground(Brush::Solid(BORDER_COLOR))
                .with_background(Brush::Solid(NORMAL_BACKGROUND))
                .with_child(
                    TextBuilder::new(
                        WidgetBuilder::new()
                            .with_vertical_alignment(VerticalAlignment::Center)
                            .with_horizontal_alignment(HorizontalAlignment::Center),
                    )
                    .with_text(&self.name)
                    .build(ctx),
                ),
        )
        .with_stroke_thickness(Thickness::uniform(4.0))
        .build(ctx);

        let node = AbsmStateNode {
            widget: self
                .widget_builder
                .with_min_size(Vector2::new(200.0, 100.0))
                .with_child(background)
                .build(),
            background,
            selectable: Default::default(),
            model_handle,
            name: self.name,
        };

        ctx.add_node(UiNode::new(node))
    }
}
