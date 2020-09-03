use crate::{
    brush::Brush,
    core::{
        color::Color,
        math::{vec2::Vec2, Rect},
        pool::Handle,
    },
    draw::CommandTexture,
    draw::{CommandKind, DrawingContext},
    formatted_text::{FormattedText, FormattedTextBuilder},
    message::{KeyCode, MouseButton, TextBoxMessage, UiMessage, UiMessageData, WidgetMessage},
    ttf::Font,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, HorizontalAlignment, UINode, UserInterface, VerticalAlignment,
};
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use std::{
    cell::RefCell,
    cmp,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum HorizontalDirection {
    Left,
    Right,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum VerticalDirection {
    Down,
    Up,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Position {
    line: usize,
    offset: usize,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct SelectionRange {
    begin: Position,
    end: Position,
}

pub type FilterCallback = dyn FnMut(char) -> bool;

pub struct TextBox<M: 'static + std::fmt::Debug, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    caret_line: usize,
    caret_offset: usize,
    caret_visible: bool,
    blink_timer: f32,
    blink_interval: f32,
    formatted_text: RefCell<FormattedText>,
    selection_range: Option<SelectionRange>,
    selecting: bool,
    has_focus: bool,
    caret_brush: Brush,
    selection_brush: Brush,
    filter: Option<Rc<RefCell<FilterCallback>>>,
}

impl<M: 'static + std::fmt::Debug, C: 'static + Control<M, C>> Debug for TextBox<M, C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("TextBox")
    }
}

impl<M: 'static + std::fmt::Debug, C: 'static + Control<M, C>> Deref for TextBox<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: 'static + std::fmt::Debug, C: 'static + Control<M, C>> DerefMut for TextBox<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M: 'static + std::fmt::Debug, C: 'static + Control<M, C>> TextBox<M, C> {
    pub fn new(widget: Widget<M, C>) -> Self {
        Self {
            widget,
            caret_line: 0,
            caret_offset: 0,
            caret_visible: false,
            blink_timer: 0.0,
            blink_interval: 0.0,
            formatted_text: RefCell::new(
                FormattedTextBuilder::new()
                    .with_font(crate::DEFAULT_FONT.clone())
                    .build(),
            ),
            selection_range: None,
            selecting: false,
            has_focus: false,
            caret_brush: Brush::Solid(Color::WHITE),
            selection_brush: Brush::Solid(Color::opaque(65, 65, 90)),
            filter: None,
        }
    }

    pub fn reset_blink(&mut self) {
        self.caret_visible = true;
        self.blink_timer = 0.0;
    }

    pub fn move_caret_x(&mut self, mut offset: usize, direction: HorizontalDirection) {
        self.selection_range = None;

        self.reset_blink();

        let text = self.formatted_text.borrow();
        let lines = text.get_lines();

        if lines.is_empty() {
            self.caret_offset = 0;
            self.caret_line = 0;
            return;
        }

        while offset > 0 {
            match direction {
                HorizontalDirection::Left => {
                    if self.caret_offset > 0 {
                        self.caret_offset -= 1
                    } else if self.caret_line > 0 {
                        self.caret_line -= 1;
                        self.caret_offset = lines[self.caret_line].len();
                    } else {
                        self.caret_offset = 0;
                        break;
                    }
                }
                HorizontalDirection::Right => {
                    let line = lines.get(self.caret_line).unwrap();
                    if self.caret_offset < line.len() {
                        self.caret_offset += 1;
                    } else if self.caret_line < lines.len() - 1 {
                        self.caret_line += 1;
                        self.caret_offset = 0;
                    } else {
                        self.caret_offset = line.len();
                        break;
                    }
                }
            }
            offset -= 1;
        }
    }

    pub fn move_caret_y(&mut self, offset: usize, direction: VerticalDirection) {
        let text = self.formatted_text.borrow();
        let lines = text.get_lines();

        if lines.is_empty() {
            return;
        }

        let line_count = lines.len();

        match direction {
            VerticalDirection::Down => {
                if self.caret_line + offset >= line_count {
                    self.caret_line = line_count - 1;
                } else {
                    self.caret_line += offset;
                }
            }
            VerticalDirection::Up => {
                if self.caret_line > offset {
                    self.caret_line -= offset;
                } else {
                    self.caret_line = 0;
                }
            }
        }
    }

    pub fn get_absolute_position(&self) -> Option<usize> {
        if let Some(line) = self
            .formatted_text
            .borrow()
            .get_lines()
            .get(self.caret_line)
        {
            Some(line.begin + cmp::min(self.caret_offset, line.len()))
        } else {
            None
        }
    }

    /// Inserts given character at current caret position.
    fn insert_char(&mut self, c: char, ui: &UserInterface<M, C>) {
        if !c.is_control() {
            let position = self.get_absolute_position().unwrap_or(0);
            self.formatted_text
                .borrow_mut()
                .insert_char(c, position)
                .build();
            self.move_caret_x(1, HorizontalDirection::Right);
            ui.send_message(UiMessage {
                handled: false,
                data: UiMessageData::TextBox(TextBoxMessage::Text(
                    self.formatted_text.borrow().text(),
                )), // Requires allocation
                destination: self.handle(),
            });
        }
    }

    pub fn get_text_len(&self) -> usize {
        self.formatted_text.borrow_mut().get_raw_text().len()
    }

    fn remove_char(&mut self, direction: HorizontalDirection, ui: &UserInterface<M, C>) {
        if let Some(position) = self.get_absolute_position() {
            let text_len = self.get_text_len();
            if text_len != 0 {
                let position = match direction {
                    HorizontalDirection::Left => {
                        if position == 0 {
                            return;
                        }
                        position - 1
                    }
                    HorizontalDirection::Right => {
                        if position >= text_len {
                            return;
                        }
                        position
                    }
                };
                self.formatted_text.borrow_mut().remove_at(position);
                self.formatted_text.borrow_mut().build();

                ui.send_message(UiMessage {
                    handled: false,
                    data: UiMessageData::TextBox(TextBoxMessage::Text(
                        self.formatted_text.borrow().text(),
                    )), // Requires allocation
                    destination: self.handle(),
                });

                if direction == HorizontalDirection::Left {
                    self.move_caret_x(1, direction);
                }
            }
        }
    }

    pub fn screen_pos_to_text_pos(&self, screen_pos: Vec2) -> Option<Position> {
        let mut caret_pos = self.widget.screen_position;
        if let Some(font) = self.formatted_text.borrow().get_font() {
            let font = font.lock().unwrap();
            for (line_index, line) in self.formatted_text.borrow().get_lines().iter().enumerate() {
                let line_bounds = Rect::new(
                    caret_pos.x + line.x_offset,
                    caret_pos.y,
                    line.width,
                    font.get_ascender(),
                );
                if line_bounds.contains(screen_pos.x, screen_pos.y) {
                    let mut x = line_bounds.x;
                    // Check each character in line.
                    for (offset, index) in (line.begin..line.end).enumerate() {
                        let symbol = self.formatted_text.borrow().get_raw_text()[index];
                        let (width, height, advance) = if let Some(glyph) = font.get_glyph(symbol) {
                            (
                                glyph.get_bitmap_width(),
                                glyph.get_bitmap_height(),
                                glyph.get_advance(),
                            )
                        } else {
                            // Stub
                            let h = font.get_height();
                            (h, h, h)
                        };
                        let char_bounds = Rect::new(x, line_bounds.y, width, height);
                        if char_bounds.contains(screen_pos.x, screen_pos.y) {
                            return Some(Position {
                                line: line_index,
                                offset,
                            });
                        }
                        x += advance;
                    }
                }
                caret_pos.y += line_bounds.h;
            }
        }
        None
    }

    pub fn set_text<P: AsRef<str>>(&mut self, text: P) -> &mut Self {
        let mut equals = false;
        for (&new, old) in self
            .formatted_text
            .borrow()
            .get_raw_text()
            .iter()
            .zip(text.as_ref().chars())
        {
            if old as u32 != new {
                equals = false;
                break;
            }
        }
        if !equals {
            self.formatted_text.borrow_mut().set_text(text);
            self.invalidate_layout();
        }
        self
    }

    pub fn text(&self) -> String {
        self.formatted_text.borrow().text()
    }

    pub fn set_wrap(&mut self, wrap: bool) -> &mut Self {
        self.formatted_text.borrow_mut().set_wrap(wrap);
        self
    }

    pub fn is_wrap(&self) -> bool {
        self.formatted_text.borrow().is_wrap()
    }

    pub fn set_font(&mut self, font: Arc<Mutex<Font>>) -> &mut Self {
        self.formatted_text.borrow_mut().set_font(font);
        self
    }

    pub fn font(&self) -> Arc<Mutex<Font>> {
        self.formatted_text.borrow().get_font().unwrap()
    }

    pub fn set_vertical_alignment(&mut self, valign: VerticalAlignment) -> &mut Self {
        self.formatted_text
            .borrow_mut()
            .set_vertical_alignment(valign);
        self
    }

    pub fn vertical_alignment(&self) -> VerticalAlignment {
        self.formatted_text.borrow().vertical_alignment()
    }

    pub fn set_horizontal_alignment(&mut self, halign: HorizontalAlignment) -> &mut Self {
        self.formatted_text
            .borrow_mut()
            .set_horizontal_alignment(halign);
        self
    }

    pub fn horizontal_alignment(&self) -> HorizontalAlignment {
        self.formatted_text.borrow().horizontal_alignment()
    }
}

impl<M: 'static + std::fmt::Debug, C: 'static + Control<M, C>> Control<M, C> for TextBox<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        UINode::TextBox(Self {
            widget: self.widget.raw_copy(),
            caret_line: self.caret_line,
            caret_offset: self.caret_offset,
            caret_visible: self.caret_visible,
            blink_timer: self.blink_timer,
            blink_interval: self.blink_interval,
            formatted_text: RefCell::new(
                FormattedTextBuilder::new()
                    .with_font(self.formatted_text.borrow().get_font().unwrap())
                    .build(),
            ),
            selection_range: self.selection_range,
            selecting: self.selecting,
            selection_brush: self.selection_brush.clone(),
            caret_brush: self.caret_brush.clone(),
            has_focus: false,
            filter: None,
        })
    }

    fn measure_override(&self, _: &UserInterface<M, C>, available_size: Vec2) -> Vec2 {
        self.formatted_text
            .borrow_mut()
            .set_constraint(available_size)
            .build()
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        let bounds = self.widget.screen_bounds();
        drawing_context.push_rect_filled(&bounds, None);
        drawing_context.commit(
            CommandKind::Geometry,
            self.widget.background(),
            CommandTexture::None,
        );

        self.formatted_text
            .borrow_mut()
            .set_constraint(Vec2::new(bounds.w, bounds.h))
            .set_brush(self.widget.foreground())
            .build();

        if let Some(ref selection_range) = self.selection_range {
            let text = self.formatted_text.borrow();
            let lines = text.get_lines();
            if selection_range.begin.line == selection_range.end.line {
                let line = lines[selection_range.begin.line];
                let begin = selection_range.begin.offset;
                let end = selection_range.end.offset;
                // Begin line
                let offset = text.get_range_width(line.begin..(line.begin + begin));
                let width = text.get_range_width((line.begin + begin)..(line.begin + end));
                let bounds = Rect::new(
                    bounds.x + line.x_offset + offset,
                    bounds.y + line.y_offset,
                    width,
                    line.height,
                );
                drawing_context.push_rect_filled(&bounds, None);
            } else {
                for (i, line) in text.get_lines().iter().enumerate() {
                    if i >= selection_range.begin.line && i <= selection_range.end.line {
                        let bounds = if i == selection_range.begin.line {
                            // Begin line
                            let offset = text.get_range_width(
                                line.begin..(line.begin + selection_range.begin.offset),
                            );
                            let width = text.get_range_width(
                                (line.begin + selection_range.begin.offset)..line.end,
                            );
                            Rect::new(
                                bounds.x + line.x_offset + offset,
                                bounds.y + line.y_offset,
                                width,
                                line.height,
                            )
                        } else if i == selection_range.end.line {
                            // End line
                            let width = text.get_range_width(
                                line.begin..(line.begin + selection_range.end.offset),
                            );
                            Rect::new(
                                bounds.x + line.x_offset,
                                bounds.y + line.y_offset,
                                width,
                                line.height,
                            )
                        } else {
                            // Everything between
                            Rect::new(
                                bounds.x + line.x_offset,
                                bounds.y + line.y_offset,
                                line.width,
                                line.height,
                            )
                        };
                        drawing_context.push_rect_filled(&bounds, None);
                    }
                }
            }
        }
        drawing_context.commit(
            CommandKind::Geometry,
            self.selection_brush.clone(),
            CommandTexture::None,
        );

        let screen_position = Vec2::new(bounds.x, bounds.y);
        drawing_context.draw_text(screen_position, &self.formatted_text.borrow());

        if self.caret_visible {
            let text = self.formatted_text.borrow();
            if let Some(font) = text.get_font() {
                let font = font.lock().unwrap();
                if let Some(line) = text.get_lines().get(self.caret_line) {
                    let text = text.get_raw_text();
                    let mut caret_pos = Vec2::new(
                        screen_position.x + line.x_offset,
                        screen_position.y
                            + line.y_offset
                            + self.caret_line as f32 * font.get_ascender(),
                    );
                    for (offset, char_index) in (line.begin..line.end).enumerate() {
                        if offset >= self.caret_offset {
                            break;
                        }
                        if let Some(glyph) = font.get_glyph(text[char_index]) {
                            caret_pos.x += glyph.get_advance();
                        } else {
                            caret_pos.x += font.get_height();
                        }
                    }

                    let caret_bounds = Rect::new(caret_pos.x, caret_pos.y, 2.0, font.get_height());
                    drawing_context.push_rect_filled(&caret_bounds, None);
                    drawing_context.commit(
                        CommandKind::Geometry,
                        self.caret_brush.clone(),
                        CommandTexture::None,
                    );
                }
            }
        }
    }

    fn update(&mut self, dt: f32) {
        if self.has_focus {
            self.blink_timer += dt;
            if self.blink_timer >= self.blink_interval {
                self.blink_timer = 0.0;
                self.caret_visible = !self.caret_visible;
            }
        } else {
            self.caret_visible = false;
        }
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);

        if message.destination == self.handle() {
            match &message.data {
                UiMessageData::Widget(msg) => match msg {
                    &WidgetMessage::Text(symbol) => {
                        let insert = if let Some(filter) = self.filter.as_ref() {
                            let filter = &mut *filter.borrow_mut();
                            filter(symbol)
                        } else {
                            true
                        };
                        if insert {
                            self.insert_char(symbol, ui);
                        }
                    }
                    WidgetMessage::KeyDown(code) => match code {
                        KeyCode::Up => {
                            self.move_caret_y(1, VerticalDirection::Up);
                        }
                        KeyCode::Down => {
                            self.move_caret_y(1, VerticalDirection::Down);
                        }
                        KeyCode::Right => {
                            self.move_caret_x(1, HorizontalDirection::Right);
                        }
                        KeyCode::Left => {
                            self.move_caret_x(1, HorizontalDirection::Left);
                        }
                        KeyCode::Delete => {
                            self.remove_char(HorizontalDirection::Right, ui);
                        }
                        KeyCode::Backspace => {
                            self.remove_char(HorizontalDirection::Left, ui);
                        }
                        _ => (),
                    },
                    WidgetMessage::GotFocus => {
                        self.reset_blink();
                        self.has_focus = true;
                    }
                    WidgetMessage::LostFocus => {
                        self.has_focus = false;
                    }
                    WidgetMessage::MouseDown { pos, button } => {
                        if *button == MouseButton::Left {
                            self.selection_range = None;
                            self.selecting = true;

                            if let Some(position) = self.screen_pos_to_text_pos(*pos) {
                                self.caret_line = position.line;
                                self.caret_offset = position.offset;

                                self.selection_range = Some(SelectionRange {
                                    begin: position,
                                    end: position,
                                })
                            }

                            ui.capture_mouse(self.handle());
                        }
                    }
                    WidgetMessage::MouseMove { pos, .. } => {
                        if self.selecting {
                            if let Some(position) = self.screen_pos_to_text_pos(*pos) {
                                if let Some(ref mut sel_range) = self.selection_range {
                                    sel_range.end = position;
                                }
                            }
                        }
                    }
                    WidgetMessage::MouseUp { .. } => {
                        self.selecting = false;

                        ui.release_mouse_capture();
                    }
                    _ => {}
                },
                UiMessageData::TextBox(msg) => {
                    if let TextBoxMessage::Text(new_text) = msg {
                        self.set_text(new_text);
                    }
                }
                _ => {}
            }
        }
    }
}

