use crate::gui::{builder::{CommonBuilderFields, GenericNodeBuilder}, node::{UINode, UINodeKind}, UserInterface, Layout, EventSource};
use rg3d_core::{
    pool::Handle,
    math::{
        vec2::Vec2,
        Rect,
    },
};
use crate::gui::event::UIEvent;

pub struct ScrollContentPresenter {
    scroll: Vec2,
    vertical_scroll_allowed: bool,
    horizontal_scroll_allowed: bool,
}

impl Layout for ScrollContentPresenter {
    fn measure_override(&self, self_handle: Handle<UINode>, ui: &UserInterface, available_size: Vec2) -> Vec2 {
        let size_for_child = Vec2::make(
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

        let mut desired_size = Vec2::zero();

        let node = ui.nodes.borrow(self_handle);
        for child_handle in node.children.iter() {
            ui.measure(*child_handle, size_for_child);

            let child = ui.nodes.borrow(*child_handle);
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

    fn arrange_override(&self, self_handle: Handle<UINode>, ui: &UserInterface, final_size: Vec2) -> Vec2 {
        let child_rect = Rect::new(
            -self.scroll.x,
            -self.scroll.y,
            final_size.x + self.scroll.x,
            final_size.y + self.scroll.y,
        );

        let node = ui.nodes.borrow(self_handle);
        for child_handle in node.children.iter() {
            ui.arrange(*child_handle, &child_rect);
        }

        final_size
    }
}

impl ScrollContentPresenter {
    fn new() -> Self {
        Self {
            scroll: Vec2::zero(),
            vertical_scroll_allowed: true,
            horizontal_scroll_allowed: false,
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
}

pub struct ScrollContentPresenterBuilder {
    vertical_scroll_allowed: Option<bool>,
    horizontal_scroll_allowed: Option<bool>,
    content: Handle<UINode>,
    common: CommonBuilderFields,
}

impl Default for ScrollContentPresenterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ScrollContentPresenterBuilder {
    pub fn new() -> Self {
        Self {
            vertical_scroll_allowed: None,
            horizontal_scroll_allowed: None,
            common: CommonBuilderFields::new(),
            content: Handle::NONE,
        }
    }

    impl_default_builder_methods!();

    pub fn with_content(mut self, content: Handle<UINode>) -> Self {
        self.content = content;
        self
    }

    pub fn with_vertical_scroll_allowed(mut self, value: bool) -> Self {
        self.vertical_scroll_allowed = Some(value);
        self
    }

    pub fn with_horizontal_scroll_allowed(mut self, value: bool) -> Self {
        self.horizontal_scroll_allowed = Some(value);
        self
    }

    pub fn build(self, ui: &mut UserInterface) -> Handle<UINode> {
        let mut scp = ScrollContentPresenter::new();
        if let Some(vertical_scroll_allowed) = self.vertical_scroll_allowed {
            scp.vertical_scroll_allowed = vertical_scroll_allowed;
        }
        if let Some(horizontal_scroll_allowed) = self.horizontal_scroll_allowed {
            scp.horizontal_scroll_allowed = horizontal_scroll_allowed;
        }
        GenericNodeBuilder::new(UINodeKind::ScrollContentPresenter(scp), self.common)
            .with_child(self.content)
            .build(ui)
    }
}

impl EventSource for ScrollContentPresenter {
    fn emit_event(&mut self) -> Option<UIEvent> {
        None
    }
}