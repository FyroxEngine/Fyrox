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

//! [`Menu`] and [`MenuItem`] widgets are used to create menu chains like standard `File`, `Edit`, etc. menus. See doc
//! of respective widget for more info and usage examples.

#![warn(missing_docs)]

use crate::style::resource::StyleResourceExt;
use crate::style::Style;
use crate::{
    border::BorderBuilder,
    brush::Brush,
    core::{
        algebra::Vector2, color::Color, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        uuid_provider, variable::InheritableVariable, visitor::prelude::*,
    },
    decorator::{DecoratorBuilder, DecoratorMessage},
    define_constructor,
    draw::DrawingContext,
    grid::{Column, GridBuilder, Row},
    message::{ButtonState, KeyCode, MessageDirection, OsEvent, UiMessage},
    popup::{Placement, Popup, PopupBuilder, PopupMessage},
    stack_panel::StackPanelBuilder,
    text::TextBuilder,
    utils::{make_arrow_primitives, ArrowDirection},
    vector_image::VectorImageBuilder,
    widget,
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, HorizontalAlignment, Orientation, RestrictionEntry, Thickness, UiNode,
    UserInterface, VerticalAlignment,
};

use fyrox_graph::{
    constructor::{ConstructorProvider, GraphNodeConstructor},
    BaseSceneGraph, SceneGraph, SceneGraphNode,
};
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
    sync::mpsc::Sender,
};

/// A set of messages that can be used to manipulate a [`Menu`] widget at runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuMessage {
    /// Activates the menu so it captures mouse input by itself and allows you to open menu item by a simple mouse
    /// hover.
    Activate,
    /// Deactivates the menu.
    Deactivate,
}

impl MenuMessage {
    define_constructor!(
        /// Creates [`MenuMessage::Activate`] message.
        MenuMessage:Activate => fn activate(), layout: false
    );
    define_constructor!(
        /// Creates [`MenuMessage::Deactivate`] message.
        MenuMessage:Deactivate => fn deactivate(), layout: false
    );
}

/// A predicate that is used to sort menu items.
#[derive(Clone)]
pub struct SortingPredicate(
    pub Arc<dyn Fn(&MenuItemContent, &MenuItemContent, &UserInterface) -> Ordering + Send + Sync>,
);

impl SortingPredicate {
    /// Creates new sorting predicate.
    pub fn new<F>(func: F) -> Self
    where
        F: Fn(&MenuItemContent, &MenuItemContent, &UserInterface) -> Ordering
            + Send
            + Sync
            + 'static,
    {
        Self(Arc::new(func))
    }

    /// Creates new sorting predicate that sorts menu items by their textual content. This predicate
    /// won't work with custom menu item content!
    pub fn sort_by_text() -> Self {
        Self::new(|a, b, _| {
            if let MenuItemContent::Text { text: a_text, .. } = a {
                if let MenuItemContent::Text { text: b_text, .. } = b {
                    return a_text.cmp(b_text);
                }
            }

            if let MenuItemContent::TextCentered(a_text) = a {
                if let MenuItemContent::TextCentered(b_text) = b {
                    return a_text.cmp(b_text);
                }
            }

            Ordering::Equal
        })
    }
}

impl Debug for SortingPredicate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "SortingPredicate")
    }
}

impl PartialEq for SortingPredicate {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0.as_ref(), other.0.as_ref())
    }
}

/// A set of messages that can be used to manipulate a [`MenuItem`] widget at runtime.
#[derive(Debug, Clone, PartialEq)]
pub enum MenuItemMessage {
    /// Opens the menu item's popup with inner items.
    Open,
    /// Closes the menu item's popup with inner items.
    Close {
        /// Defines, whether the item should be deselected when closed or not.
        deselect: bool,
    },
    /// The message is generated by a menu item when it is clicked.
    Click,
    /// Adds a new item to the menu item.
    AddItem(Handle<UiNode>),
    /// Removes an item from the menu item.
    RemoveItem(Handle<UiNode>),
    /// Sets the new items of the menu item.
    Items(Vec<Handle<UiNode>>),
    /// Selects/deselects the item.
    Select(bool),
    /// Sorts menu items by the given predicate.
    Sort(SortingPredicate),
}

impl MenuItemMessage {
    define_constructor!(
        /// Creates [`MenuItemMessage::Open`] message.
        MenuItemMessage:Open => fn open(), layout: false
    );
    define_constructor!(
          /// Creates [`MenuItemMessage::Close`] message.
        MenuItemMessage:Close => fn close(deselect: bool), layout: false
    );
    define_constructor!(
          /// Creates [`MenuItemMessage::Click`] message.
        MenuItemMessage:Click => fn click(), layout: false
    );
    define_constructor!(
          /// Creates [`MenuItemMessage::AddItem`] message.
        MenuItemMessage:AddItem => fn add_item(Handle<UiNode>), layout: false
    );
    define_constructor!(
          /// Creates [`MenuItemMessage::RemoveItem`] message.
        MenuItemMessage:RemoveItem => fn remove_item(Handle<UiNode>), layout: false
    );
    define_constructor!(
          /// Creates [`MenuItemMessage::Items`] message.
        MenuItemMessage:Items => fn items(Vec<Handle<UiNode>>), layout: false
    );
    define_constructor!(
          /// Creates [`MenuItemMessage::Select`] message.
        MenuItemMessage:Select => fn select(bool), layout: false
    );
    define_constructor!(
          /// Creates [`MenuItemMessage::Sort`] message.
        MenuItemMessage:Sort => fn sort(SortingPredicate), layout: false
    );
}