pub struct TextBoxBuilder<M: 'static + std::fmt::Debug, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    font: Option<Arc<Mutex<Font>>>,
    text: String,
    caret_brush: Brush,
    selection_brush: Brush,
    filter: Option<Rc<RefCell<FilterCallback>>>,
    vertical_alignment: VerticalAlignment,
    horizontal_alignment: HorizontalAlignment,
    wrap: bool,
}

impl<M: 'static + std::fmt::Debug, C: 'static + Control<M, C>> TextBoxBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            font: None,
            text: "".to_owned(),
            caret_brush: Brush::Solid(Color::WHITE),
            selection_brush: Brush::Solid(Color::opaque(65, 65, 90)),
            filter: None,
            vertical_alignment: VerticalAlignment::Top,
            horizontal_alignment: HorizontalAlignment::Left,
            wrap: false,
        }
    }

    pub fn with_font(mut self, font: Arc<Mutex<Font>>) -> Self {
        self.font = Some(font);
        self
    }

    pub fn with_text<P: AsRef<str>>(mut self, text: P) -> Self {
        self.text = text.as_ref().to_owned();
        self
    }

    pub fn with_caret_brush(mut self, brush: Brush) -> Self {
        self.caret_brush = brush;
        self
    }

    pub fn with_selection_brush(mut self, brush: Brush) -> Self {
        self.selection_brush = brush;
        self
    }

    pub fn with_filter(mut self, filter: Rc<RefCell<FilterCallback>>) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn with_vertical_text_alignment(mut self, alignment: VerticalAlignment) -> Self {
        self.vertical_alignment = alignment;
        self
    }

    pub fn with_horizontal_text_alignment(mut self, alignment: HorizontalAlignment) -> Self {
        self.horizontal_alignment = alignment;
        self
    }

    pub fn with_wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn build(mut self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        if self.widget_builder.foreground.is_none() {
            self.widget_builder.foreground = Some(Brush::Solid(Color::opaque(220, 220, 220)));
        }
        if self.widget_builder.background.is_none() {
            self.widget_builder.background = Some(Brush::Solid(Color::opaque(100, 100, 100)));
        }

        let text_box = TextBox {
            widget: self.widget_builder.build(),
            caret_line: 0,
            caret_offset: 0,
            caret_visible: false,
            blink_timer: 0.0,
            blink_interval: 0.5,
            formatted_text: RefCell::new(
                FormattedTextBuilder::new()
                    .with_text(self.text)
                    .with_font(self.font.unwrap_or_else(|| crate::DEFAULT_FONT.clone()))
                    .with_horizontal_alignment(self.horizontal_alignment)
                    .with_vertical_alignment(self.vertical_alignment)
                    .with_wrap(self.wrap)
                    .build(),
            ),
            selection_range: None,
            selecting: false,
            selection_brush: self.selection_brush,
            caret_brush: self.caret_brush,
            has_focus: false,
            filter: self.filter,
        };

        ctx.add_node(UINode::TextBox(text_box))
    }
}
