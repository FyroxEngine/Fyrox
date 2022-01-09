use crate::{
    brush::Brush,
    core::{algebra::Vector2, math::Rect, pool::Handle},
    define_constructor,
    message::{CursorIcon, KeyCode, MessageDirection, UiMessage},
    HorizontalAlignment, LayoutEvent, MouseButton, MouseState, Thickness, UiNode, UserInterface,
    VerticalAlignment, BRUSH_FOREGROUND, BRUSH_PRIMARY,
};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    rc::Rc,
    sync::mpsc::Sender,
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
    Text(char),

    /// Initiated when widget is in focus and user presses a button on a keyboard.
    ///
    /// Direction: **From UI**.
    KeyDown(KeyCode),

    /// Initiated when widget is in focus and user releases a button on a keyboard.
    ///
    /// Direction: **From UI**.
    KeyUp(KeyCode),

    /// Initiated when widget received focus. In most cases focus is received by clicking on
    /// widget.
    ///
    /// Direction: **From UI**.
    GotFocus,

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

    /// Initiated when widget has lost its focus.
    ///
    /// Direction: **From UI**.
    LostFocus,

    /// A request to make widget topmost. Widget can be made topmost only in the same hierarchy
    /// level only!
    ///
    /// Direction: **From/To UI**.
    TopMost,

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

    /// A request to enable or disable widget. Disabled widget won't receive mouse events and may look differently (it is defined
    /// by internal styling).
    ///
    /// Direction: **From/To UI**
    Enabled(bool),

    /// A request to set desired position at center in local coordinates.
    ///
    /// Direction: **From/To UI**
    Center,

    /// A request to set new cursor icon for widget.
    ///
    /// Direction: **From/To UI**
    Cursor(Option<CursorIcon>),

    /// A request to set new opacity for widget.
    ///
    /// Direction: **From/To UI**
    Opacity(Option<f32>),
}

impl WidgetMessage {
    define_constructor!(WidgetMessage:Remove => fn remove(), layout: false);
    define_constructor!(WidgetMessage:Unlink => fn unlink(), layout: false);
    define_constructor!(WidgetMessage:LinkWith => fn link(Handle<UiNode>), layout: false);
    define_constructor!(WidgetMessage:LinkWithReverse => fn link_reverse(Handle<UiNode>), layout: false);
    define_constructor!(WidgetMessage:Background => fn background(Brush), layout: false);
    define_constructor!(WidgetMessage:Foreground => fn foreground(Brush), layout: false);
    define_constructor!(WidgetMessage:Visibility => fn visibility(bool), layout: false);
    define_constructor!(WidgetMessage:Width => fn width(f32), layout: false);
    define_constructor!(WidgetMessage:Height => fn height(f32), layout: false);
    define_constructor!(WidgetMessage:DesiredPosition => fn desired_position(Vector2<f32>), layout: false);
    define_constructor!(WidgetMessage:Center => fn center(), layout: true);
    define_constructor!(WidgetMessage:TopMost => fn topmost(), layout: false);
    define_constructor!(WidgetMessage:Enabled => fn enabled(bool), layout: false);
    define_constructor!(WidgetMessage:Name => fn name(String), layout: false);
    define_constructor!(WidgetMessage:Row => fn row(usize), layout: false);
    define_constructor!(WidgetMessage:Column => fn column(usize), layout: false);
    define_constructor!(WidgetMessage:Cursor => fn cursor(Option<CursorIcon>), layout: false);
    define_constructor!(WidgetMessage:ZIndex => fn z_index(usize), layout: false);
    define_constructor!(WidgetMessage:HitTestVisibility => fn hit_test_visibility(bool), layout: false);
    define_constructor!(WidgetMessage:Margin => fn margin(Thickness), layout: false);
    define_constructor!(WidgetMessage:MinSize => fn min_size(Vector2<f32>), layout: false);
    define_constructor!(WidgetMessage:MaxSize => fn max_size(Vector2<f32>), layout: false);
    define_constructor!(WidgetMessage:HorizontalAlignment => fn horizontal_alignment(HorizontalAlignment), layout: false);
    define_constructor!(WidgetMessage:VerticalAlignment => fn vertical_alignment(VerticalAlignment), layout: false);
    define_constructor!(WidgetMessage:Opacity => fn opacity(Option<f32>), layout: false);

