//! Popup is used to display other widgets in floating panel, that could lock input in self bounds. See [`Popup`] docs
//! for more info and usage examples.

#![warn(missing_docs)]

use crate::{
    border::BorderBuilder,
    core::{
        algebra::Vector2, math::Rect, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        uuid_provider, variable::InheritableVariable, visitor::prelude::*,
    },
    define_constructor,
    message::{ButtonState, KeyCode, MessageDirection, OsEvent, UiMessage},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, RestrictionEntry, Thickness, UiNode, UserInterface, BRUSH_DARKEST,
    BRUSH_PRIMARY,
};
use fyrox_graph::BaseSceneGraph;
use std::ops::{Deref, DerefMut};

/// A set of messages for [`Popup`] widget.
#[derive(Debug, Clone, PartialEq)]
pub enum PopupMessage {
    /// Used to open a [`Popup`] widgets. Use [`PopupMessage::open`] to create the message.
    Open,
    /// Used to close a [`Popup`] widgets. Use [`PopupMessage::close`] to create the message.
    Close,
    /// Used to change the content of a [`Popup`] widgets. Use [`PopupMessage::content`] to create the message.
    Content(Handle<UiNode>),
    /// Used to change popup's placement. Use [`PopupMessage::placement`] to create the message.
    Placement(Placement),
    /// Used to adjust position of a popup widget, so it will be on screen. Use [`PopupMessage::adjust_position`] to create
    /// the message.
    AdjustPosition,
    /// Used to set the owner of a Popup. The owner will receive Event messages.
    Owner(Handle<UiNode>),
    /// Sent by the Popup to its owner when handling messages from the Popup's children.
    RelayedMessage(UiMessage),
}

impl PopupMessage {
    define_constructor!(
        /// Creates [`PopupMessage::Open`] message.
        PopupMessage:Open => fn open(), layout: false
    );
    define_constructor!(
        /// Creates [`PopupMessage::Close`] message.
        PopupMessage:Close => fn close(), layout: false
    );
    define_constructor!(
        /// Creates [`PopupMessage::Content`] message.
        PopupMessage:Content => fn content(Handle<UiNode>), layout: false
    );
    define_constructor!(
        /// Creates [`PopupMessage::Placement`] message.
        PopupMessage:Placement => fn placement(Placement), layout: false
    );
    define_constructor!(
        /// Creates [`PopupMessage::AdjustPosition`] message.
        PopupMessage:AdjustPosition => fn adjust_position(), layout: true
    );
    define_constructor!(
        /// Creates [`PopupMessage::Owner`] message.
        PopupMessage:Owner => fn owner(Handle<UiNode>), layout: false
    );
    define_constructor!(
        /// Creates [`PopupMessage::RelayedMessage`] message.
        PopupMessage:RelayedMessage => fn relayed_message(UiMessage), layout: false
    );
}

/// Defines a method of popup placement.
#[derive(Copy, Clone, PartialEq, Debug, Visit, Reflect)]
pub enum Placement {
    /// A popup should be placed relative to given widget at the left top corner of the widget screen bounds.
    /// Widget handle could be [`Handle::NONE`], in this case the popup will be placed at the left top corner of the screen.
    LeftTop(Handle<UiNode>),

    /// A popup should be placed relative to given widget at the right top corner of the widget screen bounds.
    /// Widget handle could be [`Handle::NONE`], in this case the popup will be placed at the right top corner of the screen.
    RightTop(Handle<UiNode>),

    /// A popup should be placed relative to given widget at the center of the widget screen bounds.
    /// Widget handle could be [`Handle::NONE`], in this case the popup will be placed at the center of the screen.
    Center(Handle<UiNode>),

    /// A popup should be placed relative to given widget at the left bottom corner of the widget screen bounds.
    /// Widget handle could be [`Handle::NONE`], in this case the popup will be placed at the left bottom corner of the screen.
    LeftBottom(Handle<UiNode>),

    /// A popup should be placed relative to given widget at the right bottom corner of the widget screen bounds.
    /// Widget handle could be [`Handle::NONE`], in this case the popup will be placed at the right bottom corner of the screen.
    RightBottom(Handle<UiNode>),

    /// A popup should be placed at the cursor position. The widget handle could be either [`Handle::NONE`] or a handle of a
    /// widget that is directly behind the cursor.
    Cursor(Handle<UiNode>),

