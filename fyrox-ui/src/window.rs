//! The Window widget provides a standard window that can contain another widget. See [`Window`] docs
//! for more info and usage examples.

use crate::font::FontResource;
use crate::{
    border::BorderBuilder,
    brush::Brush,
    button::{ButtonBuilder, ButtonMessage},
    core::{
        algebra::Vector2, color::Color, math::Rect, pool::Handle, reflect::prelude::*,
        type_traits::prelude::*, uuid_provider, visitor::prelude::*,
    },
    decorator::DecoratorBuilder,
    define_constructor,
    grid::{Column, GridBuilder, Row},
    message::{CursorIcon, KeyCode, MessageDirection, UiMessage},
    navigation::NavigationLayerBuilder,
    text::{Text, TextBuilder, TextMessage},
    vector_image::{Primitive, VectorImageBuilder},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, HorizontalAlignment, RestrictionEntry, Thickness, UiNode, UserInterface,
    VerticalAlignment, BRUSH_BRIGHT, BRUSH_DARKER, BRUSH_LIGHT, BRUSH_LIGHTEST,
};
use fyrox_graph::{BaseSceneGraph, SceneGraph};
use std::{
    cell::RefCell,
    ops::{Deref, DerefMut},
};

/// A set of possible messages that can be used to modify the state of a window or listen to changes
/// in the window.
#[derive(Debug, Clone, PartialEq)]
pub enum WindowMessage {
    /// Opens a window.
    Open {
        /// A flag that defines whether the window should be centered or not.
        center: bool,
        /// A flag that defines whether the content of the window should be focused when the window
        /// is opening.
        focus_content: bool,
    },

    /// Opens a window at the given local coordinates.
    OpenAt {
        position: Vector2<f32>,
        /// A flag that defines whether the content of the window should be focused when the window
        /// is opening.
        focus_content: bool,
    },

    /// Opens a window (optionally modal) and aligns it relative the to the given node.
    OpenAndAlign {
        /// A handle of a node to which the sender of this message should be aligned to.
        relative_to: Handle<UiNode>,
        /// Horizontal alignment of the widget.
        horizontal_alignment: HorizontalAlignment,
        /// Vertical alignment of the widget.
        vertical_alignment: VerticalAlignment,
        /// Margins for each side.
        margin: Thickness,
        /// Should the window be opened in modal mode or not.
        modal: bool,
        /// A flag that defines whether the content of the window should be focused when the window
        /// is opening.
        focus_content: bool,
    },

    /// Opens window in modal mode. Modal mode does **not** blocks current thread, instead
    /// it just restricts mouse and keyboard events only to window so other content is not
    /// clickable/type-able. Closing a window removes that restriction.
    OpenModal {
        /// A flag that defines whether the window should be centered or not.
        center: bool,
        /// A flag that defines whether the content of the window should be focused when the window
        /// is opening.
        focus_content: bool,
    },

    /// Closes a window.
    Close,

    /// Minimizes a window - it differs from classic minimization in window managers,
    /// instead of putting window in system tray, it just collapses internal content panel.
    Minimize(bool),

    /// Forces the window to take the inner size of main application window.
    Maximize,

    /// Whether or not window can be minimized by _ mark. false hides _ mark.
    CanMinimize(bool),

    /// Whether or not window can be closed by X mark. false hides X mark.
    CanClose(bool),

    /// Whether or not window can be resized by resize grips.
    CanResize(bool),

    /// Indicates that move has been started. You should never send this message by hand.
    MoveStart,

    /// Moves window to a new position in local coordinates.
    Move(Vector2<f32>),

    /// Indicated that move has ended. You should never send this message by hand.
    MoveEnd,

    /// Sets new window title.
    Title(WindowTitle),

    /// Safe border size defines "part" of a window that should always be on screen when dragged.
    /// It is used to prevent moving window outside of main application window bounds, to still
    /// be able to drag it.  
    SafeBorderSize(Option<Vector2<f32>>),
}

impl WindowMessage {
    define_constructor!(
        /// Creates [`WindowMessage::Open`] message.
        WindowMessage:Open => fn open(center: bool, focus_content: bool), layout: false
    );
    define_constructor!(
        /// Creates [`WindowMessage::OpenAt`] message.
        WindowMessage:OpenAt => fn open_at(position: Vector2<f32>, focus_content: bool), layout: false
    );
    define_constructor!(
        /// Creates [`WindowMessage::OpenAndAlign`] message.
        WindowMessage:OpenAndAlign => fn open_and_align(
            relative_to: Handle<UiNode>,
            horizontal_alignment: HorizontalAlignment,
            vertical_alignment: VerticalAlignment,
            margin: Thickness,
            modal: bool,
            focus_content: bool
        ), layout: false
    );
    define_constructor!(
        /// Creates [`WindowMessage::OpenModal`] message.
        WindowMessage:OpenModal => fn open_modal(center: bool, focus_content: bool), layout: false
    );
    define_constructor!(
        /// Creates [`WindowMessage::Close`] message.
        WindowMessage:Close => fn close(), layout: false
    );
    define_constructor!(
        /// Creates [`WindowMessage::Minimize`] message.
        WindowMessage:Minimize => fn minimize(bool), layout: false
    );
    define_constructor!(
        /// Creates [`WindowMessage::Maximize`] message.
        WindowMessage:Maximize => fn maximize(), layout: false
    );
    define_constructor!(
        /// Creates [`WindowMessage::CanMinimize`] message.
        WindowMessage:CanMinimize => fn can_minimize(bool), layout: false
    );
    define_constructor!(
        /// Creates [`WindowMessage::CanClose`] message.
        WindowMessage:CanClose => fn can_close(bool), layout: false
    );
    define_constructor!(
        /// Creates [`WindowMessage::CanResize`] message.
        WindowMessage:CanResize => fn can_resize(bool), layout: false
    );
    define_constructor!(
        /// Creates [`WindowMessage::MoveStart`] message.
        WindowMessage:MoveStart => fn move_start(), layout: false
    );
    define_constructor!(
        /// Creates [`WindowMessage::Move`] message.
        WindowMessage:Move => fn move_to(Vector2<f32>), layout: false
    );
    define_constructor!(
        /// Creates [`WindowMessage::MoveEnd`] message.
        WindowMessage:MoveEnd => fn move_end(), layout: false
    );
    define_constructor!(
        /// Creates [`WindowMessage::Title`] message.
        WindowMessage:Title => fn title(WindowTitle), layout: false
    );
    define_constructor!(
        /// Creates [`WindowMessage::SafeBorderSize`] message.
        WindowMessage:SafeBorderSize => fn safe_border_size(Option<Vector2<f32>>), layout: false
    );
}

