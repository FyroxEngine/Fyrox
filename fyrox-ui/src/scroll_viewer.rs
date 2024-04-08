//! Scroll viewer is a scrollable region with two scroll bars for each axis. It is used to wrap a content of unknown
//! size to ensure that all of it will be accessible in a parent widget bounds. See [`ScrollViewer`] docs for more
//! info and usage examples.

#![warn(missing_docs)]

use crate::{
    core::uuid_provider,
    core::{
        algebra::Vector2, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        visitor::prelude::*,
    },
    define_constructor,
    grid::{Column, GridBuilder, Row},
    message::{MessageDirection, UiMessage},
    scroll_bar::{ScrollBar, ScrollBarBuilder, ScrollBarMessage},
    scroll_panel::{ScrollPanelBuilder, ScrollPanelMessage},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, Orientation, UiNode, UserInterface,
};
use fyrox_graph::BaseSceneGraph;
use std::ops::{Deref, DerefMut};

/// A set of messages that could be used to alternate the state of a [`ScrollViewer`] widget.
#[derive(Debug, Clone, PartialEq)]
pub enum ScrollViewerMessage {
    /// Sets the new content of the scroll viewer.
    Content(Handle<UiNode>),
    /// Adjusts vertical and horizontal scroll values so given node will be in "view box" of the scroll viewer.
    BringIntoView(Handle<UiNode>),
    /// Sets the new vertical scrolling speed.
    VScrollSpeed(f32),
    /// Sets the new horizontal scrolling speed.
    HScrollSpeed(f32),
    /// Scrolls to end of the content.
    ScrollToEnd,
}

impl ScrollViewerMessage {
    define_constructor!(
        /// Creates [`ScrollViewerMessage::Content`] message.
        ScrollViewerMessage:Content => fn content(Handle<UiNode>), layout: false
    );
    define_constructor!(
        /// Creates [`ScrollViewerMessage::BringIntoView`] message.
        ScrollViewerMessage:BringIntoView => fn bring_into_view(Handle<UiNode>), layout: true
    );
    define_constructor!(
        /// Creates [`ScrollViewerMessage::VScrollSpeed`] message.
        ScrollViewerMessage:VScrollSpeed => fn v_scroll_speed(f32), layout: true
    );
    define_constructor!(
        /// Creates [`ScrollViewerMessage::HScrollSpeed`] message.
        ScrollViewerMessage:HScrollSpeed => fn h_scroll_speed(f32), layout: true
    );
    define_constructor!(
        /// Creates [`ScrollViewerMessage::ScrollToEnd`] message.
        ScrollViewerMessage:ScrollToEnd => fn scroll_to_end(), layout: true
    );
}

