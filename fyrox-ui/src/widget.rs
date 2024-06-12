//! Base widget for every other widget in the crate. It contains layout-specific info, parent-child relations
//! visibility, various transforms, drag'n'drop-related data, etc. See [`Widget`] docs for more info.

#![warn(missing_docs)]

use crate::{
    brush::Brush,
    core::{
        algebra::{Matrix3, Point2, Vector2},
        math::Rect,
        pool::Handle,
        reflect::prelude::*,
        uuid::Uuid,
        visitor::prelude::*,
        ImmutableString,
    },
    define_constructor,
    message::{CursorIcon, Force, KeyCode, MessageDirection, UiMessage},
    HorizontalAlignment, LayoutEvent, MouseButton, MouseState, RcUiNodeHandle, Thickness, UiNode,
    UserInterface, VerticalAlignment, BRUSH_FOREGROUND, BRUSH_PRIMARY,
};
use fyrox_core::{parking_lot::Mutex, variable::InheritableVariable};
use fyrox_graph::BaseSceneGraph;
use fyrox_resource::Resource;
use std::{
    any::Any,
    cell::{Cell, RefCell},
    sync::{mpsc::Sender, Arc},
};

/// A set of messages for any kind of widgets (including user controls). These messages provides basic
/// communication elements of the UI library.
#[derive(Debug, Clone, PartialEq)]
pub enum WidgetMessage {
    /// Initiated when user clicks on a widget's geometry.
    ///
    /// Direction: **From UI**.
    MouseDown {
        /// Position of cursor.
        pos: Vector2<f32>,
        /// A button that was pressed.
        button: MouseButton,
    },

    /// Initiated when user releases mouse button while cursor is over widget's geometry.
    ///
    /// Direction: **From UI**.
    MouseUp {
        /// Position of cursor.
        pos: Vector2<f32>,
        /// A button that was released.
        button: MouseButton,
    },

    /// Initiated when user moves cursor over widget's geometry.
    ///
    /// Direction: **From/To UI**.
    MouseMove {
        /// New position of cursor in screen coordinates.
        pos: Vector2<f32>,
        /// State of mouse buttons.
        state: MouseState,
    },

    /// Initiated when user scrolls mouse wheel while cursor is over widget's geometry.
    ///
    /// Direction: **From/To UI**.
    MouseWheel {
        /// Position of cursor.
        pos: Vector2<f32>,
        /// Amount of lines per mouse wheel turn.
        amount: f32,
    },

    /// Initiated when cursor leaves geometry of a widget.
    ///
    /// Direction: **From UI**.
    MouseLeave,

    /// Initiated when cursor enters geometry of a widget.
    ///
    /// Direction: **From UI**.
    MouseEnter,

    /// Initiated when widget is in focus and user types something on a keyboard.
    ///
    /// Direction: **From/To UI**.
    Text(String),

    /// Initiated when widget is in focus and user presses a button on a keyboard.
    ///
    /// Direction: **From UI**.
    KeyDown(KeyCode),

    /// Initiated when widget is in focus and user releases a button on a keyboard.
    ///
    /// Direction: **From UI**.
    KeyUp(KeyCode),

    /// Initiated when widget received focus (when direction is [`MessageDirection::FromWidget`]). In most cases focus is received
    /// by clicking on widget. You can request focus explicitly by sending this message to a widget with [`MessageDirection::ToWidget`]
    ///
    /// Direction: **From UI/To UI**.
    Focus,

    /// Initiated when widget has lost its focus (when direction is [`MessageDirection::FromWidget`]). Can be used to
    /// removed focus from widget if sent with [`MessageDirection::ToWidget`]
    ///
    /// Direction: **From UI/To UI**.
    Unfocus,

    /// Initiated when dragging of a widget has started.
    ///
    /// Direction: **From UI**.
    DragStarted(Handle<UiNode>),

    /// Initiated when user drags a widget over some other widget.
    ///
    /// Direction: **From UI**.
    DragOver(Handle<UiNode>),

    /// Initiated when user drops a widget onto some other widget.
    ///
    /// Direction: **From UI**.
    Drop(Handle<UiNode>),

    /// A request to make widget topmost. Widget can be made topmost only in the same hierarchy
    /// level only!
    ///
    /// Direction: **From/To UI**.
    Topmost,

    /// A request to make widget lowermost. Widget can be made lowermost only in the same hierarchy
    /// level only!
    ///
    /// Direction: **From/To UI**.
    Lowermost,

    /// A request to detach widget from its current parent and attach to root canvas.
    ///
    /// Direction: **From/To UI**.
    Unlink,

    /// A request to delete widget with all its children widgets. All handles to a node and its
    /// children will be invalid after processing such message!
    ///
    /// Direction: **From/To UI**.
    Remove,

    /// A request to link initiator with specified widget.
    ///
    /// Direction: **From/To UI**.
    LinkWith(Handle<UiNode>),

    /// A request to link initiator with specified widget and put it in front of children list.
    ///
    /// Direction: **From/To UI**.
    LinkWithReverse(Handle<UiNode>),

    /// A request to change background brush of a widget. Background brushes are used to fill volume of widgets.
    ///
    /// Direction: **From/To UI**
    Background(Brush),

    /// A request to change foreground brush of a widget. Foreground brushes are used for text, borders and so on.
    ///
    /// Direction: **From/To UI**
    Foreground(Brush),

    /// A request to change name of a widget. Name is given to widget mostly for debugging purposes.
    ///
    /// Direction: **From/To UI**
    Name(String),

    /// A request to set width of a widget. In most cases there is no need to explicitly set width of a widget,
    /// because fyrox-ui uses automatic layout engine which will correctly calculate desired width of a widget.
    ///
    /// Direction: **From/To UI**
    Width(f32),

    /// A request to set height of a widget. In most cases there is no need to explicitly set height of a widget,
    /// because fyrox-ui uses automatic layout engine which will correctly calculate desired height of a widget.
    ///
    /// Direction: **From/To UI**
    Height(f32),

    /// A request to set vertical alignment of a widget. Vertical alignment tells where to put widget in the parent
    /// widget's bounds in vertical direction.
    ///
    /// Direction: **From/To UI**
    VerticalAlignment(VerticalAlignment),

    /// A request to set horizontal alignment of a widget. Horizontal alignment tells where to put widget in the parent
    /// widget's bounds in horizontal direction.
    ///
    /// Direction: **From/To UI**
    HorizontalAlignment(HorizontalAlignment),

    /// A request to set maximum size of widget. Maximum size restricts size of a widget during layout pass. For example
    /// you can set maximum size to a button which was placed into a grid's cell, if maximum size wouldn't be set, button
    /// would be stretched to fill entire cell.
    ///
    /// Direction: **From/To UI**
    MaxSize(Vector2<f32>),

    /// A request to set minimum size of widget. Minimum size restricts size of a widget during layout pass. For example
    /// you can set minimum size to a button which was placed into a grid's cell, if minimum size wouldn't be set, button
    /// would be compressed to fill entire cell.
    ///
    /// Direction: **From/To UI**
    MinSize(Vector2<f32>),

    /// A request to set row number of a grid to which widget should belong to.
    ///
    /// Direction: **From/To UI**
    ///
    /// # Notes
    ///
    /// This is bad API and it should be changed in future. Grid should have explicit list of pairs (row, child) instead
    /// of this indirect attachment.
    Row(usize),

    /// A request to set column number of a grid to which widget should belong to.
    ///
    /// Direction: **From/To UI**
    ///
    /// # Notes
    ///
    /// This is bad API and it should be changed in future. Grid should have explicit list of pairs (column, child) instead
    /// of this indirect attachment.
    Column(usize),

    /// A request to set new margin of widget. Margin could be used to add some free space around widget to make UI look less
    /// dense.
    ///
    /// Direction: **From/To UI**
    Margin(Thickness),

    /// A request to set new state hit test visibility. If set to false, widget will become "non-clickable". It is useful for
    /// decorations which should be transparent for mouse events.
    ///
    /// Direction: **From/To UI**
    HitTestVisibility(bool),

    /// A request to set new visibility of a widget. Widget can be either visible or not. Invisible widgets does not take space
    /// in layout pass and collapsed to a point.
    ///
    /// Direction: **From/To UI**
    Visibility(bool),

    /// A request to set new z index of a widget. Z index is used to change drawing order of widgets. Please note that it works
    /// only in same hierarchy level, which means that it is impossible to set z index to 9999 (or similar huge value) to force
    /// widget to be drawn on top of everything.
    ///
    /// Direction: **From/To UI**
    ZIndex(usize),