/// The Window widget provides a standard window that can contain another widget. Based on setting
/// windows can be configured so users can do any of the following:
///
/// * Movable by the user. Not configurable.
/// * Have title text on the title bar. Set by the *with_title* function.
/// * Able to be exited by the user. Set by the *can_close* function.
/// * Able to be minimized to just the Title bar, and of course maximized again. Set by the
/// *can_minimize* function.
/// * Able to resize the window. Set by the *can_resize* function.
///
/// As with other UI elements, you create and configure the window using the WindowBuilder.
///
/// ```rust,no_run
/// # use fyrox_ui::{
/// #     core::{pool::Handle, algebra::Vector2},
/// #     window::{WindowBuilder, WindowTitle},
/// #     text::TextBuilder,
/// #     widget::WidgetBuilder,
/// #     UiNode,
/// #     UserInterface
/// # };
/// fn create_window(ui: &mut UserInterface) {
///     WindowBuilder::new(
///         WidgetBuilder::new()
///             .with_desired_position(Vector2::new(300.0, 0.0))
///             .with_width(300.0),
///     )
///     .with_content(
///         TextBuilder::new(WidgetBuilder::new())
///             .with_text("Example Window content.")
///             .build(&mut ui.build_ctx())
///     )
///     .with_title(WindowTitle::text("Window"))
///     .can_close(true)
///     .can_minimize(true)
///     .open(true)
///     .can_resize(false)
///     .build(&mut ui.build_ctx());
/// }
/// ```
///
/// You will likely want to constrain the initial size of the window to something as shown in the
/// example by providing a set width and/or height to the base WidgetBuilder. Otherwise it will
/// expand to fit it's content.
///
/// You may also want to set an initial position with the *with_desired_position* function called
/// on the base WidgetBuilder which sets the position of the window's top-left corner. Otherwise all
/// your windows will start with it's top-left corner at 0,0 and be stacked on top of each other.
///
/// Windows can only contain a single direct child widget, set by using the *with_content* function.
/// Additional calls to *with_content* replaces the widgets given in previous calls, and the old
/// widgets exist outside the window, so you should delete old widgets before changing a window's
/// widget. If you want multiple widgets, you need to use one of the layout container widgets like
/// the Grid, Stack Panel, etc then add the additional widgets to that widget as needed.
///
/// The Window is a user editable object, but can only be affected by UI Messages they trigger if
/// the message's corresponding variable has been set to true aka what is set by the *can_close*,
/// *can_minimize*, and *can_resize* functions.
///
/// ## Initial Open State
///
/// By default, the window will be created in the open, or maximized, state. You can manually set
/// this state via the *open* function providing a true or false as desired.
///
/// ## Styling the Buttons
///
/// The window close and minimise buttons can be configured with the *with_close_button* and
/// *with_minimize_button* functions. You will want to pass them a button widget, but can do anything
/// else you like past that.
///
/// ## Modal (AKA Forced Focus)
///
/// A Modal in UI design terms indicates a window or box that has forced focus. The user is not able
/// to interact with anything else until the modal is dismissed.
///
/// Any window can be set and unset as a modal via the *modal* function.
#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider)]
pub struct Window {
    /// Base widget of the window.
    pub widget: Widget,
    /// Mouse click position.
    pub mouse_click_pos: Vector2<f32>,
    /// Initial mouse position.
    pub initial_position: Vector2<f32>,
    /// Initial size of the window.
    pub initial_size: Vector2<f32>,
    /// Whether the window is being dragged or not.
    pub is_dragging: bool,
    /// Whether the window is minimized or not.
    pub minimized: bool,
    /// Whether the window can be minimized or not.
    pub can_minimize: bool,
    /// Whether the window can be maximized or not.
    pub can_maximize: bool,
    /// Whether the window can be closed or not.
    pub can_close: bool,
    /// Whether the window can be resized or not.
    pub can_resize: bool,
    /// Handle of a header widget.
    pub header: Handle<UiNode>,
    /// Handle of a minimize button.
    pub minimize_button: Handle<UiNode>,
    /// Handle of a maximize button.
    pub maximize_button: Handle<UiNode>,
    /// Handle of a close button.
    pub close_button: Handle<UiNode>,
    /// A distance per each axis when the dragging starts.
    pub drag_delta: Vector2<f32>,
    /// Handle of a current content.
    pub content: Handle<UiNode>,
    /// Eight grips of the window that are used to resize the window.
    pub grips: RefCell<[Grip; 8]>,
    /// Handle of a title widget of the window.
    pub title: Handle<UiNode>,
    /// Handle of a container widget of the title.
    pub title_grid: Handle<UiNode>,
    /// Optional size of the border around the screen in which the window will be forced to stay.
    pub safe_border_size: Option<Vector2<f32>>,
    /// Bounds of the window before maximization, it is used to return the window to previous
    /// size when it is either "unmaximized" or dragged.
    pub prev_bounds: Option<Rect<f32>>,
    /// If `true`, then the window can be closed using `Esc` key. Default is `true`. Works only if
    /// `can_close` is also `true`.
    #[visit(optional)] // Backward compatibility
    pub close_by_esc: bool,
}