/// Menu widget is a root widget of an arbitrary menu hierarchy. An example could be "standard" menu strip with `File`, `Edit`, `View`, etc.
/// items. Menu widget can contain any number of children item (`File`, `Edit` in the previous example). These items should be [`MenuItem`]
/// widgets, however you can use any widget type (for example - to create some sort of a separator).
///
/// ## Examples
///
/// The next example creates a menu with the following structure:
///
/// ```text
/// |  File |  Edit |
/// |--Save |--Undo
/// |--Load |--Redo
/// ```
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle,
/// #     menu::{MenuBuilder, MenuItemBuilder, MenuItemContent},
/// #     widget::WidgetBuilder,
/// #     BuildContext, UiNode,
/// # };
/// #
/// fn create_menu(ctx: &mut BuildContext) -> Handle<UiNode> {
///     MenuBuilder::new(WidgetBuilder::new())
///         .with_items(vec![
///             MenuItemBuilder::new(WidgetBuilder::new())
///                 .with_content(MenuItemContent::text_no_arrow("File"))
///                 .with_items(vec![
///                     MenuItemBuilder::new(WidgetBuilder::new())
///                         .with_content(MenuItemContent::text_no_arrow("Save"))
///                         .build(ctx),
///                     MenuItemBuilder::new(WidgetBuilder::new())
///                         .with_content(MenuItemContent::text_no_arrow("Load"))
///                         .build(ctx),
///                 ])
///                 .build(ctx),
///             MenuItemBuilder::new(WidgetBuilder::new())
///                 .with_content(MenuItemContent::text_no_arrow("Edit"))
///                 .with_items(vec![
///                     MenuItemBuilder::new(WidgetBuilder::new())
///                         .with_content(MenuItemContent::text_no_arrow("Undo"))
///                         .build(ctx),
///                     MenuItemBuilder::new(WidgetBuilder::new())
///                         .with_content(MenuItemContent::text_no_arrow("Redo"))
///                         .build(ctx),
///                 ])
///                 .build(ctx),
///         ])
///         .build(ctx)
/// }
/// ```
#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct Menu {
    widget: Widget,
    active: bool,
    #[component(include)]
    items: ItemsContainer,
}

impl ConstructorProvider<UiNode, UserInterface> for Menu {
    fn constructor() -> GraphNodeConstructor<UiNode, UserInterface> {
        GraphNodeConstructor::new::<Self>()
            .with_variant("Menu", |ui| {
                MenuBuilder::new(WidgetBuilder::new().with_name("Menu"))
                    .build(&mut ui.build_ctx())
                    .into()
            })
            .with_group("Input")
    }
}

crate::define_widget_deref!(Menu);

uuid_provider!(Menu = "582a04f3-a7fd-4e70-bbd1-eb95e2275b75");

impl Control for Menu {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<MenuMessage>() {
            match msg {
                MenuMessage::Activate => {
                    if !self.active {
                        ui.push_picking_restriction(RestrictionEntry {
                            handle: self.handle(),
                            stop: false,
                        });
                        self.active = true;
                    }
                }
                MenuMessage::Deactivate => {
                    if self.active {
                        self.active = false;
                        ui.remove_picking_restriction(self.handle());

                        // Close descendant menu items.
                        let mut stack = self.children().to_vec();
                        while let Some(handle) = stack.pop() {
                            let node = ui.node(handle);
                            if let Some(item) = node.cast::<MenuItem>() {
                                ui.send_message(MenuItemMessage::close(
                                    handle,
                                    MessageDirection::ToWidget,
                                    true,
                                ));
                                // We have to search in popup content too because menu shows its content
                                // in popup and content could be another menu item.
                                stack.push(*item.items_panel);
                            }
                            // Continue depth search.
                            stack.extend_from_slice(node.children());
                        }
                    }
                }
            }
        } else if let Some(WidgetMessage::KeyDown(key_code)) = message.data() {
            if !message.handled() {
                if keyboard_navigation(ui, *key_code, self, self.handle) {
                    message.set_handled(true);
                } else if *key_code == KeyCode::Escape {
                    ui.send_message(MenuMessage::deactivate(
                        self.handle,
                        MessageDirection::ToWidget,
                    ));
                    message.set_handled(true);
                }
            }
        }
    }

    fn handle_os_event(
        &mut self,
        _self_handle: Handle<UiNode>,
        ui: &mut UserInterface,
        event: &OsEvent,
    ) {
        // Handle menu items close by clicking outside of menu item. We using
        // raw event here because we need to know the fact that mouse was clicked
        // and we do not care which element was clicked so we'll get here in any
        // case.
        if let OsEvent::MouseInput { state, .. } = event {
            if *state == ButtonState::Pressed && self.active {
                // TODO: Make picking more accurate - right now it works only with rects.
                let pos = ui.cursor_position();
                if !self.widget.screen_bounds().contains(pos) {
                    // Also check if we clicked inside some descendant menu item - in this
                    // case we don't need to close menu.
                    let mut any_picked = false;
                    let mut stack = self.children().to_vec();
                    'depth_search: while let Some(handle) = stack.pop() {
                        let node = ui.node(handle);
                        if let Some(item) = node.cast::<MenuItem>() {
                            let popup = ui.node(*item.items_panel);
                            if popup.screen_bounds().contains(pos) && popup.is_globally_visible() {
                                // Once we found that we clicked inside some descendant menu item
                                // we can immediately stop search - we don't want to close menu
                                // items popups in this case and can safely skip all stuff below.
                                any_picked = true;
                                break 'depth_search;
                            }
                            // We have to search in popup content too because menu shows its content
                            // in popup and content could be another menu item.
                            stack.push(*item.items_panel);
                        }
                        // Continue depth search.
                        stack.extend_from_slice(node.children());
                    }

                    if !any_picked {
                        ui.send_message(MenuMessage::deactivate(
                            self.handle(),
                            MessageDirection::ToWidget,
                        ));
                    }
                }
            }
        }
    }
}