    /// A popup should be placed at given screen-space position.
    Position {
        /// Screen-space position.
        position: Vector2<f32>,

        /// A handle of the node that is located behind the given position. Could be [`Handle::NONE`] if there is nothing behind
        /// given position.
        target: Handle<UiNode>,
    },
}

impl Default for Placement {
    fn default() -> Self {
        Self::LeftTop(Default::default())
    }
}

impl Placement {
    /// Returns a handle of the node to which this placement corresponds to.
    pub fn target(&self) -> Handle<UiNode> {
        match self {
            Placement::LeftTop(target)
            | Placement::RightTop(target)
            | Placement::Center(target)
            | Placement::LeftBottom(target)
            | Placement::RightBottom(target)
            | Placement::Cursor(target)
            | Placement::Position { target, .. } => *target,
        }
    }
}

/// Popup is used to display other widgets in floating panel, that could lock input in self bounds.
///
/// ## How to create
///
/// A simple popup with a button could be created using the following code:
///
/// ```rust
/// # use fyrox_ui::{
/// #     button::ButtonBuilder, core::pool::Handle, popup::PopupBuilder, widget::WidgetBuilder,
/// #     BuildContext, UiNode,
/// # };
/// fn create_popup_with_button(ctx: &mut BuildContext) -> Handle<UiNode> {
///     PopupBuilder::new(WidgetBuilder::new())
///         .with_content(
///             ButtonBuilder::new(WidgetBuilder::new())
///                 .with_text("Click Me!")
///                 .build(ctx),
///         )
///         .build(ctx)
/// }
/// ```
///
/// Keep in mind, that the popup is closed by default. You need to open it explicitly by sending a [`PopupMessage::Open`] to it,
/// otherwise you won't see it:
///
/// ```rust
/// # use fyrox_ui::{
/// #     button::ButtonBuilder,
/// #     core::pool::Handle,
/// #     message::MessageDirection,
/// #     popup::{Placement, PopupBuilder, PopupMessage},
/// #     widget::WidgetBuilder,
/// #     UiNode, UserInterface,
/// # };
/// fn create_popup_with_button_and_open_it(ui: &mut UserInterface) -> Handle<UiNode> {
///     let popup = PopupBuilder::new(WidgetBuilder::new())
///         .with_content(
///             ButtonBuilder::new(WidgetBuilder::new())
///                 .with_text("Click Me!")
///                 .build(&mut ui.build_ctx()),
///         )
///         .build(&mut ui.build_ctx());
///
///     // Open the popup explicitly.
///     ui.send_message(PopupMessage::open(popup, MessageDirection::ToWidget));
///
///     popup
/// }
/// ```
///
/// ## Placement
///
/// Since popups are usually used to show useful context-specific information (like context menus, drop-down lists, etc.), they're usually
/// open above some other widget with specific alignment (right, left, center, etc.).
///
/// ```rust
/// # use fyrox_ui::{
/// #     button::ButtonBuilder,
/// #     core::pool::Handle,
/// #     message::MessageDirection,
/// #     popup::{Placement, PopupBuilder, PopupMessage},
/// #     widget::WidgetBuilder,
/// #     UiNode, UserInterface,
/// # };
/// fn create_popup_with_button_and_open_it(ui: &mut UserInterface) -> Handle<UiNode> {
///     let popup = PopupBuilder::new(WidgetBuilder::new())
///         .with_content(
///             ButtonBuilder::new(WidgetBuilder::new())
///                 .with_text("Click Me!")
///                 .build(&mut ui.build_ctx()),
///         )
///         // Set the placement. For simplicity it is just a cursor position with Handle::NONE as placement target.
///         .with_placement(Placement::Cursor(Handle::NONE))
///         .build(&mut ui.build_ctx());
///
///     // Open the popup explicitly at the current placement.
///     ui.send_message(PopupMessage::open(popup, MessageDirection::ToWidget));
///
///     popup
/// }
/// ```
///
/// The example uses [`Placement::Cursor`] with [`Handle::NONE`] placement target for simplicity reasons, however in
/// the real-world usages this handle must be a handle of some widget that is located under the popup. It is very
/// important to specify it correctly, otherwise you will lost the built-in ability to fetch the actual placement target.
/// For example, imagine that you're building your own custom [`crate::dropdown_list::DropdownList`] widget and the popup
/// is used to display content of the list. In this case you could specify the placement target like this:
///
/// ```rust
/// # use fyrox_ui::{
/// #     button::ButtonBuilder,
/// #     core::pool::Handle,
/// #     message::MessageDirection,
/// #     popup::{Placement, PopupBuilder, PopupMessage},
/// #     widget::WidgetBuilder,
/// #     UiNode, UserInterface,
/// # };
/// fn create_popup_with_button_and_open_it(
///     dropdown_list: Handle<UiNode>,
///     ui: &mut UserInterface,
/// ) -> Handle<UiNode> {
///     let popup = PopupBuilder::new(WidgetBuilder::new())
///         .with_content(
///             ButtonBuilder::new(WidgetBuilder::new())
///                 .with_text("Click Me!")
///                 .build(&mut ui.build_ctx()),
///         )
///         // Set the placement to the dropdown list.
///         .with_placement(Placement::LeftBottom(dropdown_list))
///         .build(&mut ui.build_ctx());
///
///     // Open the popup explicitly at the current placement.
///     ui.send_message(PopupMessage::open(popup, MessageDirection::ToWidget));
///
///     popup
/// }
/// ```
///
/// In this case, the popup will open at the left bottom corner of the dropdown list automatically. Placement target is also
/// useful to build context menus, especially for lists with multiple items. Each item in the list usually have the same context
/// menu, and this is ideal use case for popups, since the single context menu can be shared across multiple list items. To find
/// which item cause the context menu to open, catch [`PopupMessage::Placement`] and extract the node handle - this will be your
/// actual item.
///
/// ## Opening mode
///
/// By default, when you click outside of your popup it will automatically close. It is pretty common behaviour in the UI, you
/// can see it almost everytime you use context menus in various apps. There are cases when this behaviour is undesired and it
/// can be turned off:
///
/// ```rust
/// # use fyrox_ui::{
/// #     button::ButtonBuilder, core::pool::Handle, popup::PopupBuilder, widget::WidgetBuilder,
/// #     BuildContext, UiNode,
/// # };
/// fn create_popup_with_button(ctx: &mut BuildContext) -> Handle<UiNode> {
///     PopupBuilder::new(WidgetBuilder::new())
///         .with_content(
///             ButtonBuilder::new(WidgetBuilder::new())
///                 .with_text("Click Me!")
///                 .build(ctx),
///         )
///         // This forces the popup to stay open when clicked outside of its bounds
///         .stays_open(true)
///         .build(ctx)
/// }
/// ```
///
/// ## Smart placement
///
/// Popup widget can automatically adjust its position to always remain on screen, which is useful for tooltips, dropdown lists,
/// etc. To enable this option, use [`PopupBuilder::with_smart_placement`] with `true` as the first argument.
#[derive(Default, Clone, Visit, Debug, Reflect, ComponentProvider)]
pub struct Popup {
    /// Base widget of the popup.
    pub widget: Widget,
    /// Current placement of the popup.
    pub placement: InheritableVariable<Placement>,
    /// A flag, that defines whether the popup will stay open if a user click outside of its bounds.
    pub stays_open: InheritableVariable<bool>,
    /// A flag, that defines whether the popup is open or not.
    pub is_open: InheritableVariable<bool>,
    /// Current content of the popup.
    pub content: InheritableVariable<Handle<UiNode>>,
    /// Background widget of the popup. It is used as a container for the content.
    pub body: InheritableVariable<Handle<UiNode>>,
    /// Smart placement prevents the popup from going outside of the screen bounds. It is usually used for tooltips,
    /// dropdown lists, etc. to prevent the content from being outside of the screen.
    pub smart_placement: InheritableVariable<bool>,
    /// The destination for Event messages that relay messages from the children of this popup.
    pub owner: Handle<UiNode>,
}

