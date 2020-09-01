//! Message and events module contains all possible widget messages and OS events.
//!
//! This UI library uses message passing mechanism to communicate with widgets.
//! This is very simple and reliable mechanism that effectively decouples widgets
//! from each other. There is no direct way of modify something during runtime,
//! you have to use messages to change state of ui elements.
//!
//! # Direction
//!
//! Each message marked with "Direction" field, which means supported routes for
//! message. For example [ButtonMessage::Click](enum.ButtonMessage.html) has "Direction: To/From UI" which
//! means that it can be sent either from internals of library or from user code.
//! However [WidgetMessage::GotFocus](enum.WidgetMessage.html) has "Direction: From UI" which means that only
//! internal library code can send such messages without a risk of breaking anything.

use crate::{
    brush::Brush,
    core::{
        math::{vec2::Vec2, vec3::Vec3},
        pool::Handle,
    },
    dock::TileContent,
    draw::Texture,
    messagebox::MessageBoxResult,
    popup::Placement,
    ttf::Font,
    window::WindowTitle,
    Control, HorizontalAlignment, MouseState, Thickness, UINode, VerticalAlignment,
};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

/// A set of messages for any kind of widgets (including user controls). These messages provides basic
/// communication elements of the UI library.
#[derive(Debug)]
pub enum WidgetMessage<M: 'static, C: 'static + Control<M, C>> {
    /// Initiated when user clicks on a widget's geometry.
    ///
    /// Direction: **From UI**.
    MouseDown {
        /// Position of cursor.
        pos: Vec2,
        /// A button that was pressed.
        button: MouseButton,
    },

    /// Initiated when user releases mouse button while cursor is over widget's geometry.
    ///
    /// Direction: **From UI**.
    MouseUp {
        /// Position of cursor.
        pos: Vec2,
        /// A button that was released.
        button: MouseButton,
    },

    /// Initiated when user moves cursor over widget's geometry.
    ///
    /// Direction: **From/To UI**.
    MouseMove {
        /// New position of cursor in screen coordinates.
        pos: Vec2,
        /// State of mouse buttons.
        state: MouseState,
    },