/// A set of possible placements of a popup with items of a menu item.
#[derive(Copy, Clone, PartialOrd, PartialEq, Eq, Hash, Visit, Reflect, Default, Debug)]
pub enum MenuItemPlacement {
    /// Bottom placement.
    Bottom,
    /// Right placement.
    #[default]
    Right,
}

#[derive(Copy, Clone, PartialOrd, PartialEq, Eq, Hash, Visit, Reflect, Default, Debug)]
enum NavigationDirection {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Default, Clone, Debug, Visit, Reflect, ComponentProvider)]
#[doc(hidden)]
pub struct ItemsContainer {
    #[doc(hidden)]
    pub items: InheritableVariable<Vec<Handle<UiNode>>>,
    navigation_direction: NavigationDirection,
}

impl Deref for ItemsContainer {
    type Target = Vec<Handle<UiNode>>;

    fn deref(&self) -> &Self::Target {
        self.items.deref()
    }
}

impl DerefMut for ItemsContainer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.items.deref_mut()
    }
}

impl ItemsContainer {
    fn selected_item_index(&self, ui: &UserInterface) -> Option<usize> {
        for (index, item) in self.items.iter().enumerate() {
            if let Some(item_ref) = ui.try_get_of_type::<MenuItem>(*item) {
                if *item_ref.is_selected {
                    return Some(index);
                }
            }
        }

        None
    }

    fn next_item_to_select_in_dir(&self, ui: &UserInterface, dir: isize) -> Option<Handle<UiNode>> {
        self.selected_item_index(ui)
            .map(|i| i as isize)
            .and_then(|mut index| {
                // Do a full circle search.
                let count = self.items.len() as isize;
                for _ in 0..count {
                    index += dir;
                    if index < 0 {
                        index += count;
                    }
                    index %= count;
                    let handle = self.items.get(index as usize).cloned();
                    if let Some(item) = handle.and_then(|h| ui.try_get_of_type::<MenuItem>(h)) {
                        if item.enabled() {
                            return handle;
                        }
                    }
                }

                None
            })
    }
}

/// Menu item is a widget with arbitrary content, that has a "floating" panel (popup) for sub-items if the menu item. This was menu items can form
/// arbitrary hierarchies. See [`Menu`] docs for examples.
#[derive(Default, Clone, Debug, Visit, Reflect, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct MenuItem {
    /// Base widget of the menu item.
    pub widget: Widget,
    /// Current items of the menu item
    #[component(include)]
    pub items_container: ItemsContainer,
    /// A handle of a popup that holds the items of the menu item.
    pub items_panel: InheritableVariable<Handle<UiNode>>,
    /// A handle of a panel widget that arranges items of the menu item.
    pub panel: InheritableVariable<Handle<UiNode>>,
    /// Current placement of the menu item.
    pub placement: InheritableVariable<MenuItemPlacement>,
    /// A flag, that defines whether the menu item is clickable when it has sub-items or not.
    pub clickable_when_not_empty: InheritableVariable<bool>,
    /// A handle to the decorator of the item.
    pub decorator: InheritableVariable<Handle<UiNode>>,
    /// Is this item selected or not.
    pub is_selected: InheritableVariable<bool>,
    /// An arrow primitive that is used to indicate that there's sub-items in the menu item.
    pub arrow: InheritableVariable<Handle<UiNode>>,
    /// Content of the menu item with which it was created.
    pub content: InheritableVariable<Option<MenuItemContent>>,
}

impl ConstructorProvider<UiNode, UserInterface> for MenuItem {
    fn constructor() -> GraphNodeConstructor<UiNode, UserInterface> {
        GraphNodeConstructor::new::<Self>()
            .with_variant("Menu Item", |ui| {
                MenuItemBuilder::new(WidgetBuilder::new().with_name("Menu Item"))
                    .build(&mut ui.build_ctx())
                    .into()
            })
            .with_group("Input")
    }
}

crate::define_widget_deref!(MenuItem);

impl MenuItem {
    fn is_opened(&self, ui: &UserInterface) -> bool {
        ui.try_get_of_type::<ContextMenu>(*self.items_panel)
            .is_some_and(|items_panel| *items_panel.popup.is_open)
    }

    fn sync_arrow_visibility(&self, ui: &UserInterface) {
        ui.send_message(WidgetMessage::visibility(
            *self.arrow,
            MessageDirection::ToWidget,
            !self.items_container.is_empty(),
        ));
    }
}

// MenuItem uses popup to show its content, popup can be top-most only if it is
// direct child of root canvas of UI. This fact adds some complications to search
// of parent menu - we can't just traverse the tree because popup is not a child
// of menu item, instead we trying to fetch handle to parent menu item from popup's
// user data and continue up-search until we find menu.
fn find_menu(from: Handle<UiNode>, ui: &UserInterface) -> Handle<UiNode> {
    let mut handle = from;
    while handle.is_some() {
        if let Some((_, panel)) = ui.find_component_up::<ContextMenu>(handle) {
            // Continue search from parent menu item of popup.
            handle = panel.parent_menu_item;
        } else {
            // Maybe we have Menu as parent for MenuItem.
            return ui.find_handle_up(handle, &mut |n| n.cast::<Menu>().is_some());
        }
    }
    Default::default()
}