crate::define_widget_deref!(Popup);

fn adjust_placement_position(
    node_screen_bounds: Rect<f32>,
    screen_size: Vector2<f32>,
) -> Vector2<f32> {
    let mut new_position = node_screen_bounds.position;
    let right_bottom = node_screen_bounds.right_bottom_corner();
    if right_bottom.x > screen_size.x {
        new_position.x -= right_bottom.x - screen_size.x;
    }
    if right_bottom.y > screen_size.y {
        new_position.y -= right_bottom.y - screen_size.y;
    }
    new_position
}

impl Popup {
    fn left_top_placement(&self, ui: &UserInterface, target: Handle<UiNode>) -> Vector2<f32> {
        ui.try_get(target)
            .map(|n| n.screen_position())
            .unwrap_or_default()
    }

    fn right_top_placement(&self, ui: &UserInterface, target: Handle<UiNode>) -> Vector2<f32> {
        ui.try_get(target)
            .map(|n| n.screen_position() + Vector2::new(n.actual_global_size().x, 0.0))
            .unwrap_or_else(|| {
                Vector2::new(ui.screen_size().x - self.widget.actual_global_size().x, 0.0)
            })
    }

    fn center_placement(&self, ui: &UserInterface, target: Handle<UiNode>) -> Vector2<f32> {
        ui.try_get(target)
            .map(|n| n.screen_position() + n.actual_global_size().scale(0.5))
            .unwrap_or_else(|| (ui.screen_size - self.widget.actual_global_size()).scale(0.5))
    }

