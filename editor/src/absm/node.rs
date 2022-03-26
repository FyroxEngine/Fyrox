use crate::absm::{BORDER_COLOR, NORMAL_BACKGROUND, SELECTED_BACKGROUND};
use fyrox::animation::machine::state::StateDefinition;
use fyrox::{
    core::{algebra::Vector2, pool::Handle},
    gui::{
        border::BorderBuilder,
        brush::Brush,
        define_constructor, define_widget_deref,
        message::{MessageDirection, MouseButton, UiMessage},
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
    selected: bool,
    pub model_handle: Handle<StateDefinition>,
}

define_widget_deref!(AbsmStateNode);

#[derive(Debug, Clone, PartialEq)]
pub enum AbsmStateNodeMessage {
    Select(bool),
}

impl AbsmStateNodeMessage {
    define_constructor!(AbsmStateNodeMessage:Select => fn select(bool), layout: false);
}

impl Control for AbsmStateNode {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(WidgetMessage::MouseDown { button, .. }) = message.data() {
            if *button == MouseButton::Left || *button == MouseButton::Right {
                message.set_handled(true);

                ui.send_message(AbsmStateNodeMessage::select(
                    self.handle(),
                    MessageDirection::ToWidget,
                    true,
                ));

                ui.send_message(WidgetMessage::topmost(
                    self.handle(),
                    MessageDirection::ToWidget,
                ));

                ui.capture_mouse(self.handle());
            }
        } else if let Some(WidgetMessage::MouseUp { button, .. }) = message.data() {
            if *button == MouseButton::Left || *button == MouseButton::Right {
                ui.release_mouse_capture();
            }
        } else if let Some(AbsmStateNodeMessage::Select(state)) = message.data() {
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::ToWidget
                && self.selected != *state
            {
                self.selected = *state;

                ui.send_message(WidgetMessage::background(
                    self.background,
                    MessageDirection::ToWidget,
                    Brush::Solid(if self.selected {
                        SELECTED_BACKGROUND
                    } else {
                        NORMAL_BACKGROUND
                    }),
                ));
            }
        }
    }
}

pub struct AbsmStateNodeBuilder {
    widget_builder: WidgetBuilder,
}

impl AbsmStateNodeBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self { widget_builder }
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
                    .with_text("State")
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
            selected: false,
            model_handle,
        };

        ctx.add_node(UiNode::new(node))
    }
}
