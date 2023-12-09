//! A special container that is able to create widgets by their type UUID.
use crate::{
    bit::BitField,
    border::Border,
    button::Button,
    canvas::Canvas,
    check_box::CheckBox,
    color::gradient::{ColorGradientEditor, ColorGradientField, ColorPoint},
    color::{AlphaBar, ColorField, ColorPicker, HueBar, SaturationBrightnessField},
    core::{parking_lot::Mutex, uuid::Uuid, TypeUuidProvider},
    curve::CurveEditor,
    decorator::Decorator,
    dock::{DockingManager, Tile},
    dropdown_list::DropdownList,
    expander::Expander,
    file_browser::{FileBrowser, FileSelector, FileSelectorField},
    grid::Grid,
    image::Image,
    inspector::Inspector,
    key::{HotKeyEditor, KeyBindingEditor},
    list_view::{ListView, ListViewItem},
    menu::{Menu, MenuItem},
    messagebox::MessageBox,
    nine_patch::NinePatch,
    numeric::NumericUpDown,
    path::PathEditor,
    progress_bar::ProgressBar,
    range::RangeEditor,
    rect::RectEditor,
    scroll_bar::ScrollBar,
    scroll_panel::ScrollPanel,
    scroll_viewer::ScrollViewer,
    searchbar::SearchBar,
    stack_panel::StackPanel,
    tab_control::TabControl,
    text::Text,
    text_box::TextBox,
    tree::{Tree, TreeRoot},
    uuid::UuidEditor,
    vec::VecEditor,
    vector_image::VectorImage,
    window::Window,
    wrap_panel::WrapPanel,
    Control, UiNode,
};
use fxhash::FxHashMap;

/// A simple type alias for boxed widget constructor.
pub type WidgetConstructor = Box<dyn FnMut() -> UiNode + Send>;

/// A special container that is able to create widgets by their type UUID.
#[derive(Default)]
pub struct WidgetConstructorContainer {
    map: Mutex<FxHashMap<Uuid, WidgetConstructor>>,
}

