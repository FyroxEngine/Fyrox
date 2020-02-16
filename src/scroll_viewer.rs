use crate::{
    UserInterface,
    scroll_content_presenter::{
        ScrollContentPresenterBuilder,
    },
    scroll_bar::{
        ScrollBarBuilder,
        Orientation,
    },
    grid::{
        Row,
        GridBuilder,
        Column,
    },
    message::{
        UiMessageData,
        UiMessage,
        ScrollBarMessage,
        WidgetMessage
    },
    widget::{
        Widget,
        WidgetBuilder,
    },
    Control,
    UINode,
    ControlTemplate,
    UINodeContainer,
    Builder,
    core::{
        pool::Handle,
        math::vec2::Vec2,
    },
    NodeHandleMapping
};

pub struct ScrollViewer<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    content: Handle<UINode<M, C>>,
    content_presenter: Handle<UINode<M, C>>,
    v_scroll_bar: Handle<UINode<M, C>>,
    h_scroll_bar: Handle<UINode<M, C>>,
}

impl<M, C: 'static + Control<M, C>> ScrollViewer<M, C> {
    pub fn new(
        widget: Widget<M, C>,
        content: Handle<UINode<M, C>>,
        content_presenter: Handle<UINode<M, C>>,
        v_scroll_bar: Handle<UINode<M, C>>,
        h_scroll_bar: Handle<UINode<M, C>>,
    ) -> Self {
        Self {
            widget,
            content,
            content_presenter,
            v_scroll_bar,
            h_scroll_bar,
        }
    }
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for ScrollViewer<M, C> {
    fn widget(&self) -> &Widget<M, C> {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget<M, C> {
        &mut self.widget
    }

    fn raw_copy(&self) -> UINode<M, C> {
        UINode::ScrollViewer(Self {
            widget: self.widget.raw_copy(),
            content: self.content,
            content_presenter: self.content_presenter,
            v_scroll_bar: self.v_scroll_bar,
            h_scroll_bar: self.h_scroll_bar,
        })
    }

    fn resolve(&mut self, _: &ControlTemplate<M, C>, node_map: &NodeHandleMapping<M, C>) {
        self.content = *node_map.get(&self.content).unwrap();
        self.content_presenter = *node_map.get(&self.content_presenter).unwrap();
        self.v_scroll_bar = *node_map.get(&self.v_scroll_bar).unwrap();
        self.h_scroll_bar = *node_map.get(&self.h_scroll_bar).unwrap();
    }

    fn arrange_override(&self, ui: &UserInterface<M, C>, final_size: Vec2) -> Vec2 {
        let size = self.widget.arrange_override(ui, final_size);

        if self.content.is_some() {
            let content_size = ui.node(self.content).widget().desired_size();
            let available_size_for_content = ui.node(self.content_presenter).widget().desired_size();

            let x_max = (content_size.x - available_size_for_content.x).max(0.0);
            self.widget.outgoing_messages.borrow_mut()
                .push_back(UiMessage::targeted(self.h_scroll_bar, UiMessageData::ScrollBar(
                    ScrollBarMessage::Value(x_max))));

            let y_max = (content_size.y - available_size_for_content.y).max(0.0);
            self.widget.outgoing_messages.borrow_mut()
                .push_back(UiMessage::targeted(self.v_scroll_bar, UiMessageData::ScrollBar(
                    ScrollBarMessage::Value(y_max))));
        }

        size
    }

    fn handle_message(&mut self, self_handle: Handle<UINode<M, C>>, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_message(self_handle, ui, message);

        match &message.data {
            UiMessageData::Widget(msg) => {
                if let WidgetMessage::MouseWheel { amount, .. } = msg {
                    if self.v_scroll_bar.is_some() && !message.handled && (message.source == self_handle || self.widget().has_descendant(message.source, ui)) {
                        if let UINode::ScrollBar(v_scroll_bar) = ui.node_mut(self.v_scroll_bar) {
                            v_scroll_bar.scroll(-amount * 10.0);
                            message.handled = true;
                        }
                    }
                }
            }
            UiMessageData::ScrollBar(msg) => {
                if let ScrollBarMessage::Value(new_value) = msg {
                    if message.target == self.v_scroll_bar && self.v_scroll_bar.is_some() {
                        if let UINode::ScrollBar(scroll_bar) = ui.node_mut(self.v_scroll_bar) {
                            scroll_bar.set_max_value(*new_value);
                            if (scroll_bar.max_value() - scroll_bar.min_value()).abs() <= std::f32::EPSILON {
                                scroll_bar.widget_mut().set_visibility(false);
                            } else {
                                scroll_bar.widget_mut().set_visibility(true);
                            }
                        }
                    } else if message.target == self.h_scroll_bar && self.h_scroll_bar.is_some() {
                        if let UINode::ScrollBar(scroll_bar) = ui.node_mut(self.h_scroll_bar) {
                            scroll_bar.set_max_value(*new_value);
                            if (scroll_bar.max_value() - scroll_bar.min_value()).abs() <= std::f32::EPSILON {
                                scroll_bar.widget_mut().set_visibility(false);
                            } else {
                                scroll_bar.widget_mut().set_visibility(true);
                            }
                        }
                    } else if message.source == self.h_scroll_bar && self.content_presenter.is_some() {
                        if let UINode::ScrollContentPresenter(content_presenter) = ui.node_mut(self.content_presenter) {
                            content_presenter.set_horizontal_scroll(*new_value);
                        }
                    } else if message.source == self.v_scroll_bar && self.content_presenter.is_some() {
                        if let UINode::ScrollContentPresenter(content_presenter) = ui.node_mut(self.content_presenter) {
                            content_presenter.set_vertical_scroll(*new_value);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn remove_ref(&mut self, handle: Handle<UINode<M, C>>) {
        if self.content == handle {
            self.content = Handle::NONE;
        }
        if self.v_scroll_bar == handle {
            self.v_scroll_bar = Handle::NONE;
        }
        if self.h_scroll_bar == handle {
            self.h_scroll_bar = Handle::NONE;
        }
        if self.content_presenter == handle {
            self.content_presenter = Handle::NONE;
        }
    }
}

pub struct ScrollViewerBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    content: Handle<UINode<M, C>>,
}

impl<M, C: 'static + Control<M, C>> ScrollViewerBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            content: Handle::NONE,
        }
    }

    pub fn with_content(mut self, content: Handle<UINode<M, C>>) -> Self {
        self.content = content;
        self
    }
}

impl<M, C: 'static + Control<M, C>> Builder<M, C> for ScrollViewerBuilder<M, C> {
    fn build(self, ui: &mut dyn UINodeContainer<M, C>) -> Handle<UINode<M, C>> {
        let content_presenter = ScrollContentPresenterBuilder::new(WidgetBuilder::new()
            .with_child(self.content)
            .on_row(0)
            .on_column(0))
            .build(ui);

        let v_scroll_bar = ScrollBarBuilder::new(WidgetBuilder::new()
            .on_row(0)
            .on_column(1)
            .with_width(20.0))
            .with_orientation(Orientation::Vertical)
            .build(ui);

        let h_scroll_bar = ScrollBarBuilder::new(WidgetBuilder::new()
            .on_row(1)
            .on_column(0)
            .with_height(20.0))
            .with_orientation(Orientation::Horizontal)
            .build(ui);

        let scroll_viewer = ScrollViewer {
            widget: self.widget_builder
                .with_child(GridBuilder::new(WidgetBuilder::new()
                    .with_child(content_presenter)
                    .with_child(h_scroll_bar)
                    .with_child(v_scroll_bar))
                    .add_row(Row::stretch())
                    .add_row(Row::auto())
                    .add_column(Column::stretch())
                    .add_column(Column::auto())
                    .build(ui))
                .build(),
            content: self.content,
            v_scroll_bar,
            h_scroll_bar,
            content_presenter,
        };
        ui.add_node(UINode::ScrollViewer(scroll_viewer))
    }
}