//! TextBox is a text widget that allows you to edit text and create specialized input fields. See [`TextBox`] docs for more
//! info and usage examples.

#![warn(missing_docs)]

use crate::{
    brush::Brush,
    core::{
        algebra::{Point2, Vector2},
        color::Color,
        math::Rect,
        parking_lot::Mutex,
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        uuid_provider,
        variable::InheritableVariable,
        visitor::prelude::*,
    },
    define_constructor,
    draw::{CommandTexture, Draw, DrawingContext},
    font::FontResource,
    formatted_text::{FormattedText, FormattedTextBuilder, WrapMode},
    message::{CursorIcon, KeyCode, MessageDirection, MouseButton, UiMessage},
    text::TextMessage,
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, HorizontalAlignment, UiNode, UserInterface, VerticalAlignment,
    BRUSH_DARKER, BRUSH_TEXT,
};
use copypasta::ClipboardProvider;
use std::{
    cell::RefCell,
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
    sync::Arc,
};
use strum_macros::{AsRefStr, EnumString, VariantNames};

/// A message that could be used to alternate text box widget's state or receive changes from it.
///
/// # Important notes
///
/// Text box widget also supports [`TextMessage`] and [`WidgetMessage`].
#[derive(Debug, Clone, PartialEq)]
pub enum TextBoxMessage {
    /// Used to change selection brush of a text box. Use [TextBoxMessage::selection_brush`] to create the message.
    SelectionBrush(Brush),
    /// Used to change caret brush of a text box. Use [TextBoxMessage::caret_brush`] to create the message.
    CaretBrush(Brush),
    /// Used to change text commit mode of a text box. Use [TextBoxMessage::text_commit_mode`] to create the message.
    TextCommitMode(TextCommitMode),
    /// Used to enable or disable multiline mode of a text box. Use [TextBoxMessage::multiline`] to create the message.
    Multiline(bool),
    /// Used to enable or disable an ability to edit text box content. Use [TextBoxMessage::editable`] to create the message.
    Editable(bool),
}

impl TextBoxMessage {
    define_constructor!(
        /// Creates [`TextBoxMessage::SelectionBrush`].
        TextBoxMessage:SelectionBrush => fn selection_brush(Brush), layout: false
    );
    define_constructor!(
        /// Creates [`TextBoxMessage::CaretBrush`].
        TextBoxMessage:CaretBrush => fn caret_brush(Brush), layout: false
    );
    define_constructor!(
        /// Creates [`TextBoxMessage::TextCommitMode`].
        TextBoxMessage:TextCommitMode => fn text_commit_mode(TextCommitMode), layout: false
    );
    define_constructor!(
        /// Creates [`TextBoxMessage::Multiline`].
        TextBoxMessage:Multiline => fn multiline(bool), layout: false
    );
    define_constructor!(
        /// Creates [`TextBoxMessage::Editable`].
        TextBoxMessage:Editable => fn editable(bool), layout: false
    );
}

/// Specifies a direction on horizontal axis.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum HorizontalDirection {
    /// Left direction.
    Left,
    /// Right direction.
    Right,
}

/// Specifies a direction on vertical axis.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum VerticalDirection {
    /// Down direction.
    Down,
    /// Up direction.
    Up,
}

pub use crate::formatted_text::Position;

/// Defines the way, how the text box widget will commit the text that was typed in
#[derive(
    Copy,
    Clone,
    PartialOrd,
    PartialEq,
    Eq,
    Ord,
    Hash,
    Debug,
    Default,
    Visit,
    Reflect,
    AsRefStr,
    EnumString,
    VariantNames,
    TypeUuidProvider,
)]
#[repr(u32)]
#[type_uuid(id = "5fb7d6f0-c151-4a30-8350-2060749d74c6")]
pub enum TextCommitMode {
    /// Text box will immediately send [`TextMessage::Text`] message after any change (after any pressed button).
    Immediate = 0,

    /// Text box will send Text message only when it loses focus (when a user "clicks" outside of it or with any other
    /// event that forces the text box to lose focus).
    LostFocus = 1,

    /// Text box will send Text message when it loses focus or if Enter key was pressed. This is **default** behavior.
    ///
    /// # Notes
    ///
    /// In case of multiline text box hitting Enter key won't commit the text!
    #[default]
    LostFocusPlusEnter = 2,

    /// Text box will send Text message when it loses focus or if Enter key was pressed, but **only** if the content
    /// of the text box changed since the last time it gained focus or the text was committed.
    ///
    /// # Notes
    ///
    /// In case of multiline text box hitting Enter key won't commit the text!
    Changed = 3,
}

/// Defines a set of two positions in the text, that forms a specific range.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Visit, Reflect, Default, TypeUuidProvider)]
#[type_uuid(id = "04c8101b-cb34-47a5-af34-ecfb9b2fc426")]
pub struct SelectionRange {
    /// Position of the beginning.
    pub begin: Position,
    /// Position of the end.
    pub end: Position,
}

impl SelectionRange {
    /// Creates a new range, that have its begin always before end. It could be useful in case if user
    /// selects a range right-to-left.
    #[must_use = "method creates new value which must be used"]
    pub fn normalized(&self) -> SelectionRange {
        SelectionRange {
            begin: self.left(),
            end: self.right(),
        }
    }
    /// Creates a Range iterator of the positions in this range from left to right, excluding the rightmost position.
    pub fn range(&self) -> std::ops::Range<Position> {
        std::ops::Range {
            start: self.left(),
            end: self.right(),
        }
    }
    /// `true` if the given position is inside this range, including the beginning and end.
    pub fn contains(&self, position: Position) -> bool {
        (self.begin..=self.end).contains(&position)
    }
    /// The leftmost position.
    pub fn left(&self) -> Position {
        Position::min(self.begin, self.end)
    }
    /// The rightmost position.
    pub fn right(&self) -> Position {
        Position::max(self.begin, self.end)
    }
}

/// Defines a function, that could be used to filter out desired characters. It must return `true` for characters, that pass
/// the filter, and `false` - otherwise.
pub type FilterCallback = dyn FnMut(char) -> bool + Send;

