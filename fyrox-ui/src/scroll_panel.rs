//! Scroll panel widget is used to arrange its children widgets, so they can be offset by a certain amount of units
//! from top-left corner. It is used to provide basic scrolling functionality. See [`ScrollPanel`] docs for more
//! info and usage examples.

#![allow(missing_docs)]

use crate::{
    brush::Brush,
    core::{
        algebra::Vector2, color::Color, math::Rect, pool::Handle, reflect::prelude::*,
        scope_profile, type_traits::prelude::*, visitor::prelude::*,
    },
    define_constructor,
    draw::{CommandTexture, Draw, DrawingContext},
    message::{MessageDirection, UiMessage},
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UiNode, UserInterface,
};
use fyrox_core::uuid_provider;
use fyrox_graph::BaseSceneGraph;
use std::ops::{Deref, DerefMut};

/// A set of messages, that is used to modify the state of a scroll panel.
#[derive(Debug, Clone, PartialEq)]
pub enum ScrollPanelMessage {
    /// Sets the desired scrolling value for the vertical axis.
    VerticalScroll(f32),
    /// Sets the desired scrolling value for the horizontal axis.
    HorizontalScroll(f32),
    /// Adjusts vertical and horizontal scroll values so given node will be in "view box" of scroll panel.
    BringIntoView(Handle<UiNode>),
    /// Scrolls to end of the content.
    ScrollToEnd,
}

impl ScrollPanelMessage {
    define_constructor!(
        /// Creates [`ScrollPanelMessage::VerticalScroll`] message.
        ScrollPanelMessage:VerticalScroll => fn vertical_scroll(f32), layout: false
    );
    define_constructor!(
        /// Creates [`ScrollPanelMessage::HorizontalScroll`] message.
        ScrollPanelMessage:HorizontalScroll => fn horizontal_scroll(f32), layout: false
    );
    define_constructor!(
        /// Creates [`ScrollPanelMessage::BringIntoView`] message.
        ScrollPanelMessage:BringIntoView => fn bring_into_view(Handle<UiNode>), layout: true
    );
    define_constructor!(
        /// Creates [`ScrollPanelMessage::ScrollToEnd`] message.
        ScrollPanelMessage:ScrollToEnd => fn scroll_to_end(), layout: true
    );
}

