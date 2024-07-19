//! Drop-down list. This is control which shows currently selected item and provides drop-down
//! list to select its current item. It is build using composition with standard list view.
//! See [`DropdownList`] docs for more info and usage examples.

#![warn(missing_docs)]

use crate::{
    border::BorderBuilder,
    core::{
        algebra::Vector2, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        uuid_provider, variable::InheritableVariable, visitor::prelude::*,
    },
    define_constructor,
    grid::{Column, GridBuilder, Row},
    list_view::{ListViewBuilder, ListViewMessage},
    message::{KeyCode, MessageDirection, UiMessage},
    popup::{Placement, PopupBuilder, PopupMessage},
    utils::{make_arrow_non_uniform_size, ArrowDirection},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, Thickness, UiNode, UserInterface, BRUSH_DARKER, BRUSH_LIGHT,
};
use fyrox_graph::BaseSceneGraph;
use std::{
    ops::{Deref, DerefMut},
    sync::mpsc::Sender,
};

/// A set of possible messages for [`DropdownList`] widget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DropdownListMessage {
    /// A message, that is used to set new selection and receive selection changes.
    SelectionChanged(Option<usize>),
    /// A message, that is used to set new items of a dropdown list.
    Items(Vec<Handle<UiNode>>),
    /// A message, that is used to add an item to a dropdown list.
    AddItem(Handle<UiNode>),
    /// A message, that is used to open a dropdown list.
    Open,
    /// A message, that is used to close a dropdown list.
    Close,
}

impl DropdownListMessage {
    define_constructor!(
        /// Creates [`DropdownListMessage::SelectionChanged`] message.
        DropdownListMessage:SelectionChanged => fn selection(Option<usize>), layout: false
    );
    define_constructor!(
           /// Creates [`DropdownListMessage::Items`] message.
        DropdownListMessage:Items => fn items(Vec<Handle<UiNode >>), layout: false
    );
    define_constructor!(
        /// Creates [`DropdownListMessage::AddItem`] message.
        DropdownListMessage:AddItem => fn add_item(Handle<UiNode>), layout: false
    );
    define_constructor!(
        /// Creates [`DropdownListMessage::Open`] message.
        DropdownListMessage:Open => fn open(), layout: false
    );
    define_constructor!(
        /// Creates [`DropdownListMessage::Close`] message.
        DropdownListMessage:Close => fn close(), layout: false
    );
}