    /// A request to set new desired position of a widget. It is called "desired" because layout system may ignore it and set
    /// some other position. Desired position works with a combination of a layout panel that supports direct coordinated
    /// (Canvas for example).
    ///
    /// Direction: **From/To UI**
    DesiredPosition(Vector2<f32>),

    /// Aligns the widget in the specified widget's bounds using the given options. It can be used only if the
    /// widget is a child of a container, that supports arbitrary positions (such as Canvas).
    Align {
        /// A handle of a node to which the sender of this message should be aligned to.
        relative_to: Handle<UiNode>,
        /// Horizontal alignment of the widget.
        horizontal_alignment: HorizontalAlignment,
        /// Vertical alignment of the widget.
        vertical_alignment: VerticalAlignment,
        /// Margins for each side.
        margin: Thickness,
    },

    /// A request to enable or disable widget. Disabled widget won't receive mouse events and may look differently (it is defined
    /// by internal styling).
    ///
    /// Direction: **From/To UI**
    Enabled(bool),

    /// A request to set desired position at center in local coordinates.
    ///
    /// Direction: **From/To UI**
    Center,

    /// A request to adjust widget's position to fit in parent's bounds.
    AdjustPositionToFit,

    /// A request to set new cursor icon for widget.
    ///
    /// Direction: **From/To UI**
    Cursor(Option<CursorIcon>),

    /// A request to set new opacity for widget.
    ///
    /// Direction: **From/To UI**
    Opacity(Option<f32>),

    /// A request to set new layout transform.
    LayoutTransform(Matrix3<f32>),

    /// A request to set new render transform.
    RenderTransform(Matrix3<f32>),

    /// A double click of a mouse button has occurred on a widget.
    DoubleClick {
        /// A button, that was double-clicked.
        button: MouseButton,
    },

    /// A request to set new context menu for a widget. Old context menu will be removed only if its
    /// reference counter was 1.
    ContextMenu(Option<RcUiNodeHandle>),

    /// A request to set new tooltip for a widget. Old tooltip will be removed only if its reference
    /// counter was 1.
    Tooltip(Option<RcUiNodeHandle>),

    /// Initiated when user places finger on the screen.
    ///
    /// Direction: **From UI**.
    TouchStarted {
        /// position of user's finger
        pos: Vector2<f32>,
        /// pressure exerted on screen at pos
        force: Option<Force>,
        /// unique identifier for touch event
        id: u64,
    },

    /// Initiated when user removes finger from the screen.
    ///
    /// Direction: **From UI**.
    TouchEnded {
        /// position of user's finger
        pos: Vector2<f32>,
        /// unique identifier for touch event
        id: u64,
    },

    /// Initiated when user drags their finger across the screen.
    ///
    /// Direction: **From UI**.
    TouchMoved {
        /// position of user's finger
        pos: Vector2<f32>,
        /// pressure exerted on screen at pos
        force: Option<Force>,
        /// unique identifier for touch event
        id: u64,
    },

    /// Initiated when user cancels their touch event.
    ///
    /// Direction: **From UI**.
    TouchCancelled {
        /// position of user's finger
        pos: Vector2<f32>,
        /// unique identifier for touch event
        id: u64,
    },

    /// Initiated when user taps the screen two or more times in rapid succession.
    ///
    /// Direction: **From UI**.
    DoubleTap {
        /// position of user's finger
        pos: Vector2<f32>,
        /// pressure exerted on screen at pos
        force: Option<Force>,
        /// unique identifier for touch event
        id: u64,
    },
}

impl WidgetMessage {
    define_constructor!(
        /// Creates [`WidgetMessage::Remove`] message.
        WidgetMessage:Remove => fn remove(), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::Unlink`] message.
        WidgetMessage:Unlink => fn unlink(), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::LinkWith`] message.
        WidgetMessage:LinkWith => fn link(Handle<UiNode>), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::LinkWithReverse`] message.
        WidgetMessage:LinkWithReverse => fn link_reverse(Handle<UiNode>), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::Background`] message.
        WidgetMessage:Background => fn background(Brush), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::Foreground`] message.
        WidgetMessage:Foreground => fn foreground(Brush), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::Visibility`] message.
        WidgetMessage:Visibility => fn visibility(bool), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::Width`] message.
        WidgetMessage:Width => fn width(f32), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::Height`] message.
        WidgetMessage:Height => fn height(f32), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::DesiredPosition`] message.
        WidgetMessage:DesiredPosition => fn desired_position(Vector2<f32>), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::Center`] message.
        WidgetMessage:Center => fn center(), layout: true
    );

    define_constructor!(
        /// Creates [`WidgetMessage::AdjustPositionToFit`] message.
        WidgetMessage:AdjustPositionToFit => fn adjust_position_to_fit(), layout: true
    );

    define_constructor!(
        /// Creates [`WidgetMessage::Align`] message.
        WidgetMessage:Align => fn align(
            relative_to: Handle<UiNode>,
            horizontal_alignment: HorizontalAlignment,
            vertical_alignment: VerticalAlignment,
            margin: Thickness),
        layout: true
    );

    define_constructor!(
        /// Creates [`WidgetMessage::Topmost`] message.
        WidgetMessage:Topmost => fn topmost(), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::Lowermost`] message.
        WidgetMessage:Lowermost => fn lowermost(), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::Enabled`] message.
        WidgetMessage:Enabled => fn enabled(bool), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::Name`] message.
        WidgetMessage:Name => fn name(String), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::Row`] message.
        WidgetMessage:Row => fn row(usize), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::Column`] message.
        WidgetMessage:Column => fn column(usize), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::Cursor`] message.
        WidgetMessage:Cursor => fn cursor(Option<CursorIcon>), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::ZIndex`] message.
        WidgetMessage:ZIndex => fn z_index(usize), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::HitTestVisibility`] message.
        WidgetMessage:HitTestVisibility => fn hit_test_visibility(bool), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::Margin`] message.
        WidgetMessage:Margin => fn margin(Thickness), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::MinSize`] message.
        WidgetMessage:MinSize => fn min_size(Vector2<f32>), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::MaxSize`] message.
        WidgetMessage:MaxSize => fn max_size(Vector2<f32>), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::HorizontalAlignment`] message.
        WidgetMessage:HorizontalAlignment => fn horizontal_alignment(HorizontalAlignment), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::VerticalAlignment`] message.
        WidgetMessage:VerticalAlignment => fn vertical_alignment(VerticalAlignment), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::Opacity`] message.
        WidgetMessage:Opacity => fn opacity(Option<f32>), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::LayoutTransform`] message.
        WidgetMessage:LayoutTransform => fn layout_transform(Matrix3<f32>), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::RenderTransform`] message.
        WidgetMessage:RenderTransform => fn render_transform(Matrix3<f32>), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::ContextMenu`] message.
        WidgetMessage:ContextMenu => fn context_menu(Option<RcUiNodeHandle>), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::Tooltip`] message.
        WidgetMessage:Tooltip => fn tooltip(Option<RcUiNodeHandle>), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::Focus`] message.
        WidgetMessage:Focus => fn focus(), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::Unfocus`] message.
        WidgetMessage:Unfocus => fn unfocus(), layout: false
    );

    // Internal messages. Do not use.
    define_constructor!(
        /// Creates [`WidgetMessage::MouseDown`] message. This method is for internal use only, and should not
        /// be used anywhere else.
        WidgetMessage:MouseDown => fn mouse_down(pos: Vector2<f32>, button: MouseButton), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::MouseUp`] message. This method is for internal use only, and should not
        /// be used anywhere else.
        WidgetMessage:MouseUp => fn mouse_up(pos: Vector2<f32>, button: MouseButton), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::MouseMove`] message. This method is for internal use only, and should not
        /// be used anywhere else.
        WidgetMessage:MouseMove => fn mouse_move(pos: Vector2<f32>, state: MouseState), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::MouseWheel`] message. This method is for internal use only, and should not
        /// be used anywhere else.
        WidgetMessage:MouseWheel => fn mouse_wheel(pos: Vector2<f32>, amount: f32), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::MouseLeave`] message. This method is for internal use only, and should not
        /// be used anywhere else.
        WidgetMessage:MouseLeave => fn mouse_leave(), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::MouseEnter`] message. This method is for internal use only, and should not
        /// be used anywhere else.
        WidgetMessage:MouseEnter => fn mouse_enter(), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::Text`] message. This method is for internal use only, and should not
        /// be used anywhere else.
        WidgetMessage:Text => fn text(String), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::KeyDown`] message. This method is for internal use only, and should not
        /// be used anywhere else.
        WidgetMessage:KeyDown => fn key_down(KeyCode), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::KeyUp`] message. This method is for internal use only, and should not
        /// be used anywhere else.
        WidgetMessage:KeyUp => fn key_up(KeyCode), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::DragStarted`] message. This method is for internal use only, and should not
        /// be used anywhere else.
        WidgetMessage:DragStarted => fn drag_started(Handle<UiNode>), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::DragOver`] message. This method is for internal use only, and should not
        /// be used anywhere else.
        WidgetMessage:DragOver => fn drag_over(Handle<UiNode>), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::Drop`] message. This method is for internal use only, and should not
        /// be used anywhere else.
        WidgetMessage:Drop => fn drop(Handle<UiNode>), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::DoubleClick`] message. This method is for internal use only, and should not
        /// be used anywhere else.
        WidgetMessage:DoubleClick => fn double_click(button: MouseButton), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::TouchStarted`] message. This method is for internal use only, and should not
        /// be used anywhere else.
        WidgetMessage:TouchStarted => fn touch_started(pos: Vector2<f32>, force: Option<Force>, id: u64), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::TouchEnded`] message. This method is for internal use only, and should not
        /// be used anywhere else.
        WidgetMessage:TouchEnded => fn touch_ended(pos: Vector2<f32>, id: u64), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::TouchMoved`] message. This method is for internal use only, and should not
        /// be used anywhere else.
        WidgetMessage:TouchMoved => fn touch_moved(pos: Vector2<f32>, force: Option<Force>, id: u64), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::TouchCancelled`] message. This method is for internal use only, and should not
        /// be used anywhere else.
        WidgetMessage:TouchCancelled => fn touch_cancelled(pos: Vector2<f32>, id: u64), layout: false
    );

    define_constructor!(
        /// Creates [`WidgetMessage::DoubleTap`] message. This method is for internal use only, and should not
        /// be used anywhere else.
        WidgetMessage:DoubleTap => fn double_tap(pos: Vector2<f32>, force: Option<Force>, id: u64), layout: false
    );
}