    fn left_bottom_placement(&self, ui: &UserInterface, target: Handle<UiNode>) -> Vector2<f32> {
        ui.try_get(target)
            .map(|n| n.screen_position() + Vector2::new(0.0, n.actual_global_size().y))
            .unwrap_or_else(|| {
                Vector2::new(0.0, ui.screen_size().y - self.widget.actual_global_size().y)
            })
    }

    fn right_bottom_placement(&self, ui: &UserInterface, target: Handle<UiNode>) -> Vector2<f32> {
        ui.try_get(target)
            .map(|n| n.screen_position() + n.actual_global_size())
            .unwrap_or_else(|| ui.screen_size - self.widget.actual_global_size())
    }
}

uuid_provider!(Popup = "1c641540-59eb-4ccd-a090-2173dab02245");

impl Control for Popup {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<PopupMessage>() {
            if message.destination() == self.handle() {
                match msg {
                    PopupMessage::Open => {
                        if !*self.is_open {
                            self.is_open.set_value_and_mark_modified(true);
                            ui.send_message(WidgetMessage::visibility(
                                self.handle(),
                                MessageDirection::ToWidget,
                                true,
                            ));
                            ui.push_picking_restriction(RestrictionEntry {
                                handle: self.handle(),
                                stop: false,
                            });
                            ui.send_message(WidgetMessage::topmost(
                                self.handle(),
                                MessageDirection::ToWidget,
                            ));
                            let position = match *self.placement {
                                Placement::LeftTop(target) => self.left_top_placement(ui, target),
                                Placement::RightTop(target) => self.right_top_placement(ui, target),
                                Placement::Center(target) => self.center_placement(ui, target),
                                Placement::LeftBottom(target) => {
                                    self.left_bottom_placement(ui, target)
                                }
                                Placement::RightBottom(target) => {
                                    self.right_bottom_placement(ui, target)
                                }
                                Placement::Cursor(_) => ui.cursor_position(),
                                Placement::Position { position, .. } => position,
                            };

                            ui.send_message(WidgetMessage::desired_position(
                                self.handle(),
                                MessageDirection::ToWidget,
                                ui.screen_to_root_canvas_space(position),
                            ));
                            ui.send_message(WidgetMessage::focus(
                                if self.content.is_some() {
                                    *self.content
                                } else {
                                    self.handle
                                },
                                MessageDirection::ToWidget,
                            ));
                            if *self.smart_placement {
                                ui.send_message(PopupMessage::adjust_position(
                                    self.handle,
                                    MessageDirection::ToWidget,
                                ));
                            }
                        }
                    }
                    PopupMessage::Close => {
                        if *self.is_open {
                            self.is_open.set_value_and_mark_modified(false);
                            ui.send_message(WidgetMessage::visibility(
                                self.handle(),
                                MessageDirection::ToWidget,
                                false,
                            ));
                            ui.remove_picking_restriction(self.handle());

                            if let Some(top) = ui.top_picking_restriction() {
                                ui.send_message(WidgetMessage::focus(
                                    top.handle,
                                    MessageDirection::ToWidget,
                                ));
                            }

                            if ui.captured_node() == self.handle() {
                                ui.release_mouse_capture();
                            }
                        }
                    }
                    PopupMessage::Content(content) => {
                        if self.content.is_some() {
                            ui.send_message(WidgetMessage::remove(
                                *self.content,
                                MessageDirection::ToWidget,
                            ));
                        }
                        self.content.set_value_and_mark_modified(*content);

                        ui.send_message(WidgetMessage::link(
                            *self.content,
                            MessageDirection::ToWidget,
                            *self.body,
                        ));
                    }
                    PopupMessage::Placement(placement) => {
                        self.placement.set_value_and_mark_modified(*placement);
                        self.invalidate_layout();
                    }
                    PopupMessage::AdjustPosition => {
                        let new_position =
                            adjust_placement_position(self.screen_bounds(), ui.screen_size());

                        if new_position != self.screen_position() {
                            ui.send_message(WidgetMessage::desired_position(
                                self.handle,
                                MessageDirection::ToWidget,
                                ui.screen_to_root_canvas_space(new_position),
                            ));
                        }
                    }
                    PopupMessage::Owner(owner) => {
                        self.owner = *owner;
                    }
                    PopupMessage::RelayedMessage(_) => (),
                }
            }
        } else if let Some(WidgetMessage::KeyDown(key)) = message.data() {
            if !message.handled() && *key == KeyCode::Escape {
                ui.send_message(PopupMessage::close(self.handle, MessageDirection::ToWidget));
                message.set_handled(true);
            }
        }
        if ui.is_valid_handle(self.owner) && !message.handled() {
            ui.send_message(PopupMessage::relayed_message(
                self.owner,
                MessageDirection::ToWidget,
                message.clone(),
            ));
        }
    }

    fn handle_os_event(
        &mut self,
        self_handle: Handle<UiNode>,
        ui: &mut UserInterface,
        event: &OsEvent,
    ) {
        if let OsEvent::MouseInput { state, .. } = event {
            if let Some(top_restriction) = ui.top_picking_restriction() {
                if *state == ButtonState::Pressed
                    && top_restriction.handle == self_handle
                    && *self.is_open
                {
                    let pos = ui.cursor_position();
                    if !self.widget.screen_bounds().contains(pos) && !*self.stays_open {
                        ui.send_message(PopupMessage::close(
                            self.handle(),
                            MessageDirection::ToWidget,
                        ));
                    }
                }
            }
        }
    }
}