    /// Initiated when user scrolls mouse wheel while cursor is over widget's geometry.
    ///
    /// Direction: **From/To UI**.
    MouseWheel {
        /// Position of cursor.
        pos: Vec2,
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
    DragStarted(Handle<UINode<M, C>>),

    /// Initiated when user drags a widget over some other widget.
    ///
    /// Direction: **From UI**.
    DragOver(Handle<UINode<M, C>>),

    /// Initiated when user drops a widget onto some other widget.
    ///
    /// Direction: **From UI**.
    Drop(Handle<UINode<M, C>>),

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
    LinkWith(Handle<UINode<M, C>>),

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
    /// because rg3d-ui uses automatic layout engine which will correctly calculate desired width of a widget.
    ///
    /// Direction: **From/To UI**
    Width(f32),

    /// A request to set height of a widget. In most cases there is no need to explicitly set height of a widget,
    /// because rg3d-ui uses automatic layout engine which will correctly calculate desired height of a widget.
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
    MaxSize(Vec2),

    /// A request to set minimum size of widget. Minimum size restricts size of a widget during layout pass. For example
    /// you can set minimum size to a button which was placed into a grid's cell, if minimum size wouldn't be set, button
    /// would be compressed to fill entire cell.
    ///
    /// Direction: **From/To UI**
    MinSize(Vec2),

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
    DesiredPosition(Vec2),

    /// A request to enable or disable widget. Disabled widget won't receive mouse events and may look differently (it is defined
    /// by internal styling).
    ///
    /// Direction: **From/To UI**
    Enabled(bool),

    /// A request to set desired position at center in local coordinates.
    ///
    /// Direction: **From/To UI**
    Center,
}

impl<M: 'static, C: 'static + Control<M, C>> WidgetMessage<M, C> {
    fn make(destination: Handle<UINode<M, C>>, msg: WidgetMessage<M, C>) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::Widget(msg),
            destination,
        }
    }

    /// Creates a message to remove `destination` node.
    pub fn remove(destination: Handle<UINode<M, C>>) -> UiMessage<M, C> {
        Self::make(destination, WidgetMessage::Remove)
    }

    /// Creates a message to unlink `destination` node from its current parent.
    pub fn unlink(destination: Handle<UINode<M, C>>) -> UiMessage<M, C> {
        Self::make(destination, WidgetMessage::Unlink)
    }

    /// Creates message to link `destination` node with specified `parent` node.
    pub fn link(
        destination: Handle<UINode<M, C>>,
        parent: Handle<UINode<M, C>>,
    ) -> UiMessage<M, C> {
        Self::make(destination, WidgetMessage::LinkWith(parent))
    }

    /// Creates message to set background of `destination` node.
    pub fn background(destination: Handle<UINode<M, C>>, background: Brush) -> UiMessage<M, C> {
        Self::make(destination, WidgetMessage::Background(background))
    }

    /// Creates message to set visibility of `destination` node.
    pub fn visibility(destination: Handle<UINode<M, C>>, visibility: bool) -> UiMessage<M, C> {
        Self::make(destination, WidgetMessage::Visibility(visibility))
    }

    /// Creates message to set width of `destination` node.
    pub fn width(destination: Handle<UINode<M, C>>, width: f32) -> UiMessage<M, C> {
        Self::make(destination, WidgetMessage::Width(width))
    }

    /// Creates message to set height of `destination` node.
    pub fn height(destination: Handle<UINode<M, C>>, height: f32) -> UiMessage<M, C> {
        Self::make(destination, WidgetMessage::Height(height))
    }

    /// Creates message to set desired position of `destination` node.
    pub fn desired_position(destination: Handle<UINode<M, C>>, position: Vec2) -> UiMessage<M, C> {
        Self::make(destination, WidgetMessage::DesiredPosition(position))
    }

    /// Creates message to set desired position of `destination` node.
    pub fn center(destination: Handle<UINode<M, C>>) -> UiMessage<M, C> {
        Self::make(destination, WidgetMessage::Center)
    }

    /// Creates message to make given widget topmost.
    pub fn topmost(destination: Handle<UINode<M, C>>) -> UiMessage<M, C> {
        Self::make(destination, WidgetMessage::TopMost)
    }

    /// Creates message to enable or disable given widget.
    pub fn enabled(destination: Handle<UINode<M, C>>, enabled: bool) -> UiMessage<M, C> {
        Self::make(destination, WidgetMessage::Enabled(enabled))
    }

    // TODO: Add rest items.
}

#[derive(Debug)]
pub enum ButtonMessage<M: 'static, C: 'static + Control<M, C>> {
    Click,
    Content(Handle<UINode<M, C>>),
}

impl<M: 'static, C: 'static + Control<M, C>> ButtonMessage<M, C> {
    fn make(destination: Handle<UINode<M, C>>, msg: ButtonMessage<M, C>) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::Button(msg),
            destination,
        }
    }

    pub fn click(destination: Handle<UINode<M, C>>) -> UiMessage<M, C> {
        Self::make(destination, ButtonMessage::Click)
    }

    pub fn content(
        destination: Handle<UINode<M, C>>,
        content: Handle<UINode<M, C>>,
    ) -> UiMessage<M, C> {
        Self::make(destination, ButtonMessage::Content(content))
    }
}

#[derive(Debug)]
pub enum ScrollBarMessage {
    Value(f32),
    MinValue(f32),
    MaxValue(f32),
}

impl ScrollBarMessage {
    fn make<M: 'static, C: 'static + Control<M, C>>(
        destination: Handle<UINode<M, C>>,
        msg: ScrollBarMessage,
    ) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::ScrollBar(msg),
            destination,
        }
    }

    pub fn value<M: 'static, C: 'static + Control<M, C>>(
        destination: Handle<UINode<M, C>>,
        value: f32,
    ) -> UiMessage<M, C> {
        Self::make(destination, ScrollBarMessage::Value(value))
    }

    pub fn max_value<M: 'static, C: 'static + Control<M, C>>(
        destination: Handle<UINode<M, C>>,
        value: f32,
    ) -> UiMessage<M, C> {
        Self::make(destination, ScrollBarMessage::MaxValue(value))
    }

    pub fn min_value<M: 'static, C: 'static + Control<M, C>>(
        destination: Handle<UINode<M, C>>,
        value: f32,
    ) -> UiMessage<M, C> {
        Self::make(destination, ScrollBarMessage::MinValue(value))
    }
}

