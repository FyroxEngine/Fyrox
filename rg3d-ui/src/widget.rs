use crate::{
    brush::Brush,
    core::{algebra::Vector2, math::Rect, pool::Handle},
    message::{CursorIcon, MessageData, MessageDirection, UiMessage, UiMessageData, WidgetMessage},
    Control, HorizontalAlignment, Thickness, UINode, UserInterface, VerticalAlignment,
    BRUSH_FOREGROUND, BRUSH_PRIMARY,
};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    marker::PhantomData,
    rc::Rc,
};

#[derive(Debug, Clone)]
pub struct Widget<M: MessageData, C: Control<M, C>> {
    pub(in crate) handle: Handle<UINode<M, C>>,
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
    children: Vec<Handle<UINode<M, C>>>,
    parent: Handle<UINode<M, C>>,
    /// Indices of commands in command buffer emitted by the node.
    pub(in crate) command_indices: RefCell<Vec<usize>>,
    pub(in crate) is_mouse_directly_over: bool,
    hit_test_visibility: bool,
    z_index: usize,
    allow_drag: bool,
    allow_drop: bool,
    pub user_data: Option<Rc<dyn Any>>,
    draw_on_top: bool,
    marker: PhantomData<M>,
    enabled: bool,
    cursor: Option<CursorIcon>,
    opacity: f32,

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

impl<M: MessageData, C: Control<M, C>> Widget<M, C> {
    pub fn handle(&self) -> Handle<UINode<M, C>> {
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

    pub fn is_drag_allowed(&self) -> bool {
        self.allow_drag
    }

    pub fn is_drop_allowed(&self) -> bool {
        self.allow_drop
    }

    #[inline]
    pub fn invalidate_layout(&self) {
        self.measure_valid.set(false);
        self.arrange_valid.set(false);
    }

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
    pub(in crate) fn add_child(&mut self, child: Handle<UINode<M, C>>, in_front: bool) {
        self.invalidate_layout();
        if in_front && !self.children.is_empty() {
            self.children.insert(0, child)
        } else {
            self.children.push(child)
        }
    }

    #[inline]
    pub fn children(&self) -> &[Handle<UINode<M, C>>] {
        &self.children
    }

    #[inline]
    pub(in crate) fn clear_children(&mut self) {
        self.invalidate_layout();
        self.children.clear();
    }

    #[inline]
    pub(in crate) fn remove_child(&mut self, child: Handle<UINode<M, C>>) {
        if let Some(i) = self.children.iter().position(|h| *h == child) {
            self.children.remove(i);
            self.invalidate_layout();
        }
    }

    #[inline]
    pub fn parent(&self) -> Handle<UINode<M, C>> {
        self.parent
    }

    #[inline]
    pub fn set_parent(&mut self, parent: Handle<UINode<M, C>>) {
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
    pub fn set_visibility(&mut self, visibility: bool) -> &mut Self {
        self.visibility = visibility;
        self
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

    pub fn has_descendant(
        &self,
        node_handle: Handle<UINode<M, C>>,
        ui: &UserInterface<M, C>,
    ) -> bool {
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
    pub fn find_by_criteria_up<Func: Fn(&UINode<M, C>) -> bool>(
        &self,
        ui: &UserInterface<M, C>,
        func: Func,
    ) -> Handle<UINode<M, C>> {
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

    pub fn handle_routed_message(
        &mut self,
        _ui: &mut UserInterface<M, C>,
        msg: &mut UiMessage<M, C>,
    ) {
        if msg.destination() == self.handle() && msg.direction() == MessageDirection::ToWidget {
            if let UiMessageData::Widget(msg) = &msg.data() {
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
                        if self.visibility != visibility {
                            self.visibility = visibility;
                            self.invalidate_layout();
                        }
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
        ui: &UserInterface<M, C>,
        available_size: Vector2<f32>,
    ) -> Vector2<f32> {
        let mut size: Vector2<f32> = Vector2::default();

        for child in self.children.iter().map(|&h| ui.node(h)) {
            child.measure(ui, available_size);
            size.x = size.x.max(child.desired_size.get().x);
            size.y = size.y.max(child.desired_size.get().y);
        }

        size
    }

    #[inline]
    pub fn arrange_override(
        &self,
        ui: &UserInterface<M, C>,
        final_size: Vector2<f32>,
    ) -> Vector2<f32> {
        let final_rect = Rect::new(0.0, 0.0, final_size.x, final_size.y);

        for child in self.children.iter().map(|&h| ui.node(h)) {
            child.arrange(ui, &final_rect);
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
    pub(in crate) fn set_children(&mut self, children: Vec<Handle<UINode<M, C>>>) {
        self.invalidate_layout();
        self.children = children;
    }

    #[inline]
    pub fn is_arrange_valid(&self) -> bool {
        self.arrange_valid.get()
    }

    #[inline]
    pub(in crate) fn commit_measure(&self, desired_size: Vector2<f32>) {
        self.desired_size.set(desired_size);
        self.measure_valid.set(true);
    }

    #[inline]
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
    pub fn visibility(&self) -> bool {
        self.visibility
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
    pub fn user_data_ref<T: 'static>(&self) -> &T {
        self.user_data
            .as_ref()
            .unwrap()
            .downcast_ref::<T>()
            .unwrap()
    }

    pub fn clip_bounds(&self) -> Rect<f32> {
        self.clip_bounds.get()
    }

    pub fn opacity(&self) -> f32 {
        self.opacity
    }
}

#[macro_export]
macro_rules! define_widget_deref {
    ($ty: ty) => {
        impl<M: MessageData, C: Control<M, C>> Deref for $ty {
            type Target = Widget<M, C>;

            fn deref(&self) -> &Self::Target {
                &self.widget
            }
        }

        impl<M: MessageData, C: Control<M, C>> DerefMut for $ty {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.widget
            }
        }
    };
}

pub struct WidgetBuilder<M: MessageData, C: Control<M, C>> {
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
    pub children: Vec<Handle<UINode<M, C>>>,
    pub is_hit_test_visible: bool,
    pub visibility: bool,
    pub z_index: usize,
    pub allow_drag: bool,
    pub allow_drop: bool,
    pub user_data: Option<Rc<dyn Any>>,
    pub draw_on_top: bool,
    pub enabled: bool,
    pub cursor: Option<CursorIcon>,
    pub opacity: f32,
}

impl<M: MessageData, C: Control<M, C>> Default for WidgetBuilder<M, C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: MessageData, C: Control<M, C>> WidgetBuilder<M, C> {
    pub fn new() -> Self {
        Self {
            name: Default::default(),
            width: std::f32::NAN,
            height: std::f32::NAN,
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
            opacity: 1.0,
        }
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

    pub fn with_child(mut self, handle: Handle<UINode<M, C>>) -> Self {
        if handle.is_some() {
            self.children.push(handle);
        }
        self
    }

    pub fn with_draw_on_top(mut self, draw_on_top: bool) -> Self {
        self.draw_on_top = draw_on_top;
        self
    }

    pub fn with_children<'a, I: IntoIterator<Item = &'a Handle<UINode<M, C>>>>(
        mut self,
        children: I,
    ) -> Self {
        for &child in children.into_iter() {
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

    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    pub fn build(self) -> Widget<M, C> {
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
                .unwrap_or_else(|| Vector2::new(std::f32::INFINITY, std::f32::INFINITY)),
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
            marker: PhantomData,
            enabled: self.enabled,
            cursor: self.cursor,
            clip_bounds: Cell::new(Default::default()),
            opacity: self.opacity,
        }
    }
}