/// Drop-down list is a control which shows currently selected item and provides drop-down
/// list to select its current item. It is used to show a single selected item in compact way.
///
/// ## Example
///
/// A dropdown list with two text items with the last one selected, could be created like so:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle, dropdown_list::DropdownListBuilder, text::TextBuilder,
/// #     widget::WidgetBuilder, BuildContext, UiNode,
/// # };
/// #
/// fn create_drop_down_list(ctx: &mut BuildContext) -> Handle<UiNode> {
///     DropdownListBuilder::new(WidgetBuilder::new())
///         .with_items(vec![
///             TextBuilder::new(WidgetBuilder::new())
///                 .with_text("Item 0")
///                 .build(ctx),
///             TextBuilder::new(WidgetBuilder::new())
///                 .with_text("Item 1")
///                 .build(ctx),
///         ])
///         .with_selected(1)
///         .build(ctx)
/// }
/// ```
///
/// Keep in mind, that items of a dropdown list could be any widget, but usually each item is wrapped
/// in some other widget that shows current state of items (selected, hovered, clicked, etc.). One
/// of the most convenient way of doing this is to use Decorator widget:
///
/// ```rust
/// # use fyrox_ui::{
/// #     border::BorderBuilder, core::pool::Handle, decorator::DecoratorBuilder,
/// #     dropdown_list::DropdownListBuilder, text::TextBuilder, widget::WidgetBuilder, BuildContext,
/// #     UiNode,
/// # };
/// #
/// fn make_item(text: &str, ctx: &mut BuildContext) -> Handle<UiNode> {
///     DecoratorBuilder::new(BorderBuilder::new(
///         WidgetBuilder::new().with_child(
///             TextBuilder::new(WidgetBuilder::new())
///                 .with_text(text)
///                 .build(ctx),
///         ),
///     ))
///     .build(ctx)
/// }
///
/// fn create_drop_down_list_with_decorators(ctx: &mut BuildContext) -> Handle<UiNode> {
///     DropdownListBuilder::new(WidgetBuilder::new())
///         .with_items(vec![make_item("Item 0", ctx), make_item("Item 1", ctx)])
///         .with_selected(1)
///         .build(ctx)
/// }
/// ```
///
/// ## Selection
///
/// Dropdown list supports two kinds of selection - `None` or `Some(index)`. To catch a moment when
/// selection changes, use the following code:
///
/// ```rust
/// use fyrox_ui::{
///     core::pool::Handle,
///     dropdown_list::DropdownListMessage,
///     message::{MessageDirection, UiMessage},
///     UiNode,
/// };
///
/// struct Foo {
///     dropdown_list: Handle<UiNode>,
/// }
///
/// impl Foo {
///     fn on_ui_message(&mut self, message: &UiMessage) {
///         if let Some(DropdownListMessage::SelectionChanged(new_selection)) = message.data() {
///             if message.destination() == self.dropdown_list
///                 && message.direction() == MessageDirection::FromWidget
///             {
///                 // Do something.
///                 dbg!(new_selection);
///             }
///         }
///     }
/// }
/// ```
///
/// To change selection of a dropdown list, send [`DropdownListMessage::SelectionChanged`] message
/// to it.
///
/// ## Items
///
/// To change current items of a dropdown list, create the items first and then send them to the
/// dropdown list using [`DropdownListMessage::Items`] message.
///
/// ## Opening and Closing
///
/// A dropdown list could be opened and closed manually using [`DropdownListMessage::Open`] and
/// [`DropdownListMessage::Close`] messages.  
#[derive(Default, Clone, Debug, Visit, Reflect, ComponentProvider)]
pub struct DropdownList {
    /// Base widget of the dropdown list.
    pub widget: Widget,
    /// A handle of the inner popup of the dropdown list. It holds the actual items of the list.
    pub popup: InheritableVariable<Handle<UiNode>>,
    /// A list of handles of items of the dropdown list.
    pub items: InheritableVariable<Vec<Handle<UiNode>>>,
    /// A handle to the `ListView` widget, that holds the items of the dropdown list.
    pub list_view: InheritableVariable<Handle<UiNode>>,
    /// A handle to a currently selected item.
    pub current: InheritableVariable<Handle<UiNode>>,
    /// An index of currently selected item (or [`None`] if there's nothing selected).
    pub selection: InheritableVariable<Option<usize>>,
    /// A flag, that defines whether the dropdown list's popup should close after selection or not.
    pub close_on_selection: InheritableVariable<bool>,
    /// A handle to an inner Grid widget, that holds currently selected item and other decorators.
    pub main_grid: InheritableVariable<Handle<UiNode>>,
}

crate::define_widget_deref!(DropdownList);

uuid_provider!(DropdownList = "1da2f69a-c8b4-4ae2-a2ad-4afe61ee2a32");

