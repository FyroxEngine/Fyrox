use crate::file_browser::FileSelector;
use crate::{
    border::Border,
    button::Button,
    canvas::Canvas,
    check_box::CheckBox,
    core::{
        math::{vec2::Vec2, Rect},
        pool::Handle,
    },
    decorator::Decorator,
    dock::{DockingManager, Tile},
    draw::DrawingContext,
    dropdown_list::DropdownList,
    file_browser::FileBrowser,
    grid::Grid,
    image::Image,
    list_view::{ListView, ListViewItem},
    menu::{Menu, MenuItem},
    message::{OsEvent, UiMessage},
    messagebox::MessageBox,
    numeric::NumericUpDown,
    popup::Popup,
    progress_bar::ProgressBar,
    scroll_bar::ScrollBar,
    scroll_panel::ScrollPanel,
    scroll_viewer::ScrollViewer,
    stack_panel::StackPanel,
    tab_control::TabControl,
    text::Text,
    text_box::TextBox,
    tree::{Tree, TreeRoot},
    vec::Vec3Editor,
    widget::Widget,
    window::Window,
    wrap_panel::WrapPanel,
    Control, NodeHandleMapping, UserInterface,
};
use std::ops::{Deref, DerefMut};

#[allow(clippy::large_enum_variant)]
pub enum UINode<M: 'static + std::fmt::Debug, C: 'static + Control<M, C>> {
    Border(Border<M, C>),
    Button(Button<M, C>),
    Canvas(Canvas<M, C>),
    CheckBox(CheckBox<M, C>),
    Grid(Grid<M, C>),
    Image(Image<M, C>),
    ListView(ListView<M, C>),
    ListViewItem(ListViewItem<M, C>),
    ScrollBar(ScrollBar<M, C>),
    ScrollContentPresenter(ScrollPanel<M, C>),
    ScrollViewer(ScrollViewer<M, C>),
    StackPanel(StackPanel<M, C>),
    TabControl(TabControl<M, C>),
    Text(Text<M, C>),
    TextBox(TextBox<M, C>),
    Window(Window<M, C>),
    Popup(Popup<M, C>),
    DropdownList(DropdownList<M, C>),
    Decorator(Decorator<M, C>),
    ProgressBar(ProgressBar<M, C>),
    Tree(Tree<M, C>),
    TreeRoot(TreeRoot<M, C>),
    FileBrowser(FileBrowser<M, C>),
    FileSelector(FileSelector<M, C>),
    DockingManager(DockingManager<M, C>),
    Tile(Tile<M, C>),
    Vec3Editor(Vec3Editor<M, C>),
    NumericUpDown(NumericUpDown<M, C>),
    Menu(Menu<M, C>),
    MenuItem(MenuItem<M, C>),
    MessageBox(MessageBox<M, C>),
    WrapPanel(WrapPanel<M, C>),
    User(C),
}

macro_rules! static_dispatch {
    ($self:ident, $func:ident, $($args:expr),*) => {
        match $self {
            UINode::Border(v) => v.$func($($args),*),
            UINode::Button(v) => v.$func($($args),*),
            UINode::Canvas(v) => v.$func($($args),*),
            UINode::CheckBox(v) => v.$func($($args),*),
            UINode::Grid(v) => v.$func($($args),*),
            UINode::Image(v) => v.$func($($args),*),
            UINode::ScrollBar(v) => v.$func($($args),*),
            UINode::ScrollContentPresenter(v) => v.$func($($args),*),
            UINode::ScrollViewer(v) => v.$func($($args),*),
            UINode::StackPanel(v) => v.$func($($args),*),
            UINode::TabControl(v) => v.$func($($args),*),
            UINode::Text(v) => v.$func($($args),*),
            UINode::TextBox(v) => v.$func($($args),*),
            UINode::Window(v) => v.$func($($args),*),
            UINode::User(v) => v.$func($($args),*),
            UINode::Popup(v) => v.$func($($args),*),
            UINode::DropdownList(v) => v.$func($($args),*),
            UINode::ListView(v) => v.$func($($args),*),
            UINode::ListViewItem(v) => v.$func($($args),*),
            UINode::ProgressBar(v) => v.$func($($args),*),
            UINode::Decorator(v) => v.$func($($args),*),
            UINode::Tree(v) => v.$func($($args),*),
            UINode::TreeRoot(v) => v.$func($($args),*),
            UINode::FileBrowser(v) => v.$func($($args),*),
            UINode::FileSelector(v) => v.$func($($args),*),
            UINode::DockingManager(v) => v.$func($($args),*),
            UINode::Tile(v) => v.$func($($args),*),
            UINode::Vec3Editor(v) => v.$func($($args),*),
            UINode::NumericUpDown(v) => v.$func($($args),*),
            UINode::Menu(v) => v.$func($($args),*),
            UINode::MenuItem(v) => v.$func($($args),*),
            UINode::MessageBox(v) => v.$func($($args),*),
            UINode::WrapPanel(v) => v.$func($($args),*),
        }
    };
}

impl<M: 'static + std::fmt::Debug, C: 'static + Control<M, C>> Deref for UINode<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        static_dispatch!(self, deref,)
    }
}

impl<M: 'static + std::fmt::Debug, C: 'static + Control<M, C>> DerefMut for UINode<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        static_dispatch!(self, deref_mut,)
    }
}

impl<M: 'static + std::fmt::Debug, C: 'static + Control<M, C>> Control<M, C> for UINode<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        static_dispatch!(self, raw_copy,)
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        static_dispatch!(self, resolve, node_map);
    }

    fn measure_override(&self, ui: &UserInterface<M, C>, available_size: Vec2) -> Vec2 {
        static_dispatch!(self, measure_override, ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface<M, C>, final_size: Vec2) -> Vec2 {
        static_dispatch!(self, arrange_override, ui, final_size)
    }

    fn arrange(&self, ui: &UserInterface<M, C>, final_rect: &Rect<f32>) {
        static_dispatch!(self, arrange, ui, final_rect)
    }

    fn measure(&self, ui: &UserInterface<M, C>, available_size: Vec2) {
        static_dispatch!(self, measure, ui, available_size)
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        static_dispatch!(self, draw, drawing_context)
    }

    fn update(&mut self, dt: f32) {
        static_dispatch!(self, update, dt)
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        static_dispatch!(self, handle_routed_message, ui, message)
    }

    fn preview_message(&mut self, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        static_dispatch!(self, preview_message, ui, message)
    }

    fn handle_os_event(
        &mut self,
        self_handle: Handle<UINode<M, C>>,
        ui: &mut UserInterface<M, C>,
        event: &OsEvent,
    ) {
        static_dispatch!(self, handle_os_event, self_handle, ui, event)
    }

    fn remove_ref(&mut self, handle: Handle<UINode<M, C>>) {
        static_dispatch!(self, remove_ref, handle)
    }
}

#[derive(Debug)]
pub enum StubNode {}

impl Control<(), StubNode> for StubNode {
    fn raw_copy(&self) -> UINode<(), StubNode> {
        unimplemented!()
    }

    fn handle_routed_message(
        &mut self,
        _: &mut UserInterface<(), StubNode>,
        _: &mut UiMessage<(), StubNode>,
    ) {
        unimplemented!()
    }
}

impl Deref for StubNode {
    type Target = Widget<(), StubNode>;

    fn deref(&self) -> &Self::Target {
        unimplemented!()
    }
}

impl DerefMut for StubNode {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unimplemented!()
    }
}