const GRIP_SIZE: f32 = 6.0;
const CORNER_GRIP_SIZE: f32 = GRIP_SIZE * 2.0;

/// Kind of a resizing grip.
#[derive(Copy, Clone, Debug, Visit, Reflect, Default)]
pub enum GripKind {
    /// Left-top corner grip.
    #[default]
    LeftTopCorner = 0,
    /// Right-top corner grip.
    RightTopCorner = 1,
    /// Right-bottom corner grip.
    RightBottomCorner = 2,
    /// Left-bottom corner grip.
    LeftBottomCorner = 3,
    /// Left corner grip.
    Left = 4,
    /// Top corner grip.
    Top = 5,
    /// Right corner grip.
    Right = 6,
    /// Bottom corner grip.
    Bottom = 7,
}

/// Resizing grip.
#[derive(Clone, Visit, Default, Debug, Reflect)]
pub struct Grip {
    /// Kind of the grip.
    pub kind: GripKind,
    /// Bounds of the grip in local-space.
    pub bounds: Rect<f32>,
    /// A flag, that is raised when the grip is being dragged.
    pub is_dragging: bool,
    /// Cursor type of the grip.
    pub cursor: CursorIcon,
}

impl Grip {
    fn new(kind: GripKind, cursor: CursorIcon) -> Self {
        Self {
            kind,
            bounds: Default::default(),
            is_dragging: false,
            cursor,
        }
    }
}

crate::define_widget_deref!(Window);

uuid_provider!(Window = "9331bf32-8614-4005-874c-5239e56bb15e");

impl Control for Window {
    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        let size = self.widget.arrange_override(ui, final_size);

        let mut grips = self.grips.borrow_mut();

        // Adjust grips.
        grips[GripKind::Left as usize].bounds =
            Rect::new(0.0, GRIP_SIZE, GRIP_SIZE, final_size.y - GRIP_SIZE * 2.0);
        grips[GripKind::Top as usize].bounds =
            Rect::new(GRIP_SIZE, 0.0, final_size.x - GRIP_SIZE * 2.0, GRIP_SIZE);
        grips[GripKind::Right as usize].bounds = Rect::new(
            final_size.x - GRIP_SIZE,
            GRIP_SIZE,
            GRIP_SIZE,
            final_size.y - GRIP_SIZE * 2.0,
        );
        grips[GripKind::Bottom as usize].bounds = Rect::new(
            GRIP_SIZE,
            final_size.y - GRIP_SIZE,
            final_size.x - GRIP_SIZE * 2.0,
            GRIP_SIZE,
        );

        // Corners have different size to improve usability.
        grips[GripKind::LeftTopCorner as usize].bounds =
            Rect::new(0.0, 0.0, CORNER_GRIP_SIZE, CORNER_GRIP_SIZE);
        grips[GripKind::RightTopCorner as usize].bounds = Rect::new(
            final_size.x - GRIP_SIZE,
            0.0,
            CORNER_GRIP_SIZE,
            CORNER_GRIP_SIZE,
        );
        grips[GripKind::RightBottomCorner as usize].bounds = Rect::new(
            final_size.x - CORNER_GRIP_SIZE,
            final_size.y - CORNER_GRIP_SIZE,
            CORNER_GRIP_SIZE,
            CORNER_GRIP_SIZE,
        );
        grips[GripKind::LeftBottomCorner as usize].bounds = Rect::new(
            0.0,
            final_size.y - CORNER_GRIP_SIZE,
            CORNER_GRIP_SIZE,
            CORNER_GRIP_SIZE,
        );

