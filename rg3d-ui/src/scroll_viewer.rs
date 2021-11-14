use crate::{
    core::algebra::Vector2,
    core::pool::Handle,
    define_constructor,
    grid::{Column, GridBuilder, Row},
    message::{MessageDirection, UiMessage},
    scroll_bar::{ScrollBar, ScrollBarBuilder, ScrollBarMessage},
    scroll_panel::{ScrollPanelBuilder, ScrollPanelMessage},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, NodeHandleMapping, Orientation, UiNode, UserInterface,
};
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, PartialEq)]
pub enum ScrollViewerMessage {
    Content(Handle<UiNode>),
    /// Adjusts vertical and horizontal scroll values so given node will be in "view box"
    /// of scroll viewer.
    BringIntoView(Handle<UiNode>),
}

impl ScrollViewerMessage {
    define_constructor!(ScrollViewerMessage:Content => fn content(Handle<UiNode>), layout: false);
    define_constructor!(ScrollViewerMessage:BringIntoView=> fn bring_into_view(Handle<UiNode>), layout: true);
}

#[derive(Clone)]
pub struct ScrollViewer {
    pub widget: Widget,
    pub content: Handle<UiNode>,
    pub scroll_panel: Handle<UiNode>,
    pub v_scroll_bar: Handle<UiNode>,
    pub h_scroll_bar: Handle<UiNode>,
}

crate::define_widget_deref!(ScrollViewer);

impl ScrollViewer {
    pub fn new(
        widget: Widget,
        content: Handle<UiNode>,
        content_presenter: Handle<UiNode>,
        v_scroll_bar: Handle<UiNode>,
        h_scroll_bar: Handle<UiNode>,
    ) -> Self {
        Self {
            widget,
            content,
            scroll_panel: content_presenter,
            v_scroll_bar,
            h_scroll_bar,
        }
    }

    pub fn content_presenter(&self) -> Handle<UiNode> {
        self.scroll_panel
    }

    pub fn content(&self) -> Handle<UiNode> {
        self.content
    }

    pub fn set_content(&mut self, content: Handle<UiNode>) {
        self.content = content;
    }
}

impl Control for ScrollViewer {
    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        node_map.resolve(&mut self.content);
        node_map.resolve(&mut self.scroll_panel);
        node_map.resolve(&mut self.v_scroll_bar);
        node_map.resolve(&mut self.h_scroll_bar);
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        let size = self.widget.arrange_override(ui, final_size);

        if self.content.is_some() {
            let content_size = ui.node(self.content).desired_size();
            let available_size_for_content = ui.node(self.scroll_panel).desired_size();

            let x_max = (content_size.x - available_size_for_content.x).max(0.0);
            ui.send_message(ScrollBarMessage::max_value(
                self.h_scroll_bar,
                MessageDirection::ToWidget,
                x_max,
            ));

            let y_max = (content_size.y - available_size_for_content.y).max(0.0);
            ui.send_message(ScrollBarMessage::max_value(
                self.v_scroll_bar,
                MessageDirection::ToWidget,
                y_max,
            ));
        }