/// TextBox is a text widget that allows you to edit text and create specialized input fields. It has various options like
/// word wrapping, text alignment, and so on.
///
/// ## How to create
///
/// An instance of the TextBox widget could be created like so:
///
/// ```rust,no_run
/// # use fyrox_ui::{
/// #     core::pool::Handle,
/// #     text_box::TextBoxBuilder, widget::WidgetBuilder, UiNode, UserInterface
/// # };
/// fn create_text_box(ui: &mut UserInterface, text: &str) -> Handle<UiNode> {
///     TextBoxBuilder::new(WidgetBuilder::new())
///         .with_text(text)
///         .build(&mut ui.build_ctx())
/// }
/// ```
///
/// ## Text alignment and word wrapping
///
/// There are various text alignment options for both vertical and horizontal axes. Typical alignment values are:
/// [`HorizontalAlignment::Left`], [`HorizontalAlignment::Center`], [`HorizontalAlignment::Right`] for horizontal axis,
/// and [`VerticalAlignment::Top`], [`VerticalAlignment::Center`], [`VerticalAlignment::Bottom`] for vertical axis.
/// An instance of centered text could be created like so:
///
/// ```rust,no_run
/// # use fyrox_ui::{
/// #     core::pool::Handle,
/// #     text_box::TextBoxBuilder, widget::WidgetBuilder, HorizontalAlignment, UiNode, UserInterface,
/// #     VerticalAlignment,
/// # };
/// fn create_centered_text(ui: &mut UserInterface, text: &str) -> Handle<UiNode> {
///     TextBoxBuilder::new(WidgetBuilder::new())
///         .with_horizontal_text_alignment(HorizontalAlignment::Center)
///         .with_vertical_text_alignment(VerticalAlignment::Center)
///     .with_text(text)
///     .build(&mut ui.build_ctx())
/// }
/// ```
///
/// Long text is usually needs to wrap on available bounds, there are three possible options for word wrapping:
/// [`WrapMode::NoWrap`], [`WrapMode::Letter`], [`WrapMode::Word`]. An instance of text with word-based wrapping could be
/// created like so:
///
/// ```rust,no_run
/// # use fyrox_ui::{
/// #     core::pool::Handle,
/// #     formatted_text::WrapMode, text_box::TextBoxBuilder, widget::WidgetBuilder, UiNode,
/// #     UserInterface,
/// # };
/// fn create_text_with_word_wrap(ui: &mut UserInterface, text: &str) -> Handle<UiNode> {
///     TextBoxBuilder::new(WidgetBuilder::new())
///         .with_wrap(WrapMode::Word)
///         .with_text(text)
///         .build(&mut ui.build_ctx())
/// }
/// ```
///
/// ## Fonts and colors
///
/// To set a color of the text just use [`WidgetBuilder::with_foreground`] while building the text instance:
///
/// ```rust,no_run
/// # use fyrox_ui::{
/// #     core::{color::Color, pool::Handle},
/// #     brush::Brush, text_box::TextBoxBuilder, widget::WidgetBuilder, UiNode, UserInterface
/// # };
/// fn create_text(ui: &mut UserInterface, text: &str) -> Handle<UiNode> {
///     //                  vvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvv
///     TextBoxBuilder::new(WidgetBuilder::new().with_foreground(Brush::Solid(Color::RED)))
///         .with_text(text)
///         .build(&mut ui.build_ctx())
/// }
/// ```
///
/// By default, text is created with default font, however it is possible to set any custom font:
///
/// ```rust,no_run
/// # use fyrox_resource::manager::ResourceManager;
/// # use fyrox_ui::{
/// #     core::{futures::executor::block_on, pool::Handle},
/// #     text_box::TextBoxBuilder,
/// #     font::{Font},
/// #     widget::WidgetBuilder,
/// #     UiNode, UserInterface,
/// # };
///
/// fn create_text(ui: &mut UserInterface, resource_manager: &ResourceManager, text: &str) -> Handle<UiNode> {
///     TextBoxBuilder::new(WidgetBuilder::new())
///         .with_font(resource_manager.request::<Font>("path/to/your/font.ttf"))
///         .with_text(text)
///         .with_font_size(20.0)
///         .build(&mut ui.build_ctx())
/// }
/// ```
///
/// Please refer to [`FontResource`] to learn more about fonts.
///
/// ### Font size
///
/// Use [`TextBoxBuilder::with_font_size`] or send [`TextMessage::font_size`] to your TextBox widget instance
/// to set the font size of it.
///
/// ## Messages
///
/// TextBox widget accepts the following list of messages:
///
/// - [`TextBoxMessage::SelectionBrush`] - change the brush that is used to highlight selection.
/// - [`TextBoxMessage::CaretBrush`] - changes the brush of the caret (small blinking vertical line).
/// - [`TextBoxMessage::TextCommitMode`] - changes the [text commit mode](TextBox#text-commit-mode).
/// - [`TextBoxMessage::Multiline`] - makes the TextBox either multiline (`true`) or single line (`false`)
/// - [`TextBoxMessage::Editable`] - enables or disables editing of the text.
///
/// **Important:** Please keep in mind, that TextBox widget also accepts [`TextMessage`]s. An example of changing text at
/// runtime could be something like this:
///
/// ```rust,no_run
/// # use fyrox_ui::{
/// #     core::pool::Handle,
/// #     message::{MessageDirection},
/// #     UiNode, UserInterface,
/// #     text::TextMessage
/// # };
/// fn request_change_text(ui: &UserInterface, text_box_widget_handle: Handle<UiNode>, text: &str) {
///     ui.send_message(TextMessage::text(
///         text_box_widget_handle,
///         MessageDirection::ToWidget,
///         text.to_owned(),
///     ))
/// }
/// ```
///
/// Please keep in mind, that like any other situation when you "changing" something via messages, you should remember
/// that the change is **not** immediate. The change will be applied on `ui.poll_message(..)` call somewhere in your
/// code.
///
/// ## Shortcuts
///
/// There are number of default shortcuts that can be used to speed up text editing:
///
/// - `Ctrl+A` - select all
/// - `Ctrl+C` - copy selected text
/// - `Ctrl+V` - paste text from clipboard
/// - `Ctrl+Home` - move caret to the beginning of the text
/// - `Ctrl+End` - move caret to the beginning of the text
/// - `Shift+Home` - select everything from current caret position until the beginning of current line
/// - `Shift+End` - select everything from current caret position until the end of current line
/// - `Arrows` - move caret accordingly
/// - `Delete` - deletes next character
/// - `Backspace` - deletes previous character
/// - `Enter` - new line (if multiline mode is set) or `commit` message
///
/// ## Multiline Text Box
///
/// By default, text box will not add new line character to the text if you press `Enter` on keyboard. To enable this
/// functionality use [`TextBoxBuilder::with_multiline`]
///
/// ## Read-only Mode
///
/// You can enable or disable content editing by using read-only mode. Use [`TextBoxBuilder::with_editable`] at build stage.
///
/// ## Mask Character
///
/// You can specify replacement character for every other characters, this is useful option for password fields. Use
/// [`TextBoxBuilder::with_mask_char`] at build stage. For example, you can set replacement character to asterisk `*` using
/// `.with_mask_char(Some('*'))`
///
/// ## Text Commit Mode
///
/// In many situations you don't need the text box to send `new text` message every new character, you either want this
/// message if `Enter` key is pressed or TextBox has lost keyboard focus (or both). There is [`TextBoxBuilder::with_text_commit_mode`]
/// on builder specifically for that purpose. Use one of the following modes:
///
/// - [`TextCommitMode::Immediate`] - text box will immediately send [`TextMessage::Text`] message after any change.
/// - [`TextCommitMode::LostFocus`] - text box will send [`TextMessage::Text`] message only when it loses focus.
/// - [`TextCommitMode::LostFocusPlusEnter`] - text box will send [`TextMessage::Text`] message when it loses focus or if Enter
/// key was pressed. This is **default** behavior. In case of multiline text box hitting Enter key won't commit text!
///
/// ## Filtering
///
/// It is possible specify custom input filter, it can be useful if you're creating special input fields like numerical or
/// phone number. A filter can be specified at build stage like so:
///
/// ```rust,no_run
/// # use fyrox_ui::{
/// #     core::pool::Handle,
/// #     text_box::TextBoxBuilder, widget::WidgetBuilder, UiNode, UserInterface
/// # };
/// # use std::sync::Arc;
/// # use fyrox_core::parking_lot::Mutex;
/// fn create_text_box(ui: &mut UserInterface) -> Handle<UiNode> {
///     TextBoxBuilder::new(WidgetBuilder::new())
///         // Specify a filter that will pass only digits.
///         .with_filter(Arc::new(Mutex::new(|c: char| c.is_ascii_digit())))
///         .build(&mut ui.build_ctx())
/// }
/// ```
///
/// ## Style
///
/// You can change brush of caret by using [`TextBoxBuilder::with_caret_brush`] and also selection brush by using
/// [`TextBoxBuilder::with_selection_brush`], it could be useful if you don't like default colors.
#[derive(Default, Clone, Visit, Reflect, ComponentProvider)]
pub struct TextBox {
    /// Base widget of the text box.
    pub widget: Widget,
    /// Current position of the caret in the text box.
    pub caret_position: InheritableVariable<Position>,
    /// Whether the caret is visible or not.
    pub caret_visible: InheritableVariable<bool>,
    /// Internal blinking timer.
    pub blink_timer: InheritableVariable<f32>,
    /// Blinking interval in seconds.
    pub blink_interval: InheritableVariable<f32>,
    /// Formatted text that stores actual text and performs its layout. See [`FormattedText`] docs for more info.
    pub formatted_text: RefCell<FormattedText>,
    /// Current selection range.
    pub selection_range: InheritableVariable<Option<SelectionRange>>,
    /// `true` if the text box is in selection mode.
    pub selecting: bool,
    /// Stores the location of the caret before it was moved by mouse click.
    #[visit(skip)]
    pub before_click_position: Position,
    /// `true` if the text box is focused.
    pub has_focus: bool,
    /// Current caret brush of the text box.
    pub caret_brush: InheritableVariable<Brush>,
    /// Current selection brush of the text box.
    pub selection_brush: InheritableVariable<Brush>,
    /// Current character filter of the text box.
    #[visit(skip)]
    #[reflect(hidden)]
    pub filter: Option<Arc<Mutex<FilterCallback>>>,
    /// Current text commit mode of the text box.
    pub commit_mode: InheritableVariable<TextCommitMode>,
    /// `true` if the the multiline mode is active.
    pub multiline: InheritableVariable<bool>,
    /// `true` if the text box is editable.
    pub editable: InheritableVariable<bool>,
    /// Position of the local "camera" (viewing rectangle) of the text box.
    pub view_position: InheritableVariable<Vector2<f32>>,
    /// A list of custom characters that will be treated as whitespace.
    pub skip_chars: InheritableVariable<Vec<char>>,
    /// Stored copy of most recent commit, when `commit_mode` is [TextCommitMode::Changed].
    #[visit(skip)]
    #[reflect(hidden)]
    pub recent: Vec<char>,
}

