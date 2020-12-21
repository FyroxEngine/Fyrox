use crate::{
    border::Border,
    button::Button,
    canvas::Canvas,
    check_box::CheckBox,
    color::{AlphaBar, ColorField, ColorPicker, HueBar, SaturationBrightnessField},
    core::{algebra::Vector2, define_is_as, math::Rect, pool::Handle},
    decorator::Decorator,
    dock::{DockingManager, Tile},
    draw::DrawingContext,
    dropdown_list::DropdownList,
    expander::Expander,
    file_browser::{FileBrowser, FileSelector},
    grid::Grid,
    image::Image,
    list_view::{ListView, ListViewItem},
    menu::{Menu, MenuItem},
    message::{MessageData, OsEvent, UiMessage},
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
    vector_image::VectorImage,
    widget::Widget,
    window::Window,
    wrap_panel::WrapPanel,
    Control, NodeHandleMapping, UserInterface,
};
use std::ops::{Deref, DerefMut};

#[derive(Clone)]
pub enum UINode<M: MessageData, C: Control<M, C>> {
    Border(Border<M, C>),
    Button(Button<M, C>),
    Canvas(Canvas<M, C>),
    ColorPicker(ColorPicker<M, C>),
    ColorField(ColorField<M, C>),
    HueBar(HueBar<M, C>),
    AlphaBar(AlphaBar<M, C>),
    SaturationBrightnessField(SaturationBrightnessField<M, C>),
    CheckBox(CheckBox<M, C>),
    Grid(Grid<M, C>),
    Image(Image<M, C>),
    ListView(ListView<M, C>),
    ListViewItem(ListViewItem<M, C>),
    ScrollBar(ScrollBar<M, C>),
    ScrollPanel(ScrollPanel<M, C>),
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
    VectorImage(VectorImage<M, C>),
    Expander(Expander<M, C>),
    User(C),
}

macro_rules! static_dispatch {
    ($self:ident, $func:ident, $($args:expr),*) => {
        match $self {
            UINode::Border(v) => v.$func($($args),*),
            UINode::Button(v) => v.$func($($args),*),
            UINode::Canvas(v) => v.$func($($args),*),
            UINode::ColorPicker(v) => v.$func($($args),*),
            UINode::ColorField(v) => v.$func($($args),*),
            UINode::HueBar(v) => v.$func($($args),*),
            UINode::AlphaBar(v) => v.$func($($args),*),
            UINode::SaturationBrightnessField(v) => v.$func($($args),*),
            UINode::CheckBox(v) => v.$func($($args),*),
            UINode::Grid(v) => v.$func($($args),*),
            UINode::Image(v) => v.$func($($args),*),
            UINode::ScrollBar(v) => v.$func($($args),*),
            UINode::ScrollPanel(v) => v.$func($($args),*),
            UINode::ScrollViewer(v) => v.$func($($args),*),
            UINode::StackPanel(v) => v.$func($($args),*),
            UINode::TabControl(v) => v.$func($($args),*),
            UINode::Text(v) => v.$func($($args),*),
            UINode::TextBox(v) => v.$func($($args),*),
            UINode::Window(v) => v.$func($($args),*),
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
            UINode::VectorImage(v) => v.$func($($args),*),
            UINode::Expander(v) => v.$func($($args),*),
            UINode::User(v) => v.$func($($args),*),
        }
    };
}

impl<M: MessageData, C: Control<M, C>> Deref for UINode<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        static_dispatch!(self, deref,)
    }
}

impl<M: MessageData, C: Control<M, C>> DerefMut for UINode<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        static_dispatch!(self, deref_mut,)
    }
}

