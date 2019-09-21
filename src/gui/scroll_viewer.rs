use crate::gui::{
    UserInterface,
    maxf,
    builder::{GenericNodeBuilder, CommonBuilderFields},
    node::{UINodeKind, UINode},
    scroll_content_presenter::{ScrollContentPresenter, ScrollContentPresenterBuilder},
    scroll_bar::{ScrollBarBuilder, Orientation, ScrollBar},
    grid::{Row, GridBuilder, Column},
};

use rg3d_core::{
    pool::Handle,
    math::vec2::Vec2,
};

pub struct ScrollViewer {
    pub(in crate::gui) owner_handle: Handle<UINode>,
    content: Handle<UINode>,
    content_presenter: Handle<UINode>,
    v_scroll_bar: Handle<UINode>,
    h_scroll_bar: Handle<UINode>,
}

impl ScrollViewer {
    pub fn update(handle: Handle<UINode>, ui: &mut UserInterface) {
        let mut content_size = Vec2::zero();
        let mut available_size_for_content = Vec2::zero();
        let mut horizontal_scroll_bar_handle = Handle::none();
        let mut vertical_scroll_bar_handle = Handle::none();

        if let Some(node) = ui.nodes.borrow(handle) {
            if let UINodeKind::ScrollViewer(scroll_viewer) = node.get_kind() {
                horizontal_scroll_bar_handle = scroll_viewer.h_scroll_bar;
                vertical_scroll_bar_handle = scroll_viewer.v_scroll_bar;
                if let Some(content_presenter) = ui.nodes.borrow(scroll_viewer.content_presenter) {
                    available_size_for_content = content_presenter.desired_size.get();
                    for content_handle in content_presenter.children.iter() {
                        if let Some(content) = ui.nodes.borrow(*content_handle) {
                            let content_desired_size = content.desired_size.get();
                            if content_desired_size.x > content_size.x {
                                content_size.x = content_desired_size.x;
                            }
                            if content_desired_size.y > content_size.y {
                                content_size.y = content_desired_size.y;
                            }
                        }
                    }
                }
            }
        }

        // Then adjust scroll bars according to content size.
        ScrollBar::set_max_value(horizontal_scroll_bar_handle, ui, maxf(0.0, content_size.x - available_size_for_content.x));
        ScrollBar::set_max_value(vertical_scroll_bar_handle, ui, maxf(0.0, content_size.y - available_size_for_content.y));
    }
}

pub struct ScrollViewerBuilder {
    common: CommonBuilderFields,
    content: Handle<UINode>,
}

impl Default for ScrollViewerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ScrollViewerBuilder {
    pub fn new() -> Self {
        Self {
            common: CommonBuilderFields::new(),
            content: Handle::none(),
        }
    }

    impl_default_builder_methods!();

    pub fn with_content(mut self, content: Handle<UINode>) -> Self {
        self.content = content;
        self
    }

    pub fn build(self, ui: &mut UserInterface) -> Handle<UINode> {
        let content_presenter = ScrollContentPresenterBuilder::new()
            .with_child(self.content)
            .on_row(0)
            .on_column(0)
            .build(ui);

        let v_scroll_bar = ScrollBarBuilder::new()
            .with_orientation(Orientation::Vertical)
            .on_row(0)
            .on_column(1)
            .with_value_changed({
                let content_presenter = content_presenter;
                Box::new(move |ui, args| {
                    ScrollContentPresenter::set_vertical_scroll(content_presenter, ui, args.new_value);
                })
            })
            .build(ui);

        let h_scroll_bar = ScrollBarBuilder::new()
            .with_orientation(Orientation::Horizontal)
            .on_row(1)
            .on_column(0)
            .with_value_changed({
                let content_presenter = content_presenter;
                Box::new(move |ui, args| {
                    ScrollContentPresenter::set_horizontal_scroll(content_presenter, ui, args.new_value);
                })
            })
            .build(ui);

        let scroll_viewer = ScrollViewer {
            content: self.content,
            owner_handle: Handle::none(),
            v_scroll_bar,
            h_scroll_bar,
            content_presenter,
        };

        GenericNodeBuilder::new(UINodeKind::ScrollViewer(scroll_viewer), self.common)
            .with_child(GridBuilder::new()
                .add_row(Row::stretch())
                .add_row(Row::strict(20.0))
                .add_column(Column::stretch())
                .add_column(Column::strict(20.0))
                .with_child(content_presenter)
                .with_child(h_scroll_bar)
                .with_child(v_scroll_bar)
                .build(ui))
            .build(ui)
    }
}