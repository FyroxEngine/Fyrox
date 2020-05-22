use crate::{
    popup::Popup,
    message::{
        UiMessage,
        OsEvent
    },
    draw::DrawingContext,
    image::Image,
    grid::Grid,
    check_box::CheckBox,
    canvas::Canvas,
    button::Button,
    border::Border,
    scroll_bar::ScrollBar,
    scroll_content_presenter::ScrollContentPresenter,
    scroll_viewer::ScrollViewer,
    stack_panel::StackPanel,
    tab_control::TabControl,
    text::Text,
    text_box::TextBox,
    window::Window,
    Control,
    UserInterface,
    widget::Widget,
    core::{
        math::{
            Rect,
            vec2::Vec2
        },
        pool::Handle,
    },
    dropdown_list::DropdownList,
    list_view::{ListView, ListViewItem},
    decorator::Decorator,
    NodeHandleMapping,
    progress_bar::ProgressBar,
    tree::{Tree, TreeRoot},
    file_browser::FileBrowser,
    dock::{
        DockingManager,
        Tile
    },
    vec::Vec3Editor,
    numeric::NumericUpDown
};
use std::ops::{Deref, DerefMut};

#[allow(clippy::large_enum_variant)]
pub enum UINode<M: 'static, C: 'static + Control<M, C>> {
    Border(Border<M, C>),
    Button(Button<M, C>),
    Canvas(Canvas<M, C>),
    CheckBox(CheckBox<M, C>),
    Grid(Grid<M, C>),
    Image(Image<M, C>),
    ListView(ListView<M, C>),
    ListViewItem(ListViewItem<M, C>),
    ScrollBar(ScrollBar<M, C>),
    ScrollContentPresenter(ScrollContentPresenter<M, C>),
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
    DockingManager(DockingManager<M, C>),
    Tile(Tile<M, C>),
    Vec3Editor(Vec3Editor<M, C>),
    NumericUpDown(NumericUpDown<M, C>),
    User(C)
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
            UINode::DockingManager(v) => v.$func($($args),*),
            UINode::Tile(v) => v.$func($($args),*),
            UINode::Vec3Editor(v) => v.$func($($args),*),
            UINode::NumericUpDown(v) => v.$func($($args),*),
        }
    };
}

macro_rules! static_dispatch_deref {
    ($self:ident) => {
        match $self {
            UINode::Border(v) => v,
            UINode::Button(v) => v,
            UINode::Canvas(v) => v,
            UINode::CheckBox(v) => v,
            UINode::Grid(v) => v,
            UINode::Image(v) => v,
            UINode::ScrollBar(v) => v,
            UINode::ScrollContentPresenter(v) => v,
            UINode::ScrollViewer(v) => v,
            UINode::StackPanel(v) => v,
            UINode::TabControl(v) => v,
            UINode::Text(v) => v,
            UINode::TextBox(v) => v,
            UINode::Window(v) => v,
            UINode::User(v) => v,
            UINode::Popup(v) => v,
            UINode::DropdownList(v) => v,
            UINode::ListView(v) => v,
            UINode::ListViewItem(v) => v,
            UINode::ProgressBar(v) => v,
            UINode::Decorator(v) => v,
            UINode::Tree(v) => v,
            UINode::TreeRoot(v) => v,
            UINode::FileBrowser(v) => v,
            UINode::DockingManager(v) => v,
            UINode::Tile(v) => v,
            UINode::Vec3Editor(v) => v,
            UINode::NumericUpDown(v) => v,
        }
    };
}

impl<M, C: 'static + Control<M, C>> Deref for UINode<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        static_dispatch_deref!(self)
    }
}

impl<M, C: 'static + Control<M, C>> DerefMut for UINode<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        static_dispatch_deref!(self)
    }
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for UINode<M, C> {
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

    fn handle_routed_message(&mut self, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        static_dispatch!(self, handle_routed_message, ui, message)
    }

    fn preview_message(&mut self, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        static_dispatch!(self, preview_message, ui, message)
    }

    fn handle_os_event(&mut self, self_handle: Handle<UINode<M, C>>, ui: &mut UserInterface<M, C>, event: &OsEvent) {
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

    fn handle_routed_message(&mut self, _: &mut UserInterface<(), StubNode>, _: &mut UiMessage<(), StubNode>) {
        unimplemented!()
    }
}

impl Deref for StubNode {
    type Target = Widget<(), StubNode>;

    fn deref(&self) -> &Self::Target {
        unimplemented!()
    }
}

impl DerefMut for StubNode{
    fn deref_mut(&mut self) -> &mut Self::Target {
        unimplemented!()
    }
}