impl Debug for TextBox {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("TextBox")
    }
}

crate::define_widget_deref!(TextBox);

impl TextBox {
    fn commit_if_changed(&mut self, ui: &mut UserInterface) {
        let formatted_text = self.formatted_text.borrow();
        let raw = formatted_text.get_raw_text();
        if self.recent != raw {
            self.recent.clear();
            self.recent.extend(raw);
            ui.send_message(TextMessage::text(
                self.handle,
                MessageDirection::FromWidget,
                formatted_text.text(),
            ));
        }
    }
    fn filter_paste_str_multiline(&self, str: &str) -> String {
        let mut str = str.replace("\r\n", "\n");
        str.retain(|c| c == '\n' || !c.is_control());
        if let Some(filter) = self.filter.as_ref() {
            let filter = &mut *filter.lock();
            str.retain(filter);
        }
        str
    }

    fn filter_paste_str_single_line(&self, str: &str) -> String {
        let mut str: String = str
            .chars()
            .map(|c| if c == '\n' { ' ' } else { c })
            .filter(|c| !c.is_control())
            .collect();
        if let Some(filter) = self.filter.as_ref() {
            let filter = &mut *filter.lock();
            str.retain(filter);
        }
        str
    }

    fn reset_blink(&mut self) {
        self.caret_visible.set_value_and_mark_modified(true);
        self.blink_timer.set_value_and_mark_modified(0.0);
    }

    fn move_caret(&mut self, position: Position, select: bool) {
        let text = self.formatted_text.borrow();
        let lines = text.get_lines();
        if select && !lines.is_empty() {
            if self.selection_range.is_none() {
                self.selection_range
                    .set_value_and_mark_modified(Some(SelectionRange {
                        begin: *self.caret_position,
                        end: *self.caret_position,
                    }));
            }
        } else {
            self.selection_range.set_value_and_mark_modified(None);
        }

        if lines.is_empty() {
            drop(text);
            self.set_caret_position(Default::default());
            return;
        }

        drop(text);

        if let Some(selection_range) = self.selection_range.as_mut() {
            if select {
                if *self.caret_position != selection_range.end {
                    if !selection_range.contains(position) {
                        if position < selection_range.left() {
                            selection_range.begin = selection_range.right();
                        } else {
                            selection_range.begin = selection_range.left();
                        }
                        selection_range.end = position;
                    }
                } else {
                    selection_range.end = position;
                }
            }
        }

        self.set_caret_position(position);
        self.ensure_caret_visible();
    }

    fn move_caret_x(&mut self, offset: isize, select: bool) {
        let pos = self
            .formatted_text
            .borrow()
            .get_relative_position_x(*self.caret_position, offset);
        self.move_caret(pos, select);
    }

    fn move_caret_y(&mut self, offset: isize, select: bool) {
        let pos = self
            .formatted_text
            .borrow()
            .get_relative_position_y(*self.caret_position, offset);
        self.move_caret(pos, select);
    }

