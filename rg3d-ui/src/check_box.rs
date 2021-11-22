use crate::{
    border::BorderBuilder,
    brush::Brush,
    core::{color::Color, pool::Handle},
    define_constructor,
    grid::{Column, GridBuilder, Row},
    message::{MessageDirection, UiMessage},
    vector_image::{Primitive, VectorImageBuilder},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, HorizontalAlignment, NodeHandleMapping, Thickness, UiNode,
    UserInterface, VerticalAlignment, BRUSH_BRIGHT, BRUSH_DARK, BRUSH_LIGHT, BRUSH_TEXT,
};
use rg3d_core::algebra::Vector2;
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone, PartialEq)]
pub enum CheckBoxMessage {
    Check(Option<bool>),
}

impl CheckBoxMessage {
    define_constructor!(CheckBoxMessage:Check => fn checked(Option<bool>), layout: false);
}

#[derive(Clone)]
pub struct CheckBox {
    pub widget: Widget,
    pub checked: Option<bool>,
    pub check_mark: Handle<UiNode>,
    pub uncheck_mark: Handle<UiNode>,
    pub undefined_mark: Handle<UiNode>,
}

crate::define_widget_deref!(CheckBox);

impl Control for CheckBox {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        node_map.resolve(&mut self.check_mark);
        node_map.resolve(&mut self.uncheck_mark);
        node_map.resolve(&mut self.undefined_mark);
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<WidgetMessage>() {
            match msg {
                WidgetMessage::MouseDown { .. } => {
                    if message.destination() == self.handle()
                        || self.widget.has_descendant(message.destination(), ui)
                    {
                        ui.capture_mouse(self.handle());
                    }
                }
                WidgetMessage::MouseUp { .. } => {
                    if message.destination() == self.handle()
                        || self.widget.has_descendant(message.destination(), ui)
                    {
                        ui.release_mouse_capture();

                        if let Some(value) = self.checked {
                            // Invert state if it is defined.
                            ui.send_message(CheckBoxMessage::checked(
                                self.handle(),
                                MessageDirection::ToWidget,
                                Some(!value),
                            ));
                        } else {
                            // Switch from undefined state to checked.
                            ui.send_message(CheckBoxMessage::checked(
                                self.handle(),
                                MessageDirection::ToWidget,
                                Some(true),
                            ));
                        }
                    }
                }
                _ => (),
            }
        } else if let Some(&CheckBoxMessage::Check(value)) = message.data::<CheckBoxMessage>() {
            if message.direction() == MessageDirection::ToWidget
                && message.destination() == self.handle()
                && self.checked != value
            {
                self.checked = value;

                ui.send_message(message.reverse());

                if self.check_mark.is_some() {
                    match value {
                        None => {
                            ui.send_message(WidgetMessage::visibility(
                                self.check_mark,
                                MessageDirection::ToWidget,
                                false,
                            ));
                            ui.send_message(WidgetMessage::visibility(
                                self.uncheck_mark,
                                MessageDirection::ToWidget,
                                false,
                            ));
                            ui.send_message(WidgetMessage::visibility(
                                self.undefined_mark,
                                MessageDirection::ToWidget,
                                true,
                            ));
                        }
                        Some(value) => {
                            ui.send_message(WidgetMessage::visibility(
                                self.check_mark,
                                MessageDirection::ToWidget,
                                value,
                            ));
                            ui.send_message(WidgetMessage::visibility(
                                self.uncheck_mark,
                                MessageDirection::ToWidget,
                                !value,
                            ));
                            ui.send_message(WidgetMessage::visibility(
                                self.undefined_mark,
                                MessageDirection::ToWidget,
                                false,
                            ));
                        }
                    }
                }
            }
        }
    }
}

pub struct CheckBoxBuilder {
    widget_builder: WidgetBuilder,
    checked: Option<bool>,
    check_mark: Option<Handle<UiNode>>,
    uncheck_mark: Option<Handle<UiNode>>,
    undefined_mark: Option<Handle<UiNode>>,
    content: Handle<UiNode>,
}