/// Widget is a base UI element, that is always used to build derived, more complex, widgets. In general, it is a container
/// for layout information, basic visual appearance, visibility options, parent-child information. It does almost nothing
/// on its own, instead, the user interface modifies its state accordingly.
#[derive(Default, Debug, Clone, Reflect, Visit)]
pub struct Widget {
    /// Self handle of the widget. It is valid **only**, if the widget is added to the user interface, in other
    /// cases it will most likely be [`Handle::NONE`].
    #[reflect(hidden)]
    pub handle: Handle<UiNode>,
    /// Name of the widget. Could be useful for debugging purposes.
    pub name: ImmutableString,
    /// Desired position relative to the parent node. It is just a recommendation for the layout system, actual position
    /// will be stored in the `actual_local_position` field and can be fetched using [`Widget::actual_local_position`]
    /// method.
    #[reflect(setter = "set_desired_local_position_notify")]
    pub desired_local_position: InheritableVariable<Vector2<f32>>,
    /// Explicit width for the widget, or automatic if [`f32::NAN`] (means the value is undefined). Default is [`f32::NAN`].
    #[reflect(setter = "set_width_notify")]
    pub width: InheritableVariable<f32>,
    /// Explicit height for the widget, or automatic if [`f32::NAN`] (means the value is undefined). Default is [`f32::NAN`].
    #[reflect(setter = "set_height_notify")]
    pub height: InheritableVariable<f32>,
    /// Minimum width and height. Default is 0.0 for both axes.
    #[reflect(setter = "set_min_size_notify")]
    pub min_size: InheritableVariable<Vector2<f32>>,
    /// Maximum width and height. Default is [`f32::INFINITY`] for both axes.
    #[reflect(setter = "set_max_size_notify")]
    pub max_size: InheritableVariable<Vector2<f32>>,
    /// Background brush of the widget.
    pub background: InheritableVariable<Brush>,
    /// Foreground brush of the widget.
    pub foreground: InheritableVariable<Brush>,
    /// Index of the row to which this widget belongs to. It is valid only in when used in [`crate::grid::Grid`] widget.
    #[reflect(setter = "set_row_notify")]
    pub row: InheritableVariable<usize>,
    /// Index of the column to which this widget belongs to. It is valid only in when used in [`crate::grid::Grid`] widget.
    #[reflect(setter = "set_column_notify")]
    pub column: InheritableVariable<usize>,
    /// Vertical alignment of the widget.
    #[reflect(setter = "set_vertical_alignment_notify")]
    pub vertical_alignment: InheritableVariable<VerticalAlignment>,
    /// Horizontal alignment of the widget.
    #[reflect(setter = "set_horizontal_alignment_notify")]
    pub horizontal_alignment: InheritableVariable<HorizontalAlignment>,
    /// Margin for every sides of bounding rectangle. See [`Thickness`] docs for more info.
    #[reflect(setter = "set_margin_notify")]
    pub margin: InheritableVariable<Thickness>,
    /// Current, **local**, visibility state of the widget.
    #[reflect(setter = "set_visibility_notify")]
    pub visibility: InheritableVariable<bool>,
    /// Current, **global** (including the chain of parent widgets), visibility state of the widget.
    #[reflect(hidden)]
    pub global_visibility: bool,
    /// A set of handles to children nodes of this widget.
    #[reflect(hidden)]
    pub children: Vec<Handle<UiNode>>,
    /// A handle to the parent node of this widget.
    #[reflect(hidden)]
    pub parent: Handle<UiNode>,
    /// Indices of drawing commands in the drawing context emitted by this widget. It is used for picking.
    #[reflect(hidden)]
    #[visit(skip)]
    pub command_indices: RefCell<Vec<usize>>,
    /// A flag, that indicates that the mouse is directly over the widget. It will be raised only for top-most widget in the
    /// "stack" of widgets.
    #[reflect(hidden)]
    pub is_mouse_directly_over: bool,
    /// A flag, that defines whether the widget is "visible" for hit testing (picking). Could be useful to prevent some widgets
    /// from any interactions with mouse.
    pub hit_test_visibility: InheritableVariable<bool>,
    /// Index of the widget in parent's children list that defines its order in drawing and picking.
    pub z_index: InheritableVariable<usize>,
    /// A flag, that defines whether the drag from drag'n'drop functionality can be started by the widget or not.
    pub allow_drag: InheritableVariable<bool>,
    /// A flag, that defines whether the drop from drag'n'drop functionality can be accepted by the widget or not.
    pub allow_drop: InheritableVariable<bool>,
    /// Optional, user-defined data.
    #[reflect(hidden)]
    #[visit(skip)]
    pub user_data: Option<Arc<Mutex<dyn Any + Send>>>,
    /// A flag, that defines whether the widget should be drawn in a separate drawind pass after any other widget that draws
    /// normally.
    pub draw_on_top: InheritableVariable<bool>,
    /// A flag, that defines whether the widget is enabled or not. Disabled widgets cannot be interacted by used and they're
    /// greyed out.
    pub enabled: InheritableVariable<bool>,
    /// Optional cursor icon that will be used for mouse cursor when hovering over the widget.
    pub cursor: InheritableVariable<Option<CursorIcon>>,
    /// Optional opacity of the widget. It should be in `[0.0..1.0]` range, where 0.0 - fully transparent, 1.0 - fully opaque.
    pub opacity: InheritableVariable<Option<f32>>,
    /// An optional ref counted handle to a tooltip used by the widget.
    #[visit(optional)]
    pub tooltip: Option<RcUiNodeHandle>,
    /// Maximum available time to show the tooltip after the cursor was moved away from the widget.
    pub tooltip_time: f32,
    /// An optional ref counted handle to a context menu used by the widget.
    #[visit(optional)]
    pub context_menu: Option<RcUiNodeHandle>,
    /// A flag, that defines whether the widget should be clipped by the parent bounds or not.
    pub clip_to_bounds: InheritableVariable<bool>,
    /// Current render transform of the node. It modifies layout information of the widget, as well as it affects visual transform
    /// of the widget.
    #[reflect(hidden)]
    pub layout_transform: Matrix3<f32>,
    /// Current render transform of the node. It only modifies the widget at drawing stage, layout information remains unmodified.
    #[reflect(hidden)]
    pub render_transform: Matrix3<f32>,
    /// Current visual transform of the node. It always contains a result of mixing the layout and render transformation matrices.
    #[reflect(hidden)]
    pub visual_transform: Matrix3<f32>,
    /// A flag, that defines whether the widget will preview UI messages or not. Basically, it defines whether [crate::Control::preview_message]
    /// is called or not.
    pub preview_messages: bool,
    /// A flag, that defines whether the widget will receive any OS events or not. Basically, it defines whether [crate::Control::handle_os_event]
    /// is called or not.
    pub handle_os_events: bool,
    /// Defines the order in which this widget will get keyboard focus when Tab key is pressed.
    /// If set to [`None`], Tab key won't do anything on such widget. Default is [`None`].
    #[visit(optional)]
    pub tab_index: InheritableVariable<Option<usize>>,
    /// A flag, that defines whether the Tab key navigation is enabled or disabled for this widget.
    #[visit(optional)]
    pub tab_stop: InheritableVariable<bool>,
    /// A flag, that defines whether the widget will be update or not. Basically, it defines whether [crate::Control::update]
    /// is called or not.
    #[visit(optional)]
    pub need_update: bool,
    /// Enables (`false`) or disables (`true`) layout rounding.
    #[visit(optional)]
    pub ignore_layout_rounding: bool,
    /// A flag, that indicates that the widget accepts user input. It could be used to determine, if
    /// a user can interact with the widget using keyboard. It is also used for automatic assignment
    /// of the tab index. Keep in mind, that this flag is only a marker and does not do anything else
    /// on its own. Default value is `false`.
    #[visit(optional)]
    pub accepts_input: bool,
    /// Internal sender for layout events.
    #[reflect(hidden)]
    #[visit(skip)]
    pub layout_events_sender: Option<Sender<LayoutEvent>>,
    /// Unique identifier of the widget.
    pub id: Uuid,
    /// A flag, that indicates whether this widget is a root widget of a hierarchy of widgets
    /// instantiated from a resource.
    #[reflect(hidden)]
    #[visit(optional)]
    pub is_resource_instance_root: bool,
    /// A resource from which this widget was instantiated from, can work in pair with `original`
    /// handle to get a corresponding widget from resource.
    #[reflect(read_only)]
    #[visit(optional)]
    pub resource: Option<Resource<UserInterface>>,
    /// Handle to a widget in a user interface resource from which this node was instantiated from.
    #[reflect(hidden)]
    #[visit(optional)]
    pub original_handle_in_resource: Handle<UiNode>,
    //
    // Layout. Interior mutability is a must here because layout performed in a series of recursive calls.
    //
    /// A flag, that defines whether the measurement results are still valid or not.
    #[reflect(hidden)]
    #[visit(skip)]
    pub measure_valid: Cell<bool>,
    /// A flag, that defines whether the arrangement results are still valid or not.
    #[reflect(hidden)]
    #[visit(skip)]
    pub arrange_valid: Cell<bool>,
    /// Results or previous measurement.
    #[reflect(hidden)]
    #[visit(skip)]
    pub prev_measure: Cell<Vector2<f32>>,
    /// Results or previous arrangement.
    #[reflect(hidden)]
    #[visit(skip)]
    pub prev_arrange: Cell<Rect<f32>>,
    /// Desired size of the node after Measure pass.
    #[reflect(hidden)]
    #[visit(skip)]
    pub desired_size: Cell<Vector2<f32>>,
    /// Actual local position of the widget after Arrange pass.
    #[reflect(hidden)]
    #[visit(skip)]
    pub actual_local_position: Cell<Vector2<f32>>,
    /// Actual local size of the widget after Arrange pass.
    #[reflect(hidden)]
    #[visit(skip)]
    pub actual_local_size: Cell<Vector2<f32>>,
    /// Previous global visibility of the widget.
    #[reflect(hidden)]
    #[visit(skip)]
    pub prev_global_visibility: bool,
    /// Current clip bounds of the widget.
    #[reflect(hidden)]
    #[visit(skip)]
    pub clip_bounds: Cell<Rect<f32>>,
}