    /// Maps input [`Position`] to a linear position in character array.
    /// The index returned is the index of the character after the position, which may be
    /// out-of-bounds if thee position is at the end of the text.
    /// You should check the index before trying to use it to fetch data from inner array of characters.
    pub fn position_to_char_index_unclamped(&self, position: Position) -> Option<usize> {
        self.formatted_text
            .borrow()
            .position_to_char_index_unclamped(position)
    }

    /// Maps input [`Position`] to a linear position in character array.
    /// The index returned is usually the index of the character after the position,
    /// but if the position is at the end of a line then return the index of the character _before_ the position.
    /// In other words, the last two positions of each line are mapped to the same character index.
    /// Output index will always be valid for fetching, if the method returned `Some(index)`.
    /// The index however cannot be used for text insertion, because it cannot point to a "place after last char".
    pub fn position_to_char_index_clamped(&self, position: Position) -> Option<usize> {
        self.formatted_text
            .borrow()
            .position_to_char_index_clamped(position)
    }

    /// Maps linear character index (as in string) to its actual location in the text.
    pub fn char_index_to_position(&self, i: usize) -> Option<Position> {
        self.formatted_text.borrow().char_index_to_position(i)
    }

    /// Returns end position of the text.
    pub fn end_position(&self) -> Position {
        self.formatted_text.borrow().end_position()
    }

    /// Returns a position of a next word after the caret in the text.
    pub fn find_next_word(&self, from: Position) -> Position {
        self.position_to_char_index_unclamped(from)
            .and_then(|i| {
                self.formatted_text
                    .borrow()
                    .get_raw_text()
                    .iter()
                    .enumerate()
                    .skip(i)
                    .skip_while(|(_, c)| !(c.is_whitespace() || self.skip_chars.contains(*c)))
                    .find(|(_, c)| !(c.is_whitespace() || self.skip_chars.contains(*c)))
                    .and_then(|(n, _)| self.char_index_to_position(n))
            })
            .unwrap_or_else(|| self.end_position())
    }

    /// Returns a position of a next word before the caret in the text.
    pub fn find_prev_word(&self, from: Position) -> Position {
        self.position_to_char_index_unclamped(from)
            .and_then(|i| {
                let text = self.formatted_text.borrow();
                let len = text.get_raw_text().len();
                text.get_raw_text()
                    .iter()
                    .enumerate()
                    .rev()
                    .skip(len.saturating_sub(i))
                    .skip_while(|(_, c)| !(c.is_whitespace() || self.skip_chars.contains(*c)))
                    .find(|(_, c)| !(c.is_whitespace() || self.skip_chars.contains(*c)))
                    .and_then(|(n, _)| self.char_index_to_position(n + 1))
            })
            .unwrap_or_default()
    }

    /// Inserts given character at current caret position.
    fn insert_char(&mut self, c: char, ui: &UserInterface) {
        self.remove_before_insert();
        let position = self
            .position_to_char_index_unclamped(*self.caret_position)
            .unwrap_or_default();
        self.formatted_text
            .borrow_mut()
            .insert_char(c, position)
            .build();
        self.set_caret_position(
            self.char_index_to_position(position + 1)
                .unwrap_or_default(),
        );
        if *self.commit_mode == TextCommitMode::Immediate {
            ui.send_message(TextMessage::text(
                self.handle,
                MessageDirection::FromWidget,
                self.formatted_text.borrow().text(),
            ));
        }
    }

    fn insert_str(&mut self, str: &str, ui: &UserInterface) {
        if str.is_empty() {
            return;
        }
        let str: String = if *self.multiline {
            self.filter_paste_str_multiline(str)
        } else {
            self.filter_paste_str_single_line(str)
        };
        self.remove_before_insert();
        let position = self
            .position_to_char_index_unclamped(*self.caret_position)
            .unwrap_or_default();
        let mut text = self.formatted_text.borrow_mut();
        text.insert_str(&str, position);
        text.build();
        drop(text);
        self.set_caret_position(
            self.char_index_to_position(position + str.chars().count())
                .unwrap_or_default(),
        );
        if *self.commit_mode == TextCommitMode::Immediate {
            ui.send_message(TextMessage::text(
                self.handle,
                MessageDirection::FromWidget,
                self.formatted_text.borrow().text(),
            ));
        }
    }

    fn remove_before_insert(&mut self) {
        let Some(selection) = *self.selection_range else {
            return;
        };
        let range = self
            .formatted_text
            .borrow()
            .position_range_to_char_index_range(selection.range());
        if range.is_empty() {
            return;
        }
        self.formatted_text.borrow_mut().remove_range(range);
        self.selection_range.set_value_and_mark_modified(None);
        self.set_caret_position(selection.left());
    }

    /// Returns current text length in characters.
    pub fn get_text_len(&self) -> usize {
        self.formatted_text.borrow_mut().get_raw_text().len()
    }

    /// Returns current position the caret in the local coordinates.
    pub fn caret_local_position(&self) -> Vector2<f32> {
        self.formatted_text
            .borrow_mut()
            .position_to_local(*self.caret_position)
    }

    fn point_to_view_pos(&self, position: Vector2<f32>) -> Vector2<f32> {
        position - *self.view_position
    }

    fn rect_to_view_pos(&self, mut rect: Rect<f32>) -> Rect<f32> {
        rect.position -= *self.view_position;
        rect
    }

    fn ensure_caret_visible(&mut self) {
        let local_bounds = self.bounding_rect();
        let caret_view_position = self.point_to_view_pos(self.caret_local_position());
        // Move view position to contain the caret + add some spacing.
        let spacing_step = self
            .formatted_text
            .borrow()
            .get_font()
            .state()
            .data()
            .map(|font| font.ascender(*self.height))
            .unwrap_or_default();
        let spacing = spacing_step * 3.0;
        let top_left_corner = local_bounds.left_top_corner();
        let bottom_right_corner = local_bounds.right_bottom_corner();
        if caret_view_position.x > bottom_right_corner.x {
            self.view_position.x += caret_view_position.x - bottom_right_corner.x + spacing;
        }
        if caret_view_position.x < top_left_corner.x {
            self.view_position.x -= top_left_corner.x - caret_view_position.x + spacing;
        }
        if caret_view_position.y > bottom_right_corner.y {
            self.view_position.y += bottom_right_corner.y - caret_view_position.y + spacing;
        }
        if caret_view_position.y < top_left_corner.y {
            self.view_position.y -= top_left_corner.y - caret_view_position.y + spacing;
        }
        self.view_position.x = self.view_position.x.max(0.0);
        self.view_position.y = self.view_position.y.max(0.0);
    }