/// Scroll viewer is a scrollable region with two scroll bars for each axis. It is used to wrap a content of unknown
/// size to ensure that all of it will be accessible in a parent widget bounds. For example, it could be used in a
/// Window widget to allow a content of the window to be accessible, even if the window is smaller than the content.
///
/// ## Example
///
/// A scroll viewer widget could be created using [`ScrollViewerBuilder`]:
///
/// ```rust
/// # use fyrox_ui::{
/// #     button::ButtonBuilder, core::pool::Handle, scroll_viewer::ScrollViewerBuilder,
/// #     stack_panel::StackPanelBuilder, text::TextBuilder, widget::WidgetBuilder, BuildContext,
/// #     UiNode,
/// # };
/// #
/// fn create_scroll_viewer(ctx: &mut BuildContext) -> Handle<UiNode> {
///     ScrollViewerBuilder::new(WidgetBuilder::new())
///         .with_content(
///             StackPanelBuilder::new(
///                 WidgetBuilder::new()
///                     .with_child(
///                         ButtonBuilder::new(WidgetBuilder::new())
///                             .with_text("Click Me!")
///                             .build(ctx),
///                     )
///                     .with_child(
///                         TextBuilder::new(WidgetBuilder::new())
///                             .with_text("Some\nlong\ntext")
///                             .build(ctx),
///                     ),
///             )
///             .build(ctx),
///         )
///         .build(ctx)
/// }
/// ```
///
/// Keep in mind, that you can change the content of a scroll viewer at runtime using [`ScrollViewerMessage::Content`] message.
///
/// ## Scrolling Speed and Controls
///
/// Scroll viewer can have an arbitrary scrolling speed for each axis. Scrolling is performed via mouse wheel and by default it
/// scrolls vertical axis, which can be changed by holding `Shift` key. Scrolling speed can be set during the build phase:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle, scroll_viewer::ScrollViewerBuilder, widget::WidgetBuilder,
/// #     BuildContext, UiNode,
/// # };
/// #
/// fn create_scroll_viewer(ctx: &mut BuildContext) -> Handle<UiNode> {
///     ScrollViewerBuilder::new(WidgetBuilder::new())
///         // Set vertical scrolling speed twice as fast as default scrolling speed.
///         .with_v_scroll_speed(60.0)
///         // Set horizontal scrolling speed slightly lower than the default value (30.0).
///         .with_h_scroll_speed(20.0)
///         .build(ctx)
/// }
/// ```
///
/// Also it could be set using [`ScrollViewerMessage::HScrollSpeed`] or [`ScrollViewerMessage::VScrollSpeed`] messages.
///
/// ## Bringing a child into view
///
/// Calculates the scroll values to bring a desired child into view, it can be used for automatic navigation:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle, message::MessageDirection, scroll_viewer::ScrollViewerMessage, UiNode,
/// #     UserInterface,
/// # };
/// fn bring_child_into_view(
///     scroll_viewer: Handle<UiNode>,
///     child: Handle<UiNode>,
///     ui: &UserInterface,
/// ) {
///     ui.send_message(ScrollViewerMessage::bring_into_view(
///         scroll_viewer,
///         MessageDirection::ToWidget,
///         child,
///     ))
/// }
/// ```
#[derive(Default, Clone, Debug, Visit, Reflect, ComponentProvider)]
pub struct ScrollViewer {
    /// Base widget of the scroll viewer.
    pub widget: Widget,
    /// A handle of a content.
    pub content: Handle<UiNode>,
    /// A handle of [`crate::scroll_panel::ScrollPanel`] widget instance that does the actual layouting.
    pub scroll_panel: Handle<UiNode>,
    /// A handle of scroll bar widget for vertical axis.
    pub v_scroll_bar: Handle<UiNode>,
    /// A handle of scroll bar widget for horizontal axis.
    pub h_scroll_bar: Handle<UiNode>,
    /// Current vertical scrolling speed.
    pub v_scroll_speed: f32,
    /// Current horizontal scrolling speed.
    pub h_scroll_speed: f32,
}

crate::define_widget_deref!(ScrollViewer);

uuid_provider!(ScrollViewer = "173e869f-7da0-4ae2-915a-5d545d8150cc");

impl Control for ScrollViewer {
    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        let size = self.widget.arrange_override(ui, final_size);

        if self.content.is_some() {
            let content_size = ui.node(self.content).desired_size();
            let available_size_for_content = ui.node(self.scroll_panel).desired_size();

            let x_max = (content_size.x - available_size_for_content.x).max(0.0);
            let x_size_ratio = if content_size.x > f32::EPSILON {
                (available_size_for_content.x / content_size.x).min(1.0)
            } else {
                1.0
            };
            ui.send_message(ScrollBarMessage::max_value(
                self.h_scroll_bar,
                MessageDirection::ToWidget,
                x_max,
            ));
            ui.send_message(ScrollBarMessage::size_ratio(
                self.h_scroll_bar,
                MessageDirection::ToWidget,
                x_size_ratio,
            ));

            let y_max = (content_size.y - available_size_for_content.y).max(0.0);
            let y_size_ratio = if content_size.y > f32::EPSILON {
                (available_size_for_content.y / content_size.y).min(1.0)
            } else {
                1.0
            };
            ui.send_message(ScrollBarMessage::max_value(
                self.v_scroll_bar,
                MessageDirection::ToWidget,
                y_max,
            ));
            ui.send_message(ScrollBarMessage::size_ratio(
                self.v_scroll_bar,
                MessageDirection::ToWidget,
                y_size_ratio,
            ));
        }