    // Internal messages. Do not use.
    define_constructor!(WidgetMessage:GotFocus => fn got_focus(), layout: false);
    define_constructor!(WidgetMessage:LostFocus => fn lost_focus(), layout: false);
    define_constructor!(WidgetMessage:MouseDown => fn mouse_down(pos: Vector2<f32>, button: MouseButton), layout: false);
    define_constructor!(WidgetMessage:MouseUp => fn mouse_up(pos: Vector2<f32>, button: MouseButton), layout: false);
    define_constructor!(WidgetMessage:MouseMove => fn mouse_move(pos: Vector2<f32>, state: MouseState), layout: false);
    define_constructor!(WidgetMessage:MouseWheel => fn mouse_wheel(pos: Vector2<f32>, amount: f32), layout: false);
    define_constructor!(WidgetMessage:MouseLeave => fn mouse_leave(), layout: false);
    define_constructor!(WidgetMessage:MouseEnter => fn mouse_enter(), layout: false);
    define_constructor!(WidgetMessage:Text => fn text(char), layout: false);
    define_constructor!(WidgetMessage:KeyDown => fn key_down(KeyCode), layout: false);
    define_constructor!(WidgetMessage:KeyUp => fn key_up(KeyCode), layout: false);
    define_constructor!(WidgetMessage:DragStarted => fn drag_started(Handle<UiNode>), layout: false);
    define_constructor!(WidgetMessage:DragOver => fn drag_over(Handle<UiNode>), layout: false);
    define_constructor!(WidgetMessage:Drop => fn drop(Handle<UiNode>), layout: false);
}

#[derive(Debug, Clone)]
pub struct Widget {
    pub(in crate) handle: Handle<UiNode>,
    name: String,
    /// Desired position relative to parent node
    desired_local_position: Vector2<f32>,
    /// Explicit width for node or automatic if NaN (means value is undefined). Default is NaN
    width: f32,
    /// Explicit height for node or automatic if NaN (means value is undefined). Default is NaN
    height: f32,
    /// Screen position of the node
    pub(in crate) screen_position: Vector2<f32>,
    /// Minimum width and height
    min_size: Vector2<f32>,
    /// Maximum width and height
    max_size: Vector2<f32>,
    background: Brush,
    foreground: Brush,
    /// Index of row to which this node belongs
    row: usize,
    /// Index of column to which this node belongs
    column: usize,
    /// Vertical alignment
    vertical_alignment: VerticalAlignment,
    /// Horizontal alignment
    horizontal_alignment: HorizontalAlignment,
    /// Margin (four sides)
    margin: Thickness,
    /// Current visibility state
    visibility: bool,
    global_visibility: bool,
    children: Vec<Handle<UiNode>>,
    parent: Handle<UiNode>,
    /// Indices of commands in command buffer emitted by the node.
    pub(in crate) command_indices: RefCell<Vec<usize>>,
    pub(in crate) is_mouse_directly_over: bool,
    hit_test_visibility: bool,
    z_index: usize,
    allow_drag: bool,
    allow_drop: bool,
    pub user_data: Option<Rc<dyn Any>>,
    draw_on_top: bool,
    enabled: bool,
    cursor: Option<CursorIcon>,
    opacity: Option<f32>,
    tooltip: Handle<UiNode>,
    tooltip_time: f32,
    context_menu: Handle<UiNode>,
    pub(in crate) preview_messages: bool,
    pub(in crate) handle_os_events: bool,
    pub(in crate) layout_events_sender: Option<Sender<LayoutEvent>>,

    /// Layout. Interior mutability is a must here because layout performed in
    /// a series of recursive calls.
    pub(in crate) measure_valid: Cell<bool>,
    pub(in crate) arrange_valid: Cell<bool>,
    pub(in crate) prev_measure: Cell<Vector2<f32>>,
    pub(in crate) prev_arrange: Cell<Rect<f32>>,
    /// Desired size of the node after Measure pass.
    pub(in crate) desired_size: Cell<Vector2<f32>>,
    /// Actual node local position after Arrange pass.
    pub(in crate) actual_local_position: Cell<Vector2<f32>>,
    /// Actual size of the node after Arrange pass.
    pub(in crate) actual_size: Cell<Vector2<f32>>,
    pub(in crate) prev_global_visibility: bool,
    pub(in crate) clip_bounds: Cell<Rect<f32>>,
}