impl Control for DropdownList {
    fn on_remove(&self, sender: &Sender<UiMessage>) {
        // Popup won't be deleted with the dropdown list, because it is not the child of the list.
        // So we have to remove it manually.
        sender
            .send(WidgetMessage::remove(
                *self.popup,
                MessageDirection::ToWidget,
            ))
            .unwrap();
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<WidgetMessage>() {
            match msg {
                WidgetMessage::MouseDown { .. } => {
                    if message.destination() == self.handle()
                        || self.widget.has_descendant(message.destination(), ui)
                    {
                        ui.send_message(DropdownListMessage::open(
                            self.handle,
                            MessageDirection::ToWidget,
                        ));
                    }
                }
                WidgetMessage::KeyDown(key_code) => {
                    if !message.handled() {
                        if *key_code == KeyCode::ArrowDown {
                            ui.send_message(DropdownListMessage::open(
                                self.handle,
                                MessageDirection::ToWidget,
                            ));
                        } else if *key_code == KeyCode::ArrowUp {
                            ui.send_message(DropdownListMessage::close(
                                self.handle,
                                MessageDirection::ToWidget,
                            ));
                        }
                        message.set_handled(true);
                    }
                }
                _ => (),
            }
        } else if let Some(msg) = message.data::<DropdownListMessage>() {
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::ToWidget
            {
                match msg {
                    DropdownListMessage::Open => {
                        ui.send_message(WidgetMessage::width(
                            *self.popup,
                            MessageDirection::ToWidget,
                            self.actual_local_size().x,
                        ));
                        ui.send_message(PopupMessage::placement(
                            *self.popup,
                            MessageDirection::ToWidget,
                            Placement::LeftBottom(self.handle),
                        ));
                        ui.send_message(PopupMessage::open(
                            *self.popup,
                            MessageDirection::ToWidget,
                        ));
                    }
                    DropdownListMessage::Close => {
                        ui.send_message(PopupMessage::close(
                            *self.popup,
                            MessageDirection::ToWidget,
                        ));
                    }
                    DropdownListMessage::Items(items) => {
                        ui.send_message(ListViewMessage::items(
                            *self.list_view,
                            MessageDirection::ToWidget,
                            items.clone(),
                        ));
                        self.items.set_value_and_mark_modified(items.clone());
                        self.sync_selected_item_preview(ui);
                    }
                    &DropdownListMessage::AddItem(item) => {
                        ui.send_message(ListViewMessage::add_item(
                            *self.list_view,
                            MessageDirection::ToWidget,
                            item,
                        ));
                        self.items.push(item);
                    }
                    &DropdownListMessage::SelectionChanged(selection) => {
                        if selection != *self.selection {
                            self.selection.set_value_and_mark_modified(selection);
                            ui.send_message(ListViewMessage::selection(
                                *self.list_view,
                                MessageDirection::ToWidget,
                                selection.map(|index| vec![index]).unwrap_or_default(),
                            ));

                            self.sync_selected_item_preview(ui);

                            if *self.close_on_selection {
                                ui.send_message(PopupMessage::close(
                                    *self.popup,
                                    MessageDirection::ToWidget,
                                ));
                            }

                            ui.send_message(message.reverse());
                        }
                    }
                }
            }
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        if let Some(ListViewMessage::SelectionChanged(selection)) =
            message.data::<ListViewMessage>()
        {
            let selection = selection.first().cloned();
            if message.direction() == MessageDirection::FromWidget
                && message.destination() == *self.list_view
                && *self.selection != selection
            {
                // Post message again but from name of this drop-down list so user can catch
                // message and respond properly.
                ui.send_message(DropdownListMessage::selection(
                    self.handle,
                    MessageDirection::ToWidget,
                    selection,
                ));
            }
        } else if let Some(msg) = message.data::<PopupMessage>() {
            if message.destination() == *self.popup {
                match msg {
                    PopupMessage::Open => {
                        ui.send_message(DropdownListMessage::open(
                            self.handle,
                            MessageDirection::FromWidget,
                        ));
                    }
                    PopupMessage::Close => {
                        ui.send_message(DropdownListMessage::close(
                            self.handle,
                            MessageDirection::FromWidget,
                        ));

                        ui.send_message(WidgetMessage::focus(
                            self.handle,
                            MessageDirection::ToWidget,
                        ));
                    }
                    _ => (),
                }
            }
        }
    }
}

impl DropdownList {
    fn sync_selected_item_preview(&mut self, ui: &mut UserInterface) {
        // Copy node from current selection in list view. This is not
        // always suitable because if an item has some visual behaviour
        // (change color on mouse hover, change something on click, etc)
        // it will be also reflected in selected item.
        if self.current.is_some() {
            ui.send_message(WidgetMessage::remove(
                *self.current,
                MessageDirection::ToWidget,
            ));
        }
        if let Some(index) = *self.selection {
            if let Some(item) = self.items.get(index) {
                self.current
                    .set_value_and_mark_modified(ui.copy_node(*item));
                ui.send_message(WidgetMessage::link(
                    *self.current,
                    MessageDirection::ToWidget,
                    *self.main_grid,
                ));
                ui.node(*self.current).request_update_visibility();
                ui.send_message(WidgetMessage::margin(
                    *self.current,
                    MessageDirection::ToWidget,
                    Thickness::uniform(0.0),
                ));
            } else {
                self.current.set_value_and_mark_modified(Handle::NONE);
            }
        } else {
            self.current.set_value_and_mark_modified(Handle::NONE);
        }
    }
}

/// Dropdown list builder allows to create [`DropdownList`] widgets and add them a user interface.
pub struct DropdownListBuilder {
    widget_builder: WidgetBuilder,
    items: Vec<Handle<UiNode>>,
    selected: Option<usize>,
    close_on_selection: bool,
}

impl DropdownListBuilder {
    /// Creates new dropdown list builder.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            items: Default::default(),
            selected: None,
            close_on_selection: false,
        }
    }

    /// Sets the desired items of the dropdown list.
    pub fn with_items(mut self, items: Vec<Handle<UiNode>>) -> Self {
        self.items = items;
        self
    }

    /// Sets the selected item of the dropdown list.
    pub fn with_selected(mut self, index: usize) -> Self {
        self.selected = Some(index);
        self
    }

    /// Sets the desired items of the dropdown list.
    pub fn with_opt_selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }

    /// Sets a flag, that defines whether the dropdown list should close on selection or not.
    pub fn with_close_on_selection(mut self, value: bool) -> Self {
        self.close_on_selection = value;
        self
    }

    /// Finishes list building and adds it to the given user interface.
    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode>
    where
        Self: Sized,
    {
        let items_control = ListViewBuilder::new(
            WidgetBuilder::new().with_max_size(Vector2::new(f32::INFINITY, 200.0)),
        )
        .with_items(self.items.clone())
        .build(ctx);

        let popup = PopupBuilder::new(WidgetBuilder::new())
            .with_content(items_control)
            .build(ctx);

        let current = if let Some(selected) = self.selected {
            self.items
                .get(selected)
                .map_or(Handle::NONE, |&f| ctx.copy(f))
        } else {
            Handle::NONE
        };

        let arrow = make_arrow_non_uniform_size(ctx, ArrowDirection::Bottom, 10.0, 5.0);
        ctx[arrow].set_margin(Thickness::left_right(2.0));
        ctx[arrow].set_column(1);

        let main_grid =
            GridBuilder::new(WidgetBuilder::new().with_child(current).with_child(arrow))
                .add_row(Row::stretch())
                .add_column(Column::stretch())
                .add_column(Column::auto())
                .build(ctx);

        let dropdown_list = UiNode::new(DropdownList {
            widget: self
                .widget_builder
                .with_accepts_input(true)
                .with_preview_messages(true)
                .with_child(
                    BorderBuilder::new(
                        WidgetBuilder::new()
                            .with_background(BRUSH_DARKER)
                            .with_foreground(BRUSH_LIGHT)
                            .with_child(main_grid),
                    )
                    .with_pad_by_corner_radius(false)
                    .with_corner_radius(4.0)
                    .build(ctx),
                )
                .build(),
            popup: popup.into(),
            items: self.items.into(),
            list_view: items_control.into(),
            current: current.into(),
            selection: self.selected.into(),
            close_on_selection: self.close_on_selection.into(),
            main_grid: main_grid.into(),
        });

        ctx.add_node(dropdown_list)
    }
}
