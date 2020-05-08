use crate::{
    core::{
        color::Color,
        math::{
            vec2::Vec2,
            Rect,
        },
        pool::Handle,
    },
    VerticalAlignment,
    HorizontalAlignment,
    Thickness,
    UserInterface,
    UINode,
    Control,
    brush::Brush,
    message::{
        UiMessageData,
        WidgetMessage,
        UiMessage,
        WidgetProperty,
    },
};
use std::{
    cell::{
        RefCell,
        Cell,
    },
};
use std::sync::mpsc::Sender;
use std::any::Any;
use std::rc::Rc;

pub struct Widget<M: 'static, C: 'static + Control<M, C>> {
    pub(in crate) handle: Handle<UINode<M, C>>,
    name: String,
    /// Desired position relative to parent node
    desired_local_position: Cell<Vec2>,
    /// Explicit width for node or automatic if NaN (means value is undefined). Default is NaN
    width: Cell<f32>,
    /// Explicit height for node or automatic if NaN (means value is undefined). Default is NaN
    height: Cell<f32>,
    /// Screen position of the node
    pub(in crate) screen_position: Vec2,
    /// Desired size of the node after Measure pass.
    desired_size: Cell<Vec2>,
    /// Actual node local position after Arrange pass.
    actual_local_position: Cell<Vec2>,
    /// Actual size of the node after Arrange pass.
    actual_size: Cell<Vec2>,
    /// Minimum width and height
    min_size: Vec2,
    /// Maximum width and height
    max_size: Vec2,
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
    pub(in crate) prev_global_visibility: bool,
    children: Vec<Handle<UINode<M, C>>>,
    parent: Handle<UINode<M, C>>,
    /// Indices of commands in command buffer emitted by the node.
    pub(in crate) command_indices: RefCell<Vec<usize>>,
    pub(in crate) is_mouse_directly_over: bool,
    measure_valid: Cell<bool>,
    arrange_valid: Cell<bool>,
    hit_test_visibility: bool,
    pub(in crate) prev_measure: Cell<Vec2>,
    pub(in crate) prev_arrange: Cell<Rect<f32>>,
    z_index: usize,
    pub sender: Sender<UiMessage<M, C>>,
    allow_drag: bool,
    allow_drop: bool,
    pub user_data: Option<Rc<dyn Any>>
}

