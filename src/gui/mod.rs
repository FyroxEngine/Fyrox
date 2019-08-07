pub mod draw;

use crate::{
    utils::{
        pool::{Pool, Handle},
        rcpool::RcHandle,
        UnsafeCollectionView,
    },
    math::{
        vec2::Vec2,
        Rect,
    },
    gui::draw::{Color, DrawingContext, FormattedText, CommandKind, FormattedTextBuilder},
    resource::{
        Resource,
        ttf::Font,
    },
};
use glutin::{VirtualKeyCode, MouseButton, WindowEvent, ElementState};

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum HorizontalAlignment {
    Stretch,
    Left,
    Center,
    Right,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum VerticalAlignment {
    Stretch,
    Top,
    Center,
    Bottom,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Thickness {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}

impl Thickness {
    pub fn zero() -> Thickness {
        Thickness {
            left: 0.0,
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
        }
    }

    pub fn uniform(v: f32) -> Thickness {
        Thickness {
            left: v,
            top: v,
            right: v,
            bottom: v,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Visibility {
    Visible,
    Collapsed,
    Hidden,
}

#[derive(Debug)]
pub struct Text {
    need_update: bool,
    text: String,
    font: Handle<Font>,
    vertical_alignment: VerticalAlignment,
    horizontal_alignment: HorizontalAlignment,
    formatted_text: Option<FormattedText>,
}

impl Text {
    pub fn new(text: &str) -> Text {
        Text {
            text: String::from(text),
            need_update: true,
            vertical_alignment: VerticalAlignment::Top,
            horizontal_alignment: HorizontalAlignment::Left,
            formatted_text: Some(FormattedTextBuilder::new().build()),
            font: Handle::none(),
        }
    }

    pub fn set_text(&mut self, text: &str) -> &mut Self {
        self.text.clear();
        self.text += text;
        self.need_update = true;
        self
    }

    pub fn get_text(&self) -> &str {
        self.text.as_str()
    }

    pub fn set_font(&mut self, font: Handle<Font>) -> &mut Self {
        self.font = font;
        self.need_update = true;
        self
    }

    pub fn set_vertical_alignment(&mut self, valign: VerticalAlignment) -> &mut Self {
        self.vertical_alignment = valign;
        self
    }

    pub fn set_horizontal_alignment(&mut self, halign: HorizontalAlignment) -> &mut Self {
        self.horizontal_alignment = halign;
        self
    }
}

#[derive(Debug)]
pub struct Border {
    stroke_thickness: Thickness,
    stroke_color: Color,
}

impl Border {
    pub fn new() -> Border {
        Border {
            stroke_thickness: Thickness {
                left: 1.0,
                right: 1.0,
                top: 1.0,
                bottom: 1.0,
            },
            stroke_color: Color::white(),
        }
    }

    pub fn set_stroke_thickness(&mut self, thickness: Thickness) -> &mut Self {
        self.stroke_thickness = thickness;
        self
    }

    pub fn set_stroke_color(&mut self, color: Color) -> &mut Self {
        self.stroke_color = color;
        self
    }
}

pub struct Image {
    texture: RcHandle<Resource>
}

pub type ButtonClickEventHandler = dyn FnMut(&mut UserInterface, Handle<UINode>);

pub struct Button {
    click: Option<Box<ButtonClickEventHandler>>,
    was_pressed: bool,
}

impl Button {
    pub fn new() -> Button {
        Button {
            click: None,
            was_pressed: false,
        }
    }

    pub fn set_on_click(&mut self, handler: Box<ButtonClickEventHandler>) {
        self.click = Some(handler);
    }
}

pub enum ButtonContent {
    Text(String),
    Node(Handle<UINode>),
}

pub struct ButtonBuilder {
    content: Option<ButtonContent>,
    click: Option<Box<ButtonClickEventHandler>>,
    common: CommonBuilderFields
}

struct CommonBuilderFields {
    width: Option<f32>,
    height: Option<f32>,
    vertical_alignment: Option<VerticalAlignment>,
    horizontal_alignment: Option<HorizontalAlignment>,
    max_size: Option<Vec2>,
    min_size: Option<Vec2>,
    color: Option<Color>,
    row: Option<usize>,
    column: Option<usize>,
    margin: Option<Thickness>,
    parent: Option<Handle<UINode>>
}

impl CommonBuilderFields {
    pub fn new() -> Self {
        Self {
            width: None,
            height: None,
            vertical_alignment: None,
            horizontal_alignment: None,
            max_size: None,
            min_size: None,
            color: None,
            row: None,
            column: None,
            margin: None,
            parent: None,
        }
    }

    pub fn apply(&self, ui: &mut UserInterface, node_handle: &Handle<UINode>) {
        if let Some(node) = ui.nodes.borrow_mut(node_handle) {
            if let Some(width) = self.width {
                node.width = width;
            }
            if let Some(height) = self.height {
                node.height = height;
            }
            if let Some(valign) = self.vertical_alignment {
                node.vertical_alignment = valign;
            }
            if let Some(halign) = self.horizontal_alignment {
                node.horizontal_alignment = halign;
            }
            if let Some(max_size) = self.max_size {
                node.max_size = max_size;
            }
            if let Some(min_size) = self.min_size {
                node.min_size = min_size;
            }
            if let Some(color) = self.color {
                node.color = color;
            }
            if let Some(row) = self.row {
                node.row = row;
            }
            if let Some(column) = self.column {
                node.column = column;
            }
            if let Some(margin) = self.margin {
                node.margin = margin;
            }
        }
        if let Some(ref parent) = self.parent {
            ui.link_nodes(node_handle, &parent);
        }
    }
}

macro_rules! impl_default_builder_methods {
    () => (
        pub fn with_width(mut self, width: f32) -> Self {
            self.common.width = Some(width);
            self
        }

        pub fn with_height(mut self, height: f32) -> Self {
            self.common.height = Some(height);
            self
        }

        pub fn with_vertical_alignment(mut self, valign: VerticalAlignment) -> Self {
            self.common.vertical_alignment = Some(valign);
            self
        }

        pub fn with_horizontal_alignment(mut self, halign: HorizontalAlignment) -> Self {
            self.common.horizontal_alignment = Some(halign);
            self
        }

        pub fn with_max_size(mut self, max_size: Vec2) -> Self {
            self.common.max_size = Some(max_size);
            self
        }

        pub fn with_min_size(mut self, min_size: Vec2) -> Self {
            self.common.min_size = Some(min_size);
            self
        }

        pub fn with_color(mut self, color: Color) -> Self {
            self.common.color = Some(color);
            self
        }

        pub fn on_row(mut self, row: usize) -> Self {
            self.common.row = Some(row);
            self
        }

        pub fn on_column(mut self, column: usize) -> Self {
            self.common.column = Some(column);
            self
        }

        pub fn with_margin(mut self, margin: Thickness) -> Self {
            self.common.margin = Some(margin);
            self
        }

        pub fn with_parent(mut self, parent: &Handle<UINode>) -> Self {
            self.common.parent = Some(parent.clone());
            self
        }
    )
}

impl ButtonBuilder {
    pub fn new() -> Self {
        Self {
            content: None,
            click: None,
            common: CommonBuilderFields::new(),
        }
    }

    impl_default_builder_methods!();

    pub fn with_text(mut self, text: &str) -> Self {
        self.content = Some(ButtonContent::Text(text.to_owned()));
        self
    }

    pub fn with_node(mut self, node: Handle<UINode>) -> Self {
        self.content = Some(ButtonContent::Node(node));
        self
    }

    pub fn with_click(mut self, handler: Box<ButtonClickEventHandler>) -> Self {
        self.click = Some(handler);
        self
    }

    pub fn build(self, ui: &mut UserInterface) -> Handle<UINode> {
        let normal_color = Color::opaque(120, 120, 120);
        let pressed_color = Color::opaque(100, 100, 100);
        let hover_color = Color::opaque(160, 160, 160);

        let mut button = Button::new();
        button.click = self.click;

        let mut button_node = UINode::new(UINodeKind::Button(button));
        button_node
            .set_handler(RoutedEventHandlerType::MouseDown, Box::new(move |ui, handle, _evt| {
                ui.capture_mouse(&handle);
                if let Some(button_node) = ui.nodes.borrow_mut(&handle) {
                    if let UINodeKind::Button(button) = button_node.get_kind_mut() {
                        button.was_pressed = true;
                    }
                }
            }))
            .set_handler(RoutedEventHandlerType::MouseUp, Box::new(move |ui, handle, evt| {
                // Take-Call-PutBack trick to bypass borrow checker
                let mut click_handler = None;

                if let Some(button_node) = ui.nodes.borrow_mut(&handle) {
                    if let UINodeKind::Button(button) = button_node.get_kind_mut() {
                        click_handler = button.click.take();
                        button.was_pressed = false;
                    }
                }

                if let Some(ref mut handler) = click_handler {
                    handler(ui, handle.clone());
                    evt.handled = true;
                }

                // Second check required because event handler can remove node.
                if let Some(button_node) = ui.nodes.borrow_mut(&handle) {
                    if let UINodeKind::Button(button) = button_node.get_kind_mut() {
                        button.click = click_handler;
                    }
                }

                ui.release_mouse_capture();
            }));



        let mut border = Border::new();
        border.set_stroke_color(Color::opaque(200, 200, 200))
            .set_stroke_thickness(Thickness { left: 2.0, right: 2.0, top: 2.0, bottom: 2.0 });

        let mut back = UINode::new(UINodeKind::Border(border));
        back.set_color(normal_color)
            .set_handler(RoutedEventHandlerType::MouseEnter, Box::new(move |ui, handle, _evt| {
                if let Some(back) = ui.nodes.borrow_mut(&handle) {
                    back.color = hover_color;
                }
            }))
            .set_handler(RoutedEventHandlerType::MouseLeave, Box::new(move |ui, handle, _evt| {
                if let Some(back) = ui.nodes.borrow_mut(&handle) {
                    back.color = normal_color;
                }
            }))
            .set_handler(RoutedEventHandlerType::MouseDown, Box::new(move |ui, handle, _evt| {
                if let Some(back) = ui.nodes.borrow_mut(&handle) {
                    back.color = pressed_color;
                }
            }))
            .set_handler(RoutedEventHandlerType::MouseUp, Box::new(move |ui, handle, _evt| {
                if let Some(back) = ui.nodes.borrow_mut(&handle) {
                    if back.is_mouse_over {
                        back.color = hover_color;
                    } else {
                        back.color = normal_color;
                    }
                }
            }));

        let back_handle = ui.add_node(back);
        let button_handle = ui.add_node(button_node);
        self.common.apply(ui, &button_handle);
        if let Some(content) = self.content {
            let content_handle = match content {
                ButtonContent::Text(txt) => {
                    let mut text = Text::new(txt.as_str());
                    text.set_font(ui.default_font.clone())
                        .set_horizontal_alignment(HorizontalAlignment::Center)
                        .set_vertical_alignment(VerticalAlignment::Center);
                    ui.add_node(UINode::new(UINodeKind::Text(text)))
                }
                ButtonContent::Node(node) => {
                    node
                }
            };
            ui.link_nodes(&content_handle, &back_handle);
        }
        ui.link_nodes(&back_handle, &button_handle);
        button_handle
    }
}

#[derive(PartialEq)]
pub enum SizeMode {
    Strict,
    Auto,
    Stretch,
}

pub struct Column {
    size_mode: SizeMode,
    desired_width: f32,
    actual_width: f32,
    x: f32,
}

impl Column {
    pub fn new(size_mode: SizeMode, desired_width: f32) -> Column {
        Column {
            size_mode,
            desired_width,
            actual_width: 0.0,
            x: 0.0,
        }
    }
}

pub struct Row {
    size_mode: SizeMode,
    desired_height: f32,
    actual_height: f32,
    y: f32,
}

impl Row {
    pub fn new(size_mode: SizeMode, desired_height: f32) -> Row {
        Row {
            size_mode,
            desired_height,
            actual_height: 0.0,
            y: 0.0,
        }
    }
}

pub struct Grid {
    rows: Vec<Row>,
    columns: Vec<Column>,
    draw_borders: bool,
}

pub struct GridBuilder {
    rows: Vec<Row>,
    columns: Vec<Column>,
    common: CommonBuilderFields
}

impl GridBuilder {
    pub fn new() -> Self {
        GridBuilder {
            rows: Vec::new(),
            columns: Vec::new(),
            common: CommonBuilderFields::new(),
        }
    }

    impl_default_builder_methods!();

    pub fn add_row(mut self, row: Row) -> Self {
        self.rows.push(row);
        self
    }

    pub fn add_column(mut self, column: Column) -> Self {
        self.columns.push(column);
        self
    }

    pub fn build(self, ui: &mut UserInterface) -> Handle<UINode> {
        let node = UINode::new(UINodeKind::Grid(Grid {
            columns: self.columns,
            rows: self.rows,
            draw_borders: false,
        }));

        let handle = ui.add_node(node);
        self.common.apply(ui, &handle);
        handle
    }
}

impl Grid {
    pub fn add_row(&mut self, row: Row) -> &mut Self {
        self.rows.push(row);
        self
    }

    pub fn add_column(&mut self, column: Column) -> &mut Self {
        self.columns.push(column);
        self
    }
}

pub enum UINodeKind {
    Base,
    Text(Text),
    Border(Border),
    /// TODO
    Window,
    Button(Button),
    /// TODO
    ScrollBar,
    /// TODO
    ScrollViewer,
    /// TODO
    TextBox,
    /// TODO
    Image,
    /// Automatically arranges children by rows and columns
    Grid(Grid),
    /// TODO Allows user to directly set position and size of a node
    Canvas,
    /// TODO Allows user to scroll content
    ScrollContentPresenter,
    /// TODO
    SlideSelector,
    /// TODO
    CheckBox,
}

#[derive(Copy, Clone, PartialEq)]
pub enum RoutedEventHandlerType {
    MouseMove,
    MouseEnter,
    MouseLeave,
    MouseDown,
    MouseUp,
    Count,
}

pub type EventHandler = dyn FnMut(&mut UserInterface, Handle<UINode>, &mut RoutedEvent);

pub struct UINode {
    kind: UINodeKind,
    /// Desired position relative to parent node
    desired_local_position: Vec2,
    /// Explicit width for node or automatic if NaN (means value is undefined). Default is NaN
    width: f32,
    /// Explicit height for node or automatic if NaN (means value is undefined). Default is NaN
    height: f32,
    /// Screen position of the node
    screen_position: Vec2,
    /// Desired size of the node after Measure pass.
    desired_size: Vec2,
    /// Actual node local position after Arrange pass.
    actual_local_position: Vec2,
    /// Actual size of the node after Arrange pass.
    actual_size: Vec2,
    /// Minimum width and height
    min_size: Vec2,
    /// Maximum width and height
    max_size: Vec2,
    /// Overlay color of the node
    color: Color,
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
    visibility: Visibility,
    children: Vec<Handle<UINode>>,
    parent: Handle<UINode>,
    /// Indices of commands in command buffer emitted by the node.
    command_indices: Vec<usize>,
    is_mouse_over: bool,
    event_handlers: [Option<Box<EventHandler>>; RoutedEventHandlerType::Count as usize],
}

pub enum RoutedEventKind {
    MouseDown {
        pos: Vec2,
        button: MouseButton,
    },
    MouseMove {
        pos: Vec2
    },
    MouseUp {
        pos: Vec2,
        button: MouseButton,
    },
    Text {
        symbol: char
    },
    KeyDown {
        code: VirtualKeyCode
    },
    KeyUp {
        code: VirtualKeyCode
    },
    MouseWheel {
        pos: Vec2,
        amount: u32,
    },
    MouseLeave,
    MouseEnter,
}

pub struct RoutedEvent {
    kind: RoutedEventKind,
    handled: bool,
}

impl RoutedEvent {
    pub fn new(kind: RoutedEventKind) -> RoutedEvent {
        RoutedEvent {
            kind,
            handled: false,
        }
    }
}

pub struct UserInterface {
    nodes: Pool<UINode>,
    drawing_context: DrawingContext,
    default_font: Handle<Font>,
    visual_debug: bool,
    /// Every UI node will live on the window-sized canvas.
    root_canvas: Handle<UINode>,
    picked_node: Handle<UINode>,
    prev_picked_node: Handle<UINode>,
    captured_node: Handle<UINode>,
}

#[inline]
fn maxf(a: f32, b: f32) -> f32 {
    if a > b {
        a
    } else {
        b
    }
}

#[inline]
fn minf(a: f32, b: f32) -> f32 {
    if a < b {
        a
    } else {
        b
    }
}

impl UserInterface {
    pub fn new(default_font: Handle<Font>) -> UserInterface {
        let mut nodes = Pool::new();
        UserInterface {
            visual_debug: false,
            default_font,
            captured_node: Handle::none(),
            root_canvas: nodes.spawn(UINode::new(UINodeKind::Canvas)),
            nodes,
            drawing_context: DrawingContext::new(),
            picked_node: Handle::none(),
            prev_picked_node: Handle::none(),
        }
    }

    pub fn add_node(&mut self, node: UINode) -> Handle<UINode> {
        let node_handle = self.nodes.spawn(node);
        self.link_nodes(&node_handle, &self.root_canvas.clone());
        node_handle
    }

    pub fn capture_mouse(&mut self, node: &Handle<UINode>) -> bool {
        if self.captured_node.is_none() {
            if self.nodes.is_valid_handle(node) {
                self.captured_node = node.clone();
                return true;
            }
        }

        false
    }

    pub fn release_mouse_capture(&mut self) {
        self.captured_node = Handle::none();
    }

    /// Links specified child with specified parent.
    #[inline]
    pub fn link_nodes(&mut self, child_handle: &Handle<UINode>, parent_handle: &Handle<UINode>) {
        self.unlink_node(child_handle);
        if let Some(child) = self.nodes.borrow_mut(child_handle) {
            child.parent = parent_handle.clone();
            if let Some(parent) = self.nodes.borrow_mut(parent_handle) {
                parent.children.push(child_handle.clone());
            }
        }
    }

    /// Unlinks specified node from its parent, so node will become root.
    #[inline]
    pub fn unlink_node(&mut self, node_handle: &Handle<UINode>) {
        let mut parent_handle: Handle<UINode> = Handle::none();
        // Replace parent handle of child
        if let Some(node) = self.nodes.borrow_mut(node_handle) {
            parent_handle = node.parent.clone();
            node.parent = Handle::none();
        }
        // Remove child from parent's children list
        if let Some(parent) = self.nodes.borrow_mut(&parent_handle) {
            if let Some(i) = parent.children.iter().position(|h| h == node_handle) {
                parent.children.remove(i);
            }
        }
    }

    #[inline]
    pub fn get_node(&self, node_handle: &Handle<UINode>) -> Option<&UINode> {
        self.nodes.borrow(node_handle)
    }

    #[inline]
    pub fn get_node_mut(&mut self, node_handle: &Handle<UINode>) -> Option<&mut UINode> {
        self.nodes.borrow_mut(node_handle)
    }

    #[inline]
    pub fn get_drawing_context(&self) -> &DrawingContext {
        &self.drawing_context
    }

    #[inline]
    pub fn get_drawing_context_mut(&mut self) -> &mut DrawingContext {
        &mut self.drawing_context
    }

    fn default_measure_override(&mut self, children: &UnsafeCollectionView<Handle<UINode>>, available_size: &Vec2) -> Vec2 {
        let mut size = Vec2::new();

        for child_handle in children.iter() {
            self.measure(child_handle, &available_size);

            if let Some(child) = self.nodes.borrow(child_handle) {
                if child.desired_size.x > size.x {
                    size.x = child.desired_size.x;
                }
                if child.desired_size.y > size.y {
                    size.y = child.desired_size.y;
                }
            }
        }

        size
    }

    /// Performs recursive kind-specific measurement of children nodes
    ///
    /// Returns desired size.
    fn measure_override(&mut self, node_kind: &mut UINodeKind, children: &UnsafeCollectionView<Handle<UINode>>, available_size: &Vec2) -> Vec2 {
        match node_kind {
            // TODO: Type-specific measure
            UINodeKind::Border(border) => {
                let margin_x = border.stroke_thickness.left + border.stroke_thickness.right;
                let margin_y = border.stroke_thickness.top + border.stroke_thickness.bottom;

                let size_for_child = Vec2::make(
                    available_size.x - margin_x,
                    available_size.y - margin_y,
                );
                let mut desired_size = Vec2::new();
                for child_handle in children.iter() {
                    self.measure(child_handle, &size_for_child);

                    if let Some(child) = self.nodes.borrow(child_handle) {
                        if child.desired_size.x > desired_size.x {
                            desired_size.x = child.desired_size.x;
                        }
                        if child.desired_size.y > desired_size.y {
                            desired_size.y = child.desired_size.y;
                        }
                    }
                }
                desired_size.x += margin_x;
                desired_size.y += margin_y;

                desired_size
            }
            UINodeKind::Canvas => {
                let size_for_child = Vec2::make(
                    std::f32::INFINITY,
                    std::f32::INFINITY,
                );

                for child_handle in children.iter() {
                    self.measure(child_handle, &size_for_child);
                }

                Vec2::new()
            }
            UINodeKind::Grid(grid) => {
                // In case of no rows or columns, grid acts like default panel.
                if grid.columns.is_empty() || grid.rows.is_empty() {
                    return self.default_measure_override(children, available_size);
                }

                // Step 1. Measure every children with relaxed constraints (size of grid).
                for child_handle in children.iter() {
                    self.measure(child_handle, available_size);
                }

                // Step 2. Calculate width of columns and heights of rows.
                let mut preset_width = 0.0;
                let mut preset_height = 0.0;

                // Step 2.1. Calculate size of strict-sized and auto-sized columns.
                for (i, col) in grid.columns.iter_mut().enumerate() {
                    if col.size_mode == SizeMode::Strict {
                        col.actual_width = col.desired_width;
                        preset_width += col.actual_width;
                    } else if col.size_mode == SizeMode::Auto {
                        col.actual_width = col.desired_width;
                        for child_handle in children.iter() {
                            if let Some(child) = self.nodes.borrow(child_handle) {
                                if child.column == i && child.visibility == Visibility::Visible {
                                    if child.desired_size.x > col.actual_width {
                                        col.actual_width = child.desired_size.x;
                                    }
                                }
                            }
                        }
                        preset_width += col.actual_width;
                    }
                }

                // Step 2.2. Calculate size of strict-sized and auto-sized rows.
                for (i, row) in grid.rows.iter_mut().enumerate() {
                    if row.size_mode == SizeMode::Strict {
                        row.actual_height = row.desired_height;
                        preset_height += row.actual_height;
                    } else if row.size_mode == SizeMode::Auto {
                        row.actual_height = row.desired_height;
                        for child_handle in children.iter() {
                            if let Some(child) = self.nodes.borrow(child_handle) {
                                if child.row == i && child.visibility == Visibility::Visible {
                                    if child.desired_size.y > row.actual_height {
                                        row.actual_height = child.desired_size.y;
                                    }
                                }
                            }
                        }
                        preset_height += row.actual_height;
                    }
                }

                // Step 2.3. Fit stretch-sized columns

                let mut rest_width = 0.0;
                if available_size.x.is_infinite() {
                    for child_handle in children.iter() {
                        if let Some(child) = self.nodes.borrow(child_handle) {
                            if let Some(column) = grid.columns.get(child.column) {
                                if column.size_mode == SizeMode::Stretch {
                                    rest_width += child.desired_size.x;
                                }
                            }
                        }
                    }
                } else {
                    rest_width = available_size.x - preset_width;
                }

                // count columns first
                let mut stretch_sized_columns = 0;
                for column in grid.columns.iter() {
                    if column.size_mode == SizeMode::Stretch {
                        stretch_sized_columns += 1;
                    }
                }
                if stretch_sized_columns > 0 {
                    let width_per_col = rest_width / stretch_sized_columns as f32;
                    for column in grid.columns.iter_mut() {
                        if column.size_mode == SizeMode::Stretch {
                            column.actual_width = width_per_col;
                        }
                    }
                }

                // Step 2.4. Fit stretch-sized rows.
                let mut stretch_sized_rows = 0;
                let mut rest_height = 0.0;
                if available_size.y.is_infinite() {
                    for child_handle in children.iter() {
                        if let Some(child) = self.nodes.borrow(child_handle) {
                            if let Some(row) = grid.rows.get(child.row) {
                                if row.size_mode == SizeMode::Stretch {
                                    rest_height += child.desired_size.y;
                                }
                            }
                        }
                    }
                } else {
                    rest_height = available_size.y - preset_height;
                }
                // count rows first
                for row in grid.rows.iter() {
                    if row.size_mode == SizeMode::Stretch {
                        stretch_sized_rows += 1;
                    }
                }
                if stretch_sized_rows > 0 {
                    let height_per_row = rest_height / stretch_sized_rows as f32;
                    for row in grid.rows.iter_mut() {
                        if row.size_mode == SizeMode::Stretch {
                            row.actual_height = height_per_row;
                        }
                    }
                }

                // Step 2.5. Calculate positions of each column.
                let mut y = 0.0;
                for row in grid.rows.iter_mut() {
                    row.y = y;
                    y += row.actual_height;
                }

                // Step 2.6. Calculate positions of each row.
                let mut x = 0.0;
                for column in grid.columns.iter_mut() {
                    column.x = x;
                    x += column.actual_width;
                }

                // Step 3. Re-measure children with new constraints.
                for child_handle in children.iter() {
                    let size_for_child = {
                        if let Some(child) = self.nodes.borrow(child_handle) {
                            Vec2 {
                                x: grid.columns[child.column].actual_width,
                                y: grid.rows[child.row].actual_height,
                            }
                        } else {
                            Vec2 {
                                x: match grid.columns.first() {
                                    Some(column) => column.actual_width,
                                    None => 0.0
                                },
                                y: match grid.rows.first() {
                                    Some(row) => row.actual_height,
                                    None => 0.0
                                },
                            }
                        }
                    };
                    self.measure(child_handle, &size_for_child);
                }

                // Step 4. Calculate desired size of grid.
                let mut desired_size = Vec2::new();
                for column in grid.columns.iter() {
                    desired_size.x += column.actual_width;
                }
                for row in grid.rows.iter() {
                    desired_size.y += row.actual_height;
                }

                desired_size
            }
            // Default measure
            _ => {
                self.default_measure_override(children, available_size)
            }
        }
    }

    fn measure(&mut self, node_handle: &Handle<UINode>, available_size: &Vec2) {
        let mut children: UnsafeCollectionView<Handle<UINode>> = UnsafeCollectionView::empty();
        let mut node_kind: *mut UINodeKind = std::ptr::null_mut();
        let size_for_child;
        let margin;

        match self.nodes.borrow_mut(&node_handle) {
            None => return,
            Some(node) => {
                margin = Vec2 {
                    x: node.margin.left + node.margin.right,
                    y: node.margin.top + node.margin.bottom,
                };

                size_for_child = Vec2 {
                    x: {
                        let w = if node.width > 0.0 {
                            node.width
                        } else {
                            maxf(0.0, available_size.x - margin.x)
                        };

                        if w > node.max_size.x {
                            node.max_size.x
                        } else if w < node.min_size.x {
                            node.min_size.x
                        } else {
                            w
                        }
                    },
                    y: {
                        let h = if node.height > 0.0 {
                            node.height
                        } else {
                            maxf(0.0, available_size.y - margin.y)
                        };

                        if h > node.max_size.y {
                            node.max_size.y
                        } else if h < node.min_size.y {
                            node.min_size.y
                        } else {
                            h
                        }
                    },
                };

                if node.visibility == Visibility::Visible {
                    // Remember immutable pointer to collection of children nodes on which we'll continue
                    // measure. It is one hundred percent safe to have immutable pointer to list of
                    // children handles, because we guarantee that children collection won't be modified
                    // during measure pass. Also this step *cannot* be performed in parallel so we don't
                    // have to bother about thread-safety here.
                    children = UnsafeCollectionView::from_vec(&node.children);
                    node_kind = &mut node.kind as *mut UINodeKind;
                } else {
                    // We do not have any children so node want to collapse into point.
                    node.desired_size = Vec2::make(0.0, 0.0);
                }
            }
        }

        let desired_size = self.measure_override(unsafe { &mut *node_kind }, &children, &size_for_child);

        if let Some(node) = self.nodes.borrow_mut(&node_handle) {
            node.desired_size = desired_size;

            if !node.width.is_nan() {
                node.desired_size.x = node.width;
            }

            if node.desired_size.x > node.max_size.x {
                node.desired_size.x = node.max_size.x;
            } else if node.desired_size.x < node.min_size.x {
                node.desired_size.x = node.min_size.x;
            }

            if node.desired_size.y > node.max_size.y {
                node.desired_size.y = node.max_size.y;
            } else if node.desired_size.y < node.min_size.y {
                node.desired_size.y = node.min_size.y;
            }

            if !node.height.is_nan() {
                node.desired_size.y = node.height;
            }

            node.desired_size += margin;

            // Make sure that node won't go outside of available bounds.
            if node.desired_size.x > available_size.x {
                node.desired_size.x = available_size.x;
            }
            if node.desired_size.y > available_size.y {
                node.desired_size.y = available_size.y;
            }
        }
    }

    /// Performs recursive kind-specific arrangement of children nodes
    ///
    /// Returns actual size.
    fn arrange_override(&mut self, node_kind: &UINodeKind, children: &UnsafeCollectionView<Handle<UINode>>, final_size: &Vec2) -> Vec2 {
        match node_kind {
            // TODO: Kind-specific arrangement
            UINodeKind::Border(border) => {
                let rect_for_child = Rect::new(
                    border.stroke_thickness.left, border.stroke_thickness.top,
                    final_size.x - (border.stroke_thickness.right + border.stroke_thickness.left),
                    final_size.y - (border.stroke_thickness.bottom + border.stroke_thickness.top),
                );

                for child_handle in children.iter() {
                    self.arrange(child_handle, &rect_for_child);
                }

                *final_size
            }
            UINodeKind::Canvas => {
                for child_handle in children.iter() {
                    let mut final_rect = None;

                    if let Some(child) = self.nodes.borrow(&child_handle) {
                        final_rect = Some(Rect::new(
                            child.desired_local_position.x,
                            child.desired_local_position.y,
                            child.desired_size.x,
                            child.desired_size.y));
                    }

                    if let Some(rect) = final_rect {
                        self.arrange(child_handle, &rect);
                    }
                }

                *final_size
            }
            UINodeKind::Grid(grid) => {
                if grid.columns.is_empty() || grid.rows.is_empty() {
                    let rect = Rect::new(0.0, 0.0, final_size.x, final_size.y);
                    for child_handle in children.iter() {
                        self.arrange(child_handle, &rect);
                    }
                    return *final_size;
                }

                for child_handle in children.iter() {
                    let mut final_rect = None;

                    if let Some(child) = self.nodes.borrow(&child_handle) {
                        if let Some(column) = grid.columns.get(child.column) {
                            if let Some(row) = grid.rows.get(child.row) {
                                final_rect = Some(Rect::new(
                                    column.x,
                                    row.y,
                                    column.actual_width,
                                    row.actual_height,
                                ));
                            }
                        }
                    }

                    if let Some(rect) = final_rect {
                        self.arrange(child_handle, &rect);
                    }
                }

                *final_size
            }
            // Default arrangement
            _ => {
                let final_rect = Rect::new(0.0, 0.0, final_size.x, final_size.y);

                for child_handle in children.iter() {
                    self.arrange(child_handle, &final_rect);
                }

                *final_size
            }
        }
    }

    fn arrange(&mut self, node_handle: &Handle<UINode>, final_rect: &Rect<f32>) {
        let children: UnsafeCollectionView<Handle<UINode>>;

        let mut size;
        let size_without_margin;
        let mut origin_x;
        let mut origin_y;
        let node_kind: *const UINodeKind;

        match self.nodes.borrow_mut(node_handle) {
            None => return,
            Some(node) => {
                if node.visibility != Visibility::Visible {
                    return;
                }

                let margin_x = node.margin.left + node.margin.right;
                let margin_y = node.margin.top + node.margin.bottom;

                origin_x = final_rect.x + node.margin.left;
                origin_y = final_rect.y + node.margin.top;

                size = Vec2 {
                    x: maxf(0.0, final_rect.w - margin_x),
                    y: maxf(0.0, final_rect.h - margin_y),
                };

                size_without_margin = size;

                if node.horizontal_alignment != HorizontalAlignment::Stretch {
                    size.x = minf(size.x, node.desired_size.x - margin_x);
                }
                if node.vertical_alignment != VerticalAlignment::Stretch {
                    size.y = minf(size.y, node.desired_size.y - margin_y);
                }

                if node.width > 0.0 {
                    size.x = node.width;
                }
                if node.height > 0.0 {
                    size.y = node.height;
                }

                // Remember immutable pointer to collection of children nodes on which
                // we'll continue arrange recursively.
                children = UnsafeCollectionView::from_vec(&node.children);
                node_kind = &node.kind as *const UINodeKind;
            }
        }

        size = self.arrange_override(unsafe { &*node_kind }, &children, &size);

        if let Some(node) = self.nodes.borrow_mut(node_handle) {
            if size.x > final_rect.w {
                size.x = final_rect.w;
            }
            if size.y > final_rect.h {
                size.y = final_rect.h;
            }

            match node.horizontal_alignment {
                HorizontalAlignment::Center | HorizontalAlignment::Stretch => {
                    origin_x += (size_without_margin.x - size.x) * 0.5;
                }
                HorizontalAlignment::Right => {
                    origin_x += size_without_margin.x - size.x;
                }
                _ => ()
            }

            match node.vertical_alignment {
                VerticalAlignment::Center | VerticalAlignment::Stretch => {
                    origin_y += (size_without_margin.y - size.y) * 0.5;
                }
                VerticalAlignment::Bottom => {
                    origin_y += size_without_margin.y - size.y;
                }
                _ => ()
            }

            node.actual_size = size;
            node.actual_local_position = Vec2 { x: origin_x, y: origin_y };
        }
    }

    fn update_transform(&mut self, node_handle: &Handle<UINode>) {
        let mut children = UnsafeCollectionView::empty();

        let mut screen_position = Vec2::new();
        if let Some(node) = self.nodes.borrow(node_handle) {
            children = UnsafeCollectionView::from_vec(&node.children);
            if let Some(parent) = self.nodes.borrow(&node.parent) {
                screen_position = node.actual_local_position + parent.screen_position;
            } else {
                screen_position = node.actual_local_position;
            }
        }

        if let Some(node) = self.nodes.borrow_mut(node_handle) {
            node.screen_position = screen_position;
        }

        // Continue on children
        for child_handle in children.iter() {
            self.update_transform(child_handle);
        }
    }

    pub fn update(&mut self, screen_size: &Vec2) {
        let root_canvas_handle = self.root_canvas.clone();
        self.measure(&root_canvas_handle, screen_size);
        self.arrange(&root_canvas_handle, &Rect::new(0.0, 0.0, screen_size.x, screen_size.y));
        self.update_transform(&root_canvas_handle);
    }

    fn draw_node(&mut self, node_handle: &Handle<UINode>, font_cache: &Pool<Font>, nesting: u8) {
        let mut children: UnsafeCollectionView<Handle<UINode>> = UnsafeCollectionView::empty();

        if let Some(node) = self.nodes.borrow_mut(node_handle) {
            let bounds = node.get_screen_bounds();

            self.drawing_context.set_nesting(nesting);
            node.command_indices.push(self.drawing_context.commit_clip_rect(&bounds.inflate(0.9, 0.9)));


            match &mut node.kind {
                UINodeKind::Border(border) => {
                    self.drawing_context.push_rect_filled(&bounds, None, node.color);
                    self.drawing_context.push_rect_vary(&bounds, border.stroke_thickness, border.stroke_color);
                    node.command_indices.push(self.drawing_context.commit(CommandKind::Geometry, 0).unwrap());
                }
                UINodeKind::Text(text) => {
                    if text.need_update {
                        if let Some(font) = font_cache.borrow(&text.font) {
                            let formatted_text = FormattedTextBuilder::reuse(text.formatted_text.take().unwrap())
                                .with_size(node.actual_size)
                                .with_font(font)
                                .with_text(text.text.as_str())
                                .with_color(node.color)
                                .with_horizontal_alignment(text.horizontal_alignment)
                                .with_vertical_alignment(text.vertical_alignment)
                                .build();
                            text.formatted_text = Some(formatted_text);
                        }
                        text.need_update = true; // TODO
                    }
                    if let Some(command_index) = self.drawing_context.draw_text(node.screen_position, text.formatted_text.as_ref().unwrap()) {
                        node.command_indices.push(command_index);
                    }
                }
                _ => ()
            }

            children = UnsafeCollectionView::from_vec(&node.children);
        }

        // Continue on children
        for child_node in children.iter() {
            self.draw_node(child_node, font_cache, nesting + 1);
        }

        self.drawing_context.revert_clip_geom();
    }

    pub fn draw(&mut self, font_cache: &Pool<Font>) -> &DrawingContext {
        self.drawing_context.clear();

        let root_canvas = self.root_canvas.clone();
        self.draw_node(&root_canvas, font_cache, 1);

        if self.visual_debug {
            self.drawing_context.set_nesting(0);

            let picked_bounds =
                if let Some(picked_node) = self.nodes.borrow(&self.picked_node) {
                    Some(picked_node.get_screen_bounds())
                } else {
                    None
                };

            if let Some(picked_bounds) = picked_bounds {
                self.drawing_context.push_rect(&picked_bounds, 1.0, Color::white());
                self.drawing_context.commit(CommandKind::Geometry, 0);
            }
        }

        &self.drawing_context
    }

    fn is_node_clipped(&self, node_handle: &Handle<UINode>, pt: &Vec2) -> bool {
        let mut clipped = true;

        if let Some(node) = self.nodes.borrow(node_handle) {
            if node.visibility != Visibility::Visible {
                return clipped;
            }

            for command_index in node.command_indices.iter() {
                if let Some(command) = self.drawing_context.get_commands().get(*command_index) {
                    if *command.get_kind() == CommandKind::Clip {
                        if self.drawing_context.is_command_contains_point(command, pt) {
                            clipped = false;

                            break;
                        }
                    }
                }
            }

            // Point can be clipped by parent's clipping geometry.
            if !node.parent.is_none() {
                if !clipped {
                    clipped |= self.is_node_clipped(&node.parent, pt);
                }
            }
        }

        clipped
    }

    fn is_node_contains_point(&self, node_handle: &Handle<UINode>, pt: &Vec2) -> bool {
        if let Some(node) = self.nodes.borrow(node_handle) {
            if node.visibility != Visibility::Visible {
                return false;
            }

            if !self.is_node_clipped(node_handle, pt) {
                for command_index in node.command_indices.iter() {
                    if let Some(command) = self.drawing_context.get_commands().get(*command_index) {
                        if *command.get_kind() == CommandKind::Geometry {
                            if self.drawing_context.is_command_contains_point(command, pt) {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        false
    }

    fn pick_node(&self, node_handle: &Handle<UINode>, pt: &Vec2, level: &mut i32) -> Handle<UINode> {
        let mut picked = Handle::none();
        let mut topmost_picked_level = 0;

        if self.is_node_contains_point(node_handle, pt) {
            picked = node_handle.clone();
            topmost_picked_level = *level;
        }

        if let Some(node) = self.nodes.borrow(node_handle) {
            for child_handle in node.children.iter() {
                *level += 1;
                let picked_child = self.pick_node(child_handle, pt, level);
                if !picked_child.is_none() && *level > topmost_picked_level {
                    topmost_picked_level = *level;
                    picked = picked_child;
                }
            }
        }

        return picked;
    }

    pub fn hit_test(&self, pt: &Vec2) -> Handle<UINode> {
        let mut level = 0;
        let node =
            if self.nodes.is_valid_handle(&self.captured_node) {
                self.captured_node.clone()
            } else {
                self.root_canvas.clone()
            };
        self.pick_node(&node, pt, &mut level)
    }

    fn route_event(&mut self, node_handle: Handle<UINode>, event_type: RoutedEventHandlerType, event_args: &mut RoutedEvent) {
        let mut handler = None;
        let mut parent = Handle::none();
        let index = event_type as usize;

        if let Some(node) = self.nodes.borrow_mut(&node_handle) {
            // Take event handler.
            handler = node.event_handlers[index].take();
            parent = node.parent.clone();
        }

        // Execute event handler.
        if let Some(ref mut mouse_enter) = handler {
            mouse_enter(self, node_handle.clone(), event_args);
        }

        if let Some(node) = self.nodes.borrow_mut(&node_handle) {
            // Put event handler back.
            node.event_handlers[index] = handler.take();
        }

        // Route event up on hierarchy (bubbling strategy) until is not handled.
        if !event_args.handled && !parent.is_none() {
            self.route_event(parent, event_type, event_args);
        }
    }

    pub fn process_event(&mut self, event: &glutin::WindowEvent) -> bool {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let pos = Vec2::make(position.x as f32, position.y as f32);
                self.picked_node = self.hit_test(&pos);

                // Fire mouse leave for previously picked node
                if self.picked_node != self.prev_picked_node {
                    let mut fire_mouse_leave = false;
                    if let Some(prev_picked_node) = self.nodes.borrow_mut(&self.prev_picked_node) {
                        if prev_picked_node.is_mouse_over {
                            prev_picked_node.is_mouse_over = false;
                            fire_mouse_leave = true;
                        }
                    }

                    if fire_mouse_leave {
                        let mut evt = RoutedEvent::new(RoutedEventKind::MouseLeave);
                        self.route_event(self.prev_picked_node.clone(), RoutedEventHandlerType::MouseLeave, &mut evt);
                    }
                }

                if !self.picked_node.is_none() {
                    let mut fire_mouse_enter = false;
                    if let Some(picked_node) = self.nodes.borrow_mut(&self.picked_node) {
                        if !picked_node.is_mouse_over {
                            picked_node.is_mouse_over = true;
                            fire_mouse_enter = true;
                        }
                    }

                    if fire_mouse_enter {
                        let mut evt = RoutedEvent::new(RoutedEventKind::MouseEnter);
                        self.route_event(self.picked_node.clone(), RoutedEventHandlerType::MouseEnter, &mut evt);
                    }

                    // Fire mouse move
                    let mut evt = RoutedEvent::new(RoutedEventKind::MouseMove { pos });
                    self.route_event(self.picked_node.clone(), RoutedEventHandlerType::MouseMove, &mut evt);
                }
            }
            _ => ()
        }

        if !self.picked_node.is_none() {
            match event {
                WindowEvent::MouseInput { button, state, .. } => {
                    match state {
                        ElementState::Pressed => {
                            let mut evt = RoutedEvent::new(RoutedEventKind::MouseDown {
                                pos: Vec2::new(),
                                button: *button,
                            });
                            self.route_event(self.picked_node.clone(), RoutedEventHandlerType::MouseDown, &mut evt);
                        }
                        ElementState::Released => {
                            let mut evt = RoutedEvent::new(RoutedEventKind::MouseUp {
                                pos: Vec2::new(),
                                button: *button,
                            });
                            self.route_event(self.picked_node.clone(), RoutedEventHandlerType::MouseUp, &mut evt);
                        }
                    }
                }
                _ => ()
            }
        }

        self.prev_picked_node = self.picked_node.clone();

        false
    }
}

impl UINode {
    pub fn new(kind: UINodeKind) -> UINode {
        UINode {
            kind,
            desired_local_position: Vec2::new(),
            width: std::f32::NAN,
            height: std::f32::NAN,
            screen_position: Vec2::new(),
            desired_size: Vec2::new(),
            actual_local_position: Vec2::new(),
            actual_size: Vec2::new(),
            min_size: Vec2::make(0.0, 0.0),
            max_size: Vec2::make(std::f32::INFINITY, std::f32::INFINITY),
            color: Color::white(),
            row: 0,
            column: 0,
            vertical_alignment: VerticalAlignment::Stretch,
            horizontal_alignment: HorizontalAlignment::Stretch,
            margin: Thickness::zero(),
            visibility: Visibility::Visible,
            children: Vec::new(),
            parent: Handle::none(),
            command_indices: Vec::new(),
            event_handlers: Default::default(),
            is_mouse_over: false,
        }
    }

    #[inline]
    pub fn set_color(&mut self, color: Color) -> &mut Self {
        self.color = color;
        self
    }

    #[inline]
    pub fn set_width(&mut self, width: f32) -> &mut Self {
        self.width = width;
        self
    }

    #[inline]
    pub fn set_height(&mut self, height: f32) -> &mut Self {
        self.height = height;
        self
    }

    #[inline]
    pub fn set_desired_local_position(&mut self, pos: Vec2) -> &mut Self {
        self.desired_local_position = pos;
        self
    }

    #[inline]
    pub fn get_kind(&self) -> &UINodeKind {
        &self.kind
    }

    #[inline]
    pub fn set_vertical_alignment(&mut self, valign: VerticalAlignment) -> &mut Self {
        self.vertical_alignment = valign;
        self
    }

    #[inline]
    pub fn set_horizontal_alignment(&mut self, halign: HorizontalAlignment) -> &mut Self {
        self.horizontal_alignment = halign;
        self
    }

    #[inline]
    pub fn get_kind_mut(&mut self) -> &mut UINodeKind {
        &mut self.kind
    }

    #[inline]
    pub fn get_screen_bounds(&self) -> Rect<f32> {
        Rect::new(self.screen_position.x, self.screen_position.y, self.actual_size.x, self.actual_size.y)
    }

    #[inline]
    pub fn set_handler(&mut self, handler_type: RoutedEventHandlerType, handler: Box<EventHandler>) -> &mut Self {
        self.event_handlers[handler_type as usize] = Some(handler);
        self
    }
}