impl CheckBoxBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            checked: Some(false),
            check_mark: None,
            uncheck_mark: None,
            undefined_mark: None,
            content: Handle::NONE,
        }
    }

    pub fn checked(mut self, value: Option<bool>) -> Self {
        self.checked = value;
        self
    }

    pub fn with_check_mark(mut self, check_mark: Handle<UiNode>) -> Self {
        self.check_mark = Some(check_mark);
        self
    }

    pub fn with_uncheck_mark(mut self, uncheck_mark: Handle<UiNode>) -> Self {
        self.uncheck_mark = Some(uncheck_mark);
        self
    }

    pub fn with_undefined_mark(mut self, undefined_mark: Handle<UiNode>) -> Self {
        self.undefined_mark = Some(undefined_mark);
        self
    }

    pub fn with_content(mut self, content: Handle<UiNode>) -> Self {
        self.content = content;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let check_mark = self.check_mark.unwrap_or_else(|| {
            VectorImageBuilder::new(
                WidgetBuilder::new()
                    .with_vertical_alignment(VerticalAlignment::Center)
                    .with_horizontal_alignment(HorizontalAlignment::Center)
                    .with_foreground(BRUSH_TEXT),
            )
            .with_primitives(vec![
                Primitive::Line {
                    begin: Vector2::new(0.0, 6.0),
                    end: Vector2::new(6.0, 12.0),
                    thickness: 2.0,
                },
                Primitive::Line {
                    begin: Vector2::new(6.0, 12.0),
                    end: Vector2::new(12.0, 0.0),
                    thickness: 2.0,
                },
            ])
            .build(ctx)
        });
        ctx[check_mark].set_visibility(self.checked.unwrap_or(false));

        let uncheck_mark = self.uncheck_mark.unwrap_or_else(|| {
            BorderBuilder::new(
                WidgetBuilder::new()
                    .with_background(Brush::Solid(Color::TRANSPARENT))
                    .with_foreground(Brush::Solid(Color::TRANSPARENT)),
            )
            .build(ctx)
        });
        ctx[uncheck_mark].set_visibility(!self.checked.unwrap_or(true));

        let undefined_mark = self.undefined_mark.unwrap_or_else(|| {
            BorderBuilder::new(
                WidgetBuilder::new()
                    .with_margin(Thickness::uniform(1.0))
                    .with_background(BRUSH_BRIGHT)
                    .with_foreground(Brush::Solid(Color::TRANSPARENT)),
            )
            .build(ctx)
        });
        ctx[undefined_mark].set_visibility(self.checked.is_none());

        if self.content.is_some() {
            ctx[self.content].set_row(0).set_column(1);
        }

        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(
                    BorderBuilder::new(
                        WidgetBuilder::new()
                            .with_child(check_mark)
                            .with_child(uncheck_mark)
                            .with_child(undefined_mark)
                            .with_background(BRUSH_DARK)
                            .with_foreground(BRUSH_LIGHT),
                    )
                    .with_stroke_thickness(Thickness::uniform(1.0))
                    .build(ctx),
                )
                .with_child(self.content),
        )
        .add_row(Row::stretch())
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .build(ctx);

        let cb = CheckBox {
            widget: self.widget_builder.with_child(grid).build(),
            checked: self.checked,
            check_mark,
            uncheck_mark,
            undefined_mark,
        };
        ctx.add_node(UiNode::new(cb))
    }
}

#[cfg(test)]
mod test {
    use crate::check_box::CheckBoxMessage;
    use crate::{
        check_box::CheckBoxBuilder, core::algebra::Vector2, message::MessageDirection,
        widget::WidgetBuilder, UserInterface,
    };

    #[test]
    fn check_box() {
        let mut ui = UserInterface::new(Vector2::new(1000.0, 1000.0));

        assert_eq!(ui.poll_message(), None);

        let check_box = CheckBoxBuilder::new(WidgetBuilder::new()).build(&mut ui.build_ctx());

        assert_eq!(ui.poll_message(), None);

        // Check messages
        let input_message =
            CheckBoxMessage::checked(check_box, MessageDirection::ToWidget, Some(true));

        ui.send_message(input_message.clone());

        // This message that we just send.
        assert_eq!(ui.poll_message(), Some(input_message.clone()));
        // We must get response from check box.
        assert_eq!(ui.poll_message(), Some(input_message.reverse()));
    }
}