        size
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<WidgetMessage>() {
            // Grip interaction have higher priority than other actions.
            if self.can_resize && !self.is_dragging {
                match msg {
                    &WidgetMessage::MouseDown { pos, .. } => {
                        ui.send_message(WidgetMessage::topmost(
                            self.handle(),
                            MessageDirection::ToWidget,
                        ));

                        // Check grips.
                        for grip in self.grips.borrow_mut().iter_mut() {
                            let offset = self.screen_position();
                            let screen_bounds = grip.bounds.translate(offset);
                            if screen_bounds.contains(pos) {
                                grip.is_dragging = true;
                                self.initial_position = self.screen_position();
                                self.initial_size = self.actual_local_size();
                                self.mouse_click_pos = pos;
                                ui.capture_mouse(self.handle());
                                break;
                            }
                        }
                    }
                    WidgetMessage::MouseUp { .. } => {
                        for grip in self.grips.borrow_mut().iter_mut() {
                            if grip.is_dragging {
                                ui.release_mouse_capture();
                                grip.is_dragging = false;
                                break;
                            }
                        }
                    }
                    &WidgetMessage::MouseMove { pos, .. } => {
                        let mut new_cursor = None;

                        for grip in self.grips.borrow().iter() {
                            let offset = self.screen_position();
                            let screen_bounds = grip.bounds.translate(offset);
                            if grip.is_dragging || screen_bounds.contains(pos) {
                                new_cursor = Some(grip.cursor);
                            }

                            if grip.is_dragging {
                                let delta = self.mouse_click_pos - pos;
                                let (dx, dy, dw, dh) = match grip.kind {
                                    GripKind::Left => (-1.0, 0.0, 1.0, 0.0),
                                    GripKind::Top => (0.0, -1.0, 0.0, 1.0),
                                    GripKind::Right => (0.0, 0.0, -1.0, 0.0),
                                    GripKind::Bottom => (0.0, 0.0, 0.0, -1.0),
                                    GripKind::LeftTopCorner => (-1.0, -1.0, 1.0, 1.0),
                                    GripKind::RightTopCorner => (0.0, -1.0, -1.0, 1.0),
                                    GripKind::RightBottomCorner => (0.0, 0.0, -1.0, -1.0),
                                    GripKind::LeftBottomCorner => (-1.0, 0.0, 1.0, -1.0),
                                };

                                let new_pos = self.initial_position
                                    + Vector2::new(delta.x * dx, delta.y * dy);
                                let new_size =
                                    self.initial_size + Vector2::new(delta.x * dw, delta.y * dh);

                                if new_size.x > self.min_width()
                                    && new_size.x < self.max_width()
                                    && new_size.y > self.min_height()
                                    && new_size.y < self.max_height()
                                {
                                    ui.send_message(WidgetMessage::desired_position(
                                        self.handle(),
                                        MessageDirection::ToWidget,
                                        ui.screen_to_root_canvas_space(new_pos),
                                    ));
                                    ui.send_message(WidgetMessage::width(
                                        self.handle(),
                                        MessageDirection::ToWidget,
                                        new_size.x,
                                    ));
                                    ui.send_message(WidgetMessage::height(
                                        self.handle(),
                                        MessageDirection::ToWidget,
                                        new_size.y,
                                    ));
                                }

                                break;
                            }
                        }

                        self.set_cursor(new_cursor);
                    }
                    _ => {}
                }
            } else {
                // The window cannot be resized, so leave the cursor unset.
                self.set_cursor(None);
            }

            if (message.destination() == self.header
                || ui
                    .node(self.header)
                    .has_descendant(message.destination(), ui))
                && !message.handled()
                && !self.has_active_grip()
            {
                match msg {
                    WidgetMessage::MouseDown { pos, .. } => {
                        self.mouse_click_pos = *pos;
                        ui.send_message(WindowMessage::move_start(
                            self.handle,
                            MessageDirection::ToWidget,
                        ));
                        message.set_handled(true);
                    }
                    WidgetMessage::MouseUp { .. } => {
                        ui.send_message(WindowMessage::move_end(
                            self.handle,
                            MessageDirection::ToWidget,
                        ));
                        message.set_handled(true);
                    }
                    WidgetMessage::MouseMove { pos, .. } => {
                        if self.is_dragging {
                            self.drag_delta = *pos - self.mouse_click_pos;
                            let new_pos = self.initial_position + self.drag_delta;
                            ui.send_message(WindowMessage::move_to(
                                self.handle(),
                                MessageDirection::ToWidget,
                                ui.screen_to_root_canvas_space(new_pos),
                            ));
                        }
                        message.set_handled(true);
                    }
                    _ => (),
                }
            }
            match msg {
                WidgetMessage::Unlink => {
                    if message.destination() == self.handle() {
                        self.initial_position = self.screen_position();
                    }
                }
                WidgetMessage::KeyDown(key_code)
                    if self.close_by_esc
                        && !self.is_docked(ui)
                        && self.can_close
                        && *key_code == KeyCode::Escape
                        && !message.handled() =>
                {
                    ui.send_message(WindowMessage::close(
                        self.handle,
                        MessageDirection::ToWidget,
                    ));
                    message.set_handled(true);
                }
                _ => {}
            }
        } else if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == self.minimize_button {
                ui.send_message(WindowMessage::minimize(
                    self.handle(),
                    MessageDirection::ToWidget,
                    !self.minimized,
                ));
            } else if message.destination() == self.maximize_button {
                ui.send_message(WindowMessage::maximize(
                    self.handle(),
                    MessageDirection::ToWidget,
                ));
            } else if message.destination() == self.close_button {
                ui.send_message(WindowMessage::close(
                    self.handle(),
                    MessageDirection::ToWidget,
                ));
            }
        } else if let Some(msg) = message.data::<WindowMessage>() {
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::ToWidget
            {
                match msg {
                    &WindowMessage::Open {
                        center,
                        focus_content,
                    } => {
                        if !self.visibility() {
                            ui.send_message(WidgetMessage::visibility(
                                self.handle(),
                                MessageDirection::ToWidget,
                                true,
                            ));
                            ui.send_message(WidgetMessage::topmost(
                                self.handle(),
                                MessageDirection::ToWidget,
                            ));
                            if center {
                                ui.send_message(WidgetMessage::center(
                                    self.handle(),
                                    MessageDirection::ToWidget,
                                ));
                            }
                            if focus_content {
                                ui.send_message(WidgetMessage::focus(
                                    self.content_to_focus(),
                                    MessageDirection::ToWidget,
                                ));
                            }
                        }
                    }
                    &WindowMessage::OpenAt {
                        position,
                        focus_content,
                    } => {
                        if !self.visibility() {
                            ui.send_message(WidgetMessage::visibility(
                                self.handle(),
                                MessageDirection::ToWidget,
                                true,
                            ));
                            ui.send_message(WidgetMessage::topmost(
                                self.handle(),
                                MessageDirection::ToWidget,
                            ));
                            ui.send_message(WidgetMessage::desired_position(
                                self.handle(),
                                MessageDirection::ToWidget,
                                position,
                            ));
                            if focus_content {
                                ui.send_message(WidgetMessage::focus(
                                    self.content_to_focus(),
                                    MessageDirection::ToWidget,
                                ));
                            }
                        }
                    }
                    &WindowMessage::OpenAndAlign {
                        relative_to,
                        horizontal_alignment,
                        vertical_alignment,
                        margin,
                        modal,
                        focus_content,
                    } => {
                        if !self.visibility() {
                            ui.send_message(WidgetMessage::visibility(
                                self.handle(),
                                MessageDirection::ToWidget,
                                true,
                            ));
                            ui.send_message(WidgetMessage::topmost(
                                self.handle(),
                                MessageDirection::ToWidget,
                            ));
                            ui.send_message(WidgetMessage::align(
                                self.handle(),
                                MessageDirection::ToWidget,
                                relative_to,
                                horizontal_alignment,
                                vertical_alignment,
                                margin,
                            ));
                            if modal {
                                ui.push_picking_restriction(RestrictionEntry {
                                    handle: self.handle(),
                                    stop: true,
                                });
                            }
                            if focus_content {
                                ui.send_message(WidgetMessage::focus(
                                    self.content_to_focus(),
                                    MessageDirection::ToWidget,
                                ));
                            }
                        }
                    }
                    &WindowMessage::OpenModal {
                        center,
                        focus_content,
                    } => {
                        if !self.visibility() {
                            ui.send_message(WidgetMessage::visibility(
                                self.handle(),
                                MessageDirection::ToWidget,
                                true,
                            ));
                            ui.send_message(WidgetMessage::topmost(
                                self.handle(),
                                MessageDirection::ToWidget,
                            ));
                            if center {
                                ui.send_message(WidgetMessage::center(
                                    self.handle(),
                                    MessageDirection::ToWidget,
                                ));
                            }
                            ui.push_picking_restriction(RestrictionEntry {
                                handle: self.handle(),
                                stop: true,
                            });
                            if focus_content {
                                ui.send_message(WidgetMessage::focus(
                                    self.content_to_focus(),
                                    MessageDirection::ToWidget,
                                ));
                            }
                        }
                    }
                    WindowMessage::Close => {
                        if self.visibility() {
                            ui.send_message(WidgetMessage::visibility(
                                self.handle(),
                                MessageDirection::ToWidget,
                                false,
                            ));
                            ui.remove_picking_restriction(self.handle());
                        }
                    }
                    &WindowMessage::Minimize(minimized) => {
                        if self.minimized != minimized {
                            self.minimized = minimized;
                            if self.content.is_some() {
                                ui.send_message(WidgetMessage::visibility(
                                    self.content,
                                    MessageDirection::ToWidget,
                                    !minimized,
                                ));
                            }
                        }
                    }
                    WindowMessage::Maximize => {
                        let current_size = self.actual_local_size();
                        let current_position = self.actual_local_position();
                        let new_bounds = self
                            .prev_bounds
                            .replace(Rect::new(
                                current_position.x,
                                current_position.y,
                                current_size.x,
                                current_size.y,
                            ))
                            .unwrap_or_else(|| {
                                Rect::new(0.0, 0.0, ui.screen_size.x, ui.screen_size.y)
                            });

                        ui.send_message(WidgetMessage::desired_position(
                            self.handle,
                            MessageDirection::ToWidget,
                            new_bounds.position,
                        ));
                        ui.send_message(WidgetMessage::width(
                            self.handle,
                            MessageDirection::ToWidget,
                            new_bounds.w(),
                        ));
                        ui.send_message(WidgetMessage::height(
                            self.handle,
                            MessageDirection::ToWidget,
                            new_bounds.h(),
                        ));
                    }
                    &WindowMessage::CanMinimize(value) => {
                        if self.can_minimize != value {
                            self.can_minimize = value;
                            if self.minimize_button.is_some() {
                                ui.send_message(WidgetMessage::visibility(
                                    self.minimize_button,
                                    MessageDirection::ToWidget,
                                    value,
                                ));
                            }
                        }
                    }
                    &WindowMessage::CanClose(value) => {
                        if self.can_close != value {
                            self.can_close = value;
                            if self.close_button.is_some() {
                                ui.send_message(WidgetMessage::visibility(
                                    self.close_button,
                                    MessageDirection::ToWidget,
                                    value,
                                ));
                            }
                        }
                    }
                    &WindowMessage::CanResize(value) => {
                        if self.can_resize != value {
                            self.can_resize = value;
                            ui.send_message(message.reverse());
                        }
                    }
                    &WindowMessage::Move(mut new_pos) => {
                        if let Some(safe_border) = self.safe_border_size {
                            // Clamp new position in allowed bounds. This will prevent moving the window outside of main
                            // application window, thus leaving an opportunity to drag window to some other place.
                            new_pos.x = new_pos.x.clamp(
                                -(self.actual_local_size().x - safe_border.x).abs(),
                                (ui.screen_size().x - safe_border.x).abs(),
                            );
                            new_pos.y = new_pos
                                .y
                                .clamp(0.0, (ui.screen_size().y - safe_border.y).abs());
                        }

                        if self.is_dragging && self.desired_local_position() != new_pos {
                            ui.send_message(WidgetMessage::desired_position(
                                self.handle(),
                                MessageDirection::ToWidget,
                                new_pos,
                            ));

                            ui.send_message(message.reverse());
                        }
                    }
                    WindowMessage::MoveStart => {
                        if !self.is_dragging {
                            ui.capture_mouse(self.header);
                            let initial_position = self.screen_position();
                            self.initial_position = initial_position;
                            self.is_dragging = true;

                            if let Some(prev_bounds) = self.prev_bounds.take() {
                                ui.send_message(WidgetMessage::width(
                                    self.handle,
                                    MessageDirection::ToWidget,
                                    prev_bounds.w(),
                                ));
                                ui.send_message(WidgetMessage::height(
                                    self.handle,
                                    MessageDirection::ToWidget,
                                    prev_bounds.h(),
                                ));
                            }

                            ui.send_message(message.reverse());
                        }
                    }
                    WindowMessage::MoveEnd => {
                        if self.is_dragging {
                            ui.release_mouse_capture();
                            self.is_dragging = false;

                            ui.send_message(message.reverse());
                        }
                    }
                    WindowMessage::Title(title) => {
                        match title {
                            WindowTitle::Text {
                                text,
                                font,
                                font_size,
                            } => {
                                if ui.try_get_of_type::<Text>(self.title).is_some() {
                                    // Just modify existing text, this is much faster than
                                    // re-create text everytime.
                                    ui.send_message(TextMessage::text(
                                        self.title,
                                        MessageDirection::ToWidget,
                                        text.clone(),
                                    ));
                                    if let Some(font) = font {
                                        ui.send_message(TextMessage::font(
                                            self.title,
                                            MessageDirection::ToWidget,
                                            font.clone(),
                                        ))
                                    }
                                    if let Some(font_size) = font_size {
                                        ui.send_message(TextMessage::font_size(
                                            self.title,
                                            MessageDirection::ToWidget,
                                            *font_size,
                                        ));
                                    }
                                } else {
                                    ui.send_message(WidgetMessage::remove(
                                        self.title,
                                        MessageDirection::ToWidget,
                                    ));
                                    let font =
                                        font.clone().unwrap_or_else(|| ui.default_font.clone());
                                    self.title = make_text_title(
                                        &mut ui.build_ctx(),
                                        text,
                                        font,
                                        (*font_size).unwrap_or(14.0),
                                    );
                                    ui.send_message(WidgetMessage::link(
                                        self.title,
                                        MessageDirection::ToWidget,
                                        self.title_grid,
                                    ));
                                }
                            }
                            WindowTitle::Node(node) => {
                                if self.title.is_some() {
                                    // Remove old title.
                                    ui.send_message(WidgetMessage::remove(
                                        self.title,
                                        MessageDirection::ToWidget,
                                    ));
                                }

                                if node.is_some() {
                                    self.title = *node;

                                    // Attach new one.
                                    ui.send_message(WidgetMessage::link(
                                        self.title,
                                        MessageDirection::ToWidget,
                                        self.title_grid,
                                    ));
                                }
                            }
                        }
                    }
                    WindowMessage::SafeBorderSize(size) => {
                        if &self.safe_border_size != size {
                            self.safe_border_size = *size;
                            ui.send_message(message.reverse());
                        }
                    }
                }
            }
        }
    }
}