impl Widget {
    #[inline]
    pub fn handle(&self) -> Handle<UiNode> {
        self.handle
    }

    #[inline]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    #[inline]
    pub fn set_name<P: AsRef<str>>(&mut self, name: P) -> &mut Self {
        self.name = name.as_ref().to_owned();
        self
    }

    #[inline]
    pub fn actual_size(&self) -> Vector2<f32> {
        self.actual_size.get()
    }

    #[inline]
    pub fn set_min_size(&mut self, value: Vector2<f32>) -> &mut Self {
        self.min_size = value;
        self
    }

    #[inline]
    pub fn set_min_width(&mut self, value: f32) -> &mut Self {
        self.min_size.x = value;
        self
    }

    #[inline]
    pub fn set_min_height(&mut self, value: f32) -> &mut Self {
        self.min_size.y = value;
        self
    }

    #[inline]
    pub fn min_size(&self) -> Vector2<f32> {
        self.min_size
    }

    #[inline]
    pub fn min_width(&self) -> f32 {
        self.min_size.x
    }

    #[inline]
    pub fn min_height(&self) -> f32 {
        self.min_size.y
    }

    #[inline]
    pub fn is_drag_allowed(&self) -> bool {
        self.allow_drag
    }

    #[inline]
    pub fn is_drop_allowed(&self) -> bool {
        self.allow_drop
    }

    #[inline]
    pub fn invalidate_layout(&self) {
        self.invalidate_measure();
        self.invalidate_arrange();
    }

    #[inline]
    pub fn invalidate_measure(&self) {
        self.measure_valid.set(false);

        if let Some(layout_events_sender) = self.layout_events_sender.as_ref() {
            let _ = layout_events_sender.send(LayoutEvent::MeasurementInvalidated(self.handle));
        }
    }

    #[inline]
    pub fn invalidate_arrange(&self) {
        self.arrange_valid.set(false);

        if let Some(layout_events_sender) = self.layout_events_sender.as_ref() {
            let _ = layout_events_sender.send(LayoutEvent::ArrangementInvalidated(self.handle));
        }
    }

    #[inline]
    pub fn is_hit_test_visible(&self) -> bool {
        self.hit_test_visibility
    }

    #[inline]
    pub fn set_max_size(&mut self, value: Vector2<f32>) -> &mut Self {
        self.max_size = value;
        self
    }

    #[inline]
    pub fn max_size(&self) -> Vector2<f32> {
        self.max_size
    }

    #[inline]
    pub fn max_width(&self) -> f32 {
        self.max_size.x
    }

    #[inline]
    pub fn max_height(&self) -> f32 {
        self.max_size.y
    }

    #[inline]
    pub fn set_z_index(&mut self, z_index: usize) -> &mut Self {
        self.z_index = z_index;
        self
    }

    #[inline]
    pub fn z_index(&self) -> usize {
        self.z_index
    }

    #[inline]
    pub fn set_background(&mut self, brush: Brush) -> &mut Self {
        self.background = brush;
        self
    }

    #[inline]
    pub fn background(&self) -> Brush {
        self.background.clone()
    }

    #[inline]
    pub fn set_foreground(&mut self, brush: Brush) -> &mut Self {
        self.foreground = brush;
        self
    }

    #[inline]
    pub fn foreground(&self) -> Brush {
        self.foreground.clone()
    }

    #[inline]
    pub fn set_width(&mut self, width: f32) -> &mut Self {
        self.width = width.max(self.min_size.x).min(self.max_size.x);
        self
    }

    #[inline]
    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn is_draw_on_top(&self) -> bool {
        self.draw_on_top
    }

    #[inline]
    pub fn set_height(&mut self, height: f32) -> &mut Self {
        self.height = height.max(self.min_size.y).min(self.max_size.y);
        self
    }

    #[inline]
    pub fn height(&self) -> f32 {
        self.height
    }

    #[inline]
    pub fn set_desired_local_position(&mut self, pos: Vector2<f32>) -> &mut Self {
        self.desired_local_position = pos;
        self
    }

    #[inline]
    pub fn screen_position(&self) -> Vector2<f32> {
        self.screen_position
    }

    #[inline]
    pub(in crate) fn add_child(&mut self, child: Handle<UiNode>, in_front: bool) {
        self.invalidate_layout();
        if in_front && !self.children.is_empty() {
            self.children.insert(0, child)
        } else {
            self.children.push(child)
        }
    }