fn is_any_menu_item_contains_point(ui: &UserInterface, pt: Vector2<f32>) -> bool {
    for (handle, menu) in ui
        .nodes()
        .pair_iter()
        .filter_map(|(h, n)| n.query_component::<MenuItem>().map(|menu| (h, menu)))
    {
        if ui.find_component_up::<Menu>(handle).is_none()
            && menu.is_globally_visible()
            && menu.screen_bounds().contains(pt)
        {
            return true;
        }
    }
    false
}

fn close_menu_chain(from: Handle<UiNode>, ui: &UserInterface) {
    let mut handle = from;
    while handle.is_some() {
        let popup_handle = ui.find_handle_up(handle, &mut |n| n.has_component::<ContextMenu>());

        if let Some(panel) = ui.try_get_of_type::<ContextMenu>(popup_handle) {
            if *panel.popup.is_open {
                ui.send_message(PopupMessage::close(
                    popup_handle,
                    MessageDirection::ToWidget,
                ));
            }

            // Continue search from parent menu item of popup.
            handle = panel.parent_menu_item;
        } else {
            // Prevent infinite loops.
            break;
        }
    }
}

uuid_provider!(MenuItem = "72e002c6-6060-4583-b5b7-0c5500244fef");

impl Control for MenuItem {
    fn on_remove(&self, sender: &Sender<UiMessage>) {
        // Popup won't be deleted with the menu item, because it is not the child of the item.
        // So we have to remove it manually.
        sender
            .send(WidgetMessage::remove(
                *self.items_panel,
                MessageDirection::ToWidget,
            ))
            .unwrap();
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<WidgetMessage>() {
            match msg {
                WidgetMessage::MouseDown { .. } => {
                    let menu = find_menu(self.parent(), ui);
                    if menu.is_some() {
                        if self.is_opened(ui) {
                            ui.send_message(MenuItemMessage::close(
                                self.handle(),
                                MessageDirection::ToWidget,
                                true,
                            ));
                            ui.send_message(MenuMessage::deactivate(
                                menu,
                                MessageDirection::ToWidget,
                            ));
                        } else {
                            // Activate menu so it user will be able to open submenus by
                            // mouse hover.
                            ui.send_message(MenuMessage::activate(
                                menu,
                                MessageDirection::ToWidget,
                            ));

                            ui.send_message(MenuItemMessage::open(
                                self.handle(),
                                MessageDirection::ToWidget,
                            ));
                        }
                    }
                }
                WidgetMessage::MouseUp { .. } => {
                    if !message.handled() {
                        if self.items_container.is_empty() || *self.clickable_when_not_empty {
                            ui.send_message(MenuItemMessage::click(
                                self.handle(),
                                MessageDirection::ToWidget,
                            ));
                        }
                        if self.items_container.is_empty() {
                            let menu = find_menu(self.parent(), ui);
                            if menu.is_some() {
                                // Deactivate menu if we have one.
                                ui.send_message(MenuMessage::deactivate(
                                    menu,
                                    MessageDirection::ToWidget,
                                ));
                            } else {
                                // Or close menu chain if menu item is in "orphaned" state.
                                close_menu_chain(self.parent(), ui);
                            }
                        }
                        message.set_handled(true);
                    }
                }
                WidgetMessage::MouseEnter => {
                    // While parent menu active it is possible to open submenus
                    // by simple mouse hover.
                    let menu = find_menu(self.parent(), ui);
                    let open = if menu.is_some() {
                        if let Some(menu) = ui.node(menu).cast::<Menu>() {
                            menu.active
                        } else {
                            false
                        }
                    } else {
                        true
                    };
                    if open {
                        ui.send_message(MenuItemMessage::open(
                            self.handle(),
                            MessageDirection::ToWidget,
                        ));
                    }
                }
                WidgetMessage::MouseLeave => {
                    if !self.is_opened(ui) {
                        ui.send_message(MenuItemMessage::select(
                            self.handle,
                            MessageDirection::ToWidget,
                            false,
                        ));
                    }
                }
                WidgetMessage::KeyDown(key_code) => {
                    if !message.handled() && *self.is_selected && *key_code == KeyCode::Enter {
                        ui.send_message(MenuItemMessage::click(
                            self.handle,
                            MessageDirection::FromWidget,
                        ));
                        let menu = find_menu(self.parent(), ui);
                        ui.send_message(MenuMessage::deactivate(menu, MessageDirection::ToWidget));
                        message.set_handled(true);
                    }
                }
                _ => {}
            }
        } else if let Some(msg) = message.data::<MenuItemMessage>() {
            if message.destination() == self.handle
                && message.direction() == MessageDirection::ToWidget
            {
                match msg {
                    MenuItemMessage::Select(selected) => {
                        if *self.is_selected != *selected {
                            self.is_selected.set_value_and_mark_modified(*selected);

                            ui.send_message(DecoratorMessage::select(
                                *self.decorator,
                                MessageDirection::ToWidget,
                                *selected,
                            ));

                            if *selected {
                                ui.send_message(WidgetMessage::focus(
                                    self.handle,
                                    MessageDirection::ToWidget,
                                ));
                            }
                        }
                    }
                    MenuItemMessage::Open => {
                        if !self.items_container.is_empty() && !self.is_opened(ui) {
                            let placement = match *self.placement {
                                MenuItemPlacement::Bottom => Placement::LeftBottom(self.handle),
                                MenuItemPlacement::Right => Placement::RightTop(self.handle),
                            };

                            if !*self.is_selected {
                                ui.send_message(MenuItemMessage::select(
                                    self.handle,
                                    MessageDirection::ToWidget,
                                    true,
                                ));
                            }

                            // Open popup.
                            ui.send_message(PopupMessage::placement(
                                *self.items_panel,
                                MessageDirection::ToWidget,
                                placement,
                            ));
                            ui.send_message(PopupMessage::open(
                                *self.items_panel,
                                MessageDirection::ToWidget,
                            ));
                        }
                    }
                    MenuItemMessage::Close { deselect } => {
                        if let Some(panel) =
                            ui.node(*self.items_panel).query_component::<ContextMenu>()
                        {
                            if *panel.popup.is_open {
                                ui.send_message(PopupMessage::close(
                                    *self.items_panel,
                                    MessageDirection::ToWidget,
                                ));

                                if *deselect && *self.is_selected {
                                    ui.send_message(MenuItemMessage::select(
                                        self.handle,
                                        MessageDirection::ToWidget,
                                        false,
                                    ));
                                }

                                // Recursively deselect everything in the sub-items container.
                                for &item in &*self.items_container.items {
                                    ui.send_message(MenuItemMessage::close(
                                        item,
                                        MessageDirection::ToWidget,
                                        true,
                                    ));
                                }
                            }
                        }
                    }
                    MenuItemMessage::Click => {}
                    MenuItemMessage::AddItem(item) => {
                        ui.send_message(WidgetMessage::link(
                            *item,
                            MessageDirection::ToWidget,
                            *self.panel,
                        ));
                        self.items_container.push(*item);
                        if self.items_container.len() == 1 {
                            self.sync_arrow_visibility(ui);
                        }
                    }
                    MenuItemMessage::RemoveItem(item) => {
                        if let Some(position) =
                            self.items_container.iter().position(|i| *i == *item)
                        {
                            self.items_container.remove(position);

                            ui.send_message(WidgetMessage::remove(
                                *item,
                                MessageDirection::ToWidget,
                            ));

                            if self.items_container.is_empty() {
                                self.sync_arrow_visibility(ui);
                            }
                        }
                    }
                    MenuItemMessage::Items(items) => {
                        for &current_item in self.items_container.iter() {
                            ui.send_message(WidgetMessage::remove(
                                current_item,
                                MessageDirection::ToWidget,
                            ));
                        }

                        for &item in items {
                            ui.send_message(WidgetMessage::link(
                                item,
                                MessageDirection::ToWidget,
                                *self.panel,
                            ));
                        }

                        self.items_container
                            .items
                            .set_value_and_mark_modified(items.clone());

                        self.sync_arrow_visibility(ui);
                    }
                    MenuItemMessage::Sort(predicate) => {
                        let predicate = predicate.clone();
                        ui.send_message(WidgetMessage::sort_children(
                            *self.panel,
                            MessageDirection::ToWidget,
                            widget::SortingPredicate::new(move |a, b, ui| {
                                let item_a = ui.try_get_of_type::<MenuItem>(a).unwrap();
                                let item_b = ui.try_get_of_type::<MenuItem>(b).unwrap();

                                if let (Some(a_content), Some(b_content)) =
                                    (item_a.content.as_ref(), item_b.content.as_ref())
                                {
                                    predicate.0(a_content, b_content, ui)
                                } else {
                                    Ordering::Equal
                                }
                            }),
                        ));
                    }
                }
            }
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        // We need to check if some new menu item opened and then close other not in
        // direct chain of menu items until to menu.
        if message.destination() != self.handle() {
            if let Some(MenuItemMessage::Open) = message.data::<MenuItemMessage>() {
                let mut found = false;
                let mut handle = message.destination();
                while handle.is_some() {
                    if handle == self.handle() {
                        found = true;
                        break;
                    } else {
                        let node = ui.node(handle);
                        if let Some(panel) = node.component_ref::<ContextMenu>() {
                            // Once we found popup in chain, we must extract handle
                            // of parent menu item to continue search.
                            handle = panel.parent_menu_item;
                        } else {
                            handle = node.parent();
                        }
                    }
                }

                if !found {
                    if let Some(panel) = ui.node(*self.items_panel).query_component::<ContextMenu>()
                    {
                        if *panel.popup.is_open {
                            ui.send_message(MenuItemMessage::close(
                                self.handle(),
                                MessageDirection::ToWidget,
                                true,
                            ));
                        }
                    }
                }
            }
        }
    }

    fn handle_os_event(
        &mut self,
        _self_handle: Handle<UiNode>,
        ui: &mut UserInterface,
        event: &OsEvent,
    ) {
        // Allow closing "orphaned" menus by clicking outside of them.
        if let OsEvent::MouseInput { state, .. } = event {
            if *state == ButtonState::Pressed {
                if let Some(panel) = ui.node(*self.items_panel).query_component::<ContextMenu>() {
                    if *panel.popup.is_open {
                        // Ensure that cursor is outside of any menus.
                        if !is_any_menu_item_contains_point(ui, ui.cursor_position())
                            && find_menu(self.parent(), ui).is_none()
                        {
                            if *panel.popup.is_open {
                                ui.send_message(PopupMessage::close(
                                    *self.items_panel,
                                    MessageDirection::ToWidget,
                                ));
                            }

                            // Close all other popups.
                            close_menu_chain(self.parent(), ui);
                        }
                    }
                }
            }
        }
    }
}

/// Menu builder creates [`Menu`] widgets and adds them to the user interface.
pub struct MenuBuilder {
    widget_builder: WidgetBuilder,
    items: Vec<Handle<UiNode>>,
}

impl MenuBuilder {
    /// Creates new builder instance.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            items: Default::default(),
        }
    }

    /// Sets the desired items of the menu.
    pub fn with_items(mut self, items: Vec<Handle<UiNode>>) -> Self {
        self.items = items;
        self
    }

    /// Finishes menu building and adds them to the user interface.
    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        for &item in self.items.iter() {
            if let Some(item) = ctx[item].cast_mut::<MenuItem>() {
                item.placement
                    .set_value_and_mark_modified(MenuItemPlacement::Bottom);
            }
        }

        let back = BorderBuilder::new(
            WidgetBuilder::new()
                .with_background(ctx.style.property(Style::BRUSH_PRIMARY))
                .with_child(
                    StackPanelBuilder::new(
                        WidgetBuilder::new().with_children(self.items.iter().cloned()),
                    )
                    .with_orientation(Orientation::Horizontal)
                    .build(ctx),
                ),
        )
        .build(ctx);

        let menu = Menu {
            widget: self
                .widget_builder
                .with_handle_os_events(true)
                .with_child(back)
                .build(ctx),
            active: false,
            items: ItemsContainer {
                items: self.items.into(),
                navigation_direction: NavigationDirection::Horizontal,
            },
        };

        ctx.add_node(UiNode::new(menu))
    }
}