impl Window {
    /// Checks whether any resizing grip is active or not.
    pub fn has_active_grip(&self) -> bool {
        for grip in self.grips.borrow().iter() {
            if grip.is_dragging {
                return true;
            }
        }
        false
    }

    fn content_to_focus(&self) -> Handle<UiNode> {
        if self.content.is_some() {
            self.content
        } else {
            self.handle
        }
    }

    fn is_docked(&self, ui: &UserInterface) -> bool {
        self.parent() != ui.root_canvas
    }
}

/// Window builder creates [`Window`] instances and adds them to the user interface.
pub struct WindowBuilder {
    /// Base widget builder.
    pub widget_builder: WidgetBuilder,
    /// Content of the window.
    pub content: Handle<UiNode>,
    /// Optional title of the window.
    pub title: Option<WindowTitle>,
    /// Whether the window can be closed or not.
    pub can_close: bool,
    /// Whether the window can be minimized or not.
    pub can_minimize: bool,
    /// Whether the window can be maximized or not.
    pub can_maximize: bool,
    /// Whether the window should be created open or not.
    pub open: bool,
    /// Optional custom closing button, if not specified, then a default button will be created.
    pub close_button: Option<Handle<UiNode>>,
    /// Optional custom minimization button, if not specified, then a default button will be created.
    pub minimize_button: Option<Handle<UiNode>>,
    /// Optional custom maximization button, if not specified, then a default button will be created.
    pub maximize_button: Option<Handle<UiNode>>,
    /// Whether the window should be created as modal or not. Warning: Any dependant builders must
    /// take this into account!
    pub modal: bool,
    /// Whether the window should be resizable or not.
    pub can_resize: bool,
    /// Optional size of the border around the screen in which the window will be forced to stay.
    pub safe_border_size: Option<Vector2<f32>>,
    /// If `true`, then the window can be closed using `Esc` key. Default is `true`. Works only if
    /// `can_close` is also `true`.
    pub close_by_esc: bool,
}