    #[inline(always)]
    pub fn children(&self) -> &[Handle<UiNode>] {
        &self.children
    }

    #[inline]
    pub(in crate) fn clear_children(&mut self) {
        self.invalidate_layout();
        self.children.clear();
    }

    #[inline]
    pub(in crate) fn remove_child(&mut self, child: Handle<UiNode>) {
        if let Some(i) = self.children.iter().position(|h| *h == child) {
            self.children.remove(i);
            self.invalidate_layout();
        }
    }

    #[inline]
    pub fn parent(&self) -> Handle<UiNode> {
        self.parent
    }

    #[inline]
    pub fn set_parent(&mut self, parent: Handle<UiNode>) {
        self.parent = parent;
    }

    #[inline]
    pub fn column(&self) -> usize {
        self.column
    }

    #[inline]
    pub fn set_row(&mut self, row: usize) -> &mut Self {
        self.row = row;
        self
    }

    #[inline]
    pub fn row(&self) -> usize {
        self.row
    }

    #[inline]
    pub fn desired_size(&self) -> Vector2<f32> {
        self.desired_size.get()
    }

    #[inline]
    pub fn desired_local_position(&self) -> Vector2<f32> {
        self.desired_local_position
    }

    #[inline]
    pub fn screen_bounds(&self) -> Rect<f32> {
        Rect::new(
            self.screen_position.x,
            self.screen_position.y,
            self.actual_size.get().x,
            self.actual_size.get().y,
        )
    }

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

    /// Searches a node up on tree starting from given root that matches a criteria
    /// defined by a given func.
    pub fn find_by_criteria_up<Func: Fn(&UiNode) -> bool>(
        &self,
        ui: &UserInterface,
        func: Func,
    ) -> Handle<UiNode> {
        let mut parent_handle = self.parent;
        while parent_handle.is_some() {
            let parent_node = ui.nodes.borrow(parent_handle);
            if func(parent_node) {
                return parent_handle;
            }
            parent_handle = parent_node.parent;
        }
        Handle::NONE
    }

    pub fn handle_routed_message(&mut self, _ui: &mut UserInterface, msg: &mut UiMessage) {
        if msg.destination() == self.handle() && msg.direction() == MessageDirection::ToWidget {
            if let Some(msg) = msg.data::<WidgetMessage>() {
                match msg {
                    &WidgetMessage::Opacity(opacity) => self.opacity = opacity,
                    WidgetMessage::Background(background) => self.background = background.clone(),
                    WidgetMessage::Foreground(foreground) => self.foreground = foreground.clone(),
                    WidgetMessage::Name(name) => self.name = name.clone(),
                    &WidgetMessage::Width(width) => {
                        if self.width != width {
                            self.width = width;
                            self.invalidate_layout();
                        }
                    }
                    &WidgetMessage::Height(height) => {
                        if self.height != height {
                            self.height = height;
                            self.invalidate_layout();
                        }
                    }
                    WidgetMessage::VerticalAlignment(vertical_alignment) => {
                        if self.vertical_alignment != *vertical_alignment {
                            self.vertical_alignment = *vertical_alignment;
                            self.invalidate_layout();
                        }
                    }
                    WidgetMessage::HorizontalAlignment(horizontal_alignment) => {
                        if self.horizontal_alignment != *horizontal_alignment {
                            self.horizontal_alignment = *horizontal_alignment;
                            self.invalidate_layout();
                        }
                    }
                    WidgetMessage::MaxSize(max_size) => {
                        if self.max_size != *max_size {
                            self.max_size = *max_size;
                            self.invalidate_layout();
                        }
                    }
                    WidgetMessage::MinSize(min_size) => {
                        if self.min_size != *min_size {
                            self.min_size = *min_size;
                            self.invalidate_layout();
                        }
                    }
                    &WidgetMessage::Row(row) => {
                        if self.row != row {
                            self.row = row;
                            self.invalidate_layout();
                        }
                    }
                    &WidgetMessage::Column(column) => {
                        if self.column != column {
                            self.column = column;
                            self.invalidate_layout();
                        }
                    }
                    &WidgetMessage::Margin(margin) => {
                        if self.margin != margin {
                            self.margin = margin;
                            self.invalidate_layout();
                        }
                    }
                    WidgetMessage::HitTestVisibility(hit_test_visibility) => {
                        self.hit_test_visibility = *hit_test_visibility
                    }
                    &WidgetMessage::Visibility(visibility) => {
                        self.set_visibility(visibility);
                    }
                    &WidgetMessage::DesiredPosition(pos) => {
                        if self.desired_local_position != pos {
                            self.desired_local_position = pos;
                            self.invalidate_layout();
                        }
                    }
                    &WidgetMessage::Enabled(enabled) => {
                        self.enabled = enabled;
                    }
                    &WidgetMessage::Cursor(icon) => {
                        self.cursor = icon;
                    }
                    _ => (),
                }
            }
        }
    }