        size
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(WidgetMessage::MouseWheel { amount, .. }) = message.data::<WidgetMessage>() {
            if self.v_scroll_bar.is_some() && !message.handled() {
                if let Some(v_scroll_bar) = ui.node(self.v_scroll_bar).cast::<ScrollBar>() {
                    let old_value = v_scroll_bar.value();
                    let new_value = old_value - amount * 17.0;
                    if (old_value - new_value).abs() > f32::EPSILON {
                        message.set_handled(true);
                    }
                    ui.send_message(ScrollBarMessage::value(
                        self.v_scroll_bar,
                        MessageDirection::ToWidget,
                        new_value,
                    ));
                }
            }
        } else if let Some(msg) = message.data::<ScrollPanelMessage>() {
            if message.destination() == self.scroll_panel {
                let msg = match *msg {
                    ScrollPanelMessage::VerticalScroll(value) => ScrollBarMessage::value(
                        self.v_scroll_bar,
                        MessageDirection::ToWidget,
                        value,
                    ),
                    ScrollPanelMessage::HorizontalScroll(value) => ScrollBarMessage::value(
                        self.h_scroll_bar,
                        MessageDirection::ToWidget,
                        value,
                    ),
                    _ => return,
                };
                // handle flag here is raised to prevent infinite message loop with the branch down below (ScrollBar::value).
                msg.set_handled(true);
                ui.send_message(msg);
            }
        } else if let Some(msg) = message.data::<ScrollBarMessage>() {
            if message.direction() == MessageDirection::FromWidget {
                match msg {
                    ScrollBarMessage::Value(new_value) => {
                        if !message.handled() {
                            if message.destination() == self.v_scroll_bar
                                && self.v_scroll_bar.is_some()
                            {
                                ui.send_message(ScrollPanelMessage::vertical_scroll(
                                    self.scroll_panel,
                                    MessageDirection::ToWidget,
                                    *new_value,
                                ));
                            } else if message.destination() == self.h_scroll_bar
                                && self.h_scroll_bar.is_some()
                            {
                                ui.send_message(ScrollPanelMessage::horizontal_scroll(
                                    self.scroll_panel,
                                    MessageDirection::ToWidget,
                                    *new_value,
                                ));
                            }
                        }
                    }
                    &ScrollBarMessage::MaxValue(_) => {
                        if message.destination() == self.v_scroll_bar && self.v_scroll_bar.is_some()
                        {
                            if let Some(scroll_bar) = ui.node(self.v_scroll_bar).cast::<ScrollBar>()
                            {
                                let visibility = (scroll_bar.max_value() - scroll_bar.min_value())
                                    .abs()
                                    >= f32::EPSILON;
                                ui.send_message(WidgetMessage::visibility(
                                    self.v_scroll_bar,
                                    MessageDirection::ToWidget,
                                    visibility,
                                ));
                            }
                        } else if message.destination() == self.h_scroll_bar
                            && self.h_scroll_bar.is_some()
                        {
                            if let Some(scroll_bar) = ui.node(self.h_scroll_bar).cast::<ScrollBar>()
                            {
                                let visibility = (scroll_bar.max_value() - scroll_bar.min_value())
                                    .abs()
                                    >= f32::EPSILON;
                                ui.send_message(WidgetMessage::visibility(
                                    self.h_scroll_bar,
                                    MessageDirection::ToWidget,
                                    visibility,
                                ));
                            }
                        }
                    }
                    _ => (),
                }
            }
        } else if let Some(msg) = message.data::<ScrollViewerMessage>() {
            if message.destination() == self.handle() {
                match msg {
                    ScrollViewerMessage::Content(content) => {
                        for child in ui.node(self.scroll_panel).children().to_vec() {
                            ui.send_message(WidgetMessage::remove(
                                child,
                                MessageDirection::ToWidget,
                            ));
                        }
                        ui.send_message(WidgetMessage::link(
                            *content,
                            MessageDirection::ToWidget,
                            self.scroll_panel,
                        ));
                    }
                    &ScrollViewerMessage::BringIntoView(handle) => {
                        // Re-cast message to inner panel.
                        ui.send_message(ScrollPanelMessage::bring_into_view(
                            self.scroll_panel,
                            MessageDirection::ToWidget,
                            handle,
                        ));
                    }
                }
            }
        }
    }

    fn remove_ref(&mut self, handle: Handle<UiNode>) {
        if self.content == handle {
            self.content = Handle::NONE;
        }
        if self.v_scroll_bar == handle {
            self.v_scroll_bar = Handle::NONE;
        }
        if self.h_scroll_bar == handle {
            self.h_scroll_bar = Handle::NONE;
        }
        if self.scroll_panel == handle {
            self.scroll_panel = Handle::NONE;
        }
    }
}

pub struct ScrollViewerBuilder {
    widget_builder: WidgetBuilder,
    content: Handle<UiNode>,
    h_scroll_bar: Option<Handle<UiNode>>,
    v_scroll_bar: Option<Handle<UiNode>>,
    horizontal_scroll_allowed: bool,
    vertical_scroll_allowed: bool,
}

impl ScrollViewerBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            content: Handle::NONE,
            h_scroll_bar: None,
            v_scroll_bar: None,
            horizontal_scroll_allowed: false,
            vertical_scroll_allowed: true,
        }
    }

    pub fn with_content(mut self, content: Handle<UiNode>) -> Self {
        self.content = content;
        self
    }

    pub fn with_vertical_scroll_bar(mut self, v_scroll_bar: Handle<UiNode>) -> Self {
        self.v_scroll_bar = Some(v_scroll_bar);
        self
    }

    pub fn with_horizontal_scroll_bar(mut self, h_scroll_bar: Handle<UiNode>) -> Self {
        self.h_scroll_bar = Some(h_scroll_bar);
        self
    }

    pub fn with_vertical_scroll_allowed(mut self, value: bool) -> Self {
        self.vertical_scroll_allowed = value;
        self
    }

    pub fn with_horizontal_scroll_allowed(mut self, value: bool) -> Self {
        self.horizontal_scroll_allowed = value;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let content_presenter = ScrollPanelBuilder::new(
            WidgetBuilder::new()
                .with_child(self.content)
                .on_row(0)
                .on_column(0),
        )
        .with_horizontal_scroll_allowed(self.horizontal_scroll_allowed)
        .with_vertical_scroll_allowed(self.vertical_scroll_allowed)
        .build(ctx);

        let v_scroll_bar = self.v_scroll_bar.unwrap_or_else(|| {
            ScrollBarBuilder::new(WidgetBuilder::new().with_width(22.0))
                .with_orientation(Orientation::Vertical)
                .build(ctx)
        });
        ctx[v_scroll_bar].set_row(0).set_column(1);

        let h_scroll_bar = self.h_scroll_bar.unwrap_or_else(|| {
            ScrollBarBuilder::new(WidgetBuilder::new().with_height(22.0))
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
            scroll_panel: content_presenter,
        };
        ctx.add_node(UiNode::new(sv))
    }
}
