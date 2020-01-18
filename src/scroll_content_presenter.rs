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
        UINode    ,

        ControlTemplate,
        UINodeContainer,
        Builder

};
use std::collections::HashMap;

/// Allows user to scroll content
pub struct ScrollContentPresenter {
    widget: Widget,
    scroll: Vec2,
    vertical_scroll_allowed: bool,
    horizontal_scroll_allowed: bool,
}

impl Control for ScrollContentPresenter {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget {
        &mut self.widget
    }

    fn raw_copy(&self) -> Box<dyn Control> {
        Box::new(Self {
            widget: *self.widget.raw_copy().downcast::<Widget>().unwrap_or_else(|_| panic!()),
            scroll: self.scroll,
            vertical_scroll_allowed: self.vertical_scroll_allowed,
            horizontal_scroll_allowed: self.horizontal_scroll_allowed
        })
    }

    fn resolve(&mut self, _: &ControlTemplate, _: &HashMap<Handle<UINode>, Handle<UINode>>) {

    }

    fn measure_override(&self, ui: &UserInterface, available_size: Vec2) -> Vec2 {
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

        for child_handle in self.widget.children.iter() {
            ui.node(*child_handle).measure(ui, size_for_child);

            let child = ui.nodes.borrow(*child_handle).widget();
            let child_desired_size = child.desired_size.get();
            if child_desired_size.x > desired_size.x {
                desired_size.x = child_desired_size.x;
            }
            if child_desired_size.y > desired_size.y {
                desired_size.y = child_desired_size.y;
            }
        }

        desired_size
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vec2) -> Vec2 {
        let child_rect = Rect::new(
            -self.scroll.x,
            -self.scroll.y,
            final_size.x + self.scroll.x,
            final_size.y + self.scroll.y,
        );

        for child_handle in self.widget.children.iter() {
            ui.node(*child_handle).arrange(ui, &child_rect);
        }

        final_size
    }
}

impl ScrollContentPresenter {
    pub fn new(widget: Widget) -> Self {
        Self {
            widget,
            scroll: Default::default(),
            vertical_scroll_allowed: true,
            horizontal_scroll_allowed: false
        }
    }

    pub fn set_scroll(&mut self, scroll: Vec2) {
        self.scroll = scroll;
    }

    pub fn set_vertical_scroll(&mut self, scroll: f32) {
        self.scroll.y = scroll;
    }

    pub fn set_horizontal_scroll(&mut self, scroll: f32) {
        self.scroll.x = scroll;
    }

    pub fn set_vertical_scroll_allowed(&mut self, state: bool) {
        self.vertical_scroll_allowed = state;
    }

    pub fn set_horizontal_scroll_allowed(&mut self, state: bool) {
        self.horizontal_scroll_allowed = state;
    }
}

pub struct ScrollContentPresenterBuilder {
    widget_builder: WidgetBuilder,
    vertical_scroll_allowed: Option<bool>,
    horizontal_scroll_allowed: Option<bool>,
}

impl ScrollContentPresenterBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
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
}

impl Builder for ScrollContentPresenterBuilder {
    fn build(self, ui: &mut dyn UINodeContainer) -> Handle<UINode> {
        ui.add_node(Box::new(ScrollContentPresenter {
            widget: self.widget_builder.build(),
            scroll: Vec2::ZERO,
            vertical_scroll_allowed: self.vertical_scroll_allowed.unwrap_or(true),
            horizontal_scroll_allowed: self.horizontal_scroll_allowed.unwrap_or(false),
        }))
    }
}