    #[inline]
    pub fn set_vertical_alignment(&mut self, vertical_alignment: VerticalAlignment) -> &mut Self {
        self.vertical_alignment = vertical_alignment;
        self
    }

    #[inline]
    pub fn vertical_alignment(&self) -> VerticalAlignment {
        self.vertical_alignment
    }

    #[inline]
    pub fn set_horizontal_alignment(
        &mut self,
        horizontal_alignment: HorizontalAlignment,
    ) -> &mut Self {
        self.horizontal_alignment = horizontal_alignment;
        self
    }

    #[inline]
    pub fn horizontal_alignment(&self) -> HorizontalAlignment {
        self.horizontal_alignment
    }

    #[inline]
    pub fn set_column(&mut self, column: usize) -> &mut Self {
        self.column = column;
        self
    }

    #[inline]
    pub fn set_margin(&mut self, margin: Thickness) -> &mut Self {
        self.margin = margin;
        self
    }

    #[inline]
    pub fn margin(&self) -> Thickness {
        self.margin
    }

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

    #[inline]
    pub fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        let final_rect = Rect::new(0.0, 0.0, final_size.x, final_size.y);

        for &child in self.children.iter() {
            ui.arrange_node(child, &final_rect);
        }

        final_size
    }

    #[inline]
    pub(in crate) fn commit_arrange(&self, position: Vector2<f32>, size: Vector2<f32>) {
        self.actual_size.set(size);
        self.actual_local_position.set(position);
        self.arrange_valid.set(true);
    }

    #[inline]
    pub(in crate) fn set_children(&mut self, children: Vec<Handle<UiNode>>) {
        self.invalidate_layout();
        self.children = children;
    }

    #[inline(always)]
    pub fn is_arrange_valid(&self) -> bool {
        self.arrange_valid.get()
    }

    #[inline]
    pub(in crate) fn commit_measure(&self, desired_size: Vector2<f32>) {
        self.desired_size.set(desired_size);
        self.measure_valid.set(true);
    }

    #[inline(always)]
    pub fn is_measure_valid(&self) -> bool {
        self.measure_valid.get()
    }

    #[inline]
    pub fn actual_local_position(&self) -> Vector2<f32> {
        self.actual_local_position.get()
    }

    #[inline]
    pub(in crate) fn set_global_visibility(&mut self, value: bool) {
        self.prev_global_visibility = self.global_visibility;
        self.global_visibility = value;
    }

    #[inline]
    pub fn is_globally_visible(&self) -> bool {
        self.global_visibility
    }

    #[inline]
    pub fn set_visibility(&mut self, visibility: bool) -> &mut Self {
        if self.visibility != visibility {
            self.visibility = visibility;
            self.invalidate_layout();
            if let Some(layout_events_sender) = self.layout_events_sender.as_ref() {
                let _ = layout_events_sender.send(LayoutEvent::VisibilityChanged(self.handle));
            }
        }
        self
    }

    #[inline]
    pub fn visibility(&self) -> bool {
        self.visibility
    }

    #[inline]
    pub fn set_enabled(&mut self, enabled: bool) -> &mut Self {
        self.enabled = enabled;
        self
    }

    #[inline]
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    #[inline]
    pub fn set_cursor(&mut self, cursor: Option<CursorIcon>) {
        self.cursor = cursor;
    }

    #[inline]
    pub fn cursor(&self) -> Option<CursorIcon> {
        self.cursor
    }

    #[inline]
    pub fn user_data_ref<T: 'static>(&self) -> Option<&T> {
        self.user_data.as_ref().and_then(|v| v.downcast_ref::<T>())
    }

    #[inline]
    pub fn clip_bounds(&self) -> Rect<f32> {
        self.clip_bounds.get()
    }

    #[inline]
    pub fn set_opacity(&mut self, opacity: Option<f32>) -> &mut Self {
        self.opacity = opacity;
        self
    }

    #[inline]
    pub fn opacity(&self) -> Option<f32> {
        self.opacity
    }

    #[inline]
    pub fn tooltip(&self) -> Handle<UiNode> {
        self.tooltip
    }

    #[inline]
    pub fn set_tooltip(&mut self, tooltip: Handle<UiNode>) -> &mut Self {
        self.tooltip = tooltip;
        self
    }

    #[inline]
    pub fn tooltip_time(&self) -> f32 {
        self.tooltip_time
    }

    #[inline]
    pub fn set_tooltip_time(&mut self, tooltip_time: f32) -> &mut Self {
        self.tooltip_time = tooltip_time;
        self
    }

    #[inline]
    pub fn context_menu(&self) -> Handle<UiNode> {
        self.context_menu
    }

    #[inline]
    /// The context menu receives `PopupMessage`s for being displayed, and so should support those.
    pub fn set_context_menu(&mut self, context_menu: Handle<UiNode>) -> &mut Self {
        self.context_menu = context_menu;
        self
    }
}

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