#[derive(Debug)]
pub enum CheckBoxMessage {
    Check(Option<bool>),
}

impl CheckBoxMessage {
    fn make<M: 'static, C: 'static + Control<M, C>>(
        destination: Handle<UINode<M, C>>,
        msg: CheckBoxMessage,
    ) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::CheckBox(msg),
            destination,
        }
    }

    pub fn check<M: 'static, C: 'static + Control<M, C>>(
        destination: Handle<UINode<M, C>>,
        value: Option<bool>,
    ) -> UiMessage<M, C> {
        Self::make(destination, CheckBoxMessage::Check(value))
    }
}

#[derive(Debug)]
pub enum WindowMessage<M: 'static, C: 'static + Control<M, C>> {
    Open,
    OpenModal,
    Close,
    Minimize(bool),
    CanMinimize(bool),
    CanClose(bool),
    MoveStart,
    /// New position is in local coordinates.
    Move(Vec2),
    MoveEnd,
    Title(WindowTitle<M, C>),
}

impl<M: 'static, C: 'static + Control<M, C>> WindowMessage<M, C> {
    fn make(destination: Handle<UINode<M, C>>, msg: WindowMessage<M, C>) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::Window(msg),
            destination,
        }
    }

    pub fn open(destination: Handle<UINode<M, C>>) -> UiMessage<M, C> {
        Self::make(destination, WindowMessage::Open)
    }

    pub fn open_modal(destination: Handle<UINode<M, C>>) -> UiMessage<M, C> {
        Self::make(destination, WindowMessage::OpenModal)
    }

    pub fn close(destination: Handle<UINode<M, C>>) -> UiMessage<M, C> {
        Self::make(destination, WindowMessage::Close)
    }
}

#[derive(Debug)]
pub enum ScrollViewerMessage<M: 'static, C: 'static + Control<M, C>> {
    Content(Handle<UINode<M, C>>),
    /// Adjusts vertical and horizontal scroll values so given node will be in "view box"
    /// of scroll viewer.
    BringIntoView(Handle<UINode<M, C>>),
}

impl<M: 'static, C: 'static + Control<M, C>> ScrollViewerMessage<M, C> {
    fn make(destination: Handle<UINode<M, C>>, msg: ScrollViewerMessage<M, C>) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::ScrollViewer(msg),
            destination,
        }
    }

    pub fn content(
        destination: Handle<UINode<M, C>>,
        content: Handle<UINode<M, C>>,
    ) -> UiMessage<M, C> {
        Self::make(destination, ScrollViewerMessage::Content(content))
    }

    pub fn bring_into_view(
        destination: Handle<UINode<M, C>>,
        handle: Handle<UINode<M, C>>,
    ) -> UiMessage<M, C> {
        Self::make(destination, ScrollViewerMessage::BringIntoView(handle))
    }
}

#[derive(Debug)]
pub enum ListViewMessage<M: 'static, C: 'static + Control<M, C>> {
    SelectionChanged(Option<usize>),
    Items(Vec<Handle<UINode<M, C>>>),
    AddItem(Handle<UINode<M, C>>),
}

#[derive(Debug)]
pub enum PopupMessage<M: 'static, C: 'static + Control<M, C>> {
    Open,
    Close,
    Content(Handle<UINode<M, C>>),
    Placement(Placement),
}

#[derive(Debug)]
pub enum FileSelectorMessage {
    Path(PathBuf),
    Commit(PathBuf),
    Cancel,
}