/// Allows you to set a content of a menu item either from a pre-built "layout" with icon/text/shortcut/arrow or a custom
/// widget.
#[derive(Clone, Debug, Visit, Reflect, PartialEq)]
pub enum MenuItemContent {
    /// Quick-n-dirty way of building elements. It can cover most of use cases - it builds classic menu item:
    ///
    /// ```text
    ///   _________________________
    ///  |    |      |        |   |
    ///  |icon| text |shortcut| > |
    ///  |____|______|________|___|
    /// ```
    Text {
        /// Text of the menu item.
        text: String,
        /// Shortcut of the menu item.
        shortcut: String,
        /// Icon of the menu item. Usually it is a [`crate::image::Image`] or [`crate::vector_image::VectorImage`] widget instance.
        icon: Handle<UiNode>,
        /// Create an arrow or not.
        arrow: bool,
    },
    /// Horizontally and Vertically centered text
    ///
    /// ```text
    ///   _________________________
    ///  |                        |
    ///  |          text          |
    ///  |________________________|
    /// ```
    TextCentered(String),
    /// Allows to put any node into menu item. It allows to customize menu item how needed - i.e. put image in it, or other user
    /// control.
    Node(Handle<UiNode>),
}

impl Default for MenuItemContent {
    fn default() -> Self {
        Self::TextCentered(Default::default())
    }
}