impl Widget {
    /// Returns self handle of the widget.
    #[inline]
    pub fn handle(&self) -> Handle<UiNode> {
        self.handle
    }

    /// Returns the name of the widget.
    #[inline]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Sets the new name of the widget.
    #[inline]
    pub fn set_name<P: AsRef<str>>(&mut self, name: P) -> &mut Self {
        self.name = ImmutableString::new(name);
        self
    }

    /// Returns the actual size of the widget after the full layout cycle.
    #[inline]
    pub fn actual_local_size(&self) -> Vector2<f32> {
        self.actual_local_size.get()
    }

    /// Returns size of the widget without any layout or rendering transform applied.
    #[inline]
    pub fn actual_initial_size(&self) -> Vector2<f32> {
        Rect::new(
            0.0,
            0.0,
            self.actual_local_size.get().x,
            self.actual_local_size.get().y,
        )
        .transform(&self.visual_transform.try_inverse().unwrap_or_default())
        .size
    }

    /// Returns the actual global size of the widget after the full layout cycle.
    #[inline]
    pub fn actual_global_size(&self) -> Vector2<f32> {
        self.screen_bounds().size
    }

    /// Sets the new minimum size of the widget.
    #[inline]
    pub fn set_min_size(&mut self, value: Vector2<f32>) -> &mut Self {
        self.min_size.set_value_and_mark_modified(value);
        self
    }

    fn set_min_size_notify(&mut self, value: Vector2<f32>) -> Vector2<f32> {
        self.invalidate_layout();
        self.min_size.set_value_and_mark_modified(value)
    }

    /// Sets the new minimum width of the widget.
    #[inline]
    pub fn set_min_width(&mut self, value: f32) -> &mut Self {
        self.min_size.x = value;
        self
    }

    /// Sets the new minimum height of the widget.
    #[inline]
    pub fn set_min_height(&mut self, value: f32) -> &mut Self {
        self.min_size.y = value;
        self
    }

    /// Sets the new minimum size of the widget.
    #[inline]
    pub fn min_size(&self) -> Vector2<f32> {
        *self.min_size
    }

    /// Returns the minimum width of the widget.
    #[inline]
    pub fn min_width(&self) -> f32 {
        self.min_size.x
    }

    /// Returns the minimum height of the widget.
    #[inline]
    pub fn min_height(&self) -> f32 {
        self.min_size.y
    }

    /// Return `true` if the dragging of the widget is allowed, `false` - otherwise.
    #[inline]
    pub fn is_drag_allowed(&self) -> bool {
        *self.allow_drag
    }

    /// Return `true` if the dropping of other widgets is allowed on this widget, `false` - otherwise.
    #[inline]
    pub fn is_drop_allowed(&self) -> bool {
        *self.allow_drop
    }

    /// Maps the given point from screen to local widget's coordinates.
    #[inline]
    pub fn screen_to_local(&self, point: Vector2<f32>) -> Vector2<f32> {
        self.visual_transform
            .try_inverse()
            .unwrap_or_default()
            .transform_point(&Point2::from(point))
            .coords
    }

    /// Invalidates layout of the widget. **WARNING**: Do not use this method, unless you understand what you're doing,
    /// it will cause new layout pass for this widget which could be quite heavy and doing so on every frame for multiple
    /// widgets **will** cause severe performance issues.
    #[inline]
    pub fn invalidate_layout(&self) {
        self.invalidate_measure();
        self.invalidate_arrange();
    }

    /// Invalidates measurement results of the widget. **WARNING**: Do not use this method, unless you understand what you're
    /// doing, it will cause new measurement pass for this widget which could be quite heavy and doing so on every frame for
    /// multiple widgets **will** cause severe performance issues.
    #[inline]
    pub fn invalidate_measure(&self) {
        self.measure_valid.set(false);

        if let Some(layout_events_sender) = self.layout_events_sender.as_ref() {
            let _ = layout_events_sender.send(LayoutEvent::MeasurementInvalidated(self.handle));
        }
    }

    /// Invalidates arrangement results of the widget. **WARNING**: Do not use this method, unless you understand what you're
    /// doing, it will cause new arrangement pass for this widget which could be quite heavy and doing so on every frame for
    /// multiple widgets **will** cause severe performance issues.
    #[inline]
    pub fn invalidate_arrange(&self) {
        self.arrange_valid.set(false);

        if let Some(layout_events_sender) = self.layout_events_sender.as_ref() {
            let _ = layout_events_sender.send(LayoutEvent::ArrangementInvalidated(self.handle));
        }
    }

    /// Returns `true` if the widget is able to participate in hit testing, `false` - otherwise.
    #[inline]
    pub fn is_hit_test_visible(&self) -> bool {
        *self.hit_test_visibility
    }

    /// Sets the new maximum size of the widget.
    #[inline]
    pub fn set_max_size(&mut self, value: Vector2<f32>) -> &mut Self {
        self.max_size.set_value_and_mark_modified(value);
        self
    }

    fn set_max_size_notify(&mut self, value: Vector2<f32>) -> Vector2<f32> {
        self.invalidate_layout();
        std::mem::replace(&mut self.max_size, value)
    }

    /// Returns current maximum size of the widget.
    #[inline]
    pub fn max_size(&self) -> Vector2<f32> {
        *self.max_size
    }