impl FileSelectorMessage {
    fn make<M: 'static, C: 'static + Control<M, C>>(
        destination: Handle<UINode<M, C>>,
        msg: FileSelectorMessage,
    ) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::FileSelector(msg),
            destination,
        }
    }

    pub fn commit<M: 'static, C: 'static + Control<M, C>>(
        destination: Handle<UINode<M, C>>,
        path: PathBuf,
    ) -> UiMessage<M, C> {
        Self::make(destination, FileSelectorMessage::Commit(path))
    }

    pub fn path<M: 'static, C: 'static + Control<M, C>>(
        destination: Handle<UINode<M, C>>,
        path: PathBuf,
    ) -> UiMessage<M, C> {
        Self::make(destination, FileSelectorMessage::Path(path))
    }

    pub fn cancel<M: 'static, C: 'static + Control<M, C>>(
        destination: Handle<UINode<M, C>>,
    ) -> UiMessage<M, C> {
        Self::make(destination, FileSelectorMessage::Cancel)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct SelectionState(pub(in crate) bool);

#[derive(Debug)]
pub enum TreeMessage<M: 'static, C: 'static + Control<M, C>> {
    Expand(bool),
    AddItem(Handle<UINode<M, C>>),
    RemoveItem(Handle<UINode<M, C>>),
    SetItems(Vec<Handle<UINode<M, C>>>),
    // Private, do not use. For internal needs only. Use TreeRootMessage::Selected.
    Select(SelectionState),
}

impl<M: 'static, C: 'static + Control<M, C>> TreeMessage<M, C> {
    fn make(destination: Handle<UINode<M, C>>, msg: TreeMessage<M, C>) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::Tree(msg),
            destination,
        }
    }

    pub fn add_item(
        destination: Handle<UINode<M, C>>,
        item: Handle<UINode<M, C>>,
    ) -> UiMessage<M, C> {
        Self::make(destination, TreeMessage::AddItem(item))
    }

    pub fn remove_item(
        destination: Handle<UINode<M, C>>,
        item: Handle<UINode<M, C>>,
    ) -> UiMessage<M, C> {
        Self::make(destination, TreeMessage::RemoveItem(item))
    }

    pub fn set_items(
        destination: Handle<UINode<M, C>>,
        items: Vec<Handle<UINode<M, C>>>,
    ) -> UiMessage<M, C> {
        Self::make(destination, TreeMessage::SetItems(items))
    }

    pub fn expand(destination: Handle<UINode<M, C>>, expand: bool) -> UiMessage<M, C> {
        Self::make(destination, TreeMessage::Expand(expand))
    }

    pub(in crate) fn select(destination: Handle<UINode<M, C>>, select: bool) -> UiMessage<M, C> {
        Self::make(destination, TreeMessage::Select(SelectionState(select)))
    }
}

#[derive(Debug)]
pub enum TreeRootMessage<M: 'static, C: 'static + Control<M, C>> {
    AddItem(Handle<UINode<M, C>>),
    RemoveItem(Handle<UINode<M, C>>),
    Items(Vec<Handle<UINode<M, C>>>),
    Selected(Vec<Handle<UINode<M, C>>>),
}

impl<M: 'static, C: 'static + Control<M, C>> TreeRootMessage<M, C> {
    fn make(destination: Handle<UINode<M, C>>, msg: TreeRootMessage<M, C>) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::TreeRoot(msg),
            destination,
        }
    }

    pub fn add_item(
        destination: Handle<UINode<M, C>>,
        item: Handle<UINode<M, C>>,
    ) -> UiMessage<M, C> {
        Self::make(destination, TreeRootMessage::AddItem(item))
    }

    pub fn remove_item(
        destination: Handle<UINode<M, C>>,
        item: Handle<UINode<M, C>>,
    ) -> UiMessage<M, C> {
        Self::make(destination, TreeRootMessage::RemoveItem(item))
    }

    pub fn items(
        destination: Handle<UINode<M, C>>,
        items: Vec<Handle<UINode<M, C>>>,
    ) -> UiMessage<M, C> {
        Self::make(destination, TreeRootMessage::Items(items))
    }

    pub fn select(
        destination: Handle<UINode<M, C>>,
        items: Vec<Handle<UINode<M, C>>>,
    ) -> UiMessage<M, C> {
        Self::make(destination, TreeRootMessage::Selected(items))
    }
}

