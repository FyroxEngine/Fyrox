use crate::draw::Draw;
use crate::{
    brush::Brush,
    core::{algebra::Vector2, color::Color, math::Rect, pool::Handle},
    draw::{CommandTexture, DrawingContext},
    formatted_text::{FormattedText, FormattedTextBuilder},
    message::{
        CursorIcon, KeyCode, MessageData, MessageDirection, MouseButton, TextBoxMessage, UiMessage,
        UiMessageData, WidgetMessage,
    },
    ttf::SharedFont,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, HorizontalAlignment, UINode, UserInterface, VerticalAlignment,
    BRUSH_DARKER, BRUSH_TEXT,
};
use std::{
    cell::RefCell,
    cmp::{self, Ordering},
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
    rc::Rc,
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

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Position {
    // Line index.
    line: usize,

    // Offset from beginning of a line.
    offset: usize,
}

#[derive(Copy, Clone, PartialOrd, PartialEq, Eq, Ord, Hash)]
#[repr(u32)]
pub enum TextCommitMode {
    /// Text box will immediately send Text message after any change.
    Immediate = 0,

    /// Text box will send Text message only when it loses focus.
    LostFocus = 1,

    /// Text box will send Text message when it loses focus or if Enter
    /// key was pressed. This is **default** behavior.
    ///
    /// # Notes
    ///
    /// In case of multiline text box hitting Enter key won't commit text!
    LostFocusPlusEnter = 2,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct SelectionRange {
    begin: Position,
    end: Position,
}

impl SelectionRange {
    #[must_use = "method creates new value which must be used"]
    pub fn normalized(&self) -> SelectionRange {
        match self.begin.line.cmp(&self.end.line) {
            Ordering::Less => *self,
            Ordering::Equal => {
                if self.begin.offset > self.end.offset {
                    SelectionRange {
                        begin: self.end,
                        end: self.begin,
                    }
                } else {
                    *self
                }
            }
            Ordering::Greater => SelectionRange {
                begin: self.end,
                end: self.begin,
            },
        }
    }
}

pub type FilterCallback = dyn FnMut(char) -> bool;

#[derive(Clone)]
pub struct TextBox<M: MessageData, C: Control<M, C>> {
    widget: Widget<M, C>,
    caret_position: Position,
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
    commit_mode: TextCommitMode,
    multiline: bool,
}

impl<M: MessageData, C: Control<M, C>> Debug for TextBox<M, C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("TextBox")
    }
}

crate::define_widget_deref!(TextBox<M, C>);

impl<M: MessageData, C: Control<M, C>> TextBox<M, C> {
    pub fn reset_blink(&mut self) {
        self.caret_visible = true;
        self.blink_timer = 0.0;
    }

    pub fn move_caret_x(
        &mut self,
        mut offset: usize,
        direction: HorizontalDirection,
        select: bool,
    ) {
        if select {
            if self.selection_range.is_none() {
                self.selection_range = Some(SelectionRange {
                    begin: self.caret_position,
                    end: self.caret_position,
                });
            }
        } else {
            self.selection_range = None;
        }

        self.reset_blink();

        let text = self.formatted_text.borrow();
        let lines = text.get_lines();

        if lines.is_empty() {
            self.caret_position = Default::default();
            return;
        }

        while offset > 0 {
            match direction {
                HorizontalDirection::Left => {
                    if self.caret_position.offset > 0 {
                        self.caret_position.offset -= 1
                    } else if self.caret_position.line > 0 {
                        self.caret_position.line -= 1;
                        self.caret_position.offset = lines[self.caret_position.line].len();
                    } else {
                        self.caret_position.offset = 0;
                        break;
                    }
                }
                HorizontalDirection::Right => {
                    let line = lines.get(self.caret_position.line).unwrap();
                    if self.caret_position.offset < line.len() {
                        self.caret_position.offset += 1;
                    } else if self.caret_position.line < lines.len() - 1 {
                        self.caret_position.line += 1;
                        self.caret_position.offset = 0;
                    } else {
                        self.caret_position.offset = line.len();
                        break;
                    }
                }
            }
            offset -= 1;
        }

        if let Some(selection_range) = self.selection_range.as_mut() {
            if select {
                selection_range.end = self.caret_position;
            }
        }
    }