impl MenuItemContent {
    /// Creates a menu item content with a text, a shortcut and an arrow (with no icon).
    pub fn text_with_shortcut(text: impl AsRef<str>, shortcut: impl AsRef<str>) -> Self {
        MenuItemContent::Text {
            text: text.as_ref().to_owned(),
            shortcut: shortcut.as_ref().to_owned(),
            icon: Default::default(),
            arrow: true,
        }
    }

    /// Creates a menu item content with a text and an arrow (with no icon or shortcut).
    pub fn text(text: impl AsRef<str>) -> Self {
        MenuItemContent::Text {
            text: text.as_ref().to_owned(),
            shortcut: Default::default(),
            icon: Default::default(),
            arrow: true,
        }
    }

    /// Creates a menu item content with a text only (with no icon, shortcut, arrow).
    pub fn text_no_arrow(text: impl AsRef<str>) -> Self {
        MenuItemContent::Text {
            text: text.as_ref().to_owned(),
            shortcut: Default::default(),
            icon: Default::default(),
            arrow: false,
        }
    }

    /// Creates a menu item content with only horizontally and vertically centered text.
    pub fn text_centered(text: impl AsRef<str>) -> Self {
        MenuItemContent::TextCentered(text.as_ref().to_owned())
    }
}

/// Menu builder creates [`MenuItem`] widgets and adds them to the user interface.
pub struct MenuItemBuilder {
    widget_builder: WidgetBuilder,
    items: Vec<Handle<UiNode>>,
    content: Option<MenuItemContent>,
    back: Option<Handle<UiNode>>,
    clickable_when_not_empty: bool,
}