#[derive(Debug)]
pub enum FileBrowserMessage {
    Path(PathBuf),
}

impl FileBrowserMessage {
    fn make<M: 'static, C: 'static + Control<M, C>>(
        destination: Handle<UINode<M, C>>,
        msg: FileBrowserMessage,
    ) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::FileBrowser(msg),
            destination,
        }
    }

    pub fn path<M: 'static, C: 'static + Control<M, C>>(
        destination: Handle<UINode<M, C>>,
        path: PathBuf,
    ) -> UiMessage<M, C> {
        Self::make(destination, FileBrowserMessage::Path(path))
    }
}

#[derive(Debug)]
pub enum TextBoxMessage {
    Text(String),
}

#[derive(Debug)]
pub enum TextMessage {
    Text(String),
    Wrap(bool),
    Font(Arc<Mutex<Font>>),
    VerticalAlignment(VerticalAlignment),
    HorizontalAlignment(HorizontalAlignment),
}

#[derive(Debug)]
pub enum ImageMessage {
    Texture(Option<Arc<Texture>>),
    Flip(bool),
}

impl ImageMessage {
    fn make<M: 'static, C: 'static + Control<M, C>>(
        destination: Handle<UINode<M, C>>,
        msg: ImageMessage,
    ) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::Image(msg),
            destination,
        }
    }

    pub fn texture<M: 'static, C: 'static + Control<M, C>>(
        destination: Handle<UINode<M, C>>,
        value: Option<Arc<Texture>>,
    ) -> UiMessage<M, C> {
        Self::make(destination, ImageMessage::Texture(value))
    }

    pub fn flip<M: 'static, C: 'static + Control<M, C>>(
        destination: Handle<UINode<M, C>>,
        value: bool,
    ) -> UiMessage<M, C> {
        Self::make(destination, ImageMessage::Flip(value))
    }
}

impl TextBoxMessage {
    fn make<M: 'static, C: 'static + Control<M, C>>(
        destination: Handle<UINode<M, C>>,
        msg: TextBoxMessage,
    ) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::TextBox(msg),
            destination,
        }
    }

    pub fn text<M: 'static, C: 'static + Control<M, C>>(
        destination: Handle<UINode<M, C>>,
        value: String,
    ) -> UiMessage<M, C> {
        Self::make(destination, TextBoxMessage::Text(value))
    }
}

impl TextMessage {
    fn make<M: 'static, C: 'static + Control<M, C>>(
        destination: Handle<UINode<M, C>>,
        msg: TextMessage,
    ) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::Text(msg),
            destination,
        }
    }

    pub fn text<M: 'static, C: 'static + Control<M, C>>(
        destination: Handle<UINode<M, C>>,
        value: String,
    ) -> UiMessage<M, C> {
        Self::make(destination, TextMessage::Text(value))
    }

    pub fn wrap<M: 'static, C: 'static + Control<M, C>>(
        destination: Handle<UINode<M, C>>,
        value: bool,
    ) -> UiMessage<M, C> {
        Self::make(destination, TextMessage::Wrap(value))
    }

    pub fn horizontal_alignment<M: 'static, C: 'static + Control<M, C>>(
        destination: Handle<UINode<M, C>>,
        value: HorizontalAlignment,
    ) -> UiMessage<M, C> {
        Self::make(destination, TextMessage::HorizontalAlignment(value))
    }

    pub fn vertical_alignment<M: 'static, C: 'static + Control<M, C>>(
        destination: Handle<UINode<M, C>>,
        value: VerticalAlignment,
    ) -> UiMessage<M, C> {
        Self::make(destination, TextMessage::VerticalAlignment(value))
    }

    pub fn font<M: 'static, C: 'static + Control<M, C>>(
        destination: Handle<UINode<M, C>>,
        value: Arc<Mutex<Font>>,
    ) -> UiMessage<M, C> {
        Self::make(destination, TextMessage::Font(value))
    }
}