    fn remove_char(&mut self, direction: HorizontalDirection, ui: &UserInterface) {
        if let Some(selection) = *self.selection_range {
            self.remove_range(ui, selection);
            return;
        }
        let Some(position) = self.position_to_char_index_unclamped(*self.caret_position) else {
            return;
        };

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

            let mut text = self.formatted_text.borrow_mut();
            text.remove_at(position);
            text.build();
            drop(text);

            if *self.commit_mode == TextCommitMode::Immediate {
                ui.send_message(TextMessage::text(
                    self.handle(),
                    MessageDirection::FromWidget,
                    self.formatted_text.borrow().text(),
                ));
            }

            self.set_caret_position(self.char_index_to_position(position).unwrap_or_default());
        }
    }

    fn remove_range(&mut self, ui: &UserInterface, selection: SelectionRange) {
        let range = self
            .formatted_text
            .borrow()
            .position_range_to_char_index_range(selection.range());
        if range.is_empty() {
            return;
        }
        self.formatted_text.borrow_mut().remove_range(range);
        self.formatted_text.borrow_mut().build();
        self.set_caret_position(selection.left());
        self.selection_range.set_value_and_mark_modified(None);
        if *self.commit_mode == TextCommitMode::Immediate {
            ui.send_message(TextMessage::text(
                self.handle(),
                MessageDirection::FromWidget,
                self.formatted_text.borrow().text(),
            ));
        }
    }

    /// Checks whether the input position is correct (in bounds) or not.
    pub fn is_valid_position(&self, position: Position) -> bool {
        self.formatted_text
            .borrow()
            .get_lines()
            .get(position.line)
            .map_or(false, |line| position.offset < line.len())
    }

    fn set_caret_position(&mut self, position: Position) {
        self.caret_position.set_value_and_mark_modified(
            self.formatted_text
                .borrow()
                .nearest_valid_position(position),
        );
        self.ensure_caret_visible();
        self.reset_blink();
    }

    /// Tries to map screen space position to a position in the text.
    pub fn screen_pos_to_text_pos(&self, screen_point: Vector2<f32>) -> Option<Position> {
        // Transform given point into local space of the text box - this way calculations can be done
        // as usual, without a need for special math.
        let point_to_check = self
            .visual_transform
            .try_inverse()?
            .transform_point(&Point2::from(screen_point))
            .coords;

        Some(
            self.formatted_text
                .borrow_mut()
                .local_to_position(point_to_check),
        )
    }

    /// Returns current text of text box.
    pub fn text(&self) -> String {
        self.formatted_text.borrow().text()
    }

    /// Returns current word wrapping mode of text box.
    pub fn wrap_mode(&self) -> WrapMode {
        self.formatted_text.borrow().wrap_mode()
    }

    /// Returns current font of text box.
    pub fn font(&self) -> FontResource {
        self.formatted_text.borrow().get_font()
    }

    /// Returns current vertical alignment of text box.
    pub fn vertical_alignment(&self) -> VerticalAlignment {
        self.formatted_text.borrow().vertical_alignment()
    }

    /// Returns current horizontal alignment of text box.
    pub fn horizontal_alignment(&self) -> HorizontalAlignment {
        self.formatted_text.borrow().horizontal_alignment()
    }

    fn select_word(&mut self, position: Position) {
        if let Some(index) = self.position_to_char_index_clamped(position) {
            let text_ref = self.formatted_text.borrow();
            let text = text_ref.get_raw_text();
            let search_whitespace = !text[index].is_whitespace();

            let mut left_index = index;
            while left_index > 0 {
                let is_whitespace = text[left_index].is_whitespace();
                if search_whitespace && is_whitespace || !search_whitespace && !is_whitespace {
                    left_index += 1;
                    break;
                }
                left_index = left_index.saturating_sub(1);
            }

            let mut right_index = index;
            while right_index < text.len() {
                let is_whitespace = text[right_index].is_whitespace();
                if search_whitespace && is_whitespace || !search_whitespace && !is_whitespace {
                    break;
                }

                right_index += 1;
            }

            drop(text_ref);

            if let (Some(left), Some(right)) = (
                self.char_index_to_position(left_index),
                self.char_index_to_position(right_index),
            ) {
                self.selection_range
                    .set_value_and_mark_modified(Some(SelectionRange {
                        begin: left,
                        end: right,
                    }));
            }
        }
    }
}

uuid_provider!(TextBox = "536276f2-a175-4c05-a376-5a7d8bf0d10b");

impl Control for TextBox {
    fn measure_override(&self, _: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        self.formatted_text
            .borrow_mut()
            .set_constraint(available_size)
            .build()
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        let bounds = self.widget.bounding_rect();
        drawing_context.push_rect_filled(&bounds, None);
        drawing_context.commit(
            self.clip_bounds(),
            self.widget.background(),
            CommandTexture::None,
            None,
        );

        self.formatted_text
            .borrow_mut()
            .set_brush(self.widget.foreground());

        let view_bounds = self.rect_to_view_pos(bounds);
        if let Some(ref selection_range) = self.selection_range.map(|r| r.normalized()) {
            let text = self.formatted_text.borrow();
            let lines = text.get_lines();
            if selection_range.begin.line == selection_range.end.line {
                if let Some(line) = lines.get(selection_range.begin.line) {
                    // Begin line
                    let offset = text
                        .get_range_width(line.begin..(line.begin + selection_range.begin.offset));
                    let width = text.get_range_width(
                        (line.begin + selection_range.begin.offset)
                            ..(line.begin + selection_range.end.offset),
                    );
                    let selection_bounds = Rect::new(
                        view_bounds.x() + line.x_offset + offset,
                        view_bounds.y() + line.y_offset,
                        width,
                        line.height,
                    );
                    drawing_context.push_rect_filled(&selection_bounds, None);
                }
            } else {
                for (i, line) in text.get_lines().iter().enumerate() {
                    if i >= selection_range.begin.line && i <= selection_range.end.line {
                        let selection_bounds = if i == selection_range.begin.line {
                            // Begin line
                            let offset = text.get_range_width(
                                line.begin..(line.begin + selection_range.begin.offset),
                            );
                            let width = text.get_range_width(
                                (line.begin + selection_range.begin.offset)..line.end,
                            );
                            Rect::new(
                                view_bounds.x() + line.x_offset + offset,
                                view_bounds.y() + line.y_offset,
                                width,
                                line.height,
                            )
                        } else if i == selection_range.end.line {
                            // End line
                            let width = text.get_range_width(
                                line.begin..(line.begin + selection_range.end.offset),
                            );
                            Rect::new(
                                view_bounds.x() + line.x_offset,
                                view_bounds.y() + line.y_offset,
                                width,
                                line.height,
                            )
                        } else {
                            // Everything between
                            Rect::new(
                                view_bounds.x() + line.x_offset,
                                view_bounds.y() + line.y_offset,
                                line.width,
                                line.height,
                            )
                        };
                        drawing_context.push_rect_filled(&selection_bounds, None);
                    }
                }
            }
        }
        drawing_context.commit(
            self.clip_bounds(),
            (*self.selection_brush).clone(),
            CommandTexture::None,
            None,
        );

        let local_position = self.point_to_view_pos(bounds.position);
        drawing_context.draw_text(
            self.clip_bounds(),
            local_position,
            &self.formatted_text.borrow(),
        );

        if *self.caret_visible {
            let caret_pos = self.point_to_view_pos(self.caret_local_position());
            let caret_bounds = Rect::new(
                caret_pos.x,
                caret_pos.y,
                2.0,
                self.formatted_text.borrow().font_size(),
            );
            drawing_context.push_rect_filled(&caret_bounds, None);
            drawing_context.commit(
                self.clip_bounds(),
                (*self.caret_brush).clone(),
                CommandTexture::None,
                None,
            );
        }
    }