    /// Returns maximum width of the widget.
    #[inline]
    pub fn max_width(&self) -> f32 {
        self.max_size.x
    }

    /// Return maximum height of the widget.
    #[inline]
    pub fn max_height(&self) -> f32 {
        self.max_size.y
    }

    /// Sets new Z index for the widget. Z index defines the sorting (stable) index which will be used to "arrange" widgets
    /// in the correct order.
    #[inline]
    pub fn set_z_index(&mut self, z_index: usize) -> &mut Self {
        self.z_index.set_value_and_mark_modified(z_index);
        self
    }

    /// Returns current Z index of the widget.
    #[inline]
    pub fn z_index(&self) -> usize {
        *self.z_index
    }

    /// Sets the new background of the widget.
    #[inline]
    pub fn set_background(&mut self, brush: Brush) -> &mut Self {
        self.background.set_value_and_mark_modified(brush);
        self
    }

    /// Returns current background of the widget.
    #[inline]
    pub fn background(&self) -> Brush {
        (*self.background).clone()
    }

    /// Sets new foreground of the widget.
    #[inline]
    pub fn set_foreground(&mut self, brush: Brush) -> &mut Self {
        self.foreground.set_value_and_mark_modified(brush);
        self
    }

    /// Returns current foreground of the widget.
    #[inline]
    pub fn foreground(&self) -> Brush {
        (*self.foreground).clone()
    }

    /// Sets new width of the widget.
    #[inline]
    pub fn set_width(&mut self, width: f32) -> &mut Self {
        self.width
            .set_value_and_mark_modified(width.clamp(self.min_size.x, self.max_size.x));
        self
    }

    fn set_width_notify(&mut self, width: f32) -> f32 {
        self.invalidate_layout();
        std::mem::replace(&mut self.width, width)
    }

    /// Returns current width of the widget.
    #[inline]
    pub fn width(&self) -> f32 {
        *self.width
    }

    /// Return `true` if the widget is set to be drawn on top of every other, normally drawn, widgets, `false` - otherwise.
    pub fn is_draw_on_top(&self) -> bool {
        *self.draw_on_top
    }

    /// Sets new height of the widget.
    #[inline]
    pub fn set_height(&mut self, height: f32) -> &mut Self {
        self.height
            .set_value_and_mark_modified(height.clamp(self.min_size.y, self.max_size.y));
        self
    }

    fn set_height_notify(&mut self, height: f32) -> f32 {
        self.invalidate_layout();
        std::mem::replace(&mut self.height, height)
    }

    /// Returns current height of the widget.
    #[inline]
    pub fn height(&self) -> f32 {
        *self.height
    }

    /// Sets the desired local position of the widget.
    #[inline]
    pub fn set_desired_local_position(&mut self, pos: Vector2<f32>) -> &mut Self {
        self.desired_local_position.set_value_and_mark_modified(pos);
        self
    }

    /// Returns current screen-space position of the widget.
    #[inline]
    pub fn screen_position(&self) -> Vector2<f32> {
        Vector2::new(self.visual_transform[6], self.visual_transform[7])
    }

    #[inline]
    pub(crate) fn add_child(&mut self, child: Handle<UiNode>, in_front: bool) {
        self.invalidate_layout();
        if in_front && !self.children.is_empty() {
            self.children.insert(0, child)
        } else {
            self.children.push(child)
        }
    }

    /// Returns a reference to the slice with the children widgets of this widget.
    #[inline(always)]
    pub fn children(&self) -> &[Handle<UiNode>] {
        &self.children
    }

    #[inline]
    pub(crate) fn clear_children(&mut self) {
        self.invalidate_layout();
        self.children.clear();
    }

    #[inline]
    pub(crate) fn remove_child(&mut self, child: Handle<UiNode>) {
        if let Some(i) = self.children.iter().position(|h| *h == child) {
            self.children.remove(i);
            self.invalidate_layout();
        }
    }

    /// Returns current parent handle of the widget.
    #[inline]
    pub fn parent(&self) -> Handle<UiNode> {
        self.parent
    }

    /// Sets new
    #[inline]
    pub(super) fn set_parent(&mut self, parent: Handle<UiNode>) {
        self.parent = parent;
    }

    /// Sets new column of the widget. Columns are used only by [`crate::grid::Grid`] widget.
    #[inline]
    pub fn set_column(&mut self, column: usize) -> &mut Self {
        self.column.set_value_and_mark_modified(column);
        self
    }

    fn set_column_notify(&mut self, column: usize) -> usize {
        self.invalidate_layout();
        std::mem::replace(&mut self.column, column)
    }

    /// Returns current column of the widget. Columns are used only by [`crate::grid::Grid`] widget.
    #[inline]
    pub fn column(&self) -> usize {
        *self.column
    }

    /// Sets new row of the widget. Rows are used only by [`crate::grid::Grid`] widget.
    #[inline]
    pub fn set_row(&mut self, row: usize) -> &mut Self {
        self.row.set_value_and_mark_modified(row);
        self
    }

    fn set_row_notify(&mut self, row: usize) -> usize {
        self.invalidate_layout();
        std::mem::replace(&mut self.row, row)
    }

    /// Returns current row of the widget. Rows are used only by [`crate::grid::Grid`] widget.
    #[inline]
    pub fn row(&self) -> usize {
        *self.row
    }

    /// Returns the desired size of the widget.
    #[inline]
    pub fn desired_size(&self) -> Vector2<f32> {
        self.desired_size.get()
    }

    /// Returns current desired local position of the widget.
    #[inline]
    pub fn desired_local_position(&self) -> Vector2<f32> {
        *self.desired_local_position
    }

    fn set_desired_local_position_notify(&mut self, position: Vector2<f32>) -> Vector2<f32> {
        self.invalidate_layout();
        self.desired_local_position
            .set_value_and_mark_modified(position)
    }

    /// Returns current screen-space bounds of the widget.
    #[inline]
    pub fn screen_bounds(&self) -> Rect<f32> {
        self.bounding_rect().transform(&self.visual_transform)
    }

    /// Returns local-space bounding rect of the widget.
    #[inline]
    pub fn bounding_rect(&self) -> Rect<f32> {
        Rect::new(
            0.0,
            0.0,
            self.actual_local_size.get().x,
            self.actual_local_size.get().y,
        )
    }

    /// Returns current visual transform of the widget.
    #[inline]
    pub fn visual_transform(&self) -> &Matrix3<f32> {
        &self.visual_transform
    }

    /// Returns current render transform of the widget.
    #[inline]
    pub fn render_transform(&self) -> &Matrix3<f32> {
        &self.render_transform
    }

    /// Returns current layout transform of the widget.
    #[inline]
    pub fn layout_transform(&self) -> &Matrix3<f32> {
        &self.layout_transform
    }

    /// Returns `true`, if the widget has a descendant widget with the specified handle, `false` - otherwise.
    pub fn has_descendant(&self, node_handle: Handle<UiNode>, ui: &UserInterface) -> bool {
        for child_handle in self.children.iter() {
            if *child_handle == node_handle {
                return true;
            }

            let result = ui
                .nodes
                .borrow(*child_handle)
                .has_descendant(node_handle, ui);
            if result {
                return true;
            }
        }
        false
    }

    /// Searches a node up on tree starting from the given root that matches a criteria defined by the given func.
    pub fn find_by_criteria_up<Func: Fn(&UiNode) -> bool>(
        &self,
        ui: &UserInterface,
        func: Func,
    ) -> Handle<UiNode> {
        let mut parent_handle = self.parent;
        while parent_handle.is_some() {
            if let Some(parent_node) = ui.nodes.try_borrow(parent_handle) {
                if func(parent_node) {
                    return parent_handle;
                }
                parent_handle = parent_node.parent;
            } else {
                break;
            }
        }
        Handle::NONE
    }

