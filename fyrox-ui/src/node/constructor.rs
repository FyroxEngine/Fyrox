// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! A special container that is able to create widgets by their type UUID.
use crate::{
    absm::{AbsmEventProvider, AnimationBlendingStateMachine},
    animation::AnimationPlayer,
    bit::BitField,
    border::Border,
    button::Button,
    canvas::Canvas,
    check_box::CheckBox,
    color::gradient::{ColorGradientEditor, ColorGradientField, ColorPoint},
    color::{AlphaBar, ColorField, ColorPicker, HueBar, SaturationBrightnessField},
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
    menu::{ContextMenu, Menu, MenuItem},
    messagebox::MessageBox,
    nine_patch::NinePatch,
    numeric::NumericUpDown,
    path::PathEditor,
    popup::Popup,
    progress_bar::ProgressBar,
    range::RangeEditor,
    rect::RectEditor,
    screen::Screen,
    scroll_bar::ScrollBar,
    scroll_panel::ScrollPanel,
    scroll_viewer::ScrollViewer,
    searchbar::SearchBar,
    selector::Selector,
    stack_panel::StackPanel,
    tab_control::TabControl,
    text::Text,
    text_box::TextBox,
    toggle::ToggleButton,
    tree::{Tree, TreeRoot},
    uuid::UuidEditor,
    vec::VecEditor,
    vector_image::VectorImage,
    window::Window,
    wrap_panel::WrapPanel,
    UiNode, UserInterface,
};
use fyrox_graph::constructor::{GraphNodeConstructor, GraphNodeConstructorContainer};

/// Node constructor.
pub type WidgetConstructor = GraphNodeConstructor<UiNode, UserInterface>;

/// A special container that is able to create widgets by their type UUID.
pub type WidgetConstructorContainer = GraphNodeConstructorContainer<UiNode, UserInterface>;

/// Creates default widget constructor container with constructors for built-in widgets.
pub fn new_widget_constructor_container() -> WidgetConstructorContainer {
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
    container.add::<ToggleButton>();
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
    container.add::<ContextMenu>();
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

    container.add::<Popup>();

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
    container.add::<Screen>();
    container.add::<AnimationPlayer>();
    container.add::<AnimationBlendingStateMachine>();
    container.add::<AbsmEventProvider>();
    container.add::<Selector>();

    container
}