#[derive(Debug)]
pub enum TileMessage<M: 'static, C: 'static + Control<M, C>> {
    Content(TileContent<M, C>),
}

impl<M: 'static, C: 'static + Control<M, C>> TileMessage<M, C> {
    fn make(destination: Handle<UINode<M, C>>, msg: TileMessage<M, C>) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::Tile(msg),
            destination,
        }
    }

    pub fn content(
        destination: Handle<UINode<M, C>>,
        content: TileContent<M, C>,
    ) -> UiMessage<M, C> {
        Self::make(destination, TileMessage::Content(content))
    }
}

#[derive(Debug)]
pub enum NumericUpDownMessage {
    Value(f32),
}

#[derive(Debug)]
pub enum Vec3EditorMessage {
    Value(Vec3),
}

#[derive(Debug)]
pub enum ScrollPanelMessage<M: 'static, C: 'static + Control<M, C>> {
    VerticalScroll(f32),
    HorizontalScroll(f32),
    /// Adjusts vertical and horizontal scroll values so given node will be in "view box"
    /// of scroll panel.
    BringIntoView(Handle<UINode<M, C>>),
}

impl<M: 'static, C: 'static + Control<M, C>> ScrollPanelMessage<M, C> {
    fn make(destination: Handle<UINode<M, C>>, msg: ScrollPanelMessage<M, C>) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::ScrollPanel(msg),
            destination,
        }
    }

    pub fn vertical_scroll(destination: Handle<UINode<M, C>>, value: f32) -> UiMessage<M, C> {
        Self::make(destination, ScrollPanelMessage::VerticalScroll(value))
    }

    pub fn horizontal_scroll(destination: Handle<UINode<M, C>>, value: f32) -> UiMessage<M, C> {
        Self::make(destination, ScrollPanelMessage::HorizontalScroll(value))
    }

    pub fn bring_into_view(
        destination: Handle<UINode<M, C>>,
        handle: Handle<UINode<M, C>>,
    ) -> UiMessage<M, C> {
        Self::make(destination, ScrollPanelMessage::BringIntoView(handle))
    }
}

#[derive(Debug)]
pub enum MenuMessage {
    Activate,
    Deactivate,
}

#[derive(Debug)]
pub enum MenuItemMessage {
    Open,
    Close,
    Click,
}

#[derive(Debug)]
pub enum MessageBoxMessage {
    Open {
        title: Option<String>,
        text: Option<String>,
    },
    Close(MessageBoxResult),
}

#[derive(Debug)]
pub enum DecoratorMessage {
    Select(bool),
}

impl DecoratorMessage {
    fn make<M: 'static, C: 'static + Control<M, C>>(
        destination: Handle<UINode<M, C>>,
        msg: DecoratorMessage,
    ) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::Decorator(msg),
            destination,
        }
    }

    pub fn select<M: 'static, C: 'static + Control<M, C>>(
        destination: Handle<UINode<M, C>>,
        value: bool,
    ) -> UiMessage<M, C> {
        Self::make(destination, DecoratorMessage::Select(value))
    }
}

#[derive(Debug)]
pub enum ProgressBarMessage {
    Progress(f32),
}

impl ProgressBarMessage {
    fn make<M: 'static, C: 'static + Control<M, C>>(
        destination: Handle<UINode<M, C>>,
        msg: ProgressBarMessage,
    ) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::ProgressBar(msg),
            destination,
        }
    }

    pub fn progress<M: 'static, C: 'static + Control<M, C>>(
        destination: Handle<UINode<M, C>>,
        value: f32,
    ) -> UiMessage<M, C> {
        Self::make(destination, ProgressBarMessage::Progress(value))
    }
}