    /// Handles incoming [`WidgetMessage`]s. This method **must** be called in [`crate::control::Control::handle_routed_message`]
    /// of any derived widgets!
    pub fn handle_routed_message(&mut self, _ui: &mut UserInterface, msg: &mut UiMessage) {
        if msg.destination() == self.handle() && msg.direction() == MessageDirection::ToWidget {
            if let Some(msg) = msg.data::<WidgetMessage>() {
                match msg {
                    &WidgetMessage::Opacity(opacity) => {
                        self.opacity.set_value_and_mark_modified(opacity);
                    }
                    WidgetMessage::Background(background) => {
                        self.background
                            .set_value_and_mark_modified(background.clone());
                    }
                    WidgetMessage::Foreground(foreground) => {
                        self.foreground
                            .set_value_and_mark_modified(foreground.clone());
                    }
                    WidgetMessage::Name(name) => self.name = ImmutableString::new(name),
                    &WidgetMessage::Width(width) => {
                        if *self.width != width {
                            self.set_width_notify(width);
                        }
                    }
                    &WidgetMessage::Height(height) => {
                        if *self.height != height {
                            self.set_height_notify(height);
                        }
                    }
                    WidgetMessage::VerticalAlignment(vertical_alignment) => {
                        if *self.vertical_alignment != *vertical_alignment {
                            self.set_vertical_alignment(*vertical_alignment);
                        }
                    }
                    WidgetMessage::HorizontalAlignment(horizontal_alignment) => {
                        if *self.horizontal_alignment != *horizontal_alignment {
                            self.set_horizontal_alignment(*horizontal_alignment);
                        }
                    }
                    WidgetMessage::MaxSize(max_size) => {
                        if *self.max_size != *max_size {
                            self.set_max_size_notify(*max_size);
                        }
                    }
                    WidgetMessage::MinSize(min_size) => {
                        if *self.min_size != *min_size {
                            self.set_min_size_notify(*min_size);
                        }
                    }
                    &WidgetMessage::Row(row) => {
                        if *self.row != row {
                            self.set_row_notify(row);
                        }
                    }
                    &WidgetMessage::Column(column) => {
                        if *self.column != column {
                            self.set_column_notify(column);
                        }
                    }
                    &WidgetMessage::Margin(margin) => {
                        if *self.margin != margin {
                            self.set_margin_notify(margin);
                        }
                    }
                    WidgetMessage::HitTestVisibility(hit_test_visibility) => {
                        self.hit_test_visibility
                            .set_value_and_mark_modified(*hit_test_visibility);
                    }
                    &WidgetMessage::Visibility(visibility) => {
                        self.set_visibility(visibility);
                    }
                    &WidgetMessage::DesiredPosition(pos) => {
                        if *self.desired_local_position != pos {
                            self.set_desired_local_position_notify(pos);
                        }
                    }
                    &WidgetMessage::Enabled(enabled) => {
                        self.enabled.set_value_and_mark_modified(enabled);
                    }
                    &WidgetMessage::Cursor(icon) => {
                        self.cursor.set_value_and_mark_modified(icon);
                    }
                    WidgetMessage::LayoutTransform(transform) => {
                        if &self.layout_transform != transform {
                            self.layout_transform = *transform;
                            self.invalidate_layout();
                        }
                    }
                    WidgetMessage::RenderTransform(transform) => {
                        self.render_transform = *transform;
                    }
                    WidgetMessage::ZIndex(index) => {
                        if *self.z_index != *index {
                            self.z_index.set_value_and_mark_modified(*index);
                            self.invalidate_layout();
                        }
                    }
                    _ => (),
                }
            }
        }
    }

    /// Sets new vertical alignment of the widget.
    #[inline]
    pub fn set_vertical_alignment(&mut self, vertical_alignment: VerticalAlignment) -> &mut Self {
        self.vertical_alignment
            .set_value_and_mark_modified(vertical_alignment);
        self
    }

    fn set_vertical_alignment_notify(
        &mut self,
        vertical_alignment: VerticalAlignment,
    ) -> VerticalAlignment {
        self.invalidate_layout();
        self.vertical_alignment
            .set_value_and_mark_modified(vertical_alignment)
    }

    /// Returns current vertical alignment of the widget.
    #[inline]
    pub fn vertical_alignment(&self) -> VerticalAlignment {
        *self.vertical_alignment
    }

    /// Sets new horizontal alignment of the widget.
    #[inline]
    pub fn set_horizontal_alignment(
        &mut self,
        horizontal_alignment: HorizontalAlignment,
    ) -> &mut Self {
        self.horizontal_alignment
            .set_value_and_mark_modified(horizontal_alignment);
        self
    }

    fn set_horizontal_alignment_notify(
        &mut self,
        horizontal_alignment: HorizontalAlignment,
    ) -> HorizontalAlignment {
        self.invalidate_layout();
        self.horizontal_alignment
            .set_value_and_mark_modified(horizontal_alignment)
    }

    /// Returns current horizontal alignment of the widget.
    #[inline]
    pub fn horizontal_alignment(&self) -> HorizontalAlignment {
        *self.horizontal_alignment
    }

    /// Sets new margin of the widget.
    #[inline]
    pub fn set_margin(&mut self, margin: Thickness) -> &mut Self {
        self.margin.set_value_and_mark_modified(margin);
        self
    }

    fn set_margin_notify(&mut self, margin: Thickness) -> Thickness {
        self.invalidate_layout();
        self.margin.set_value_and_mark_modified(margin)
    }

    /// Returns current margin of the widget.
    #[inline]
    pub fn margin(&self) -> Thickness {
        *self.margin
    }

    /// Performs standard measurement of children nodes. It provides available size as a constraint and returns
    /// the maximum desired size across all children. As a result, this widget will have this size as its desired
    /// size to fit all the children nodes.
    #[inline]
    pub fn measure_override(
        &self,
        ui: &UserInterface,
        available_size: Vector2<f32>,
    ) -> Vector2<f32> {
        let mut size: Vector2<f32> = Vector2::default();

        for &child in self.children.iter() {
            ui.measure_node(child, available_size);
            let desired_size = ui.node(child).desired_size();
            size.x = size.x.max(desired_size.x);
            size.y = size.y.max(desired_size.y);
        }

        size
    }

    /// Performs standard arrangement of the children nodes of the widget. It uses input final size to make a final
    /// bounding rectangle to arrange children. As a result, all the children nodes will be located at the top-left
    /// corner of this widget and stretched to fit its bounds.
    #[inline]
    pub fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        let final_rect = Rect::new(0.0, 0.0, final_size.x, final_size.y);

        for &child in self.children.iter() {
            ui.arrange_node(child, &final_rect);
        }

