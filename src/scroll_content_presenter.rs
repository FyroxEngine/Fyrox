use crate::{
    core::{
        pool::Handle,
        math::{
            vec2::Vec2,
            Rect,
        },
    },
    widget::{
        Widget,
        WidgetBuilder,
    },
    UserInterface, Control,
    UINode,
};

/// Allows user to scroll content
pub struct ScrollContentPresenter<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    scroll: Vec2,
    vertical_scroll_allowed: bool,
    horizontal_scroll_allowed: bool,
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for ScrollContentPresenter<M, C> {
    fn widget(&self) -> &Widget<M, C> {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget<M, C> {
        &mut self.widget
    }

    fn measure_override(&self, ui: &UserInterface<M, C>, available_size: Vec2) -> Vec2 {
        let size_for_child = Vec2::new(
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

        let mut desired_size = Vec2::ZERO;

        for child_handle in self.widget.children() {
            ui.node(*child_handle).measure(ui, size_for_child);

            let child = ui.nodes.borrow(*child_handle).widget();
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

    fn arrange_override(&self, ui: &UserInterface<M, C>, final_size: Vec2) -> Vec2 {
        let child_rect = Rect::new(
            -self.scroll.x,
            -self.scroll.y,
            final_size.x + self.scroll.x,
            final_size.y + self.scroll.y,
        );

        for child_handle in self.widget.children() {
            ui.node(*child_handle).arrange(ui, &child_rect);
        }

        final_size
    }
}

impl<M, C: 'static + Control<M, C>> ScrollContentPresenter<M, C> {
    pub fn new(widget: Widget<M, C>) -> Self {
        Self {
            widget,
            scroll: Default::default(),
            vertical_scroll_allowed: true,
            horizontal_scroll_allowed: false,
        }
    }

    pub fn set_scroll(&mut self, scroll: Vec2) {
        if self.scroll != scroll {
            self.scroll = scroll;
            self.widget.invalidate_layout();
        }
    }

    pub fn set_vertical_scroll(&mut self, scroll: f32) {
        if self.scroll.y != scroll {
            self.scroll.y = scroll;
            self.widget.invalidate_layout();
        }
    }

    pub fn set_horizontal_scroll(&mut self, scroll: f32) {
        if self.scroll.x != scroll {
            self.scroll.x = scroll;
            self.widget.invalidate_layout();
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

pub struct ScrollContentPresenterBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    vertical_scroll_allowed: Option<bool>,
    horizontal_scroll_allowed: Option<bool>,
}

impl<M, C: 'static + Control<M, C>> ScrollContentPresenterBuilder<M, C> {
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

    pub fn build(self, ui: &mut UserInterface<M, C>) -> Handle<UINode<M, C>> {
        let handle = ui.add_node(UINode::ScrollContentPresenter(ScrollContentPresenter {
            widget: self.widget_builder.build(),
            scroll: Vec2::ZERO,
            vertical_scroll_allowed: self.vertical_scroll_allowed.unwrap_or(true),
            horizontal_scroll_allowed: self.horizontal_scroll_allowed.unwrap_or(false),
        }));

        ui.flush_messages();

        handle
    }
}