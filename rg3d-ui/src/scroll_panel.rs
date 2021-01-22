use crate::draw::Draw;
use crate::{
    brush::Brush,
    core::{algebra::Vector2, color::Color, math::Rect, pool::Handle, scope_profile},
    draw::{CommandTexture, DrawingContext},
    message::{MessageData, MessageDirection, ScrollPanelMessage, UiMessage, UiMessageData},
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UINode, UserInterface,
};
use std::ops::{Deref, DerefMut};

/// Allows user to scroll content
#[derive(Clone)]
pub struct ScrollPanel<M: MessageData, C: Control<M, C>> {
    widget: Widget<M, C>,
    scroll: Vector2<f32>,
    vertical_scroll_allowed: bool,
    horizontal_scroll_allowed: bool,
}

crate::define_widget_deref!(ScrollPanel<M, C>);

impl<M: MessageData, C: Control<M, C>> Control<M, C> for ScrollPanel<M, C> {
    fn measure_override(
        &self,
        ui: &UserInterface<M, C>,
        available_size: Vector2<f32>,
    ) -> Vector2<f32> {
        scope_profile!();

        let size_for_child = Vector2::new(
            if self.horizontal_scroll_allowed {
                std::f32::INFINITY
            } else {
                available_size.x
            },
            if self.vertical_scroll_allowed {
                std::f32::INFINITY
            } else {
                available_size.y
            },
        );

        let mut desired_size = Vector2::default();

        for child_handle in self.widget.children() {
            ui.node(*child_handle).measure(ui, size_for_child);

            let child = ui.nodes.borrow(*child_handle);
            let child_desired_size = child.desired_size();
            if child_desired_size.x > desired_size.x {
                desired_size.x = child_desired_size.x;
            }
            if child_desired_size.y > desired_size.y {
                desired_size.y = child_desired_size.y;
            }
        }

        desired_size
    }

    fn arrange_override(&self, ui: &UserInterface<M, C>, final_size: Vector2<f32>) -> Vector2<f32> {
        scope_profile!();

        let mut children_size = Vector2::<f32>::default();
        for child_handle in self.widget.children() {
            let desired_size = ui.node(*child_handle).desired_size();
            children_size.x = children_size.x.max(desired_size.x);
            children_size.y = children_size.y.max(desired_size.y);
        }

        let child_rect = Rect::new(
            -self.scroll.x,
            -self.scroll.y,
            if self.horizontal_scroll_allowed {
                children_size.x.max(final_size.x)
            } else {
                final_size.x
            },
            if self.vertical_scroll_allowed {
                children_size.y.max(final_size.y)
            } else {
                final_size.y
            },
        );

        for child_handle in self.widget.children() {
            ui.node(*child_handle).arrange(ui, &child_rect);
        }

        final_size
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        // Emit transparent geometry so panel will receive mouse events.
        drawing_context.push_rect_filled(&self.widget.screen_bounds(), None);
        drawing_context.commit(
            self.clip_bounds(),
            Brush::Solid(Color::TRANSPARENT),
            CommandTexture::None,
            None,
        );
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle() {
            if let UiMessageData::ScrollPanel(msg) = &message.data() {
                match *msg {
                    ScrollPanelMessage::VerticalScroll(scroll) => {
                        self.scroll.y = scroll;
                        self.invalidate_layout();
                    }
                    ScrollPanelMessage::HorizontalScroll(scroll) => {
                        self.scroll.x = scroll;
                        self.invalidate_layout();
                    }
                    ScrollPanelMessage::BringIntoView(handle) => {
                        let mut parent = handle;
                        let mut relative_position = Vector2::default();
                        while parent.is_some() && parent != self.handle {
                            let node = ui.node(parent);
                            relative_position += node.actual_local_position();
                            parent = node.parent();
                        }
                        // Check if requested item already in "view box", this will prevent weird "jumping" effect
                        // when bring into view was requested on already visible element.
                        if relative_position.x < 0.0
                            || relative_position.y < 0.0
                            || relative_position.x > self.actual_size().x
                            || relative_position.y > self.actual_size().y
                        {
                            relative_position += self.scroll;
                            // This check is needed because it possible that given handle is not in
                            // sub-tree of current scroll panel.
                            if parent == self.handle {
                                if self.vertical_scroll_allowed {
                                    ui.send_message(ScrollPanelMessage::vertical_scroll(
                                        self.handle,
                                        MessageDirection::ToWidget,
                                        relative_position.y,
                                    ));
                                }
                                if self.horizontal_scroll_allowed {
                                    ui.send_message(ScrollPanelMessage::horizontal_scroll(
                                        self.handle,
                                        MessageDirection::ToWidget,
                                        relative_position.x,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

impl<M: MessageData, C: Control<M, C>> ScrollPanel<M, C> {
    pub fn new(widget: Widget<M, C>) -> Self {
        Self {
            widget,
            scroll: Default::default(),
            vertical_scroll_allowed: true,
            horizontal_scroll_allowed: false,
        }
    }

    pub fn set_vertical_scroll_allowed(&mut self, state: bool) {
        if self.vertical_scroll_allowed != state {
            self.vertical_scroll_allowed = state;
            self.widget.invalidate_layout();
        }
    }

    pub fn set_horizontal_scroll_allowed(&mut self, state: bool) {
        if self.horizontal_scroll_allowed != state {
            self.horizontal_scroll_allowed = state;
            self.widget.invalidate_layout();
        }
    }
}

pub struct ScrollPanelBuilder<M: MessageData, C: Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    vertical_scroll_allowed: Option<bool>,
    horizontal_scroll_allowed: Option<bool>,
}

impl<M: MessageData, C: Control<M, C>> ScrollPanelBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            vertical_scroll_allowed: None,
            horizontal_scroll_allowed: None,
        }
    }

    pub fn with_vertical_scroll_allowed(mut self, value: bool) -> Self {
        self.vertical_scroll_allowed = Some(value);
        self
    }

    pub fn with_horizontal_scroll_allowed(mut self, value: bool) -> Self {
        self.horizontal_scroll_allowed = Some(value);
        self
    }

    pub fn build(self, ui: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        ui.add_node(UINode::ScrollPanel(ScrollPanel {
            widget: self.widget_builder.build(),
            scroll: Vector2::default(),
            vertical_scroll_allowed: self.vertical_scroll_allowed.unwrap_or(true),
            horizontal_scroll_allowed: self.horizontal_scroll_allowed.unwrap_or(false),
        }))
    }
}