        final_size
    }

    #[inline]
    pub(crate) fn commit_arrange(&self, position: Vector2<f32>, size: Vector2<f32>) {
        self.actual_local_size.set(size);
        self.actual_local_position.set(position);
        self.arrange_valid.set(true);
    }

    #[inline]
    pub(crate) fn set_children(&mut self, children: Vec<Handle<UiNode>>) {
        self.invalidate_layout();
        self.request_update_visibility();
        self.children = children;
    }

    /// Returns `true` if the current results of arrangement of the widget are valid, `false` - otherwise.
    #[inline(always)]
    pub fn is_arrange_valid(&self) -> bool {
        self.arrange_valid.get()
    }

    #[inline]
    pub(crate) fn commit_measure(&self, desired_size: Vector2<f32>) {
        self.desired_size.set(desired_size);
        self.measure_valid.set(true);
    }

    /// Returns `true` if the current results of measurement of the widget are valid, `false` - otherwise.
    #[inline(always)]
    pub fn is_measure_valid(&self) -> bool {
        self.measure_valid.get()
    }

    /// Returns current actual local position of the widget. It is valid only after layout pass!
    #[inline]
    pub fn actual_local_position(&self) -> Vector2<f32> {
        self.actual_local_position.get()
    }

    /// Returns center point of the widget. It is valid only after layout pass!
    #[inline]
    pub fn center(&self) -> Vector2<f32> {
        self.actual_local_position() + self.actual_local_size().scale(0.5)
    }

    #[inline]
    pub(crate) fn set_global_visibility(&mut self, value: bool) {
        self.prev_global_visibility = self.global_visibility;
        self.global_visibility = value;
    }

    /// Returns `true` of the widget is globally visible, which means that all its parents are visible as well
    /// as this widget. It is valid only after the first update of the layout, otherwise if will be always false.
    #[inline]
    pub fn is_globally_visible(&self) -> bool {
        self.global_visibility
    }

    /// Sets new visibility of the widget.
    #[inline]
    pub fn set_visibility(&mut self, visibility: bool) -> &mut Self {
        if *self.visibility != visibility {
            self.set_visibility_notify(visibility);
        }
        self
    }

    fn set_visibility_notify(&mut self, visibility: bool) -> bool {
        self.invalidate_layout();
        self.request_update_visibility();
        std::mem::replace(&mut self.visibility, visibility)
    }

    /// Requests (via event queue, so the request is deferred) the update of the visibility of the widget.
    #[inline]
    pub fn request_update_visibility(&self) {
        if let Some(layout_events_sender) = self.layout_events_sender.as_ref() {
            let _ = layout_events_sender.send(LayoutEvent::VisibilityChanged(self.handle));
        }
    }

    /// Returns current visibility of the widget.
    #[inline]
    pub fn visibility(&self) -> bool {
        *self.visibility
    }

    /// Enables or disables the widget. Disabled widgets does not interact with user and usually greyed out.
    #[inline]
    pub fn set_enabled(&mut self, enabled: bool) -> &mut Self {
        self.enabled.set_value_and_mark_modified(enabled);
        self
    }

    /// Returns `true` if the widget if enabled, `false` - otherwise.
    #[inline]
    pub fn enabled(&self) -> bool {
        *self.enabled
    }

    /// Sets new cursor of the widget.
    #[inline]
    pub fn set_cursor(&mut self, cursor: Option<CursorIcon>) {
        self.cursor.set_value_and_mark_modified(cursor);
    }

    /// Returns current cursor of the widget.
    #[inline]
    pub fn cursor(&self) -> Option<CursorIcon> {
        *self.cursor
    }

    /// Tries to fetch user-defined data of the specified type `T`.
    #[inline]
    pub fn user_data_cloned<T: Clone + 'static>(&self) -> Option<T> {
        self.user_data.as_ref().and_then(|v| {
            let guard = v.lock();
            guard.downcast_ref::<T>().cloned()
        })
    }

    /// Returns current clipping bounds of the widget. It is valid only after at least one layout pass.
    #[inline]
    pub fn clip_bounds(&self) -> Rect<f32> {
        self.clip_bounds.get()
    }

    /// Set new opacity of the widget. Opacity should be in `[0.0..1.0]` range.
    #[inline]
    pub fn set_opacity(&mut self, opacity: Option<f32>) -> &mut Self {
        self.opacity.set_value_and_mark_modified(opacity);
        self
    }

    /// Returns current opacity of the widget.
    #[inline]
    pub fn opacity(&self) -> Option<f32> {
        *self.opacity
    }

    /// Returns current tooltip handle of the widget.
    #[inline]
    pub fn tooltip(&self) -> Option<RcUiNodeHandle> {
        self.tooltip.clone()
    }

    /// Sets new tooltip handle of the widget (if any).
    #[inline]
    pub fn set_tooltip(&mut self, tooltip: Option<RcUiNodeHandle>) -> &mut Self {
        self.tooltip = tooltip;
        self
    }

    /// Returns maximum available time to show the tooltip after the cursor was moved away from the widget.
    #[inline]
    pub fn tooltip_time(&self) -> f32 {
        self.tooltip_time
    }

    /// Set the maximum available time to show the tooltip after the cursor was moved away from the widget.
    #[inline]
    pub fn set_tooltip_time(&mut self, tooltip_time: f32) -> &mut Self {
        self.tooltip_time = tooltip_time;
        self
    }

    /// Returns current context menu of the widget.
    #[inline]
    pub fn context_menu(&self) -> Option<RcUiNodeHandle> {
        self.context_menu.clone()
    }

    /// The context menu receives `PopupMessage`s for being displayed, and so should support those.
    #[inline]
    pub fn set_context_menu(&mut self, context_menu: Option<RcUiNodeHandle>) -> &mut Self {
        self.context_menu = context_menu;
        self
    }
}

/// Implements `Deref<Target = Widget> + DerefMut` for your widget. It is used to reduce boilerplate code and
/// make it less bug-prone.
#[macro_export]
macro_rules! define_widget_deref {
    ($ty: ty) => {
        impl Deref for $ty {
            type Target = Widget;

            fn deref(&self) -> &Self::Target {
                &self.widget
            }
        }

        impl DerefMut for $ty {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.widget
            }
        }
    };
}

/// Widget builder creates [`Widget`] instances.
pub struct WidgetBuilder {
    /// Name of the widget.
    pub name: String,
    /// Width of the widget.
    pub width: f32,
    /// Height of the widget.
    pub height: f32,
    /// Desired position of the widget.
    pub desired_position: Vector2<f32>,
    /// Vertical alignment of the widget.
    pub vertical_alignment: VerticalAlignment,
    /// Horizontal alignment of the widget.
    pub horizontal_alignment: HorizontalAlignment,
    /// Max size of the widget.
    pub max_size: Option<Vector2<f32>>,
    /// Min size of the widget.
    pub min_size: Option<Vector2<f32>>,
    /// Background brush of the widget.
    pub background: Option<Brush>,
    /// Foreground brush of the widget.
    pub foreground: Option<Brush>,
    /// Row index of the widget.
    pub row: usize,
    /// Column index of the widget.
    pub column: usize,
    /// Margin of the widget.
    pub margin: Thickness,
    /// Children handles of the widget.
    pub children: Vec<Handle<UiNode>>,
    /// Whether the hit test is enabled or not.
    pub is_hit_test_visible: bool,
    /// Whether the widget is visible or not.
    pub visibility: bool,
    /// Z index of the widget.
    pub z_index: usize,
    /// Whether the dragging of the widget is allowed or not.
    pub allow_drag: bool,
    /// Whether the drop of the widget is allowed or not.
    pub allow_drop: bool,
    /// User-defined data.
    pub user_data: Option<Arc<Mutex<dyn Any + Send>>>,
    /// Whether to draw the widget on top of any other or not.
    pub draw_on_top: bool,
    /// Whether the widget is enabled or not.
    pub enabled: bool,
    /// Cursor of the widget.
    pub cursor: Option<CursorIcon>,
    /// Opacity of the widget.
    pub opacity: Option<f32>,
    /// Tooltip of the widget.
    pub tooltip: Option<RcUiNodeHandle>,
    /// Visibility interval (in seconds) of the tooltip of the widget.
    pub tooltip_time: f32,
    /// Context menu of the widget.
    pub context_menu: Option<RcUiNodeHandle>,
    /// Whether the preview messages is enabled or not.
    pub preview_messages: bool,
    /// Whether the widget will handle OS events or not.
    pub handle_os_events: bool,
    /// Whether the widget will be updated or not.
    pub need_update: bool,
    /// Layout transform of the widget.
    pub layout_transform: Matrix3<f32>,
    /// Render transform of the widget.
    pub render_transform: Matrix3<f32>,
    /// Whether the widget bounds should be clipped by its parent or not.
    pub clip_to_bounds: bool,
    /// Unique id of the widget.
    pub id: Uuid,
    /// Defines the order in which this widget will get keyboard focus when Tab key is pressed.
    /// If set to [`None`], Tab key won't do anything on such widget. Default is [`None`].
    pub tab_index: Option<usize>,
    /// A flag, that defines whether the Tab key navigation is enabled or disabled for this widget.
    pub tab_stop: bool,
    /// A flag, that indicates that the widget accepts user input.
    pub accepts_input: bool,
}

impl Default for WidgetBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl WidgetBuilder {
    /// Creates new widget builder with the default values.
    pub fn new() -> Self {
        Self {
            name: Default::default(),
            width: f32::NAN,
            height: f32::NAN,
            vertical_alignment: VerticalAlignment::default(),
            horizontal_alignment: HorizontalAlignment::default(),
            max_size: None,
            min_size: None,
            background: None,
            foreground: None,
            row: 0,
            column: 0,
            margin: Thickness::zero(),
            desired_position: Vector2::default(),
            children: Vec::new(),
            is_hit_test_visible: true,
            visibility: true,
            z_index: 0,
            allow_drag: false,
            allow_drop: false,
            user_data: None,
            draw_on_top: false,
            enabled: true,
            cursor: None,
            opacity: None,
            tooltip: Default::default(),
            tooltip_time: 0.1,
            context_menu: Default::default(),
            preview_messages: false,
            handle_os_events: false,
            need_update: false,
            layout_transform: Matrix3::identity(),
            render_transform: Matrix3::identity(),
            clip_to_bounds: true,
            id: Uuid::new_v4(),
            tab_index: None,
            tab_stop: false,
            accepts_input: false,
        }
    }

    /// Enables or disables message previewing of the widget. It basically defines whether the [`crate::Control::preview_message`] will
    /// be called or not.
    pub fn with_preview_messages(mut self, state: bool) -> Self {
        self.preview_messages = state;
        self
    }

    /// Enables or disables OS event handling of the widget. It basically defines whether the [`crate::Control::handle_os_event`] will
    /// be called or not.
    pub fn with_handle_os_events(mut self, state: bool) -> Self {
        self.handle_os_events = state;
        self
    }

    /// Enables or disables updating of the widget. It basically defines whether the [`crate::Control::update`] will
    /// be called or not.
    pub fn with_need_update(mut self, state: bool) -> Self {
        self.need_update = state;
        self
    }

    /// Sets the desired width of the widget.
    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Sets the desired height of the widget.
    pub fn with_height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Enables or disables clipping of widget's bound to its parent's bounds.
    pub fn with_clip_to_bounds(mut self, clip_to_bounds: bool) -> Self {
        self.clip_to_bounds = clip_to_bounds;
        self
    }

