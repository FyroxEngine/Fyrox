//! A special container that is able to create widgets by their type UUID.
use crate::{
    border::Border,
    button::Button,
    canvas::Canvas,
    check_box::CheckBox,
    color::gradient::{ColorGradientEditor, ColorGradientField, ColorPoint},
    core::{parking_lot::Mutex, uuid::Uuid, TypeUuidProvider},
    decorator::Decorator,
    dropdown_list::DropdownList,
    expander::Expander,
    grid::Grid,
    image::Image,
    key::{HotKeyEditor, KeyBindingEditor},
    list_view::{ListView, ListViewItem},
    menu::{Menu, MenuItem},
    messagebox::MessageBox,
    nine_patch::NinePatch,
    path::PathEditor,
    progress_bar::ProgressBar,
    scroll_bar::ScrollBar,
    scroll_panel::ScrollPanel,
    scroll_viewer::ScrollViewer,
    searchbar::SearchBar,
    stack_panel::StackPanel,
    tab_control::TabControl,
    tree::{Tree, TreeRoot},
    uuid::UuidEditor,
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

        // container.add::<BitField<>>(); TODO
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
        // container.add::<NumericUpDown<>>(); TODO
        container.add::<PathEditor>();
        container.add::<ProgressBar>();
        // container.add::<RangeEditor<>>(); TODO
        container.add::<ScrollBar>();
        container.add::<ScrollPanel>();
        container.add::<ScrollViewer>();
        container.add::<SearchBar>();
        container.add::<StackPanel>();
        container.add::<TabControl>();
        // container.add::<Text>(); TODO
        // container.add::<TextBox>(); TODO
        container.add::<Tree>();
        container.add::<TreeRoot>();
        container.add::<UuidEditor>();
        // container.add::<VecEditor<>>(); TODO
        container.add::<VectorImage>();
        container.add::<Window>();
        container.add::<WrapPanel>();
        container.add::<ColorGradientField>();
        container.add::<ColorGradientEditor>();
        container.add::<ColorPoint>();

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
