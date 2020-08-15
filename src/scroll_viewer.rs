use crate::{
    core::{math::vec2::Vec2, pool::Handle},
    grid::{Column, GridBuilder, Row},
    message::ScrollPanelMessage,
    message::{ScrollBarMessage, ScrollViewerMessage, UiMessage, UiMessageData, WidgetMessage},
    scroll_bar::ScrollBarBuilder,
    scroll_panel::ScrollPanelBuilder,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, NodeHandleMapping, Orientation, UINode, UserInterface,
};
use std::ops::{Deref, DerefMut};

pub struct ScrollViewer<M: 'static, C: 'static + Control<M, C>> {
    pub widget: Widget<M, C>,
    pub content: Handle<UINode<M, C>>,
    pub content_presenter: Handle<UINode<M, C>>,
    pub v_scroll_bar: Handle<UINode<M, C>>,
    pub h_scroll_bar: Handle<UINode<M, C>>,
}

impl<M: 'static, C: 'static + Control<M, C>> Deref for ScrollViewer<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> DerefMut for ScrollViewer<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> ScrollViewer<M, C> {
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

    pub fn content_presenter(&self) -> Handle<UINode<M, C>> {
        self.content_presenter
    }

    pub fn content(&self) -> Handle<UINode<M, C>> {
        self.content
    }

    pub fn set_content(&mut self, content: Handle<UINode<M, C>>) {
        self.content = content;
    }
}