/// Window title can be either text or node.
///
/// If `Text` is used, then builder will automatically create [`Text`] node with specified text,
/// but with default font.
///
/// If you need more flexibility (i.e. put a picture near text) then [`WindowTitle::Node`] option
/// is for you: it allows to put any UI node hierarchy you want to.
#[derive(Debug, Clone, PartialEq)]
pub enum WindowTitle {
    Text {
        /// Actual text of the title.
        text: String,
        /// Optional font, if [`None`], then the default font will be used.
        font: Option<FontResource>,
        /// Optional size of the text. Default is [`None`] (in this case default size will be used).
        font_size: Option<f32>,
    },
    Node(Handle<UiNode>),
}

impl WindowTitle {
    /// A shortcut to create [`WindowTitle::Text`]
    pub fn text<P: AsRef<str>>(text: P) -> Self {
        WindowTitle::Text {
            text: text.as_ref().to_owned(),
            font: None,
            font_size: None,
        }
    }

    /// A shortcut to create [`WindowTitle::Text`] with custom font.
    pub fn text_with_font<P: AsRef<str>>(text: P, font: FontResource) -> Self {
        WindowTitle::Text {
            text: text.as_ref().to_owned(),
            font: Some(font),
            font_size: None,
        }
    }

    /// A shortcut to create [`WindowTitle::Text`] with custom font and size.
    pub fn text_with_font_size<P: AsRef<str>>(text: P, font: FontResource, size: f32) -> Self {
        WindowTitle::Text {
            text: text.as_ref().to_owned(),
            font: Some(font),
            font_size: Some(size),
        }
    }