pub struct WidgetBuilder {
    pub name: String,
    pub width: f32,
    pub height: f32,
    pub desired_position: Vector2<f32>,
    pub vertical_alignment: VerticalAlignment,
    pub horizontal_alignment: HorizontalAlignment,
    pub max_size: Option<Vector2<f32>>,
    pub min_size: Option<Vector2<f32>>,
    pub background: Option<Brush>,
    pub foreground: Option<Brush>,
    pub row: usize,
    pub column: usize,
    pub margin: Thickness,
    pub children: Vec<Handle<UiNode>>,
    pub is_hit_test_visible: bool,
    pub visibility: bool,
    pub z_index: usize,
    pub allow_drag: bool,
    pub allow_drop: bool,
    pub user_data: Option<Rc<dyn Any>>,
    pub draw_on_top: bool,
    pub enabled: bool,
    pub cursor: Option<CursorIcon>,
    pub opacity: Option<f32>,
    pub tooltip: Handle<UiNode>,
    pub tooltip_time: f32,
    pub context_menu: Handle<UiNode>,
    pub preview_messages: bool,
    pub handle_os_events: bool,
}

impl Default for WidgetBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl WidgetBuilder {
    pub fn new() -> Self {
        Self {
            name: Default::default(),
            width: f32::NAN,
            height: f32::NAN,
            vertical_alignment: VerticalAlignment::Stretch,
            horizontal_alignment: HorizontalAlignment::Stretch,
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
            tooltip: Handle::default(),
            tooltip_time: 0.1,
            context_menu: Handle::default(),
            preview_messages: false,
            handle_os_events: false,
        }
    }

    pub fn with_preview_messages(mut self, state: bool) -> Self {
        self.preview_messages = state;
        self
    }

    pub fn with_handle_os_events(mut self, state: bool) -> Self {
        self.handle_os_events = state;
        self
    }

    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn with_height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn with_vertical_alignment(mut self, valign: VerticalAlignment) -> Self {
        self.vertical_alignment = valign;
        self
    }

    pub fn with_horizontal_alignment(mut self, halign: HorizontalAlignment) -> Self {
        self.horizontal_alignment = halign;
        self
    }

    pub fn with_max_size(mut self, max_size: Vector2<f32>) -> Self {
        self.max_size = Some(max_size);
        self
    }

    pub fn with_min_size(mut self, min_size: Vector2<f32>) -> Self {
        self.min_size = Some(min_size);
        self
    }

    pub fn with_background(mut self, brush: Brush) -> Self {
        self.background = Some(brush);
        self
    }

    pub fn with_foreground(mut self, brush: Brush) -> Self {
        self.foreground = Some(brush);
        self
    }

    pub fn on_row(mut self, row: usize) -> Self {
        self.row = row;
        self
    }

    pub fn on_column(mut self, column: usize) -> Self {
        self.column = column;
        self
    }

    pub fn with_margin(mut self, margin: Thickness) -> Self {
        self.margin = margin;
        self
    }

    pub fn with_desired_position(mut self, desired_position: Vector2<f32>) -> Self {
        self.desired_position = desired_position;
        self
    }

