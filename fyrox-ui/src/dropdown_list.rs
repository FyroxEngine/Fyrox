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
    grid::{Column, GridBuilder, Row},
    list_view::{ListViewBuilder, ListViewMessage},
    message::{KeyCode, MessageDirection, UiMessage},
    popup::{Placement, PopupBuilder, PopupMessage},
    style::{resource::StyleResourceExt, Style},
    utils::{make_arrow_non_uniform_size, ArrowDirection},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, Thickness, UiNode, UserInterface,
};

use crate::message::MessageData;
use crate::popup::Popup;
use fyrox_graph::{
    constructor::{ConstructorProvider, GraphNodeConstructor},
    BaseSceneGraph, SceneGraph,
};
use std::sync::mpsc::Sender;

/// A set of possible messages for [`DropdownList`] widget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DropdownListMessage {
    /// A message, that is used to set new selection and receive selection changes.
    Selection(Option<usize>),
    /// A message, that is used to set new items of a dropdown list.
    Items(Vec<Handle<UiNode>>),
    /// A message, that is used to add an item to a dropdown list.
    AddItem(Handle<UiNode>),
    /// A message, that is used to open a dropdown list.
    Open,
    /// A message, that is used to close a dropdown list.
    Close,
}
impl MessageData for DropdownListMessage {}

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
///         if let Some(DropdownListMessage::Selection(new_selection)) = message.data() {
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
/// To change selection of a dropdown list, send [`DropdownListMessage::Selection`] message
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
#[reflect(derived_type = "UiNode")]
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

impl ConstructorProvider<UiNode, UserInterface> for DropdownList {
    fn constructor() -> GraphNodeConstructor<UiNode, UserInterface> {
        GraphNodeConstructor::new::<Self>()
            .with_variant("Dropdown List", |ui| {
                DropdownListBuilder::new(WidgetBuilder::new().with_name("Dropdown List"))
                    .build(&mut ui.build_ctx())
                    .into()
            })
            .with_group("Input")
    }
}

crate::define_widget_deref!(DropdownList);

uuid_provider!(DropdownList = "1da2f69a-c8b4-4ae2-a2ad-4afe61ee2a32");

impl Control for DropdownList {
    fn on_remove(&self, sender: &Sender<UiMessage>) {
        // Popup won't be deleted with the dropdown list, because it is not the child of the list.
        // So we have to remove it manually.
        sender
            .send(UiMessage::for_widget(*self.popup, WidgetMessage::Remove))
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
                        ui.send(self.handle, DropdownListMessage::Open);
                    }
                }
                WidgetMessage::KeyDown(key_code) => {
                    if !message.handled() {
                        if *key_code == KeyCode::ArrowDown {
                            ui.send(self.handle, DropdownListMessage::Open);
                        } else if *key_code == KeyCode::ArrowUp {
                            ui.send(self.handle, DropdownListMessage::Close);
                        }
                        message.set_handled(true);
                    }
                }
                _ => (),
            }
        } else if let Some(msg) = message.data_for::<DropdownListMessage>(self.handle()) {
            match msg {
                DropdownListMessage::Open => {
                    ui.send(
                        *self.popup,
                        WidgetMessage::MinSize(Vector2::new(self.actual_local_size().x, 0.0)),
                    );
                    ui.send(
                        *self.popup,
                        PopupMessage::Placement(Placement::LeftBottom(self.handle)),
                    );
                    ui.send(*self.popup, PopupMessage::Open);
                }
                DropdownListMessage::Close => {
                    ui.send(*self.popup, PopupMessage::Close);
                }
                DropdownListMessage::Items(items) => {
                    ui.send(*self.list_view, ListViewMessage::Items(items.clone()));
                    self.items.set_value_and_mark_modified(items.clone());
                    self.sync_selected_item_preview(ui);
                }
                &DropdownListMessage::AddItem(item) => {
                    ui.send(*self.list_view, ListViewMessage::AddItem(item));
                    self.items.push(item);
                }
                &DropdownListMessage::Selection(selection) => {
                    if selection != *self.selection {
                        self.selection.set_value_and_mark_modified(selection);
                        ui.send(
                            *self.list_view,
                            ListViewMessage::Selection(
                                selection.map(|index| vec![index]).unwrap_or_default(),
                            ),
                        );

                        self.sync_selected_item_preview(ui);

                        if *self.close_on_selection
                            && *ui.try_get_of_type::<Popup>(*self.popup).unwrap().is_open
                        {
                            ui.send(*self.popup, PopupMessage::Close);
                        }

                        ui.send_message(message.reverse());
                    }
                }
            }
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        if let Some(ListViewMessage::Selection(selection)) = message.data::<ListViewMessage>() {
            let selection = selection.first().cloned();
            if message.direction() == MessageDirection::FromWidget
                && message.destination() == *self.list_view
                && *self.selection != selection
            {
                // Post message again but from name of this drop-down list so user can catch
                // message and respond properly.
                ui.send(self.handle, DropdownListMessage::Selection(selection));
            }
        } else if let Some(msg) = message.data_for::<PopupMessage>(*self.popup) {
            match msg {
                PopupMessage::Open => {
                    ui.post(self.handle, DropdownListMessage::Open);
                }
                PopupMessage::Close => {
                    ui.post(self.handle, DropdownListMessage::Close);
                    ui.send(self.handle, WidgetMessage::Focus);
                }
                _ => (),
            }
        }
    }
}

impl DropdownList {
    /// A name of style property, that defines corner radius of a dropdown list.
    pub const CORNER_RADIUS: &'static str = "DropdownList.CornerRadius";

    /// Returns a style of the widget. This style contains only widget-specific properties.
    pub fn style() -> Style {
        Style::default().with(Self::CORNER_RADIUS, 4.0f32)
    }

    fn sync_selected_item_preview(&mut self, ui: &mut UserInterface) {
        // Copy node from current selection in list view. This is not
        // always suitable because if an item has some visual behaviour
        // (change color on mouse hover, change something on click, etc)
        // it will be also reflected in selected item.
        if self.current.is_some() {
            ui.send(*self.current, WidgetMessage::Remove);
        }
        if let Some(index) = *self.selection {
            if let Some(item) = self.items.get(index) {
                self.current
                    .set_value_and_mark_modified(ui.copy_node(*item));
                ui.send(*self.current, WidgetMessage::LinkWith(*self.main_grid));
                ui.node(*self.current).request_update_visibility();
                ui.send(
                    *self.current,
                    WidgetMessage::Margin(Thickness::uniform(0.0)),
                );
                ui.send(*self.current, WidgetMessage::ResetVisual);
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

        let border = BorderBuilder::new(
            WidgetBuilder::new()
                .with_background(ctx.style.property(Style::BRUSH_DARKER))
                .with_foreground(ctx.style.property(Style::BRUSH_LIGHT))
                .with_child(main_grid),
        )
        .with_pad_by_corner_radius(false)
        .with_corner_radius(ctx.style.property(DropdownList::CORNER_RADIUS))
        .build(ctx);

        let dropdown_list = UiNode::new(DropdownList {
            widget: self
                .widget_builder
                .with_accepts_input(true)
                .with_preview_messages(true)
                .with_child(border)
                .build(ctx),
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

#[cfg(test)]
mod test {
    use crate::dropdown_list::DropdownListBuilder;
    use crate::{test::test_widget_deletion, widget::WidgetBuilder};

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| DropdownListBuilder::new(WidgetBuilder::new()).build(ctx));
    }
}