    /// A shortcut to create [`WindowTitle::Node`]
    pub fn node(node: Handle<UiNode>) -> Self {
        Self::Node(node)
    }
}

fn make_text_title(
    ctx: &mut BuildContext,
    text: &str,
    font: FontResource,
    size: f32,
) -> Handle<UiNode> {
    TextBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::left(5.0))
            .on_row(0)
            .on_column(0),
    )
    .with_font_size(size)
    .with_font(font)
    .with_vertical_text_alignment(VerticalAlignment::Center)
    .with_horizontal_text_alignment(HorizontalAlignment::Left)
    .with_text(text)
    .build(ctx)
}

enum HeaderButton {
    Close,
    Minimize,
    Maximize,
}

fn make_mark(ctx: &mut BuildContext, button: HeaderButton) -> Handle<UiNode> {
    VectorImageBuilder::new(
        WidgetBuilder::new()
            .with_horizontal_alignment(HorizontalAlignment::Center)
            .with_vertical_alignment(match button {
                HeaderButton::Close => VerticalAlignment::Center,
                HeaderButton::Minimize => VerticalAlignment::Bottom,
                HeaderButton::Maximize => VerticalAlignment::Center,
            })
            .with_margin(match button {
                HeaderButton::Close => Thickness::uniform(0.0),
                HeaderButton::Minimize => Thickness::bottom(3.0),
                HeaderButton::Maximize => Thickness::bottom(0.0),
            })
            .with_foreground(BRUSH_BRIGHT),
    )
    .with_primitives(match button {
        HeaderButton::Close => {
            vec![
                Primitive::Line {
                    begin: Vector2::new(0.0, 0.0),
                    end: Vector2::new(12.0, 12.0),
                    thickness: 1.0,
                },
                Primitive::Line {
                    begin: Vector2::new(12.0, 0.0),
                    end: Vector2::new(0.0, 12.0),
                    thickness: 1.0,
                },
            ]
        }
        HeaderButton::Minimize => {
            vec![Primitive::Line {
                begin: Vector2::new(0.0, 0.0),
                end: Vector2::new(12.0, 0.0),
                thickness: 1.0,
            }]
        }
        HeaderButton::Maximize => {
            let size = 12.0;
            let thickness = 1.25;
            let half_thickness = thickness * 0.5;

            vec![
                Primitive::Line {
                    begin: Vector2::new(0.0, half_thickness),
                    end: Vector2::new(size, half_thickness),
                    thickness,
                },
                Primitive::Line {
                    begin: Vector2::new(size - half_thickness, 0.0),
                    end: Vector2::new(size - half_thickness, size),
                    thickness,
                },
                Primitive::Line {
                    begin: Vector2::new(size, size - half_thickness),
                    end: Vector2::new(0.0, size - half_thickness),
                    thickness,
                },
                Primitive::Line {
                    begin: Vector2::new(half_thickness, size),
                    end: Vector2::new(half_thickness, 0.0),
                    thickness,
                },
            ]
        }
    })
    .build(ctx)
}

fn make_header_button(ctx: &mut BuildContext, button: HeaderButton) -> Handle<UiNode> {
    ButtonBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(2.0)))
        .with_back(
            DecoratorBuilder::new(
                BorderBuilder::new(WidgetBuilder::new())
                    .with_stroke_thickness(Thickness::uniform(0.0))
                    .with_pad_by_corner_radius(false)
                    .with_corner_radius(4.0),
            )
            .with_normal_brush(Brush::Solid(Color::TRANSPARENT))
            .with_hover_brush(BRUSH_LIGHT)
            .with_pressed_brush(BRUSH_LIGHTEST)
            .build(ctx),
        )
        .with_content(make_mark(ctx, button))
        .build(ctx)
}