/// Scroll panel widget is used to arrange its children widgets, so they can be offset by a certain amount of units
/// from top-left corner. It is used to provide basic scrolling functionality.
///
/// ## Examples
///
/// ```rust
/// # use fyrox_ui::{
/// #     button::ButtonBuilder,
/// #     core::{algebra::Vector2, pool::Handle},
/// #     grid::{Column, GridBuilder, Row},
/// #     scroll_panel::ScrollPanelBuilder,
/// #     widget::WidgetBuilder,
/// #     BuildContext, UiNode,
/// # };
/// #
/// fn create_scroll_panel(ctx: &mut BuildContext) -> Handle<UiNode> {
///     ScrollPanelBuilder::new(
///         WidgetBuilder::new().with_child(
///             GridBuilder::new(
///                 WidgetBuilder::new()
///                     .with_child(
///                         ButtonBuilder::new(WidgetBuilder::new())
///                             .with_text("Some Button")
///                             .build(ctx),
///                     )
///                     .with_child(
///                         ButtonBuilder::new(WidgetBuilder::new())
///                             .with_text("Some Other Button")
///                             .build(ctx),
///                     ),
///             )
///             .add_row(Row::auto())
///             .add_row(Row::auto())
///             .add_column(Column::stretch())
///             .build(ctx),
///         ),
///     )
///     .with_scroll_value(Vector2::new(100.0, 200.0))
///     .with_vertical_scroll_allowed(true)
///     .with_horizontal_scroll_allowed(true)
///     .build(ctx)
/// }
/// ```
///
/// ## Scrolling
///
/// Scrolling value for both axes can be set via [`ScrollPanelMessage::VerticalScroll`] and [`ScrollPanelMessage::HorizontalScroll`]:
///
/// ```rust
/// use fyrox_ui::{
///     core::pool::Handle, message::MessageDirection, scroll_panel::ScrollPanelMessage, UiNode,
///     UserInterface,
/// };
/// fn set_scrolling_value(
///     scroll_panel: Handle<UiNode>,
///     horizontal: f32,
///     vertical: f32,
///     ui: &UserInterface,
/// ) {
///     ui.send_message(ScrollPanelMessage::horizontal_scroll(
///         scroll_panel,
///         MessageDirection::ToWidget,
///         horizontal,
///     ));
///     ui.send_message(ScrollPanelMessage::vertical_scroll(
///         scroll_panel,
///         MessageDirection::ToWidget,
///         vertical,
///     ));
/// }
/// ```
///
/// ## Bringing child into view
///
/// Calculates the scroll values to bring a desired child into view, it can be used for automatic navigation:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle, message::MessageDirection, scroll_panel::ScrollPanelMessage, UiNode,
/// #     UserInterface,
/// # };
/// fn bring_child_into_view(
///     scroll_panel: Handle<UiNode>,
///     child: Handle<UiNode>,
///     ui: &UserInterface,
/// ) {
///     ui.send_message(ScrollPanelMessage::bring_into_view(
///         scroll_panel,
///         MessageDirection::ToWidget,
///         child,
///     ))
/// }
/// ```
#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider)]
pub struct ScrollPanel {
    /// Base widget of the scroll panel.
    pub widget: Widget,
    /// Current scroll value of the scroll panel.
    pub scroll: Vector2<f32>,
    /// A flag, that defines whether the vertical scrolling is allowed or not.
    pub vertical_scroll_allowed: bool,
    /// A flag, that defines whether the horizontal scrolling is allowed or not.
    pub horizontal_scroll_allowed: bool,
}

crate::define_widget_deref!(ScrollPanel);

uuid_provider!(ScrollPanel = "1ab4936d-58c8-4cf7-b33c-4b56092f4826");

impl ScrollPanel {
    fn children_size(&self, ui: &UserInterface) -> Vector2<f32> {
        let mut children_size = Vector2::<f32>::default();
        for child_handle in self.widget.children() {
            let desired_size = ui.node(*child_handle).desired_size();
            children_size.x = children_size.x.max(desired_size.x);
            children_size.y = children_size.y.max(desired_size.y);
        }
        children_size
    }
    fn bring_into_view(&self, ui: &UserInterface, handle: Handle<UiNode>) {
        let Some(node_to_focus_ref) = ui.try_get(handle) else {
            return;
        };
        let mut parent = handle;
        let mut relative_position = Vector2::default();
        while parent.is_some() && parent != self.handle {
            let node = ui.node(parent);
            relative_position += node.actual_local_position();
            parent = node.parent();
        }
        // This check is needed because it possible that given handle is not in
        // sub-tree of current scroll panel.
        if parent != self.handle {
            return;
        }
        let size = node_to_focus_ref.actual_local_size();
        let children_size = self.children_size(ui);
        let view_size = self.actual_local_size();
        // Check if requested item already in "view box", this will prevent weird "jumping" effect
        // when bring into view was requested on already visible element.
        if self.vertical_scroll_allowed
            && (relative_position.y < 0.0 || relative_position.y + size.y > view_size.y)
        {
            relative_position.y += self.scroll.y;
            let scroll_max = (children_size.y - view_size.y).max(0.0);
            relative_position.y = relative_position.y.clamp(0.0, scroll_max);
            ui.send_message(ScrollPanelMessage::vertical_scroll(
                self.handle,
                MessageDirection::ToWidget,
                relative_position.y,
            ));
        }
        if self.horizontal_scroll_allowed
            && (relative_position.x < 0.0 || relative_position.x + size.x > view_size.x)
        {
            relative_position.x += self.scroll.x;
            let scroll_max = (children_size.x - view_size.x).max(0.0);
            relative_position.x = relative_position.x.clamp(0.0, scroll_max);
            ui.send_message(ScrollPanelMessage::horizontal_scroll(
                self.handle,
                MessageDirection::ToWidget,
                relative_position.x,
            ));
        }
    }
}