impl MenuItemBuilder {
    /// Creates new menu item builder instance.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            items: Default::default(),
            content: None,
            back: None,
            clickable_when_not_empty: false,
        }
    }

    /// Sets the desired content of the menu item. In most cases [`MenuItemContent::text_no_arrow`] is enough here.
    pub fn with_content(mut self, content: MenuItemContent) -> Self {
        self.content = Some(content);
        self
    }

    /// Sets the desired items of the menu.
    pub fn with_items(mut self, items: Vec<Handle<UiNode>>) -> Self {
        self.items = items;
        self
    }

    /// Allows you to specify the background content. Background node is only for decoration purpose, it can be any kind of node,
    /// by default it is Decorator.
    pub fn with_back(mut self, handle: Handle<UiNode>) -> Self {
        self.back = Some(handle);
        self
    }

    /// Sets whether the menu item is clickable when it has sub-items or not.
    pub fn with_clickable_when_not_empty(mut self, value: bool) -> Self {
        self.clickable_when_not_empty = value;
        self
    }

    /// Finishes menu item building and adds it to the user interface.
    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let mut arrow_widget = Handle::NONE;
        let content = match self.content.as_ref() {
            None => Handle::NONE,
            Some(MenuItemContent::Text {
                text,
                shortcut,
                icon,
                arrow,
            }) => GridBuilder::new(
                WidgetBuilder::new()
                    .with_child(*icon)
                    .with_child(
                        TextBuilder::new(
                            WidgetBuilder::new()
                                .with_margin(Thickness::left(2.0))
                                .on_row(1)
                                .on_column(1),
                        )
                        .with_text(text)
                        .build(ctx),
                    )
                    .with_child(
                        TextBuilder::new(
                            WidgetBuilder::new()
                                .with_horizontal_alignment(HorizontalAlignment::Right)
                                .with_margin(Thickness::uniform(1.0))
                                .on_row(1)
                                .on_column(2),
                        )
                        .with_text(shortcut)
                        .build(ctx),
                    )
                    .with_child({
                        arrow_widget = if *arrow {
                            VectorImageBuilder::new(
                                WidgetBuilder::new()
                                    .with_visibility(!self.items.is_empty())
                                    .on_row(1)
                                    .on_column(3)
                                    .with_width(8.0)
                                    .with_height(8.0)
                                    .with_foreground(ctx.style.property(Style::BRUSH_BRIGHT))
                                    .with_horizontal_alignment(HorizontalAlignment::Center)
                                    .with_vertical_alignment(VerticalAlignment::Center),
                            )
                            .with_primitives(make_arrow_primitives(ArrowDirection::Right, 8.0))
                            .build(ctx)
                        } else {
                            Handle::NONE
                        };
                        arrow_widget
                    }),
            )
            .add_row(Row::stretch())
            .add_row(Row::auto())
            .add_row(Row::stretch())
            .add_column(Column::auto())
            .add_column(Column::stretch())
            .add_column(Column::auto())
            .add_column(Column::strict(10.0))
            .add_column(Column::strict(5.0))
            .build(ctx),
            Some(MenuItemContent::TextCentered(text)) => {
                TextBuilder::new(WidgetBuilder::new().with_margin(Thickness::left_right(5.0)))
                    .with_text(text)
                    .with_horizontal_text_alignment(HorizontalAlignment::Center)
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .build(ctx)
            }
            Some(MenuItemContent::Node(node)) => *node,
        };

        let decorator = self.back.unwrap_or_else(|| {
            DecoratorBuilder::new(
                BorderBuilder::new(WidgetBuilder::new())
                    .with_stroke_thickness(Thickness::uniform(0.0).into()),
            )
            .with_hover_brush(ctx.style.property(Style::BRUSH_BRIGHT_BLUE))
            .with_selected_brush(ctx.style.property(Style::BRUSH_BRIGHT_BLUE))
            .with_normal_brush(ctx.style.property(Style::BRUSH_PRIMARY))
            .with_pressed_brush(Brush::Solid(Color::TRANSPARENT).into())
            .with_pressable(false)
            .build(ctx)
        });

        if content.is_some() {
            ctx.link(content, decorator);
        }

        let panel;
        let items_panel = ContextMenuBuilder::new(
            PopupBuilder::new(WidgetBuilder::new().with_min_size(Vector2::new(10.0, 10.0)))
                .with_content({
                    panel = StackPanelBuilder::new(
                        WidgetBuilder::new().with_children(self.items.iter().cloned()),
                    )
                    .build(ctx);
                    panel
                })
                // We'll manually control if popup is either open or closed.
                .stays_open(true),
        )
        .build(ctx);

        let menu = MenuItem {
            widget: self
                .widget_builder
                .with_handle_os_events(true)
                .with_preview_messages(true)
                .with_child(decorator)
                .build(ctx),
            items_panel: items_panel.into(),
            items_container: ItemsContainer {
                items: self.items.into(),
                navigation_direction: NavigationDirection::Vertical,
            },
            placement: MenuItemPlacement::Right.into(),
            panel: panel.into(),
            clickable_when_not_empty: false.into(),
            decorator: decorator.into(),
            is_selected: Default::default(),
            arrow: arrow_widget.into(),
            content: self.content.into(),
        };

        let handle = ctx.add_node(UiNode::new(menu));

        // "Link" popup with its parent menu item.
        if let Some(popup) = ctx[items_panel].cast_mut::<ContextMenu>() {
            popup.parent_menu_item = handle;
        }

        handle
    }
}

/// A simple wrapper over [`Popup`] widget, that holds the sub-items of a menu item and provides
/// an ability for keyboard navigation.
#[derive(Default, Clone, Debug, Visit, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "ad8e9e76-c213-4232-9bab-80ebcabd69fa")]
#[reflect(derived_type = "UiNode")]
pub struct ContextMenu {
    /// Inner popup widget of the context menu.
    #[component(include)]
    pub popup: Popup,
    /// Parent menu item of the context menu. Allows you to build chained context menus.
    pub parent_menu_item: Handle<UiNode>,
}

impl ConstructorProvider<UiNode, UserInterface> for ContextMenu {
    fn constructor() -> GraphNodeConstructor<UiNode, UserInterface> {
        GraphNodeConstructor::new::<Self>()
            .with_variant("Context Menu", |ui| {
                ContextMenuBuilder::new(PopupBuilder::new(
                    WidgetBuilder::new().with_name("Context Menu"),
                ))
                .build(&mut ui.build_ctx())
                .into()
            })
            .with_group("Input")
    }
}