    pub fn move_caret_y(&mut self, offset: usize, direction: VerticalDirection, select: bool) {
        if select {
            if self.selection_range.is_none() {
                self.selection_range = Some(SelectionRange {
                    begin: self.caret_position,
                    end: self.caret_position,
                });
            }
        } else {
            self.selection_range = None;
        }

        let text = self.formatted_text.borrow();
        let lines = text.get_lines();

        if lines.is_empty() {
            return;
        }

        let line_count = lines.len();

        match direction {
            VerticalDirection::Down => {
                if self.caret_position.line + offset >= line_count {
                    self.caret_position.line = line_count - 1;
                } else {
                    self.caret_position.line += offset;
                }
            }
            VerticalDirection::Up => {
                if self.caret_position.line > offset {
                    self.caret_position.line -= offset;
                } else {
                    self.caret_position.line = 0;
                }
            }
        }

        if let Some(selection_range) = self.selection_range.as_mut() {
            if select {
                selection_range.end = self.caret_position;
            }
        }
    }

    pub fn get_absolute_position(&self, position: Position) -> Option<usize> {
        if let Some(line) = self.formatted_text.borrow().get_lines().get(position.line) {
            Some(line.begin + cmp::min(position.offset, line.len()))
        } else {
            None
        }
    }

    /// Inserts given character at current caret position.
    fn insert_char(&mut self, c: char, ui: &UserInterface<M, C>) {
        if !c.is_control() {
            let position = self.get_absolute_position(self.caret_position).unwrap_or(0);
            self.formatted_text
                .borrow_mut()
                .insert_char(c, position)
                .build();
            self.move_caret_x(1, HorizontalDirection::Right, false);
            ui.send_message(TextBoxMessage::text(
                self.handle,
                MessageDirection::ToWidget,
                self.formatted_text.borrow().text(),
            ));
        }
    }

    pub fn get_text_len(&self) -> usize {
        self.formatted_text.borrow_mut().get_raw_text().len()
    }