    /// Enables or disables the widget.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets the desired vertical alignment of the widget.
    pub fn with_vertical_alignment(mut self, valign: VerticalAlignment) -> Self {
        self.vertical_alignment = valign;
        self
    }

    /// Sets the desired horizontal alignment of the widget.
    pub fn with_horizontal_alignment(mut self, halign: HorizontalAlignment) -> Self {
        self.horizontal_alignment = halign;
        self
    }

    /// Sets the max size of the widget.
    pub fn with_max_size(mut self, max_size: Vector2<f32>) -> Self {
        self.max_size = Some(max_size);
        self
    }

    /// Sets the min size of the widget.
    pub fn with_min_size(mut self, min_size: Vector2<f32>) -> Self {
        self.min_size = Some(min_size);
        self
    }

    /// Sets the desired background brush of the widget.
    pub fn with_background(mut self, brush: Brush) -> Self {
        self.background = Some(brush);
        self
    }

    /// Sets the desired foreground brush of the widget.
    pub fn with_foreground(mut self, brush: Brush) -> Self {
        self.foreground = Some(brush);
        self
    }

    /// Sets the desired row index of the widget.
    pub fn on_row(mut self, row: usize) -> Self {
        self.row = row;
        self
    }

    /// Sets the desired column index of the widget.
    pub fn on_column(mut self, column: usize) -> Self {
        self.column = column;
        self
    }

    /// Sets the desired margin of the widget.
    pub fn with_margin(mut self, margin: Thickness) -> Self {
        self.margin = margin;
        self
    }

    /// Sets the desired position of the widget.
    pub fn with_desired_position(mut self, desired_position: Vector2<f32>) -> Self {
        self.desired_position = desired_position;
        self
    }

    /// Sets the desired layout transform of the widget.
    pub fn with_layout_transform(mut self, layout_transform: Matrix3<f32>) -> Self {
        self.layout_transform = layout_transform;
        self
    }

    /// Sets the desired render transform of the widget.
    pub fn with_render_transform(mut self, render_transform: Matrix3<f32>) -> Self {
        self.render_transform = render_transform;
        self
    }

    /// Sets the desired Z index of the widget.
    pub fn with_z_index(mut self, z_index: usize) -> Self {
        self.z_index = z_index;
        self
    }

    /// Adds a child handle to the widget. [`Handle::NONE`] values are ignored.
    pub fn with_child(mut self, handle: Handle<UiNode>) -> Self {
        if handle.is_some() {
            self.children.push(handle);
        }
        self
    }

    /// Enables or disables top-most widget drawing.
    pub fn with_draw_on_top(mut self, draw_on_top: bool) -> Self {
        self.draw_on_top = draw_on_top;
        self
    }

    /// Sets the desired set of children nodes.
    pub fn with_children<I: IntoIterator<Item = Handle<UiNode>>>(mut self, children: I) -> Self {
        for child in children.into_iter() {
            if child.is_some() {
                self.children.push(child)
            }
        }
        self
    }

    /// Sets the desired widget name.
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = String::from(name);
        self
    }

    /// Enables or disables hit test of the widget.
    pub fn with_hit_test_visibility(mut self, state: bool) -> Self {
        self.is_hit_test_visible = state;
        self
    }

    /// Sets the desired widget visibility.
    pub fn with_visibility(mut self, visibility: bool) -> Self {
        self.visibility = visibility;
        self
    }

    /// Enables or disables an ability to drop other widgets on this widget.
    pub fn with_allow_drop(mut self, allow_drop: bool) -> Self {
        self.allow_drop = allow_drop;
        self
    }

    /// Enables or disables dragging of the widget.
    pub fn with_allow_drag(mut self, allow_drag: bool) -> Self {
        self.allow_drag = allow_drag;
        self
    }

    /// Sets the desired widget user data.
    pub fn with_user_data(mut self, user_data: Arc<Mutex<dyn Any + Send>>) -> Self {
        self.user_data = Some(user_data);
        self
    }

    /// Sets the desired widget cursor.
    pub fn with_cursor(mut self, cursor: Option<CursorIcon>) -> Self {
        self.cursor = cursor;
        self
    }

    /// Sets the desired widget opacity.
    pub fn with_opacity(mut self, opacity: Option<f32>) -> Self {
        self.opacity = opacity;
        self
    }

    /// Sets the desired widget id.
    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = id;
        self
    }

    /// Sets the desired tooltip for the node.
    ///
    /// ## Important
    ///
    /// The widget will share the tooltip, which means that when widget will be deleted, the
    /// tooltip will be deleted only if there's no one use the tooltip anymore.
    pub fn with_tooltip(mut self, tooltip: RcUiNodeHandle) -> Self {
        self.tooltip = Some(tooltip);
        self
    }

    /// Sets the desired tooltip for the node.
    ///
    /// ## Important
    ///
    /// The widget will share the tooltip, which means that when widget will be deleted, the
    /// tooltip will be deleted only if there's no one use the tooltip anymore.
    pub fn with_opt_tooltip(mut self, tooltip: Option<RcUiNodeHandle>) -> Self {
        self.tooltip = tooltip;
        self
    }

    /// Sets the desired tooltip time.
    pub fn with_tooltip_time(mut self, tooltip_time: f32) -> Self {
        self.tooltip_time = tooltip_time;
        self
    }

    /// The context menu receives `PopupMessage`s for being displayed, and so should support those.
    pub fn with_context_menu(mut self, context_menu: RcUiNodeHandle) -> Self {
        self.context_menu = Some(context_menu);
        self
    }

    /// Sets the desired tab index.
    pub fn with_tab_index(mut self, tab_index: Option<usize>) -> Self {
        self.tab_index = tab_index;
        self
    }

    /// Sets a flag, that defines whether the Tab key navigation is enabled or disabled for this widget.
    pub fn with_tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }

    /// Sets a flag, that indicates that the widget accepts user input.
    pub fn with_accepts_input(mut self, accepts_input: bool) -> Self {
        self.accepts_input = accepts_input;
        self
    }

    /// Finishes building of the base widget.
    pub fn build(self) -> Widget {
        Widget {
            handle: Default::default(),
            name: self.name.into(),
            desired_local_position: self.desired_position.into(),
            width: self.width.into(),
            height: self.height.into(),
            desired_size: Cell::new(Vector2::default()),
            actual_local_position: Cell::new(Vector2::default()),
            actual_local_size: Cell::new(Vector2::default()),
            min_size: self.min_size.unwrap_or_default().into(),
            max_size: self
                .max_size
                .unwrap_or_else(|| Vector2::new(f32::INFINITY, f32::INFINITY))
                .into(),
            background: self
                .background
                .unwrap_or_else(|| BRUSH_PRIMARY.clone())
                .into(),
            foreground: self
                .foreground
                .unwrap_or_else(|| BRUSH_FOREGROUND.clone())
                .into(),
            row: self.row.into(),
            column: self.column.into(),
            vertical_alignment: self.vertical_alignment.into(),
            horizontal_alignment: self.horizontal_alignment.into(),
            margin: self.margin.into(),
            visibility: self.visibility.into(),
            global_visibility: true,
            prev_global_visibility: false,
            children: self.children,
            parent: Handle::NONE,
            command_indices: Default::default(),
            is_mouse_directly_over: false,
            measure_valid: Cell::new(false),
            arrange_valid: Cell::new(false),
            hit_test_visibility: self.is_hit_test_visible.into(),
            prev_measure: Default::default(),
            prev_arrange: Default::default(),
            z_index: self.z_index.into(),
            allow_drag: self.allow_drag.into(),
            allow_drop: self.allow_drop.into(),
            user_data: self.user_data.clone(),
            draw_on_top: self.draw_on_top.into(),
            enabled: self.enabled.into(),
            cursor: self.cursor.into(),
            clip_bounds: Cell::new(Default::default()),
            opacity: self.opacity.into(),
            tooltip: self.tooltip,
            tooltip_time: self.tooltip_time,
            context_menu: self.context_menu,
            preview_messages: self.preview_messages,
            handle_os_events: self.handle_os_events,
            tab_index: self.tab_index.into(),
            tab_stop: self.tab_stop.into(),
            need_update: self.need_update,
            ignore_layout_rounding: false,
            accepts_input: self.accepts_input,
            layout_events_sender: None,
            layout_transform: self.layout_transform,
            render_transform: self.render_transform,
            visual_transform: Matrix3::identity(),
            clip_to_bounds: self.clip_to_bounds.into(),
            id: self.id,
            is_resource_instance_root: false,
            resource: None,
            original_handle_in_resource: Default::default(),
        }
    }
}