impl<M: MessageData, C: Control<M, C>> UINode<M, C> {
    define_is_as!(UINode : Border -> ref Border<M, C> => fn is_border, fn as_border, fn as_border_mut);
    define_is_as!(UINode : Button -> ref Button<M, C> => fn is_button, fn as_button, fn as_button_mut);
    define_is_as!(UINode : Canvas -> ref Canvas<M, C> => fn is_canvas, fn as_canvas, fn as_canvas_mut);
    define_is_as!(UINode : ColorPicker -> ref ColorPicker<M, C> => fn is_color_picker, fn as_color_picker, fn as_color_picker_mut);
    define_is_as!(UINode : HueBar -> ref HueBar<M, C> => fn is_hue_bar, fn as_hue_bar, fn as_hue_bar_mut);
    define_is_as!(UINode : SaturationBrightnessField -> ref SaturationBrightnessField<M, C> => fn is_saturation_brightness_field, fn as_saturation_brightness_field, fn as_saturation_brightness_field_mut);
    define_is_as!(UINode : CheckBox -> ref CheckBox<M, C> => fn is_check_box, fn as_check_box, fn as_check_box_mut);
    define_is_as!(UINode : Grid -> ref Grid<M, C> => fn is_grid, fn as_grid, fn as_grid_mut);
    define_is_as!(UINode : Image -> ref Image<M, C> => fn is_image, fn as_image, fn as_image_mut);
    define_is_as!(UINode : ScrollBar -> ref ScrollBar<M, C> => fn is_scroll_bar, fn as_scroll_bar, fn as_scroll_bar_mut);
    define_is_as!(UINode : ScrollPanel -> ref ScrollPanel<M, C> => fn is_scroll_panel, fn as_scroll_panel, fn as_scroll_panel_mut);
    define_is_as!(UINode : ScrollViewer -> ref ScrollViewer<M, C> => fn is_scroll_viewer, fn as_scroll_viewer, fn as_scroll_viewer_mut);
    define_is_as!(UINode : StackPanel -> ref StackPanel<M, C> => fn is_stack_panel, fn as_stack_panel, fn as_stack_panel_mut);
    define_is_as!(UINode : TabControl -> ref TabControl<M, C> => fn is_tab_control, fn as_tab_control, fn as_tab_control_mut);
    define_is_as!(UINode : Text -> ref Text<M, C> => fn is_text, fn as_text, fn as_text_mut);
    define_is_as!(UINode : TextBox -> ref TextBox<M, C> => fn is_text_box, fn as_text_box, fn as_text_box_mut);
    define_is_as!(UINode : Window -> ref Window<M, C> => fn is_window, fn as_window, fn as_window_mut);
    define_is_as!(UINode : Popup -> ref Popup<M, C> => fn is_popup, fn as_popup, fn as_popup_mut);
    define_is_as!(UINode : DropdownList -> ref DropdownList<M, C> => fn is_dropdown_list, fn as_dropdown_list, fn as_dropdown_list_mut);
    define_is_as!(UINode : ListView -> ref ListView<M, C> => fn is_list_view, fn as_list_view, fn as_list_view_mut);
    define_is_as!(UINode : ListViewItem -> ref ListViewItem<M, C> => fn is_list_view_item, fn as_list_view_item, fn as_list_view_item_mut);
    define_is_as!(UINode : ProgressBar -> ref ProgressBar<M, C> => fn is_progress_bar, fn as_progress_bar, fn as_progress_bar_mut);
    define_is_as!(UINode : Decorator -> ref Decorator<M, C> => fn is_decorator, fn as_decorator, fn as_decorator_mut);
    define_is_as!(UINode : Tree -> ref Tree<M, C> => fn is_tree, fn as_tree, fn as_tree_mut);
    define_is_as!(UINode : TreeRoot -> ref TreeRoot<M, C> => fn is_tree_root, fn as_tree_root, fn as_tree_root_mut);
    define_is_as!(UINode : FileBrowser -> ref FileBrowser<M, C> => fn is_file_browser, fn as_file_browser, fn as_file_browser_mut);
    define_is_as!(UINode : FileSelector -> ref FileSelector<M, C> => fn is_file_selector, fn as_file_selector, fn as_file_selector_mut);
    define_is_as!(UINode : DockingManager -> ref DockingManager<M, C> => fn is_docking_manager, fn as_docking_manager, fn as_docking_manager_mut);
    define_is_as!(UINode : Tile -> ref Tile<M, C> => fn is_tile, fn as_tile, fn as_tile_mut);
    define_is_as!(UINode : Vec3Editor -> ref Vec3Editor<M, C> => fn is_vec3_editor, fn as_vec3_editor, fn as_vec3_editor_mut);
    define_is_as!(UINode : NumericUpDown -> ref NumericUpDown<M, C> => fn is_numeric_up_down, fn as_numeric_up_down, fn as_numeric_up_down_mut);
    define_is_as!(UINode : Menu -> ref Menu<M, C> => fn is_menu, fn as_menu, fn as_menu_mut);
    define_is_as!(UINode : MenuItem -> ref MenuItem<M, C> => fn is_menu_item, fn as_menu_item, fn as_menu_item_mut);
    define_is_as!(UINode : MessageBox -> ref MessageBox<M, C> => fn is_message_box, fn as_message_box, fn as_message_box_mut);
    define_is_as!(UINode : WrapPanel -> ref WrapPanel<M, C> => fn is_wrap_panel, fn as_wrap_panel, fn as_wrap_panel_mut);
    define_is_as!(UINode : VectorImage -> ref VectorImage<M, C> => fn is_vector_image, fn as_vector_image, fn as_vector_image_mut);
    define_is_as!(UINode : Expander -> ref Expander<M, C> => fn is_expander, fn as_expander, fn as_expander_mut);
    define_is_as!(UINode : User -> ref C => fn is_user, fn as_user, fn as_user_mut);
}

impl<M: MessageData, C: Control<M, C>> Control<M, C> for UINode<M, C> {
    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        static_dispatch!(self, resolve, node_map);
    }

    fn measure_override(
        &self,
        ui: &UserInterface<M, C>,
        available_size: Vector2<f32>,
    ) -> Vector2<f32> {
        static_dispatch!(self, measure_override, ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface<M, C>, final_size: Vector2<f32>) -> Vector2<f32> {
        static_dispatch!(self, arrange_override, ui, final_size)
    }

    fn arrange(&self, ui: &UserInterface<M, C>, final_rect: &Rect<f32>) {
        static_dispatch!(self, arrange, ui, final_rect)
    }

    fn measure(&self, ui: &UserInterface<M, C>, available_size: Vector2<f32>) {
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

    fn preview_message(&self, ui: &UserInterface<M, C>, message: &mut UiMessage<M, C>) {
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

#[derive(Debug, Clone, PartialEq)]
pub enum StubNode {}

impl MessageData for () {}

impl Control<(), StubNode> for StubNode {
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