        size
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(WidgetMessage::MouseWheel { amount, .. }) = message.data::<WidgetMessage>() {
            if !message.handled() {
                let (scroll_bar, scroll_speed) = if ui.keyboard_modifiers().shift {
                    (self.h_scroll_bar, self.h_scroll_speed)
                } else {
                    (self.v_scroll_bar, self.v_scroll_speed)
                };

                if let Some(scroll_bar) = ui.node(scroll_bar).cast::<ScrollBar>() {
                    let old_value = *scroll_bar.value;
                    let new_value = old_value - amount * scroll_speed;
                    if (old_value - new_value).abs() > f32::EPSILON {
                        message.set_handled(true);
                    }
                    ui.send_message(ScrollBarMessage::value(
                        scroll_bar.handle,
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
                                let visibility =
                                    (*scroll_bar.max - *scroll_bar.min).abs() >= f32::EPSILON;
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
                                let visibility =
                                    (*scroll_bar.max - *scroll_bar.min).abs() >= f32::EPSILON;
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
                        for child in ui.node(self.scroll_panel).children() {
                            ui.send_message(WidgetMessage::remove(
                                *child,
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
                    &ScrollViewerMessage::HScrollSpeed(speed) => {
                        if self.h_scroll_speed != speed
                            && message.direction() == MessageDirection::ToWidget
                        {
                            self.h_scroll_speed = speed;

                            ui.send_message(message.reverse());
                        }
                    }
                    &ScrollViewerMessage::VScrollSpeed(speed) => {
                        if self.v_scroll_speed != speed
                            && message.direction() == MessageDirection::ToWidget
                        {
                            self.v_scroll_speed = speed;

                            ui.send_message(message.reverse());
                        }
                    }
                    ScrollViewerMessage::ScrollToEnd => {
                        // Re-cast message to inner panel.
                        ui.send_message(ScrollPanelMessage::scroll_to_end(
                            self.scroll_panel,
                            MessageDirection::ToWidget,
                        ));
                    }
                }
            }
        }
    }
}

/// Scroll viewer builder creates [`ScrollViewer`] widget instances and adds them to the user interface.
pub struct ScrollViewerBuilder {
    widget_builder: WidgetBuilder,
    content: Handle<UiNode>,
    h_scroll_bar: Option<Handle<UiNode>>,
    v_scroll_bar: Option<Handle<UiNode>>,
    horizontal_scroll_allowed: bool,
    vertical_scroll_allowed: bool,
    v_scroll_speed: f32,
    h_scroll_speed: f32,
}

impl ScrollViewerBuilder {
    /// Creates new builder instance.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            content: Handle::NONE,
            h_scroll_bar: None,
            v_scroll_bar: None,
            horizontal_scroll_allowed: false,
            vertical_scroll_allowed: true,
            v_scroll_speed: 30.0,
            h_scroll_speed: 30.0,
        }
    }

    /// Sets the desired content of the scroll viewer.
    pub fn with_content(mut self, content: Handle<UiNode>) -> Self {
        self.content = content;
        self
    }

    /// Sets the desired vertical scroll bar widget.
    pub fn with_vertical_scroll_bar(mut self, v_scroll_bar: Handle<UiNode>) -> Self {
        self.v_scroll_bar = Some(v_scroll_bar);
        self
    }

    /// Sets the desired horizontal scroll bar widget.
    pub fn with_horizontal_scroll_bar(mut self, h_scroll_bar: Handle<UiNode>) -> Self {
        self.h_scroll_bar = Some(h_scroll_bar);
        self
    }

    /// Enables or disables vertical scrolling.
    pub fn with_vertical_scroll_allowed(mut self, value: bool) -> Self {
        self.vertical_scroll_allowed = value;
        self
    }

    /// Enables or disables horizontal scrolling.
    pub fn with_horizontal_scroll_allowed(mut self, value: bool) -> Self {
        self.horizontal_scroll_allowed = value;
        self
    }

    /// Sets the desired vertical scrolling speed.
    pub fn with_v_scroll_speed(mut self, speed: f32) -> Self {
        self.v_scroll_speed = speed;
        self
    }

    /// Sets the desired horizontal scrolling speed.
    pub fn with_h_scroll_speed(mut self, speed: f32) -> Self {
        self.h_scroll_speed = speed;
        self
    }

    /// Finishes widget building and adds it to the user interface.
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
            ScrollBarBuilder::new(WidgetBuilder::new().with_width(16.0))
                .with_step(30.0)
                .with_orientation(Orientation::Vertical)
                .build(ctx)
        });
        ctx[v_scroll_bar].set_row(0).set_column(1);

        let h_scroll_bar = self.h_scroll_bar.unwrap_or_else(|| {
            ScrollBarBuilder::new(WidgetBuilder::new().with_height(16.0))
                .with_step(30.0)
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
            v_scroll_speed: self.v_scroll_speed,
            h_scroll_speed: self.h_scroll_speed,
        };
        ctx.add_node(UiNode::new(sv))
    }
}