impl WindowBuilder {
    /// Creates new window builder.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            content: Handle::NONE,
            title: None,
            can_close: true,
            can_minimize: true,
            can_maximize: true,
            open: true,
            close_button: None,
            minimize_button: None,
            maximize_button: None,
            modal: false,
            can_resize: true,
            safe_border_size: Some(Vector2::new(25.0, 20.0)),
            close_by_esc: true,
        }
    }

    /// Sets a desired window content.
    pub fn with_content(mut self, content: Handle<UiNode>) -> Self {
        self.content = content;
        self
    }

    /// Sets a desired window title.
    pub fn with_title(mut self, title: WindowTitle) -> Self {
        self.title = Some(title);
        self
    }

    /// Sets a desired minimization button.
    pub fn with_minimize_button(mut self, button: Handle<UiNode>) -> Self {
        self.minimize_button = Some(button);
        self
    }

    /// Sets a desired maximization button.
    pub fn with_maximize_button(mut self, button: Handle<UiNode>) -> Self {
        self.minimize_button = Some(button);
        self
    }

    /// Sets a desired closing button.
    pub fn with_close_button(mut self, button: Handle<UiNode>) -> Self {
        self.close_button = Some(button);
        self
    }

    /// Sets whether the window can be closed or not.
    pub fn can_close(mut self, can_close: bool) -> Self {
        self.can_close = can_close;
        self
    }

    /// Sets whether the window can be minimized or not.
    pub fn can_minimize(mut self, can_minimize: bool) -> Self {
        self.can_minimize = can_minimize;
        self
    }

    /// Sets whether the window can be maximized or not.
    pub fn can_maximize(mut self, can_minimize: bool) -> Self {
        self.can_maximize = can_minimize;
        self
    }

    /// Sets whether the window should be open or not.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Sets whether the window should be modal or not.
    pub fn modal(mut self, modal: bool) -> Self {
        self.modal = modal;
        self
    }

    /// Sets whether the window can be resized or not.
    pub fn can_resize(mut self, can_resize: bool) -> Self {
        self.can_resize = can_resize;
        self
    }

    /// Sets a desired safe border size.
    pub fn with_safe_border_size(mut self, size: Option<Vector2<f32>>) -> Self {
        self.safe_border_size = size.map(|s| Vector2::new(s.x.abs(), s.y.abs()));
        self
    }

    /// Defines, whether the window can be closed using `Esc` key or not. Works only if `can_close`
    /// is also `true`.
    pub fn with_close_by_esc(mut self, close: bool) -> Self {
        self.close_by_esc = close;
        self
    }

    /// Finishes window building and returns its instance.
    pub fn build_window(self, ctx: &mut BuildContext) -> Window {
        let minimize_button;
        let maximize_button;
        let close_button;

        let title;
        let title_grid;
        let header = BorderBuilder::new(
            WidgetBuilder::new()
                .with_horizontal_alignment(HorizontalAlignment::Stretch)
                .with_height(22.0)
                .with_background(BRUSH_DARKER)
                .with_child({
                    title_grid = GridBuilder::new(
                        WidgetBuilder::new()
                            .with_child({
                                title = match self.title {
                                    None => Handle::NONE,
                                    Some(window_title) => match window_title {
                                        WindowTitle::Node(node) => node,
                                        WindowTitle::Text {
                                            text,
                                            font,
                                            font_size,
                                        } => make_text_title(
                                            ctx,
                                            &text,
                                            font.unwrap_or_else(|| ctx.default_font()),
                                            font_size.unwrap_or(14.0),
                                        ),
                                    },
                                };
                                title
                            })
                            .with_child({
                                minimize_button = self.minimize_button.unwrap_or_else(|| {
                                    make_header_button(ctx, HeaderButton::Minimize)
                                });
                                ctx[minimize_button]
                                    .set_visibility(self.can_minimize)
                                    .set_width(20.0)
                                    .set_row(0)
                                    .set_column(1);
                                minimize_button
                            })
                            .with_child({
                                maximize_button = self.maximize_button.unwrap_or_else(|| {
                                    make_header_button(ctx, HeaderButton::Maximize)
                                });
                                ctx[maximize_button]
                                    .set_visibility(self.can_maximize)
                                    .set_width(20.0)
                                    .set_row(0)
                                    .set_column(2);
                                maximize_button
                            })
                            .with_child({
                                close_button = self.close_button.unwrap_or_else(|| {
                                    make_header_button(ctx, HeaderButton::Close)
                                });
                                ctx[close_button]
                                    .set_width(20.0)
                                    .set_visibility(self.can_close)
                                    .set_row(0)
                                    .set_column(3);
                                close_button
                            }),
                    )
                    .add_column(Column::stretch())
                    .add_column(Column::auto())
                    .add_column(Column::auto())
                    .add_column(Column::auto())
                    .add_row(Row::stretch())
                    .build(ctx);
                    title_grid
                })
                .on_row(0),
        )
        .with_pad_by_corner_radius(false)
        .with_corner_radius(4.0)
        .with_stroke_thickness(Thickness::uniform(0.0))
        .build(ctx);

        Window {
            widget: self
                .widget_builder
                .with_visibility(self.open)
                .with_child(
                    BorderBuilder::new(
                        WidgetBuilder::new()
                            .with_foreground(BRUSH_DARKER)
                            .with_child(
                                GridBuilder::new(
                                    WidgetBuilder::new()
                                        .with_child(
                                            NavigationLayerBuilder::new(
                                                WidgetBuilder::new()
                                                    .on_row(1)
                                                    .with_child(self.content),
                                            )
                                            .build(ctx),
                                        )
                                        .with_child(header),
                                )
                                .add_column(Column::stretch())
                                .add_row(Row::auto())
                                .add_row(Row::stretch())
                                .build(ctx),
                            ),
                    )
                    .with_pad_by_corner_radius(false)
                    .with_corner_radius(4.0)
                    .with_stroke_thickness(Thickness::uniform(1.0))
                    .build(ctx),
                )
                .build(),
            mouse_click_pos: Vector2::default(),
            initial_position: Vector2::default(),
            initial_size: Default::default(),
            is_dragging: false,
            minimized: false,
            can_minimize: self.can_minimize,
            can_maximize: self.can_maximize,
            can_close: self.can_close,
            can_resize: self.can_resize,
            header,
            minimize_button,
            maximize_button,
            close_button,
            drag_delta: Default::default(),
            content: self.content,
            safe_border_size: self.safe_border_size,
            grips: RefCell::new([
                // Corners have priority
                Grip::new(GripKind::LeftTopCorner, CursorIcon::NwResize),
                Grip::new(GripKind::RightTopCorner, CursorIcon::NeResize),
                Grip::new(GripKind::RightBottomCorner, CursorIcon::SeResize),
                Grip::new(GripKind::LeftBottomCorner, CursorIcon::SwResize),
                Grip::new(GripKind::Left, CursorIcon::WResize),
                Grip::new(GripKind::Top, CursorIcon::NResize),
                Grip::new(GripKind::Right, CursorIcon::EResize),
                Grip::new(GripKind::Bottom, CursorIcon::SResize),
            ]),
            title,
            title_grid,
            prev_bounds: None,
            close_by_esc: self.close_by_esc,
        }
    }

    /// Finishes window building and returns its handle.
    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let modal = self.modal;
        let open = self.open;

        let node = self.build_window(ctx);
        let handle = ctx.add_node(UiNode::new(node));

        if modal && open {
            ctx.push_picking_restriction(RestrictionEntry { handle, stop: true });
        }

        handle
    }
}