impl Control for ScrollPanel {
    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        scope_profile!();

        let size_for_child = Vector2::new(
            if self.horizontal_scroll_allowed {
                f32::INFINITY
            } else {
                available_size.x
            },
            if self.vertical_scroll_allowed {
                f32::INFINITY
            } else {
                available_size.y
            },
        );

        let mut desired_size = Vector2::default();

        for child_handle in self.widget.children() {
            ui.measure_node(*child_handle, size_for_child);

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

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        scope_profile!();

        let children_size = self.children_size(ui);

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
            ui.arrange_node(*child_handle, &child_rect);
        }

        final_size
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        // Emit transparent geometry so panel will receive mouse events.
        drawing_context.push_rect_filled(&self.widget.bounding_rect(), None);
        drawing_context.commit(
            self.clip_bounds(),
            Brush::Solid(Color::TRANSPARENT),
            CommandTexture::None,
            None,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle() {
            if let Some(msg) = message.data::<ScrollPanelMessage>() {
                match *msg {
                    ScrollPanelMessage::VerticalScroll(scroll) => {
                        self.scroll.y = scroll;
                        self.invalidate_arrange();
                    }
                    ScrollPanelMessage::HorizontalScroll(scroll) => {
                        self.scroll.x = scroll;
                        self.invalidate_arrange();
                    }
                    ScrollPanelMessage::BringIntoView(handle) => {
                        self.bring_into_view(ui, handle);
                    }
                    ScrollPanelMessage::ScrollToEnd => {
                        let max_size = self.children_size(ui);
                        if self.vertical_scroll_allowed {
                            ui.send_message(ScrollPanelMessage::vertical_scroll(
                                self.handle,
                                MessageDirection::ToWidget,
                                (max_size.y - self.actual_local_size().y).max(0.0),
                            ));
                        }
                        if self.horizontal_scroll_allowed {
                            ui.send_message(ScrollPanelMessage::horizontal_scroll(
                                self.handle,
                                MessageDirection::ToWidget,
                                (max_size.x - self.actual_local_size().x).max(0.0),
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Scroll panel builder creates [`ScrollPanel`] widget instances and adds them to the user interface.
pub struct ScrollPanelBuilder {
    widget_builder: WidgetBuilder,
    vertical_scroll_allowed: Option<bool>,
    horizontal_scroll_allowed: Option<bool>,
    scroll_value: Vector2<f32>,
}

impl ScrollPanelBuilder {
    /// Creates new scroll panel builder.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            vertical_scroll_allowed: None,
            horizontal_scroll_allowed: None,
            scroll_value: Default::default(),
        }
    }

    /// Enables or disables vertical scrolling.
    pub fn with_vertical_scroll_allowed(mut self, value: bool) -> Self {
        self.vertical_scroll_allowed = Some(value);
        self
    }

    /// Enables or disables horizontal scrolling.
    pub fn with_horizontal_scroll_allowed(mut self, value: bool) -> Self {
        self.horizontal_scroll_allowed = Some(value);
        self
    }

    /// Sets the desired scrolling value for both axes at the same time.
    pub fn with_scroll_value(mut self, scroll_value: Vector2<f32>) -> Self {
        self.scroll_value = scroll_value;
        self
    }

    /// Finishes scroll panel building and adds it to the user interface.
    pub fn build(self, ui: &mut BuildContext) -> Handle<UiNode> {
        ui.add_node(UiNode::new(ScrollPanel {
            widget: self.widget_builder.build(),
            scroll: self.scroll_value,
            vertical_scroll_allowed: self.vertical_scroll_allowed.unwrap_or(true),
            horizontal_scroll_allowed: self.horizontal_scroll_allowed.unwrap_or(false),
        }))
    }
}