    fn update(&mut self, dt: f32, _ui: &mut UserInterface) {
        if self.has_focus {
            *self.blink_timer += dt;
            if *self.blink_timer >= *self.blink_interval {
                self.blink_timer.set_value_and_mark_modified(0.0);
                self.caret_visible
                    .set_value_and_mark_modified(!*self.caret_visible);
            }
        } else {
            self.caret_visible.set_value_and_mark_modified(false);
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle() {
            if let Some(msg) = message.data::<WidgetMessage>() {
                match msg {
                    WidgetMessage::Text(text)
                        if !ui.keyboard_modifiers().control
                            && !ui.keyboard_modifiers().alt
                            && *self.editable =>
                    {
                        for symbol in text.chars() {
                            let insert = !symbol.is_control()
                                && if let Some(filter) = self.filter.as_ref() {
                                    let filter = &mut *filter.lock();
                                    filter(symbol)
                                } else {
                                    true
                                };
                            if insert {
                                self.insert_char(symbol, ui);
                            }
                        }
                    }
                    WidgetMessage::KeyDown(code) => {
                        match code {
                            KeyCode::ArrowUp if !self.selecting => {
                                self.move_caret_y(-1, ui.keyboard_modifiers().shift);
                            }
                            KeyCode::ArrowDown if !self.selecting => {
                                self.move_caret_y(1, ui.keyboard_modifiers().shift);
                            }
                            KeyCode::ArrowRight if !self.selecting => {
                                if ui.keyboard_modifiers.control {
                                    self.move_caret(
                                        self.find_next_word(*self.caret_position),
                                        ui.keyboard_modifiers().shift,
                                    );
                                } else {
                                    self.move_caret_x(1, ui.keyboard_modifiers().shift);
                                }
                            }
                            KeyCode::ArrowLeft if !self.selecting => {
                                if ui.keyboard_modifiers.control {
                                    self.move_caret(
                                        self.find_prev_word(*self.caret_position),
                                        ui.keyboard_modifiers().shift,
                                    );
                                } else {
                                    self.move_caret_x(-1, ui.keyboard_modifiers().shift);
                                }
                            }
                            KeyCode::Delete
                                if !message.handled() && *self.editable && !self.selecting =>
                            {
                                self.remove_char(HorizontalDirection::Right, ui);
                            }
                            KeyCode::NumpadEnter | KeyCode::Enter if *self.editable => {
                                if *self.multiline {
                                    self.insert_char('\n', ui);
                                } else if *self.commit_mode == TextCommitMode::LostFocusPlusEnter {
                                    ui.send_message(TextMessage::text(
                                        self.handle,
                                        MessageDirection::FromWidget,
                                        self.text(),
                                    ));
                                } else if *self.commit_mode == TextCommitMode::Changed {
                                    self.commit_if_changed(ui);
                                }
                                // Don't set has_focus = false when enter is pressed.
                                // That messes up keyboard navigation.
                            }
                            KeyCode::Backspace if *self.editable && !self.selecting => {
                                self.remove_char(HorizontalDirection::Left, ui);
                            }
                            KeyCode::End if !self.selecting => {
                                let select = ui.keyboard_modifiers().shift;
                                let position = if ui.keyboard_modifiers().control {
                                    self.end_position()
                                } else {
                                    self.formatted_text
                                        .borrow()
                                        .get_line_range(self.caret_position.line)
                                        .end
                                };
                                self.move_caret(position, select);
                            }
                            KeyCode::Home if !self.selecting => {
                                let select = ui.keyboard_modifiers().shift;
                                let position = if ui.keyboard_modifiers().control {
                                    Position::default()
                                } else {
                                    self.formatted_text
                                        .borrow()
                                        .get_line_range(self.caret_position.line)
                                        .start
                                };
                                self.move_caret(position, select);
                            }
                            KeyCode::KeyA if ui.keyboard_modifiers().control => {
                                let end = self.end_position();
                                if end != Position::default() {
                                    self.selection_range.set_value_and_mark_modified(Some(
                                        SelectionRange {
                                            begin: Position::default(),
                                            end: self.end_position(),
                                        },
                                    ));
                                }
                            }
                            KeyCode::KeyC if ui.keyboard_modifiers().control => {
                                if let Some(mut clipboard) = ui.clipboard_mut() {
                                    if let Some(selection_range) = self.selection_range.as_ref() {
                                        let range = self
                                            .formatted_text
                                            .borrow()
                                            .position_range_to_char_index_range(
                                                selection_range.range(),
                                            );
                                        if !range.is_empty() {
                                            let _ = clipboard.set_contents(
                                                self.formatted_text.borrow().text_range(range),
                                            );
                                        }
                                    }
                                }
                            }
                            KeyCode::KeyV if ui.keyboard_modifiers().control => {
                                if let Some(mut clipboard) = ui.clipboard_mut() {
                                    if let Ok(content) = clipboard.get_contents() {
                                        self.insert_str(&content, ui);
                                    }
                                }
                            }
                            KeyCode::KeyX if ui.keyboard_modifiers().control => {
                                if let Some(mut clipboard) = ui.clipboard_mut() {
                                    if let Some(selection_range) = self.selection_range.as_ref() {
                                        let range = self
                                            .formatted_text
                                            .borrow()
                                            .position_range_to_char_index_range(
                                                selection_range.range(),
                                            );
                                        if !range.is_empty() {
                                            let _ = clipboard.set_contents(
                                                self.formatted_text.borrow().text_range(range),
                                            );
                                            self.remove_char(HorizontalDirection::Left, ui);
                                        }
                                    }
                                }
                            }
                            _ => (),
                        }

                        // TextBox "eats" all input by default, some of the keys are used for input control while
                        // others are used directly to enter text.
                        message.set_handled(true);
                    }
                    WidgetMessage::Focus => {
                        if message.direction() == MessageDirection::FromWidget {
                            self.reset_blink();
                            self.has_focus = true;
                            let end = self.end_position();
                            if end != Position::default() {
                                self.set_caret_position(end);
                                self.selection_range.set_value_and_mark_modified(Some(
                                    SelectionRange {
                                        begin: Position::default(),
                                        end,
                                    },
                                ));
                            }
                            if *self.commit_mode == TextCommitMode::Changed {
                                self.recent.clear();
                                self.recent
                                    .extend_from_slice(self.formatted_text.borrow().get_raw_text());
                            }
                        }
                    }
                    WidgetMessage::Unfocus => {
                        if message.direction() == MessageDirection::FromWidget {
                            self.selection_range.set_value_and_mark_modified(None);
                            self.has_focus = false;

                            match *self.commit_mode {
                                TextCommitMode::LostFocus | TextCommitMode::LostFocusPlusEnter => {
                                    ui.send_message(TextMessage::text(
                                        self.handle,
                                        MessageDirection::FromWidget,
                                        self.text(),
                                    ));
                                }
                                TextCommitMode::Changed => {
                                    self.commit_if_changed(ui);
                                }
                                _ => (),
                            }
                            // There is no reason to keep the stored recent value in memory
                            // while this TextBox does not have focus. Maybe this should be stored globally in UserInterface,
                            // since we only ever need one.
                            self.recent.clear();
                            self.recent.shrink_to(0);
                        }
                    }
                    WidgetMessage::MouseDown { pos, button } => {
                        if *button == MouseButton::Left {
                            let select = ui.keyboard_modifiers().shift;
                            if !select {
                                self.selection_range.set_value_and_mark_modified(None);
                            }
                            self.selecting = true;
                            self.has_focus = true;
                            self.before_click_position = *self.caret_position;

                            if let Some(position) = self.screen_pos_to_text_pos(*pos) {
                                self.move_caret(position, select);
                            }

                            ui.capture_mouse(self.handle());
                        }
                    }
                    WidgetMessage::DoubleClick {
                        button: MouseButton::Left,
                    } => {
                        if let Some(position) = self.screen_pos_to_text_pos(ui.cursor_position) {
                            if position == self.before_click_position {
                                self.select_word(position);
                            }
                        }
                    }
                    WidgetMessage::MouseMove { pos, .. } => {
                        if self.selecting {
                            if let Some(position) = self.screen_pos_to_text_pos(*pos) {
                                self.move_caret(position, true);
                            }
                        }
                    }
                    WidgetMessage::MouseUp { .. } => {
                        self.selecting = false;
                        ui.release_mouse_capture();
                    }
                    _ => {}
                }
            } else if let Some(msg) = message.data::<TextMessage>() {
                if message.direction() == MessageDirection::ToWidget {
                    let mut text = self.formatted_text.borrow_mut();

                    match msg {
                        TextMessage::Text(new_text) => {
                            fn text_equals(
                                formatted_text: &FormattedText,
                                input_string: &str,
                            ) -> bool {
                                let raw_text = formatted_text.get_raw_text();

                                if raw_text.len() != input_string.chars().count() {
                                    false
                                } else {
                                    for (raw_char, input_char) in
                                        raw_text.iter().zip(input_string.chars())
                                    {
                                        if *raw_char != input_char {
                                            return false;
                                        }
                                    }

                                    true
                                }
                            }
                            if !text_equals(&text, new_text) {
                                text.set_text(new_text);
                                drop(text);
                                self.invalidate_layout();
                                self.formatted_text.borrow_mut().build();

                                if *self.commit_mode == TextCommitMode::Immediate {
                                    ui.send_message(message.reverse());
                                }
                            }
                        }
                        TextMessage::Wrap(wrap_mode) => {
                            if text.wrap_mode() != *wrap_mode {
                                text.set_wrap(*wrap_mode);
                                drop(text);
                                self.invalidate_layout();
                                ui.send_message(message.reverse());
                            }
                        }
                        TextMessage::Font(font) => {
                            if &text.get_font() != font {
                                text.set_font(font.clone());
                                drop(text);
                                self.invalidate_layout();
                                ui.send_message(message.reverse());
                            }
                        }
                        TextMessage::VerticalAlignment(alignment) => {
                            if &text.vertical_alignment() != alignment {
                                text.set_vertical_alignment(*alignment);
                                drop(text);
                                self.invalidate_layout();
                                ui.send_message(message.reverse());
                            }
                        }
                        TextMessage::HorizontalAlignment(alignment) => {
                            if &text.horizontal_alignment() != alignment {
                                text.set_horizontal_alignment(*alignment);
                                drop(text);
                                self.invalidate_layout();
                                ui.send_message(message.reverse());
                            }
                        }
                        &TextMessage::Shadow(shadow) => {
                            if *text.shadow != shadow {
                                text.set_shadow(shadow);
                                drop(text);
                                self.invalidate_layout();
                                ui.send_message(message.reverse());
                            }
                        }
                        TextMessage::ShadowBrush(brush) => {
                            if &*text.shadow_brush != brush {
                                text.set_shadow_brush(brush.clone());
                                drop(text);
                                self.invalidate_layout();
                                ui.send_message(message.reverse());
                            }
                        }
                        &TextMessage::ShadowDilation(dilation) => {
                            if *text.shadow_dilation != dilation {
                                text.set_shadow_dilation(dilation);
                                drop(text);
                                self.invalidate_layout();
                                ui.send_message(message.reverse());
                            }
                        }
                        &TextMessage::ShadowOffset(offset) => {
                            if *text.shadow_offset != offset {
                                text.set_shadow_offset(offset);
                                drop(text);
                                self.invalidate_layout();
                                ui.send_message(message.reverse());
                            }
                        }
                        &TextMessage::FontSize(height) => {
                            if text.font_size() != height {
                                text.set_font_size(height);
                                drop(text);
                                self.invalidate_layout();
                                ui.send_message(message.reverse());
                            }
                        }
                    }
                }
            } else if let Some(msg) = message.data::<TextBoxMessage>() {
                if message.direction() == MessageDirection::ToWidget {
                    match msg {
                        TextBoxMessage::SelectionBrush(brush) => {
                            if &*self.selection_brush != brush {
                                self.selection_brush
                                    .set_value_and_mark_modified(brush.clone());
                                ui.send_message(message.reverse());
                            }
                        }
                        TextBoxMessage::CaretBrush(brush) => {
                            if &*self.caret_brush != brush {
                                self.caret_brush.set_value_and_mark_modified(brush.clone());
                                ui.send_message(message.reverse());
                            }
                        }
                        TextBoxMessage::TextCommitMode(mode) => {
                            if &*self.commit_mode != mode {
                                self.commit_mode.set_value_and_mark_modified(*mode);
                                ui.send_message(message.reverse());
                            }
                        }
                        TextBoxMessage::Multiline(multiline) => {
                            if &*self.multiline != multiline {
                                self.multiline.set_value_and_mark_modified(*multiline);
                                ui.send_message(message.reverse());
                            }
                        }
                        TextBoxMessage::Editable(editable) => {
                            if &*self.editable != editable {
                                self.editable.set_value_and_mark_modified(*editable);
                                ui.send_message(message.reverse());
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Text box builder creates new [`TextBox`] instances and adds them to the user interface.
pub struct TextBoxBuilder {
    widget_builder: WidgetBuilder,
    font: Option<FontResource>,
    text: String,
    caret_brush: Brush,
    selection_brush: Brush,
    filter: Option<Arc<Mutex<FilterCallback>>>,
    vertical_alignment: VerticalAlignment,
    horizontal_alignment: HorizontalAlignment,
    wrap: WrapMode,
    commit_mode: TextCommitMode,
    multiline: bool,
    editable: bool,
    mask_char: Option<char>,
    shadow: bool,
    shadow_brush: Brush,
    shadow_dilation: f32,
    shadow_offset: Vector2<f32>,
    skip_chars: Vec<char>,
    font_size: f32,
}

impl TextBoxBuilder {
    /// Creates new text box widget builder with the base widget builder specified.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            font: None,
            text: "".to_owned(),
            caret_brush: Brush::Solid(Color::WHITE),
            selection_brush: Brush::Solid(Color::opaque(80, 118, 178)),
            filter: None,
            vertical_alignment: VerticalAlignment::Top,
            horizontal_alignment: HorizontalAlignment::Left,
            wrap: WrapMode::NoWrap,
            commit_mode: TextCommitMode::LostFocusPlusEnter,
            multiline: false,
            editable: true,
            mask_char: None,
            shadow: false,
            shadow_brush: Brush::Solid(Color::BLACK),
            shadow_dilation: 1.0,
            shadow_offset: Vector2::new(1.0, 1.0),
            skip_chars: Default::default(),
            font_size: 14.0,
        }
    }

    /// Sets the desired font of the text box.
    pub fn with_font(mut self, font: FontResource) -> Self {
        self.font = Some(font);
        self
    }

    /// Sets the desired text of the text box.
    pub fn with_text<P: AsRef<str>>(mut self, text: P) -> Self {
        text.as_ref().clone_into(&mut self.text);
        self
    }

    /// Sets the desired caret brush of the text box.
    pub fn with_caret_brush(mut self, brush: Brush) -> Self {
        self.caret_brush = brush;
        self
    }

    /// Sets the desired selection brush of the text box.
    pub fn with_selection_brush(mut self, brush: Brush) -> Self {
        self.selection_brush = brush;
        self
    }

    /// Sets the desired character filter of the text box. See [`FilterCallback`] for more info.
    pub fn with_filter(mut self, filter: Arc<Mutex<FilterCallback>>) -> Self {
        self.filter = Some(filter);
        self
    }

    /// Sets the desired vertical text alignment of the text box.
    pub fn with_vertical_text_alignment(mut self, alignment: VerticalAlignment) -> Self {
        self.vertical_alignment = alignment;
        self
    }

    /// Sets the desired horizontal text alignment of the text box.
    pub fn with_horizontal_text_alignment(mut self, alignment: HorizontalAlignment) -> Self {
        self.horizontal_alignment = alignment;
        self
    }

    /// Sets the desired word wrapping of the text box.
    pub fn with_wrap(mut self, wrap: WrapMode) -> Self {
        self.wrap = wrap;
        self
    }

    /// Sets the desired text commit mode of the text box.
    pub fn with_text_commit_mode(mut self, mode: TextCommitMode) -> Self {
        self.commit_mode = mode;
        self
    }

    /// Enables or disables multiline mode of the text box.
    pub fn with_multiline(mut self, multiline: bool) -> Self {
        self.multiline = multiline;
        self
    }

    /// Enables or disables editing of the text box.
    pub fn with_editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    /// Sets the desired height of the text.
    pub fn with_font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;
        self
    }

    /// Sets the desired masking character of the text box.
    pub fn with_mask_char(mut self, mask_char: Option<char>) -> Self {
        self.mask_char = mask_char;
        self
    }

    /// Whether the shadow enabled or not.
    pub fn with_shadow(mut self, shadow: bool) -> Self {
        self.shadow = shadow;
        self
    }

    /// Sets desired shadow brush. It will be used to render the shadow.
    pub fn with_shadow_brush(mut self, brush: Brush) -> Self {
        self.shadow_brush = brush;
        self
    }

    /// Sets desired shadow dilation in units. Keep in mind that the dilation is absolute,
    /// not percentage-based.
    pub fn with_shadow_dilation(mut self, thickness: f32) -> Self {
        self.shadow_dilation = thickness;
        self
    }

    /// Sets desired shadow offset in units.
    pub fn with_shadow_offset(mut self, offset: Vector2<f32>) -> Self {
        self.shadow_offset = offset;
        self
    }

    /// Sets desired set of characters that will be treated like whitespace during Ctrl+Arrow navigation
    /// (Ctrl+Left Arrow and Ctrl+Right Arrow). This could be useful to treat underscores like whitespaces,
    /// which in its turn could be useful for in-game consoles where commands usually separated using
    /// underscores (`like_this_one`).
    pub fn with_skip_chars(mut self, chars: Vec<char>) -> Self {
        self.skip_chars = chars;
        self
    }

    /// Creates a new [`TextBox`] instance and adds it to the user interface.
    pub fn build(mut self, ctx: &mut BuildContext) -> Handle<UiNode> {
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
            widget: self
                .widget_builder
                .with_accepts_input(true)
                .with_need_update(true)
                .build(),
            caret_position: Position::default().into(),
            caret_visible: false.into(),
            blink_timer: 0.0.into(),
            blink_interval: 0.5.into(),
            formatted_text: RefCell::new(
                FormattedTextBuilder::new(self.font.unwrap_or_else(|| ctx.default_font()))
                    .with_text(self.text)
                    .with_horizontal_alignment(self.horizontal_alignment)
                    .with_vertical_alignment(self.vertical_alignment)
                    .with_wrap(self.wrap)
                    .with_mask_char(self.mask_char)
                    .with_shadow(self.shadow)
                    .with_shadow_brush(self.shadow_brush)
                    .with_shadow_dilation(self.shadow_dilation)
                    .with_shadow_offset(self.shadow_offset)
                    .with_font_size(self.font_size)
                    .build(),
            ),
            selection_range: None.into(),
            selecting: false,
            before_click_position: Position::default(),
            selection_brush: self.selection_brush.into(),
            caret_brush: self.caret_brush.into(),
            has_focus: false,
            filter: self.filter,
            commit_mode: self.commit_mode.into(),
            multiline: self.multiline.into(),
            editable: self.editable.into(),
            view_position: Default::default(),
            skip_chars: self.skip_chars.into(),
            recent: Default::default(),
        };

        ctx.add_node(UiNode::new(text_box))
    }
}