impl<M, C: 'static + Control<M, C>> Widget<M, C> {
    #[inline]
    pub fn send_message(&self, message: UiMessage<M, C>) {
        self.sender.send(message).unwrap();
    }

    #[inline]
    fn post_property_changed_message(&self, property: WidgetProperty) {
        self.send_message(UiMessage {
            destination: self.handle,
            data: UiMessageData::Widget(WidgetMessage::Property(property)),
            handled: false
        });
    }

    #[inline]
    pub fn set_name<P: AsRef<str>>(&mut self, name: P) -> &mut Self {
        self.name = name.as_ref().to_owned();
        self.post_property_changed_message(WidgetProperty::Name(self.name.clone()));
        self
    }

    #[inline]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    #[inline]
    pub fn actual_size(&self) -> Vec2 {
        self.actual_size.get()
    }

    #[inline]
    pub fn set_min_size(&mut self, value: Vec2) -> &mut Self {
        if self.min_size != value {
            self.min_size = value;
            self.invalidate_layout();
            self.post_property_changed_message(WidgetProperty::MinSize(value));
        }
        self
    }

    #[inline]
    pub fn min_size(&self) -> Vec2 {
        self.min_size
    }

    #[inline]
    pub fn invalidate_layout(&self) {
        self.measure_valid.set(false);
        self.arrange_valid.set(false);
    }

    #[inline]
    pub fn set_max_size(&mut self, value: Vec2) -> &mut Self {
        if self.max_size != value {
            self.max_size = value;
            self.invalidate_layout();
            self.post_property_changed_message(WidgetProperty::MaxSize(value));
        }
        self
    }

    #[inline]
    pub fn max_size(&self) -> Vec2 {
        self.max_size
    }

    #[inline]
    pub fn set_z_index(&mut self, z_index: usize) -> &mut Self {
        if self.z_index != z_index {
            self.z_index = z_index;
            self.invalidate_layout();
            self.post_property_changed_message(WidgetProperty::ZIndex(z_index));
        }
        self
    }

    #[inline]
    pub fn z_index(&self) -> usize {
        self.z_index
    }

    #[inline]
    pub fn set_background(&mut self, brush: Brush) -> &mut Self {
        self.background = brush.clone();
        self.post_property_changed_message(WidgetProperty::Background(brush));
        self
    }

    #[inline]
    pub fn background(&self) -> Brush {
        self.background.clone()
    }

    #[inline]
    pub fn set_foreground(&mut self, brush: Brush) -> &mut Self {
        self.foreground = brush.clone();
        self.post_property_changed_message(WidgetProperty::Foreground(brush));
        self
    }

    #[inline]
    pub fn foreground(&self) -> Brush {
        self.foreground.clone()
    }

    #[inline]
    pub fn set_width_mut(&mut self, width: f32) -> &mut Self {
        if self.width.get() != width {
            self.width.set(width);
            self.invalidate_layout();
            self.post_property_changed_message(WidgetProperty::Width(width));
        }
        self
    }

    #[inline]
    pub fn set_width(&self, width: f32) {
        if self.width.get() != width {
            self.width.set(width);
            self.invalidate_layout();
            self.post_property_changed_message(WidgetProperty::Width(width));
        }
    }

    #[inline]
    pub fn width(&self) -> f32 {
        self.width.get()
    }

    #[inline]
    pub fn set_height_mut(&mut self, height: f32) -> &mut Self {
        if self.height.get() != height {
            self.height.set(height);
            self.invalidate_layout();
            self.post_property_changed_message(WidgetProperty::Height(height));
        }
        self
    }

    #[inline]
    pub fn set_height(&self, height: f32) {
        if self.height.get() != height {
            self.height.set(height);
            self.invalidate_layout();
            self.post_property_changed_message(WidgetProperty::Height(height));
        }
    }

    #[inline]
    pub fn height(&self) -> f32 {
        self.height.get()
    }

    #[inline]
    pub fn set_desired_local_position_mut(&mut self, pos: Vec2) -> &mut Self {
        if self.desired_local_position.get() != pos {
            self.desired_local_position.set(pos);
            self.invalidate_layout();
        }
        self
    }

    #[inline]
    pub fn set_desired_local_position(&self, pos: Vec2) {
        if self.desired_local_position.get() != pos {
            self.desired_local_position.set(pos);
            self.invalidate_layout();
        }
    }

    pub fn raw_copy(&self) -> Self {
        Self {
            handle: Default::default(),
            name: self.name.clone(),
            desired_local_position: self.desired_local_position.clone(),
            width: self.width.clone(),
            height: self.height.clone(),
            screen_position: self.screen_position,
            desired_size: self.desired_size.clone(),
            actual_local_position: self.actual_local_position.clone(),
            actual_size: self.actual_size.clone(),
            min_size: self.min_size,
            max_size: self.max_size,
            background: self.background.clone(),
            foreground: self.foreground.clone(),
            row: self.row,
            column: self.column,
            vertical_alignment: self.vertical_alignment,
            horizontal_alignment: self.horizontal_alignment,
            margin: self.margin,
            visibility: self.visibility,
            global_visibility: self.global_visibility,
            prev_global_visibility: false,
            children: self.children.clone(),
            parent: self.parent,
            command_indices: Default::default(),
            is_mouse_directly_over: self.is_mouse_directly_over,
            measure_valid: Cell::new(false),
            arrange_valid: Cell::new(false),
            hit_test_visibility: self.hit_test_visibility,
            prev_measure: Default::default(),
            prev_arrange: Default::default(),
            z_index: self.z_index,
            sender: self.sender.clone(),
            allow_drop: self.allow_drop,
            allow_drag: self.allow_drag,
            user_data: self.user_data.clone()
        }
    }

    #[inline]
    pub fn desired_local_position(&self) -> Vec2 {
        self.desired_local_position.get()
    }

    #[inline]
    pub fn set_vertical_alignment(&mut self, vertical_alignment: VerticalAlignment) -> &mut Self {
        if self.vertical_alignment != vertical_alignment {
            self.vertical_alignment = vertical_alignment;
            self.invalidate_layout();
            self.post_property_changed_message(WidgetProperty::VerticalAlignment(vertical_alignment));
        }
        self
    }

    #[inline]
    pub fn vertical_alignment(&self) -> VerticalAlignment {
        self.vertical_alignment
    }

    #[inline]
    pub fn set_horizontal_alignment(&mut self, horizontal_alignment: HorizontalAlignment) -> &mut Self {
        self.horizontal_alignment = horizontal_alignment;
        self.invalidate_layout();
        self.post_property_changed_message(WidgetProperty::HorizontalAlignment(horizontal_alignment));
        self
    }

    #[inline]
    pub fn horizontal_alignment(&self) -> HorizontalAlignment {
        self.horizontal_alignment
    }

    #[inline]
    pub fn set_column(&mut self, column: usize) -> &mut Self {
        if self.column != column {
            self.column = column;
            self.invalidate_layout();
            self.post_property_changed_message(WidgetProperty::Column(column));
        }
        self
    }

    #[inline]
    pub fn set_margin(&mut self, margin: Thickness) -> &mut Self {
        if self.margin != margin {
            self.margin = margin;
            self.invalidate_layout();
            self.post_property_changed_message(WidgetProperty::Margin(margin));
        }
        self
    }

    #[inline]
    pub fn margin(&self) -> Thickness {
        self.margin
    }

    #[inline]
    pub(in crate) fn add_child(&mut self, child: Handle<UINode<M, C>>) {
        self.invalidate_layout();
        self.children.push(child)
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
        self.invalidate_layout();
        self.parent = parent;
    }

    #[inline]
    pub fn column(&self) -> usize {
        self.column
    }

    #[inline]
    pub fn set_row(&mut self, row: usize) -> &mut Self {
        if self.row != row {
            self.row = row;
            self.invalidate_layout();
            self.post_property_changed_message(WidgetProperty::Row(row));
        }
        self
    }

    #[inline]
    pub fn row(&self) -> usize {
        self.row
    }

    #[inline]
    pub fn desired_size(&self) -> Vec2 {
        self.desired_size.get()
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

    #[inline]
    pub fn set_visibility(&mut self, visibility: bool) -> &mut Self {
        if self.visibility != visibility {
            self.visibility = visibility;
            self.invalidate_layout();
            self.post_property_changed_message(WidgetProperty::Visibility(visibility));
        }
        self
    }

    #[inline]
    pub fn visibility(&self) -> bool {
        self.visibility
    }

    pub fn has_descendant(&self, node_handle: Handle<UINode<M, C>>, ui: &UserInterface<M, C>) -> bool {
        for child_handle in self.children.iter() {
            if *child_handle == node_handle {
                return true;
            }

            let result = ui.nodes.borrow(*child_handle).has_descendant(node_handle, ui);
            if result {
                return true;
            }
        }
        false
    }

    /// Searches a node up on tree starting from given root that matches a criteria
    /// defined by a given func.
    pub fn find_by_criteria_up<Func: Fn(&UINode<M, C>) -> bool>(&self, ui: &UserInterface<M, C>, func: Func) -> Handle<UINode<M, C>> {
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

    pub fn handle_routed_message(&mut self, _ui: &mut UserInterface<M, C>, msg: &mut UiMessage<M, C>) {
        if msg.destination == self.handle {
            if let UiMessageData::Widget(msg) = &msg.data {
                if let WidgetMessage::Property(property) = msg {
                    match property {
                        WidgetProperty::Background(background) => {
                            self.background = background.clone()
                        }
                        WidgetProperty::Foreground(foreground) => {
                            self.foreground = foreground.clone()
                        }
                        WidgetProperty::Name(name) => {
                            self.name = name.clone()
                        }
                        WidgetProperty::Width(width) => {
                            if self.width.get() != *width {
                                self.width.set(*width);
                                self.invalidate_layout();
                            }
                        }
                        WidgetProperty::Height(height) => {
                            if self.height.get() != *height {
                                self.height.set(*height);
                                self.invalidate_layout();
                            }
                        }
                        WidgetProperty::VerticalAlignment(vertical_alignment) => {
                            if self.vertical_alignment != *vertical_alignment {
                                self.vertical_alignment = *vertical_alignment;
                                self.invalidate_layout();
                            }
                        }
                        WidgetProperty::HorizontalAlignment(horizontal_alignment) => {
                            if self.horizontal_alignment != *horizontal_alignment {
                                self.horizontal_alignment = *horizontal_alignment;
                                self.invalidate_layout();
                            }
                        }
                        WidgetProperty::MaxSize(max_size) => {
                            if self.max_size != *max_size {
                                self.max_size = *max_size;
                                self.invalidate_layout();
                            }
                        }
                        WidgetProperty::MinSize(min_size) => {
                            if self.min_size != *min_size {
                                self.min_size = *min_size;
                                self.invalidate_layout();
                            }
                        }
                        WidgetProperty::Row(row) => {
                            if self.row != *row {
                                self.row = *row;
                                self.invalidate_layout();
                            }
                        }
                        WidgetProperty::Column(column) => {
                            if self.column != *column {
                                self.column = *column;
                                self.invalidate_layout();
                            }
                        }
                        WidgetProperty::Margin(margin) => {
                            if self.margin != *margin {
                                self.margin = *margin;
                                self.invalidate_layout();
                            }
                        }
                        WidgetProperty::HitTestVisibility(hit_test_visibility) => {
                            self.hit_test_visibility = *hit_test_visibility
                        }
                        WidgetProperty::Visibility(visibility) => {
                            if self.visibility != *visibility {
                                self.visibility = *visibility;
                                self.invalidate_layout();
                            }
                        }
                        WidgetProperty::DesiredPosition(pos) => {
                            if self.desired_local_position.get() != *pos {
                                self.desired_local_position.set(*pos);
                                self.invalidate_layout();
                            }
                        }
                        _ => ()
                    }
                }
            }
        }
    }

    #[inline]
    pub fn measure_override(&self, ui: &UserInterface<M, C>, available_size: Vec2) -> Vec2 {
        let mut size = Vec2::ZERO;

        for child_handle in self.children.iter() {
            ui.node(*child_handle).measure(ui, available_size);

            let child = ui.node(*child_handle);
            let child_desired_size = child.desired_size.get();
            if child_desired_size.x > size.x {
                size.x = child_desired_size.x;
            }
            if child_desired_size.y > size.y {
                size.y = child_desired_size.y;
            }
        }

        size
    }

    #[inline]
    pub fn arrange_override(&self, ui: &UserInterface<M, C>, final_size: Vec2) -> Vec2 {
        let final_rect = Rect::new(0.0, 0.0, final_size.x, final_size.y);

        for child_handle in self.children.iter() {
            ui.node(*child_handle).arrange(ui, &final_rect);
        }

        final_size
    }

    #[inline]
    pub(in crate) fn commit_arrange(&self, position: Vec2, size: Vec2) {
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
    pub(in crate) fn commit_measure(&self, desired_size: Vec2) {
        self.desired_size.set(desired_size);
        self.measure_valid.set(true);
    }

    #[inline]
    pub fn is_measure_valid(&self) -> bool {
        self.measure_valid.get()
    }

    #[inline]
    pub fn actual_local_position(&self) -> Vec2 {
        self.actual_local_position.get()
    }

    #[inline]
    pub fn is_hit_test_visible(&self) -> bool {
        self.hit_test_visibility
    }

    #[inline]
    pub fn is_globally_visible(&self) -> bool {
        self.global_visibility
    }

    #[inline]
    pub(in crate) fn set_global_visibility(&mut self, value: bool) {
        self.prev_global_visibility = self.global_visibility;
        self.global_visibility = value;
    }

    #[inline]
    pub fn handle(&self) -> Handle<UINode<M, C>> {
        self.handle
    }

    #[inline]
    pub fn sender(&self) -> Sender<UiMessage<M, C>> {
        self.sender.clone()
    }

    #[inline]
    pub fn is_drag_allowed(&self) -> bool {
        self.allow_drag
    }

    #[inline]
    pub fn is_drop_allowed(&self) -> bool {
        self.allow_drop
    }
}

pub struct WidgetBuilder<M: 'static, C: 'static + Control<M, C>> {
    pub name: String,
    pub width: f32,
    pub height: f32,
    pub desired_position: Vec2,
    pub vertical_alignment: VerticalAlignment,
    pub horizontal_alignment: HorizontalAlignment,
    pub max_size: Option<Vec2>,
    pub min_size: Option<Vec2>,
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
    pub user_data: Option<Rc<dyn Any>>
}

impl<M, C: 'static + Control<M, C>> Default for WidgetBuilder<M, C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M, C: 'static + Control<M, C>> WidgetBuilder<M, C> {
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
            desired_position: Vec2::ZERO,
            children: Vec::new(),
            is_hit_test_visible: true,
            visibility: true,
            z_index: 0,
            allow_drag: false,
            allow_drop: false,
            user_data: None
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

    pub fn with_vertical_alignment(mut self, valign: VerticalAlignment) -> Self {
        self.vertical_alignment = valign;
        self
    }

    pub fn with_horizontal_alignment(mut self, halign: HorizontalAlignment) -> Self {
        self.horizontal_alignment = halign;
        self
    }

    pub fn with_max_size(mut self, max_size: Vec2) -> Self {
        self.max_size = Some(max_size);
        self
    }

    pub fn with_min_size(mut self, min_size: Vec2) -> Self {
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

    pub fn with_desired_position(mut self, desired_position: Vec2) -> Self {
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

    pub fn with_children(mut self, children: &[Handle<UINode<M, C>>]) -> Self {
        for child in children {
            self.children.push(*child)
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

    pub fn build(self, sender: Sender<UiMessage<M, C>>) -> Widget<M, C> {
        Widget {
            handle: Default::default(),
            name: self.name,
            desired_local_position: Cell::new(self.desired_position),
            width: Cell::new(self.width),
            height: Cell::new(self.height),
            screen_position: Vec2::ZERO,
            desired_size: Cell::new(Vec2::ZERO),
            actual_local_position: Cell::new(Vec2::ZERO),
            actual_size: Cell::new(Vec2::ZERO),
            min_size: self.min_size.unwrap_or(Vec2::ZERO),
            max_size: self.max_size.unwrap_or_else(|| Vec2::new(std::f32::INFINITY, std::f32::INFINITY)),
            background: self.background.unwrap_or_else(|| Brush::Solid(Color::opaque(50, 50, 50))),
            foreground: self.foreground.unwrap_or_else(|| Brush::Solid(Color::WHITE)),
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
            sender,
            allow_drag: self.allow_drag,
            allow_drop: self.allow_drop,
            user_data: self.user_data
        }
    }
}