#[derive(Debug)]
pub enum UiMessageData<M: 'static, C: 'static + Control<M, C>> {
    Widget(WidgetMessage<M, C>),
    Button(ButtonMessage<M, C>),
    ScrollBar(ScrollBarMessage),
    CheckBox(CheckBoxMessage),
    Window(WindowMessage<M, C>),
    ListView(ListViewMessage<M, C>),
    Popup(PopupMessage<M, C>),
    ScrollViewer(ScrollViewerMessage<M, C>),
    Tree(TreeMessage<M, C>),
    TreeRoot(TreeRootMessage<M, C>),
    FileBrowser(FileBrowserMessage),
    FileSelector(FileSelectorMessage),
    TextBox(TextBoxMessage),
    NumericUpDown(NumericUpDownMessage),
    Vec3Editor(Vec3EditorMessage),
    Menu(MenuMessage),
    MenuItem(MenuItemMessage),
    MessageBox(MessageBoxMessage),
    Decorator(DecoratorMessage),
    Text(TextMessage),
    ScrollPanel(ScrollPanelMessage<M, C>),
    Tile(TileMessage<M, C>),
    ProgressBar(ProgressBarMessage),
    Image(ImageMessage),
    User(M),
}

/// Message is basic communication element that is used to deliver information to UI nodes
/// or to user code.
#[derive(Debug)]
pub struct UiMessage<M: 'static, C: 'static + Control<M, C>> {
    /// Useful flag to check if a message was already handled.
    pub handled: bool,

    /// Actual message data. Use pattern matching to get type specific data.
    ///
    /// # Notes
    ///
    /// This field should be read-only.
    pub data: UiMessageData<M, C>,

    /// Handle of node that will receive message. Please note that all nodes in hierarchy will
    /// also receive this message, order is defined by routing strategy.
    ///
    /// # Notes
    ///
    /// This field should be read-only.
    pub destination: Handle<UINode<M, C>>,
}

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy)]
pub enum ButtonState {
    Pressed,
    Released,
}

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u8),
}

pub enum OsEvent {
    MouseInput {
        button: MouseButton,
        state: ButtonState,
    },
    CursorMoved {
        position: Vec2,
    },
    KeyboardInput {
        button: KeyCode,
        state: ButtonState,
    },
    Character(char),
    KeyboardModifiers(KeyboardModifiers),
    MouseWheel(f32, f32),
}

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy)]
pub struct KeyboardModifiers {
    pub alt: bool,
    pub shift: bool,
    pub control: bool,
    pub system: bool,
}

impl Default for KeyboardModifiers {
    fn default() -> Self {
        Self {
            alt: false,
            shift: false,
            control: false,
            system: false,
        }
    }
}

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy)]
#[repr(u32)]
pub enum KeyCode {
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
    Key0,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    Escape,

    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,

    Snapshot,
    Scroll,
    Pause,

    Insert,
    Home,
    Delete,
    End,
    PageDown,
    PageUp,

    Left,
    Up,
    Right,
    Down,

    Backspace,
    Return,
    Space,

    Compose,

    Caret,

    Numlock,
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,

    AbntC1,
    AbntC2,
    Add,
    Apostrophe,
    Apps,
    At,
    Ax,
    Backslash,
    Calculator,
    Capital,
    Colon,
    Comma,
    Convert,
    Decimal,
    Divide,
    Equals,
    Grave,
    Kana,
    Kanji,
    LAlt,
    LBracket,
    LControl,
    LShift,
    LWin,
    Mail,
    MediaSelect,
    MediaStop,
    Minus,
    Multiply,
    Mute,
    MyComputer,
    NavigateForward,
    NavigateBackward,
    NextTrack,
    NoConvert,
    NumpadComma,
    NumpadEnter,
    NumpadEquals,
    OEM102,
    Period,
    PlayPause,
    Power,
    PrevTrack,
    RAlt,
    RBracket,
    RControl,
    RShift,
    RWin,
    Semicolon,
    Slash,
    Sleep,
    Stop,
    Subtract,
    Sysrq,
    Tab,
    Underline,
    Unlabeled,
    VolumeDown,
    VolumeUp,
    Wake,
    WebBack,
    WebFavorites,
    WebForward,
    WebHome,
    WebRefresh,
    WebSearch,
    WebStop,
    Yen,
    Copy,
    Paste,
    Cut,
}
