use crate::message::{MessageData, MessageDirection};
use crate::{
    border::BorderBuilder,
    brush::{Brush, GradientPoint},
    button::ButtonBuilder,
    core::{
        color::Color,
        math::{vec2::Vec2, Rect},
        pool::Handle,
    },
    grid::{Column, GridBuilder, Row},
    message::{
        ButtonMessage, CursorIcon, TextMessage, UiMessage, UiMessageData, WidgetMessage,
        WindowMessage,
    },
    text::TextBuilder,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, HorizontalAlignment, NodeHandleMapping, Thickness, UINode,
    UserInterface,
};
use std::{
    cell::RefCell,
    ops::{Deref, DerefMut},
};

/// Represents a widget looking as window in Windows - with title, minimize and close buttons.
/// It has scrollable region for content, content can be any desired node or even other window.
/// Window can be dragged by its title.
#[derive(Clone)]
pub struct Window<M: MessageData, C: Control<M, C>> {
    widget: Widget<M, C>,
    mouse_click_pos: Vec2,
    initial_position: Vec2,
    initial_size: Vec2,
    is_dragging: bool,
    minimized: bool,
    can_minimize: bool,
    can_close: bool,
    can_resize: bool,
    header: Handle<UINode<M, C>>,
    minimize_button: Handle<UINode<M, C>>,
    close_button: Handle<UINode<M, C>>,
    drag_delta: Vec2,
    content: Handle<UINode<M, C>>,
    grips: RefCell<[Grip; 8]>,
    title: Handle<UINode<M, C>>,
    title_grid: Handle<UINode<M, C>>,
}

const GRIP_SIZE: f32 = 6.0;
const CORNER_GRIP_SIZE: f32 = GRIP_SIZE * 2.0;

#[derive(Copy, Clone, Debug)]
enum GripKind {
    LeftTopCorner = 0,
    RightTopCorner = 1,
    RightBottomCorner = 2,
    LeftBottomCorner = 3,
    Left = 4,
    Top = 5,
    Right = 6,
    Bottom = 7,
}

#[derive(Clone)]
struct Grip {
    kind: GripKind,
    bounds: Rect<f32>,
    is_dragging: bool,
    cursor: CursorIcon,
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

impl<M: MessageData, C: Control<M, C>> Deref for Window<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: MessageData, C: Control<M, C>> DerefMut for Window<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M: MessageData, C: Control<M, C>> Control<M, C> for Window<M, C> {
    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        self.header = *node_map.get(&self.header).unwrap();
        self.minimize_button = *node_map.get(&self.minimize_button).unwrap();
        self.close_button = *node_map.get(&self.close_button).unwrap();
        self.title = *node_map.get(&self.title).unwrap();
        self.title_grid = *node_map.get(&self.title_grid).unwrap();
        if self.content.is_some() {
            self.content = *node_map.get(&self.content).unwrap();
        }
    }

    fn arrange_override(&self, ui: &UserInterface<M, C>, final_size: Vec2) -> Vec2 {
        let size = self.widget.arrange_override(ui, final_size);

        let mut grips = self.grips.borrow_mut();

        // Adjust grips.
        grips[GripKind::Left as usize].bounds = Rect {
            x: 0.0,
            y: GRIP_SIZE,
            w: GRIP_SIZE,
            h: final_size.y - GRIP_SIZE * 2.0,
        };
        grips[GripKind::Top as usize].bounds = Rect {
            x: GRIP_SIZE,
            y: 0.0,
            w: final_size.x - GRIP_SIZE * 2.0,
            h: GRIP_SIZE,
        };
        grips[GripKind::Right as usize].bounds = Rect {
            x: final_size.x - GRIP_SIZE,
            y: GRIP_SIZE,
            w: GRIP_SIZE,
            h: final_size.y - GRIP_SIZE * 2.0,
        };
        grips[GripKind::Bottom as usize].bounds = Rect {
            x: GRIP_SIZE,
            y: final_size.y - GRIP_SIZE,
            w: final_size.x - GRIP_SIZE * 2.0,
            h: GRIP_SIZE,
        };

        // Corners have different size to improve usability.
        grips[GripKind::LeftTopCorner as usize].bounds = Rect {
            x: 0.0,
            y: 0.0,
            w: CORNER_GRIP_SIZE,
            h: CORNER_GRIP_SIZE,
        };
        grips[GripKind::RightTopCorner as usize].bounds = Rect {
            x: final_size.x - GRIP_SIZE,
            y: 0.0,
            w: CORNER_GRIP_SIZE,
            h: CORNER_GRIP_SIZE,
        };
        grips[GripKind::RightBottomCorner as usize].bounds = Rect {
            x: final_size.x - CORNER_GRIP_SIZE,
            y: final_size.y - CORNER_GRIP_SIZE,
            w: CORNER_GRIP_SIZE,
            h: CORNER_GRIP_SIZE,
        };
        grips[GripKind::LeftBottomCorner as usize].bounds = Rect {
            x: 0.0,
            y: final_size.y - CORNER_GRIP_SIZE,
            w: CORNER_GRIP_SIZE,
            h: CORNER_GRIP_SIZE,
        };

        size
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);