impl Deref for ContextMenu {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.popup.widget
    }
}

impl DerefMut for ContextMenu {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.popup.widget
    }
}

impl Control for ContextMenu {
    fn on_remove(&self, sender: &Sender<UiMessage>) {
        self.popup.on_remove(sender)
    }

    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        self.popup.measure_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        self.popup.arrange_override(ui, final_size)
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        self.popup.draw(drawing_context)
    }

    fn post_draw(&self, drawing_context: &mut DrawingContext) {
        self.popup.post_draw(drawing_context)
    }

    fn update(&mut self, dt: f32, ui: &mut UserInterface) {
        self.popup.update(dt, ui);
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.popup.handle_routed_message(ui, message);

        if let Some(WidgetMessage::KeyDown(key_code)) = message.data() {
            if !message.handled() {
                if let Some(parent_menu_item) = ui.try_get(self.parent_menu_item) {
                    if keyboard_navigation(
                        ui,
                        *key_code,
                        parent_menu_item.deref(),
                        self.parent_menu_item,
                    ) {
                        message.set_handled(true);
                    }
                }
            }
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        self.popup.preview_message(ui, message)
    }

    fn handle_os_event(
        &mut self,
        self_handle: Handle<UiNode>,
        ui: &mut UserInterface,
        event: &OsEvent,
    ) {
        self.popup.handle_os_event(self_handle, ui, event)
    }
}

/// Creates [`ContextMenu`] widgets.
pub struct ContextMenuBuilder {
    popup_builder: PopupBuilder,
    parent_menu_item: Handle<UiNode>,
}

impl ContextMenuBuilder {
    /// Creates new builder instance using an instance of the [`PopupBuilder`].
    pub fn new(popup_builder: PopupBuilder) -> Self {
        Self {
            popup_builder,
            parent_menu_item: Default::default(),
        }
    }

    /// Sets the desired parent menu item.
    pub fn with_parent_menu_item(mut self, parent_menu_item: Handle<UiNode>) -> Self {
        self.parent_menu_item = parent_menu_item;
        self
    }

    /// Finishes context menu building.
    pub fn build_context_menu(self, ctx: &mut BuildContext) -> ContextMenu {
        ContextMenu {
            popup: self.popup_builder.build_popup(ctx),
            parent_menu_item: self.parent_menu_item,
        }
    }

    /// Finishes context menu building and adds it to the user interface.
    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let context_menu = self.build_context_menu(ctx);
        ctx.add_node(UiNode::new(context_menu))
    }
}

fn keyboard_navigation(
    ui: &UserInterface,
    key_code: KeyCode,
    parent_menu_item: &dyn Control,
    parent_menu_item_handle: Handle<UiNode>,
) -> bool {
    let Some(items_container) = parent_menu_item
        .query_component_ref(TypeId::of::<ItemsContainer>())
        .and_then(|c| c.downcast_ref::<ItemsContainer>())
    else {
        return false;
    };

    let (close_key, enter_key, next_key, prev_key) = match items_container.navigation_direction {
        NavigationDirection::Horizontal => (
            KeyCode::ArrowUp,
            KeyCode::ArrowDown,
            KeyCode::ArrowRight,
            KeyCode::ArrowLeft,
        ),
        NavigationDirection::Vertical => (
            KeyCode::ArrowLeft,
            KeyCode::ArrowRight,
            KeyCode::ArrowDown,
            KeyCode::ArrowUp,
        ),
    };

    if key_code == close_key {
        ui.send_message(MenuItemMessage::close(
            parent_menu_item_handle,
            MessageDirection::ToWidget,
            false,
        ));
        return true;
    } else if key_code == enter_key {
        if let Some(selected_item_index) = items_container.selected_item_index(ui) {
            let selected_item = items_container.items[selected_item_index];

            ui.send_message(MenuItemMessage::open(
                selected_item,
                MessageDirection::ToWidget,
            ));

            if let Some(selected_item_ref) = ui.try_get_of_type::<MenuItem>(selected_item) {
                if let Some(first_item) = selected_item_ref.items_container.first() {
                    ui.send_message(MenuItemMessage::select(
                        *first_item,
                        MessageDirection::ToWidget,
                        true,
                    ));
                }
            }
        }
        return true;
    } else if key_code == next_key || key_code == prev_key {
        if let Some(selected_item_index) = items_container.selected_item_index(ui) {
            let dir = if key_code == next_key {
                1
            } else if key_code == prev_key {
                -1
            } else {
                unreachable!()
            };

            if let Some(new_selection) = items_container.next_item_to_select_in_dir(ui, dir) {
                ui.send_message(MenuItemMessage::select(
                    items_container.items[selected_item_index],
                    MessageDirection::ToWidget,
                    false,
                ));
                ui.send_message(MenuItemMessage::select(
                    new_selection,
                    MessageDirection::ToWidget,
                    true,
                ));

                return true;
            }
        } else if let Some(first_item) = items_container.items.first() {
            ui.send_message(MenuItemMessage::select(
                *first_item,
                MessageDirection::ToWidget,
                true,
            ));

            return true;
        }
    }

    false
}

#[cfg(test)]
mod test {
    use crate::menu::{MenuBuilder, MenuItemBuilder};
    use crate::{test::test_widget_deletion, widget::WidgetBuilder};

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| MenuBuilder::new(WidgetBuilder::new()).build(ctx));
        test_widget_deletion(|ctx| MenuItemBuilder::new(WidgetBuilder::new()).build(ctx));
    }
}