impl WidgetConstructorContainer {
    /// Creates default widget constructor container with constructors for built-in widgets.
    pub fn new() -> Self {
        let container = WidgetConstructorContainer::default();

        container.add::<BitField<u8>>();
        container.add::<BitField<i8>>();
        container.add::<BitField<u16>>();
        container.add::<BitField<i16>>();
        container.add::<BitField<u32>>();
        container.add::<BitField<i32>>();
        container.add::<BitField<u64>>();
        container.add::<BitField<i64>>();

        container.add::<Border>();
        container.add::<Button>();
        container.add::<Canvas>();
        container.add::<CheckBox>();
        container.add::<Decorator>();
        container.add::<DropdownList>();
        container.add::<Expander>();
        container.add::<Grid>();
        container.add::<Image>();
        container.add::<HotKeyEditor>();
        container.add::<KeyBindingEditor>();
        container.add::<ListViewItem>();
        container.add::<ListView>();
        container.add::<Menu>();
        container.add::<MenuItem>();
        container.add::<MessageBox>();
        container.add::<NinePatch>();

        container.add::<NumericUpDown<u8>>();
        container.add::<NumericUpDown<i8>>();
        container.add::<NumericUpDown<u16>>();
        container.add::<NumericUpDown<i16>>();
        container.add::<NumericUpDown<u32>>();
        container.add::<NumericUpDown<i32>>();
        container.add::<NumericUpDown<u64>>();
        container.add::<NumericUpDown<i64>>();
        container.add::<NumericUpDown<f32>>();
        container.add::<NumericUpDown<f64>>();

        container.add::<RangeEditor<u8>>();
        container.add::<RangeEditor<i8>>();
        container.add::<RangeEditor<u16>>();
        container.add::<RangeEditor<i16>>();
        container.add::<RangeEditor<u32>>();
        container.add::<RangeEditor<i32>>();
        container.add::<RangeEditor<u64>>();
        container.add::<RangeEditor<i64>>();
        container.add::<RangeEditor<f32>>();
        container.add::<RangeEditor<f64>>();

        container.add::<RectEditor<u8>>();
        container.add::<RectEditor<i8>>();
        container.add::<RectEditor<u16>>();
        container.add::<RectEditor<i16>>();
        container.add::<RectEditor<u32>>();
        container.add::<RectEditor<i32>>();
        container.add::<RectEditor<u64>>();
        container.add::<RectEditor<i64>>();
        container.add::<RectEditor<f32>>();
        container.add::<RectEditor<f64>>();

        container.add::<PathEditor>();
        container.add::<ProgressBar>();
        container.add::<ScrollBar>();
        container.add::<ScrollPanel>();
        container.add::<ScrollViewer>();
        container.add::<SearchBar>();
        container.add::<StackPanel>();
        container.add::<TabControl>();
        container.add::<Tree>();
        container.add::<TreeRoot>();
        container.add::<UuidEditor>();

        container.add::<VectorImage>();
        container.add::<Window>();
        container.add::<WrapPanel>();
        container.add::<ColorGradientField>();
        container.add::<ColorGradientEditor>();
        container.add::<ColorPoint>();

        container.add::<AlphaBar>();
        container.add::<HueBar>();
        container.add::<SaturationBrightnessField>();
        container.add::<ColorPicker>();
        container.add::<ColorField>();

        container.add::<CurveEditor>();
        container.add::<DockingManager>();
        container.add::<Tile>();

        container.add::<FileBrowser>();
        container.add::<FileSelector>();
        container.add::<FileSelectorField>();

        container.add::<Inspector>();

        container.add::<VecEditor<u8, 2>>();
        container.add::<VecEditor<i8, 2>>();
        container.add::<VecEditor<u16, 2>>();
        container.add::<VecEditor<i16, 2>>();
        container.add::<VecEditor<u32, 2>>();
        container.add::<VecEditor<i32, 2>>();
        container.add::<VecEditor<u64, 2>>();
        container.add::<VecEditor<i64, 2>>();
        container.add::<VecEditor<f32, 2>>();
        container.add::<VecEditor<f64, 2>>();

        container.add::<VecEditor<u8, 3>>();
        container.add::<VecEditor<i8, 3>>();
        container.add::<VecEditor<u16, 3>>();
        container.add::<VecEditor<i16, 3>>();
        container.add::<VecEditor<u32, 3>>();
        container.add::<VecEditor<i32, 3>>();
        container.add::<VecEditor<u64, 3>>();
        container.add::<VecEditor<i64, 3>>();
        container.add::<VecEditor<f32, 3>>();
        container.add::<VecEditor<f64, 3>>();

        container.add::<VecEditor<u8, 4>>();
        container.add::<VecEditor<i8, 4>>();
        container.add::<VecEditor<u16, 4>>();
        container.add::<VecEditor<i16, 4>>();
        container.add::<VecEditor<u32, 4>>();
        container.add::<VecEditor<i32, 4>>();
        container.add::<VecEditor<u64, 4>>();
        container.add::<VecEditor<i64, 4>>();
        container.add::<VecEditor<f32, 4>>();
        container.add::<VecEditor<f64, 4>>();

        container.add::<Text>();
        container.add::<TextBox>();

        container
    }

    /// Adds new type constructor for a given type and return previous constructor for the type
    /// (if any).
    pub fn add<T>(&self)
    where
        T: TypeUuidProvider + Control + Default,
    {
        let previous = self
            .map
            .lock()
            .insert(T::type_uuid(), Box::new(|| UiNode::new(T::default())));

        assert!(previous.is_none());
    }

    /// Adds custom type constructor.
    pub fn add_custom(&self, type_uuid: Uuid, constructor: WidgetConstructor) {
        self.map.lock().insert(type_uuid, constructor);
    }

    /// Unregisters type constructor.
    pub fn remove(&self, type_uuid: Uuid) {
        self.map.lock().remove(&type_uuid);
    }

    /// Makes an attempt to create a widget using provided type UUID. It may fail if there is no
    /// widget constructor for specified type UUID.
    pub fn try_create(&self, type_uuid: &Uuid) -> Option<UiNode> {
        self.map.lock().get_mut(type_uuid).map(|c| (c)())
    }

    /// Returns total amount of constructors.
    pub fn len(&self) -> usize {
        self.map.lock().len()
    }

    /// Returns true if the container is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