    fn remove_char(&mut self, direction: HorizontalDirection, ui: &UserInterface<M, C>) {
        if let Some(position) = self.get_absolute_position(self.caret_position) {
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

                ui.send_message(TextBoxMessage::text(
                    self.handle(),
                    MessageDirection::ToWidget,
                    self.formatted_text.borrow().text(),
                ));

                if direction == HorizontalDirection::Left {
                    self.move_caret_x(1, direction, false);
                }
            }
        }
    }

    fn remove_range(&mut self, ui: &UserInterface<M, C>, selection: SelectionRange) {
        let selection = selection.normalized();
        if let Some(begin) = self.get_absolute_position(selection.begin) {
            if let Some(end) = self.get_absolute_position(selection.end) {
                self.formatted_text.borrow_mut().remove_range(begin..end);
                self.formatted_text.borrow_mut().build();

                ui.send_message(TextBoxMessage::text(
                    self.handle(),
                    MessageDirection::ToWidget,
                    self.formatted_text.borrow().text(),
                ));

                self.caret_position = selection.begin;
            }
        }
    }

    pub fn screen_pos_to_text_pos(&self, screen_pos: Vector2<f32>) -> Option<Position> {
        let caret_pos = self.widget.screen_position;
        if let Some(font) = self.formatted_text.borrow().get_font() {
            let font = font.0.lock().unwrap();
            for (line_index, line) in self.formatted_text.borrow().get_lines().iter().enumerate() {
                let line_bounds = Rect::new(
                    caret_pos.x + line.x_offset,
                    caret_pos.y + line.y_offset,
                    line.width,
                    font.ascender(),
                );
                if line_bounds.contains(screen_pos) {
                    let mut x = line_bounds.x();
                    // Check each character in line.
                    for (offset, index) in (line.begin..line.end).enumerate() {
                        let symbol = self.formatted_text.borrow().get_raw_text()[index];
                        let (width, height, advance) = if let Some(glyph) = font.glyph(symbol) {
                            (
                                glyph.bitmap_width as f32,
                                glyph.bitmap_height as f32,
                                glyph.advance,
                            )
                        } else {
                            // Stub
                            let h = font.height();
                            (h, h, h)
                        };
                        let char_bounds = Rect::new(x, line_bounds.y(), width, height);
                        if char_bounds.contains(screen_pos) {
                            return Some(Position {
                                line: line_index,
                                offset,
                            });
                        }
                        x += advance;
                    }
                }
            }
        }
        None
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

    pub fn set_font(&mut self, font: SharedFont) -> &mut Self {
        self.formatted_text.borrow_mut().set_font(font);
        self
    }

    pub fn font(&self) -> SharedFont {
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

impl<M: MessageData, C: Control<M, C>> Control<M, C> for TextBox<M, C> {
    fn measure_override(
        &self,
        _: &UserInterface<M, C>,
        available_size: Vector2<f32>,
    ) -> Vector2<f32> {
        self.formatted_text
            .borrow_mut()
            .set_constraint(available_size)
            .build()
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        let bounds = self.widget.screen_bounds();
        drawing_context.push_rect_filled(&bounds, None);
        drawing_context.commit(
            self.clip_bounds(),
            self.widget.background(),
            CommandTexture::None,
            None,
        );

        self.formatted_text
            .borrow_mut()
            .set_constraint(Vector2::new(bounds.w(), bounds.h()))
            .set_brush(self.widget.foreground())
            .build();

        if let Some(ref selection_range) = self.selection_range.map(|r| r.normalized()) {
            let text = self.formatted_text.borrow();
            let lines = text.get_lines();
            if selection_range.begin.line == selection_range.end.line {
                let line = lines[selection_range.begin.line];
                // Begin line
                let offset =
                    text.get_range_width(line.begin..(line.begin + selection_range.begin.offset));
                let width = text.get_range_width(
                    (line.begin + selection_range.begin.offset)
                        ..(line.begin + selection_range.end.offset),
                );
                let bounds = Rect::new(
                    bounds.x() + line.x_offset + offset,
                    bounds.y() + line.y_offset,
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
                                bounds.x() + line.x_offset + offset,
                                bounds.y() + line.y_offset,
                                width,
                                line.height,
                            )
                        } else if i == selection_range.end.line {
                            // End line
                            let width = text.get_range_width(
                                line.begin..(line.begin + selection_range.end.offset),
                            );
                            Rect::new(
                                bounds.x() + line.x_offset,
                                bounds.y() + line.y_offset,
                                width,
                                line.height,
                            )
                        } else {
                            // Everything between
                            Rect::new(
                                bounds.x() + line.x_offset,
                                bounds.y() + line.y_offset,
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
            self.clip_bounds(),
            self.selection_brush.clone(),
            CommandTexture::None,
            None,
        );

        let screen_position = bounds.position;
        drawing_context.draw_text(bounds, screen_position, &self.formatted_text.borrow());

        if self.caret_visible {
            let text = self.formatted_text.borrow();

            if let Some(font) = text.get_font() {
                let mut caret_pos = screen_position;

                let font = font.0.lock().unwrap();
                if let Some(line) = text.get_lines().get(self.caret_position.line) {
                    let text = text.get_raw_text();
                    caret_pos += Vector2::new(line.x_offset, line.y_offset);
                    for (offset, char_index) in (line.begin..line.end).enumerate() {
                        if offset >= self.caret_position.offset {
                            break;
                        }
                        if let Some(glyph) = font.glyph(text[char_index]) {
                            caret_pos.x += glyph.advance;
                        } else {
                            caret_pos.x += font.height();
                        }
                    }
                }

                let caret_bounds = Rect::new(caret_pos.x, caret_pos.y, 2.0, font.height());
                drawing_context.push_rect_filled(&caret_bounds, None);
                drawing_context.commit(
                    self.clip_bounds(),
                    self.caret_brush.clone(),
                    CommandTexture::None,
                    None,
                );
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

        if message.destination() == self.handle() {
            match &message.data() {
                UiMessageData::Widget(msg) => match msg {
                    &WidgetMessage::Text(symbol)
                        if !ui.keyboard_modifiers().control && !ui.keyboard_modifiers().alt =>
                    {
                        let insert = if let Some(filter) = self.filter.as_ref() {
                            let filter = &mut *filter.borrow_mut();
                            filter(symbol)
                        } else {
                            true
                        };
                        if insert {
                            if let Some(range) = self.selection_range {
                                self.remove_range(ui, range);
                                self.selection_range = None;
                            }
                            self.insert_char(symbol, ui);
                        }
                    }
                    WidgetMessage::KeyDown(code) => match code {
                        KeyCode::Up => {
                            self.move_caret_y(
                                1,
                                VerticalDirection::Up,
                                ui.keyboard_modifiers().shift,
                            );
                        }
                        KeyCode::Down => {
                            self.move_caret_y(
                                1,
                                VerticalDirection::Down,
                                ui.keyboard_modifiers().shift,
                            );
                        }
                        KeyCode::Right => {
                            self.move_caret_x(
                                1,
                                HorizontalDirection::Right,
                                ui.keyboard_modifiers().shift,
                            );
                        }
                        KeyCode::Left => {
                            self.move_caret_x(
                                1,
                                HorizontalDirection::Left,
                                ui.keyboard_modifiers().shift,
                            );
                        }
                        KeyCode::Delete => {
                            if let Some(range) = self.selection_range {
                                self.remove_range(ui, range);
                                self.selection_range = None;
                            } else {
                                self.remove_char(HorizontalDirection::Right, ui);
                            }
                        }
                        KeyCode::NumpadEnter | KeyCode::Return => {
                            if self.multiline {
                                self.insert_char('\n', ui);
                            } else if self.commit_mode == TextCommitMode::LostFocusPlusEnter {
                                ui.send_message(TextBoxMessage::text(
                                    self.handle,
                                    MessageDirection::FromWidget,
                                    self.text(),
                                ));
                                self.has_focus = false;
                            }
                        }
                        KeyCode::Backspace => {
                            if let Some(range) = self.selection_range {
                                self.remove_range(ui, range);
                                self.selection_range = None;
                            } else {
                                self.remove_char(HorizontalDirection::Left, ui);
                            }
                        }
                        KeyCode::End => {
                            let text = self.formatted_text.borrow();
                            if ui.keyboard_modifiers().control {
                                let line = &text.get_lines()[self.caret_position.line];
                                self.caret_position.line = text.get_lines().len() - 1;
                                self.caret_position.offset = line.end - line.begin;
                                self.selection_range = None;
                            } else if ui.keyboard_modifiers().shift {
                                let line = &text.get_lines()[self.caret_position.line];
                                let prev_position = self.caret_position;
                                self.caret_position.offset = line.end - line.begin;
                                self.selection_range = Some(SelectionRange {
                                    begin: prev_position,
                                    end: Position {
                                        line: self.caret_position.line,
                                        offset: self.caret_position.offset - 1,
                                    },
                                });
                            } else {
                                let line = &text.get_lines()[self.caret_position.line];
                                self.caret_position.offset = line.end - line.begin;
                                self.selection_range = None;
                            }
                        }
                        KeyCode::Home => {
                            if ui.keyboard_modifiers().control {
                                self.caret_position.line = 0;
                                self.caret_position.offset = 0;
                                self.selection_range = None;
                            } else if ui.keyboard_modifiers().shift {
                                let prev_position = self.caret_position;
                                self.caret_position.line = 0;
                                self.caret_position.offset = 0;
                                self.selection_range = Some(SelectionRange {
                                    begin: self.caret_position,
                                    end: Position {
                                        line: prev_position.line,
                                        offset: prev_position.offset.saturating_sub(1),
                                    },
                                });
                            } else {
                                self.caret_position.offset = 0;
                                self.selection_range = None;
                            }
                        }
                        KeyCode::A if ui.keyboard_modifiers().control => {
                            let text = self.formatted_text.borrow();
                            if let Some(last_line) = &text.get_lines().last() {
                                self.selection_range = Some(SelectionRange {
                                    begin: Position { line: 0, offset: 0 },
                                    end: Position {
                                        line: text.get_lines().len() - 1,
                                        offset: last_line.end - last_line.begin,
                                    },
                                });
                            }
                        }
                        _ => (),
                    },
                    WidgetMessage::GotFocus => {
                        self.reset_blink();
                        self.selection_range = None;
                        self.has_focus = true;
                    }
                    WidgetMessage::LostFocus => {
                        self.selection_range = None;
                        self.has_focus = false;

                        if self.commit_mode == TextCommitMode::LostFocus
                            || self.commit_mode == TextCommitMode::LostFocusPlusEnter
                        {
                            ui.send_message(TextBoxMessage::text(
                                self.handle,
                                MessageDirection::FromWidget,
                                self.text(),
                            ));
                        }
                    }
                    WidgetMessage::MouseDown { pos, button } => {
                        if *button == MouseButton::Left {
                            self.selection_range = None;
                            self.selecting = true;
                            self.has_focus = true;

                            if let Some(position) = self.screen_pos_to_text_pos(*pos) {
                                self.caret_position = position;

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
                                    if position.offset > sel_range.begin.offset {
                                        sel_range.end = Position {
                                            line: position.line,
                                            offset: position.offset + 1,
                                        };
                                    } else {
                                        sel_range.end = position;
                                    }
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
                UiMessageData::TextBox(TextBoxMessage::Text(new_text))
                    if message.direction() == MessageDirection::ToWidget =>
                {
                    let mut equals = false;
                    for (&new, old) in self
                        .formatted_text
                        .borrow()
                        .get_raw_text()
                        .iter()
                        .zip(new_text.chars())
                    {
                        if old as u32 != new {
                            equals = false;
                            break;
                        }
                    }
                    if !equals {
                        self.formatted_text.borrow_mut().set_text(new_text);
                        self.invalidate_layout();

                        if self.commit_mode == TextCommitMode::Immediate {
                            ui.send_message(message.reverse());
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

pub struct TextBoxBuilder<M: MessageData, C: Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    font: Option<SharedFont>,
    text: String,
    caret_brush: Brush,
    selection_brush: Brush,
    filter: Option<Rc<RefCell<FilterCallback>>>,
    vertical_alignment: VerticalAlignment,
    horizontal_alignment: HorizontalAlignment,
    wrap: bool,
    commit_mode: TextCommitMode,
    multiline: bool,
}

impl<M: MessageData, C: Control<M, C>> TextBoxBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            font: None,
            text: "".to_owned(),
            caret_brush: Brush::Solid(Color::WHITE),
            selection_brush: Brush::Solid(Color::opaque(80, 118, 178)),
            filter: None,
            vertical_alignment: VerticalAlignment::Top,
            horizontal_alignment: HorizontalAlignment::Left,
            wrap: false,
            commit_mode: TextCommitMode::LostFocusPlusEnter,
            multiline: false,
        }
    }

    pub fn with_font(mut self, font: SharedFont) -> Self {
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

    pub fn with_text_commit_mode(mut self, mode: TextCommitMode) -> Self {
        self.commit_mode = mode;
        self
    }

    pub fn with_multiline(mut self, multiline: bool) -> Self {
        self.multiline = multiline;
        self
    }

    pub fn build(mut self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        if self.widget_builder.foreground.is_none() {
            self.widget_builder.foreground = Some(BRUSH_TEXT);
        }
        if self.widget_builder.background.is_none() {
            self.widget_builder.background = Some(BRUSH_DARKER);
        }
        if self.widget_builder.cursor.is_none() {
            self.widget_builder.cursor = Some(CursorIcon::Text);
        }

        let text_box = TextBox {
            widget: self.widget_builder.build(),
            caret_position: Position::default(),
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
            commit_mode: self.commit_mode,
            multiline: self.multiline,
        };

        ctx.add_node(UINode::TextBox(text_box))
    }
}