impl<M: 'static, C: 'static + Control<M, C>> Control<M, C> for ScrollViewer<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        UINode::ScrollViewer(Self {
            widget: self.widget.raw_copy(),
            content: self.content,
            content_presenter: self.content_presenter,
            v_scroll_bar: self.v_scroll_bar,
            h_scroll_bar: self.h_scroll_bar,
        })
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        self.content = *node_map.get(&self.content).unwrap();
        self.content_presenter = *node_map.get(&self.content_presenter).unwrap();
        self.v_scroll_bar = *node_map.get(&self.v_scroll_bar).unwrap();
        self.h_scroll_bar = *node_map.get(&self.h_scroll_bar).unwrap();
    }

    fn arrange_override(&self, ui: &UserInterface<M, C>, final_size: Vec2) -> Vec2 {
        let size = self.widget.arrange_override(ui, final_size);

        if self.content.is_some() {
            let content_size = ui.node(self.content).desired_size();
            let available_size_for_content = ui.node(self.content_presenter).desired_size();

            let x_max = (content_size.x - available_size_for_content.x).max(0.0);
            ui.send_message(ScrollBarMessage::max_value(self.h_scroll_bar, x_max));

            let y_max = (content_size.y - available_size_for_content.y).max(0.0);
            ui.send_message(ScrollBarMessage::max_value(self.v_scroll_bar, y_max));
        }

        size
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);

        match &message.data {
            UiMessageData::Widget(msg) => {
                if let WidgetMessage::MouseWheel { amount, .. } = msg {
                    if self.v_scroll_bar.is_some() && !message.handled {
                        if let UINode::ScrollBar(v_scroll_bar) = ui.node(self.v_scroll_bar) {
                            let old_value = v_scroll_bar.value();
                            let new_value = old_value - amount * 10.0;
                            if (old_value - new_value).abs() > std::f32::EPSILON {
                                message.handled = true;
                            }
                            ui.send_message(ScrollBarMessage::value(self.v_scroll_bar, new_value));
                        }
                    }
                }
            }
            UiMessageData::ScrollBar(msg) => match msg {
                ScrollBarMessage::Value(new_value) => {
                    if message.destination == self.v_scroll_bar && self.v_scroll_bar.is_some() {
                        ui.send_message(ScrollPanelMessage::vertical_scroll(
                            self.content_presenter,
                            *new_value,
                        ));
                    } else if message.destination == self.h_scroll_bar
                        && self.h_scroll_bar.is_some()
                    {
                        ui.send_message(ScrollPanelMessage::horizontal_scroll(
                            self.content_presenter,
                            *new_value,
                        ));
                    }
                }
                &ScrollBarMessage::MaxValue(_) => {
                    if message.destination == self.v_scroll_bar && self.v_scroll_bar.is_some() {
                        if let UINode::ScrollBar(scroll_bar) = ui.node(self.v_scroll_bar) {
                            let visibility = (scroll_bar.max_value() - scroll_bar.min_value())
                                .abs()
                                >= std::f32::EPSILON;
                            ui.send_message(WidgetMessage::visibility(
                                self.v_scroll_bar,
                                visibility,
                            ));
                        }
                    } else if message.destination == self.h_scroll_bar
                        && self.h_scroll_bar.is_some()
                    {
                        if let UINode::ScrollBar(scroll_bar) = ui.node(self.h_scroll_bar) {
                            let visibility = (scroll_bar.max_value() - scroll_bar.min_value())
                                .abs()
                                >= std::f32::EPSILON;
                            ui.send_message(WidgetMessage::visibility(
                                self.h_scroll_bar,
                                visibility,
                            ));
                        }
                    }
                }
                _ => (),
            },
            UiMessageData::ScrollViewer(msg) => {
                if message.destination == self.handle() {
                    if let ScrollViewerMessage::Content(content) = msg {
                        for child in ui.node(self.content_presenter).children().to_vec() {
                            ui.send_message(WidgetMessage::remove(child));
                        }
                        ui.link_nodes(*content, self.content_presenter);
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
    h_scroll_bar: Option<Handle<UINode<M, C>>>,
    v_scroll_bar: Option<Handle<UINode<M, C>>>,
}

impl<M: 'static, C: 'static + Control<M, C>> ScrollViewerBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            content: Handle::NONE,
            h_scroll_bar: None,
            v_scroll_bar: None,
        }
    }

    pub fn with_content(mut self, content: Handle<UINode<M, C>>) -> Self {
        self.content = content;
        self
    }

    pub fn with_vertical_scroll_bar(mut self, v_scroll_bar: Handle<UINode<M, C>>) -> Self {
        self.v_scroll_bar = Some(v_scroll_bar);
        self
    }

    pub fn with_horizontal_scroll_bar(mut self, h_scroll_bar: Handle<UINode<M, C>>) -> Self {
        self.h_scroll_bar = Some(h_scroll_bar);
        self
    }

    pub fn build(self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        let content_presenter = ScrollPanelBuilder::new(
            WidgetBuilder::new()
                .with_child(self.content)
                .on_row(0)
                .on_column(0),
        )
        .build(ctx);

        let v_scroll_bar = self.v_scroll_bar.unwrap_or_else(|| {
            ScrollBarBuilder::new(WidgetBuilder::new().with_width(28.0))
                .with_orientation(Orientation::Vertical)
                .build(ctx)
        });
        ctx[v_scroll_bar].set_row(0).set_column(1);

        let h_scroll_bar = self.h_scroll_bar.unwrap_or_else(|| {
            ScrollBarBuilder::new(WidgetBuilder::new().with_height(28.0))
                .with_orientation(Orientation::Horizontal)
                .build(ctx)
        });
        ctx[h_scroll_bar].set_row(1).set_column(0);

        let sv = ScrollViewer {
            widget: self
                .widget_builder
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .with_child(content_presenter)
                            .with_child(h_scroll_bar)
                            .with_child(v_scroll_bar),
                    )
                    .add_row(Row::stretch())
                    .add_row(Row::auto())
                    .add_column(Column::stretch())
                    .add_column(Column::auto())
                    .build(ctx),
                )
                .build(),
            content: self.content,
            v_scroll_bar,
            h_scroll_bar,
            content_presenter,
        };
        ctx.add_node(UINode::ScrollViewer(sv))
    }
}