    pub fn with_z_index(mut self, z_index: usize) -> Self {
        self.z_index = z_index;
        self
    }

    pub fn with_child(mut self, handle: Handle<UiNode>) -> Self {
        if handle.is_some() {
            self.children.push(handle);
        }
        self
    }

    pub fn with_draw_on_top(mut self, draw_on_top: bool) -> Self {
        self.draw_on_top = draw_on_top;
        self
    }

    pub fn with_children<I: IntoIterator<Item = Handle<UiNode>>>(mut self, children: I) -> Self {
        for child in children.into_iter() {
            if child.is_some() {
                self.children.push(child)
            }
        }
        self
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name = String::from(name);
        self
    }

    pub fn with_hit_test_visibility(mut self, state: bool) -> Self {
        self.is_hit_test_visible = state;
        self
    }

    pub fn with_visibility(mut self, visibility: bool) -> Self {
        self.visibility = visibility;
        self
    }

    pub fn with_allow_drop(mut self, allow_drop: bool) -> Self {
        self.allow_drop = allow_drop;
        self
    }

    pub fn with_allow_drag(mut self, allow_drag: bool) -> Self {
        self.allow_drag = allow_drag;
        self
    }

    pub fn with_user_data(mut self, user_data: Rc<dyn Any>) -> Self {
        self.user_data = Some(user_data);
        self
    }

    pub fn with_cursor(mut self, cursor: Option<CursorIcon>) -> Self {
        self.cursor = cursor;
        self
    }

    pub fn with_opacity(mut self, opacity: Option<f32>) -> Self {
        self.opacity = opacity;
        self
    }

    /// Sets the desired tooltip for the node.
    ///
    /// ## Important
    ///
    /// The widget will **own** the tooltip, which means that when widget will be deleted, the
    /// tooltip will be deleted too.
    pub fn with_tooltip(mut self, tooltip: Handle<UiNode>) -> Self {
        if tooltip.is_some() {
            self.tooltip = tooltip;
        }
        self
    }

    pub fn with_tooltip_time(mut self, tooltip_time: f32) -> Self {
        self.tooltip_time = tooltip_time;
        self
    }

    /// The context menu receives `PopupMessage`s for being displayed, and so should support those.
    pub fn with_context_menu(mut self, context_menu: Handle<UiNode>) -> Self {
        if context_menu.is_some() {
            self.context_menu = context_menu;
        }
        self
    }

    pub fn build(self) -> Widget {
        Widget {
            handle: Default::default(),
            name: self.name,
            desired_local_position: self.desired_position,
            width: self.width,
            height: self.height,
            screen_position: Vector2::default(),
            desired_size: Cell::new(Vector2::default()),
            actual_local_position: Cell::new(Vector2::default()),
            actual_size: Cell::new(Vector2::default()),
            min_size: self.min_size.unwrap_or_default(),
            max_size: self
                .max_size
                .unwrap_or_else(|| Vector2::new(f32::INFINITY, f32::INFINITY)),
            background: self.background.unwrap_or_else(|| BRUSH_PRIMARY.clone()),
            foreground: self.foreground.unwrap_or_else(|| BRUSH_FOREGROUND.clone()),
            row: self.row,
            column: self.column,
            vertical_alignment: self.vertical_alignment,
            horizontal_alignment: self.horizontal_alignment,
            margin: self.margin,
            visibility: self.visibility,
            global_visibility: true,
            prev_global_visibility: false,
            children: self.children,
            parent: Handle::NONE,
            command_indices: Default::default(),
            is_mouse_directly_over: false,
            measure_valid: Cell::new(false),
            arrange_valid: Cell::new(false),
            hit_test_visibility: self.is_hit_test_visible,
            prev_measure: Default::default(),
            prev_arrange: Default::default(),
            z_index: self.z_index,
            allow_drag: self.allow_drag,
            allow_drop: self.allow_drop,
            user_data: self.user_data.clone(),
            draw_on_top: self.draw_on_top,
            enabled: self.enabled,
            cursor: self.cursor,
            clip_bounds: Cell::new(Default::default()),
            opacity: self.opacity,
            tooltip: self.tooltip,
            tooltip_time: self.tooltip_time,
            context_menu: self.context_menu,
            preview_messages: self.preview_messages,
            handle_os_events: self.handle_os_events,
            layout_events_sender: None,
        }
    }
}
