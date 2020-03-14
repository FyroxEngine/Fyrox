use crate::{
    popup::Popup,
    message::{
        UiMessage,
        OsEvent
    },
    draw::DrawingContext,
    list_box::{
        ListBox,
        ListBoxItem
    },
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
    combobox::ComboBox,
    items_control::{ItemsControl, ItemContainer},
    decorator::Decorator,
    NodeHandleMapping,
    progress_bar::ProgressBar
};

pub enum UINode<M: 'static, C: 'static + Control<M, C>> {
    Border(Border<M, C>),
    Button(Button<M, C>),
    Canvas(Canvas<M, C>),
    CheckBox(CheckBox<M, C>),
    Grid(Grid<M, C>),
    Image(Image<M, C>),
    ItemsControl(ItemsControl<M, C>),
    ItemContainer(ItemContainer<M, C>),
    ListBox(ListBox<M, C>),
    ListBoxItem(ListBoxItem<M, C>),
    ScrollBar(ScrollBar<M, C>),
    ScrollContentPresenter(ScrollContentPresenter<M, C>),
    ScrollViewer(ScrollViewer<M, C>),
    StackPanel(StackPanel<M, C>),
    TabControl(TabControl<M, C>),
    Text(Text<M, C>),
    TextBox(TextBox<M, C>),
    Window(Window<M, C>),
    Popup(Popup<M, C>),
    ComboBox(ComboBox<M, C>),
    Decorator(Decorator<M, C>),
    ProgressBar(ProgressBar<M, C>),
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
            UINode::ListBox(v) => v.$func($($args),*),
            UINode::ListBoxItem(v) => v.$func($($args),*),
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
            UINode::ComboBox(v) => v.$func($($args),*),
            UINode::ItemsControl(v) => v.$func($($args),*),
            UINode::ItemContainer(v) => v.$func($($args),*),
            UINode::ProgressBar(v) => v.$func($($args),*),
            UINode::Decorator(v) => v.$func($($args),*),
        }
    };
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for UINode<M, C> {
    fn widget(&self) -> &Widget<M, C> {
        static_dispatch!(self, widget,)
    }

    fn widget_mut(&mut self) -> &mut Widget<M, C> {
        static_dispatch!(self, widget_mut,)
    }

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

    fn handle_message(&mut self, self_handle: Handle<UINode<M, C>>, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        static_dispatch!(self, handle_message, self_handle, ui, message)
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
    fn widget(&self) -> &Widget<(), StubNode> {
        unimplemented!()
    }

    fn widget_mut(&mut self) -> &mut Widget<(), StubNode> {
        unimplemented!()
    }

    fn raw_copy(&self) -> UINode<(), StubNode> {
        unimplemented!()
    }

    fn handle_message(&mut self, _: Handle<UINode<(), StubNode>>, _: &mut UserInterface<(), StubNode>, _: &mut UiMessage<(), StubNode>) {
        unimplemented!()
    }
}