        match &message.data() {
            UiMessageData::Widget(msg) => {
                // Grip interaction have higher priority than other actions.
                if self.can_resize {
                    match msg {
                        &WidgetMessage::MouseDown { pos, .. } => {
                            ui.send_message(WidgetMessage::topmost(
                                self.handle(),
                                MessageDirection::ToWidget,
                            ));

                            // Check grips.
                            for grip in self.grips.borrow_mut().iter_mut() {
                                let offset = self.screen_position;
                                let screen_bounds = grip.bounds.translate(offset.x, offset.y);
                                if screen_bounds.contains(pos.x, pos.y) {
                                    grip.is_dragging = true;
                                    self.initial_position = self.actual_local_position();
                                    self.initial_size = self.actual_size();
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
                                let offset = self.screen_position;
                                let screen_bounds = grip.bounds.translate(offset.x, offset.y);
                                if screen_bounds.contains(pos.x, pos.y) {
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
                                        + Vec2::new(delta.x * dx, delta.y * dy);
                                    let new_size =
                                        self.initial_size + Vec2::new(delta.x * dw, delta.y * dh);

                                    if new_size.x > self.min_width()
                                        && new_size.x < self.max_width()
                                        && new_size.y > self.min_height()
                                        && new_size.y < self.max_height()
                                    {
                                        ui.send_message(WidgetMessage::desired_position(
                                            self.handle(),
                                            MessageDirection::ToWidget,
                                            new_pos,
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
                                MessageDirection::FromWidget,
                            ));
                            message.set_handled(true);
                        }
                        WidgetMessage::MouseUp { .. } => {
                            ui.send_message(WindowMessage::move_end(
                                self.handle,
                                MessageDirection::FromWidget,
                            ));
                            message.set_handled(true);
                        }
                        WidgetMessage::MouseMove { pos, .. } => {
                            if self.is_dragging {
                                self.drag_delta = *pos - self.mouse_click_pos;
                                let new_pos = self.initial_position + self.drag_delta;
                                ui.send_message(WindowMessage::move_to(
                                    self.handle(),
                                    MessageDirection::FromWidget,
                                    new_pos,
                                ));
                            }
                            message.set_handled(true);
                        }
                        _ => (),
                    }
                }
                if let WidgetMessage::Unlink = msg {
                    if message.destination() == self.handle() {
                        self.initial_position = self.screen_position;
                    }
                }
            }
            UiMessageData::Button(msg) => {
                if let ButtonMessage::Click = msg {
                    if message.destination() == self.minimize_button {
                        ui.send_message(WindowMessage::minimize(
                            self.handle(),
                            MessageDirection::FromWidget,
                            !self.minimized,
                        ));
                    } else if message.destination() == self.close_button {
                        ui.send_message(WindowMessage::close(
                            self.handle(),
                            MessageDirection::FromWidget,
                        ));
                    }
                }
            }
            UiMessageData::Window(msg) => {
                if message.destination() == self.handle() {
                    match msg {
                        WindowMessage::Open => {
                            ui.send_message(WidgetMessage::visibility(
                                self.handle(),
                                MessageDirection::ToWidget,
                                true,
                            ));
                        }
                        WindowMessage::OpenModal => {
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
                                ui.push_picking_restriction(self.handle());
                            }
                        }
                        WindowMessage::Close => {
                            ui.send_message(WidgetMessage::visibility(
                                self.handle(),
                                MessageDirection::ToWidget,
                                false,
                            ));
                            ui.remove_picking_restriction(self.handle());
                        }
                        &WindowMessage::Minimize(minimized) => {
                            if self.minimized != minimized {
                                self.minimized = minimized;
                                self.invalidate_layout();
                                if self.content.is_some() {
                                    ui.send_message(WidgetMessage::visibility(
                                        self.content,
                                        MessageDirection::ToWidget,
                                        !minimized,
                                    ));
                                }
                            }
                        }
                        &WindowMessage::CanMinimize(value) => {
                            if self.can_minimize != value {
                                self.can_minimize = value;
                                self.invalidate_layout();
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
                                self.invalidate_layout();
                                if self.close_button.is_some() {
                                    ui.send_message(WidgetMessage::visibility(
                                        self.close_button,
                                        MessageDirection::ToWidget,
                                        value,
                                    ));
                                }
                            }
                        }
                        &WindowMessage::Move(new_pos) => {
                            if self.desired_local_position() != new_pos {
                                ui.send_message(WidgetMessage::desired_position(
                                    self.handle(),
                                    MessageDirection::ToWidget,
                                    new_pos,
                                ));
                            }
                        }
                        WindowMessage::MoveStart => {
                            ui.capture_mouse(self.header);
                            let initial_position = self.actual_local_position();
                            self.initial_position = initial_position;
                            self.is_dragging = true;
                        }
                        WindowMessage::MoveEnd => {
                            ui.release_mouse_capture();
                            self.is_dragging = false;
                        }
                        WindowMessage::Title(title) => {
                            match title {
                                WindowTitle::Text(text) => {
                                    if let UINode::Text(_) = ui.node(self.title) {
                                        // Just modify existing text, this is much faster than
                                        // re-create text everytime.
                                        ui.send_message(TextMessage::text(
                                            self.title,
                                            MessageDirection::ToWidget,
                                            text.clone(),
                                        ));
                                    } else {
                                        ui.send_message(WidgetMessage::remove(
                                            self.title,
                                            MessageDirection::ToWidget,
                                        ));
                                        self.title = make_text_title(&mut ui.build_ctx(), text);
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
                    }
                }
            }
            _ => (),
        }
    }

    fn remove_ref(&mut self, handle: Handle<UINode<M, C>>) {
        if self.header == handle {
            self.header = Handle::NONE;
        }
        if self.content == handle {
            self.content = Handle::NONE;
        }
        if self.close_button == handle {
            self.close_button = Handle::NONE;
        }
        if self.minimize_button == handle {
            self.minimize_button = Handle::NONE;
        }
        if self.title == handle {
            self.title = Handle::NONE;
        }
        if self.title_grid == handle {
            self.title_grid = Handle::NONE;
        }
    }
}

impl<M: MessageData, C: Control<M, C>> Window<M, C> {
    pub fn is_dragging(&self) -> bool {
        self.is_dragging
    }

    pub fn drag_delta(&self) -> Vec2 {
        self.drag_delta
    }

    pub fn has_active_grip(&self) -> bool {
        for grip in self.grips.borrow().iter() {
            if grip.is_dragging {
                return true;
            }
        }
        false
    }
}

pub struct WindowBuilder<M: MessageData, C: Control<M, C>> {
    pub widget_builder: WidgetBuilder<M, C>,
    pub content: Handle<UINode<M, C>>,
    pub title: Option<WindowTitle<M, C>>,
    pub can_close: bool,
    pub can_minimize: bool,
    pub open: bool,
    pub close_button: Option<Handle<UINode<M, C>>>,
    pub minimize_button: Option<Handle<UINode<M, C>>>,
    // Warning: Any dependant builders must take this into account!
    pub modal: bool,
    pub can_resize: bool,
}

/// Window title can be either text or node.
///
/// If `Text` is used, then builder will automatically create Text node with specified text,
/// but with default font.
///
/// If you need more flexibility (i.e. put a picture near text) then `Node` option is for you:
/// it allows to put any UI node hierarchy you want to.
#[derive(Debug, Clone, PartialEq)]
pub enum WindowTitle<M: MessageData, C: Control<M, C>> {
    Text(String),
    Node(Handle<UINode<M, C>>),
}

impl<M: MessageData, C: Control<M, C>> WindowTitle<M, C> {
    pub fn text<P: AsRef<str>>(text: P) -> Self {
        WindowTitle::Text(text.as_ref().to_owned())
    }

    pub fn node(node: Handle<UINode<M, C>>) -> Self {
        WindowTitle::Node(node)
    }
}

fn make_text_title<M: MessageData, C: Control<M, C>>(
    ctx: &mut BuildContext<M, C>,
    text: &str,
) -> Handle<UINode<M, C>> {
    TextBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::uniform(5.0))
            .on_row(0)
            .on_column(0),
    )
    .with_text(text)
    .build(ctx)
}

impl<'a, M: MessageData, C: Control<M, C>> WindowBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            content: Handle::NONE,
            title: None,
            can_close: true,
            can_minimize: true,
            open: true,
            close_button: None,
            minimize_button: None,
            modal: false,
            can_resize: true,
        }
    }

    pub fn with_content(mut self, content: Handle<UINode<M, C>>) -> Self {
        self.content = content;
        self
    }

    pub fn with_title(mut self, title: WindowTitle<M, C>) -> Self {
        self.title = Some(title);
        self
    }

    pub fn with_minimize_button(mut self, button: Handle<UINode<M, C>>) -> Self {
        self.minimize_button = Some(button);
        self
    }

    pub fn with_close_button(mut self, button: Handle<UINode<M, C>>) -> Self {
        self.close_button = Some(button);
        self
    }

    pub fn can_close(mut self, can_close: bool) -> Self {
        self.can_close = can_close;
        self
    }

    pub fn can_minimize(mut self, can_minimize: bool) -> Self {
        self.can_minimize = can_minimize;
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn modal(mut self, modal: bool) -> Self {
        self.modal = modal;
        self
    }

    pub fn can_resize(mut self, can_resize: bool) -> Self {
        self.can_resize = can_resize;
        self
    }

    pub fn build_window(self, ctx: &mut BuildContext<M, C>) -> Window<M, C> {
        let minimize_button;
        let close_button;

        let title;
        let title_grid;
        let header = BorderBuilder::new(
            WidgetBuilder::new()
                .with_horizontal_alignment(HorizontalAlignment::Stretch)
                .with_height(30.0)
                .with_background(Brush::LinearGradient {
                    from: Vec2::new(0.5, 0.0),
                    to: Vec2::new(0.5, 1.0),
                    stops: vec![
                        GradientPoint {
                            stop: 0.0,
                            color: Color::opaque(85, 85, 85),
                        },
                        GradientPoint {
                            stop: 0.5,
                            color: Color::opaque(65, 65, 65),
                        },
                        GradientPoint {
                            stop: 1.0,
                            color: Color::opaque(75, 75, 75),
                        },
                    ],
                })
                .with_child({
                    title_grid = GridBuilder::new(
                        WidgetBuilder::new()
                            .with_child({
                                title = match self.title {
                                    None => Handle::NONE,
                                    Some(window_title) => match window_title {
                                        WindowTitle::Node(node) => node,
                                        WindowTitle::Text(text) => make_text_title(ctx, &text),
                                    },
                                };
                                title
                            })
                            .with_child({
                                minimize_button = self.minimize_button.unwrap_or_else(|| {
                                    ButtonBuilder::new(
                                        WidgetBuilder::new().with_margin(Thickness::uniform(2.0)),
                                    )
                                    .with_text("_")
                                    .build(ctx)
                                });
                                ctx[minimize_button]
                                    .set_visibility(self.can_minimize)
                                    .set_width(30.0)
                                    .set_row(0)
                                    .set_column(1);
                                minimize_button
                            })
                            .with_child({
                                close_button = self.close_button.unwrap_or_else(|| {
                                    ButtonBuilder::new(
                                        WidgetBuilder::new().with_margin(Thickness::uniform(2.0)),
                                    )
                                    .with_text("X")
                                    .build(ctx)
                                });
                                ctx[close_button]
                                    .set_width(30.0)
                                    .set_visibility(self.can_close)
                                    .set_row(0)
                                    .set_column(2);
                                close_button
                            }),
                    )
                    .add_column(Column::stretch())
                    .add_column(Column::auto())
                    .add_column(Column::auto())
                    .add_row(Row::stretch())
                    .build(ctx);
                    title_grid
                })
                .on_row(0),
        )
        .build(ctx);

        if self.content.is_some() {
            ctx[self.content].set_row(1);
        }
        Window {
            widget: self
                .widget_builder
                .with_visibility(self.open)
                .with_child(
                    BorderBuilder::new(
                        WidgetBuilder::new().with_child(
                            GridBuilder::new(
                                WidgetBuilder::new()
                                    .with_child(self.content)
                                    .with_child(header),
                            )
                            .add_column(Column::stretch())
                            .add_row(Row::auto())
                            .add_row(Row::stretch())
                            .build(ctx),
                        ),
                    )
                    .build(ctx),
                )
                .build(),
            mouse_click_pos: Vec2::ZERO,
            initial_position: Vec2::ZERO,
            initial_size: Default::default(),
            is_dragging: false,
            minimized: false,
            can_minimize: self.can_minimize,
            can_close: self.can_close,
            can_resize: self.can_resize,
            header,
            minimize_button,
            close_button,
            drag_delta: Default::default(),
            content: self.content,
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
        }
    }

    pub fn build(self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        let modal = self.modal;
        let open = self.open;

        let node = self.build_window(ctx);
        let handle = ctx.add_node(UINode::Window(node));

        if modal && open {
            ctx.ui.push_picking_restriction(handle);
        }

        handle
    }
}