/// Popup widget builder is used to create [`Popup`] widget instances and add them to the user interface.
pub struct PopupBuilder {
    widget_builder: WidgetBuilder,
    placement: Placement,
    stays_open: bool,
    content: Handle<UiNode>,
    smart_placement: bool,
    owner: Handle<UiNode>,
}

impl PopupBuilder {
    /// Creates new builder instance.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            placement: Placement::Cursor(Default::default()),
            stays_open: false,
            content: Default::default(),
            smart_placement: true,
            owner: Default::default(),
        }
    }

    /// Sets the desired popup placement.
    pub fn with_placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    /// Enables or disables smart placement.
    pub fn with_smart_placement(mut self, smart_placement: bool) -> Self {
        self.smart_placement = smart_placement;
        self
    }

    /// Defines whether to keep the popup open when user clicks outside of its content or not.
    pub fn stays_open(mut self, value: bool) -> Self {
        self.stays_open = value;
        self
    }

    /// Sets the content of the popup.
    pub fn with_content(mut self, content: Handle<UiNode>) -> Self {
        self.content = content;
        self
    }

    /// Sets the desired owner of the popup, to which the popup will relay its own messages.
    pub fn with_owner(mut self, owner: Handle<UiNode>) -> Self {
        self.owner = owner;
        self
    }

    /// Builds the popup widget, but does not add it to the user interface. Could be useful if you're making your
    /// own derived version of the popup.
    pub fn build_popup(self, ctx: &mut BuildContext) -> Popup {
        let body = BorderBuilder::new(
            WidgetBuilder::new()
                .with_background(BRUSH_PRIMARY)
                .with_foreground(BRUSH_DARKEST)
                .with_child(self.content),
        )
        .with_stroke_thickness(Thickness::uniform(1.0))
        .build(ctx);

        Popup {
            widget: self
                .widget_builder
                .with_child(body)
                .with_visibility(false)
                .with_handle_os_events(true)
                .build(),
            placement: self.placement.into(),
            stays_open: self.stays_open.into(),
            is_open: false.into(),
            content: self.content.into(),
            smart_placement: self.smart_placement.into(),
            body: body.into(),
            owner: self.owner,
        }
    }

    /// Finishes building the [`Popup`] instance and adds to the user interface and returns its handle.
    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let popup = self.build_popup(ctx);
        ctx.add_node(UiNode::new(popup))
    }
}
