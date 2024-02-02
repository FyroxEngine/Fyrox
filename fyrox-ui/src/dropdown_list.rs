//! Drop-down list. This is control which shows currently selected item and provides drop-down
//! list to select its current item. It is build using composition with standard list view.

use crate::{
    border::BorderBuilder,
    core::{algebra::Vector2, pool::Handle},
    core::{reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*},
    define_constructor,
    grid::{Column, GridBuilder, Row},
    list_view::{ListViewBuilder, ListViewMessage},
    message::{MessageDirection, UiMessage},
    popup::{Placement, PopupBuilder, PopupMessage},
    utils::{make_arrow, ArrowDirection},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, Thickness, UiNode, UserInterface, BRUSH_DARKER, BRUSH_LIGHT,
};
use fyrox_core::uuid_provider;
use std::{
    ops::{Deref, DerefMut},
    sync::mpsc::Sender,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DropdownListMessage {
    SelectionChanged(Option<usize>),
    Items(Vec<Handle<UiNode>>),
    AddItem(Handle<UiNode>),
    Open,
    Close,
}

impl DropdownListMessage {
    define_constructor!(DropdownListMessage:SelectionChanged => fn selection(Option<usize>), layout: false);
    define_constructor!(DropdownListMessage:Items => fn items(Vec<Handle<UiNode >>), layout: false);
    define_constructor!(DropdownListMessage:AddItem => fn add_item(Handle<UiNode>), layout: false);
    define_constructor!(DropdownListMessage:Open => fn open(), layout: false);
    define_constructor!(DropdownListMessage:Close => fn close(), layout: false);
}

#[derive(Default, Clone, Debug, Visit, Reflect, ComponentProvider)]
pub struct DropdownList {
    pub widget: Widget,
    pub popup: Handle<UiNode>,
    pub items: Vec<Handle<UiNode>>,
    pub list_view: Handle<UiNode>,
    pub current: Handle<UiNode>,
    pub selection: Option<usize>,
    pub close_on_selection: bool,
    pub main_grid: Handle<UiNode>,
}

crate::define_widget_deref!(DropdownList);

uuid_provider!(DropdownList = "1da2f69a-c8b4-4ae2-a2ad-4afe61ee2a32");

impl Control for DropdownList {
    fn on_remove(&self, sender: &Sender<UiMessage>) {
        // Popup won't be deleted with the dropdown list, because it is not the child of the list.
        // So we have to remove it manually.
        sender
            .send(WidgetMessage::remove(
                self.popup,
                MessageDirection::ToWidget,
            ))
            .unwrap();
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(WidgetMessage::MouseDown { .. }) = message.data::<WidgetMessage>() {
            if message.destination() == self.handle()
                || self.widget.has_descendant(message.destination(), ui)
            {
                ui.send_message(DropdownListMessage::open(
                    self.handle,
                    MessageDirection::ToWidget,
                ));
            }
        } else if let Some(msg) = message.data::<DropdownListMessage>() {
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::ToWidget
            {
                match msg {
                    DropdownListMessage::Open => {
                        ui.send_message(WidgetMessage::width(
                            self.popup,
                            MessageDirection::ToWidget,
                            self.actual_local_size().x,
                        ));
                        ui.send_message(PopupMessage::placement(
                            self.popup,
                            MessageDirection::ToWidget,
                            Placement::LeftBottom(self.handle),
                        ));
                        ui.send_message(PopupMessage::open(self.popup, MessageDirection::ToWidget));
                    }
                    DropdownListMessage::Close => {
                        ui.send_message(PopupMessage::close(
                            self.popup,
                            MessageDirection::ToWidget,
                        ));
                    }
                    DropdownListMessage::Items(items) => {
                        ui.send_message(ListViewMessage::items(
                            self.list_view,
                            MessageDirection::ToWidget,
                            items.clone(),
                        ));
                        self.items = items.clone();
                        self.sync_selected_item_preview(ui);
                    }
                    &DropdownListMessage::AddItem(item) => {
                        ui.send_message(ListViewMessage::add_item(
                            self.list_view,
                            MessageDirection::ToWidget,
                            item,
                        ));
                        self.items.push(item);
                    }
                    &DropdownListMessage::SelectionChanged(selection) => {
                        if selection != self.selection {
                            self.selection = selection;
                            ui.send_message(ListViewMessage::selection(
                                self.list_view,
                                MessageDirection::ToWidget,
                                selection,
                            ));

                            self.sync_selected_item_preview(ui);

                            if self.close_on_selection {
                                ui.send_message(PopupMessage::close(
                                    self.popup,
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
            if message.direction() == MessageDirection::FromWidget
                && message.destination() == self.list_view
                && &self.selection != selection
            {
                // Post message again but from name of this drop-down list so user can catch
                // message and respond properly.
                ui.send_message(DropdownListMessage::selection(
                    self.handle,
                    MessageDirection::ToWidget,
                    *selection,
                ));
            }
        } else if let Some(msg) = message.data::<PopupMessage>() {
            if message.destination() == self.popup {
                match msg {
                    PopupMessage::Open => {
                        ui.send_message(DropdownListMessage::open(
                            self.handle,
                            MessageDirection::FromWidget,
                        ));
                    }
                    PopupMessage::Close => {
                        ui.send_message(DropdownListMessage::open(
                            self.handle,
                            MessageDirection::FromWidget,
                        ));
                    }
                    _ => (),
                }
            }
        }
    }
}

impl DropdownList {
    pub fn selection(&self) -> Option<usize> {
        self.selection
    }

    pub fn close_on_selection(&self) -> bool {
        self.close_on_selection
    }

    pub fn items(&self) -> &[Handle<UiNode>] {
        &self.items
    }

    fn sync_selected_item_preview(&mut self, ui: &mut UserInterface) {
        // Copy node from current selection in list view. This is not
        // always suitable because if an item has some visual behaviour
        // (change color on mouse hover, change something on click, etc)
        // it will be also reflected in selected item.
        if self.current.is_some() {
            ui.send_message(WidgetMessage::remove(
                self.current,
                MessageDirection::ToWidget,
            ));
        }
        if let Some(index) = self.selection {
            if let Some(item) = self.items.get(index) {
                self.current = ui.copy_node(*item);
                ui.send_message(WidgetMessage::link(
                    self.current,
                    MessageDirection::ToWidget,
                    self.main_grid,
                ));
                ui.node(self.current).request_update_visibility();
                ui.send_message(WidgetMessage::margin(
                    self.current,
                    MessageDirection::ToWidget,
                    Thickness::uniform(0.0),
                ));
            } else {
                self.current = Handle::NONE;
            }
        } else {
            self.current = Handle::NONE;
        }
    }
}

pub struct DropdownListBuilder {
    widget_builder: WidgetBuilder,
    items: Vec<Handle<UiNode>>,
    selected: Option<usize>,
    close_on_selection: bool,
}

impl DropdownListBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            items: Default::default(),
            selected: None,
            close_on_selection: false,
        }
    }

    pub fn with_items(mut self, items: Vec<Handle<UiNode>>) -> Self {
        self.items = items;
        self
    }

    pub fn with_selected(mut self, index: usize) -> Self {
        self.selected = Some(index);
        self
    }

    pub fn with_opt_selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }

    pub fn with_close_on_selection(mut self, value: bool) -> Self {
        self.close_on_selection = value;
        self
    }

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

        let arrow = make_arrow(ctx, ArrowDirection::Bottom, 10.0);
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
                .with_preview_messages(true)
                .with_child(
                    BorderBuilder::new(
                        WidgetBuilder::new()
                            .with_background(BRUSH_DARKER)
                            .with_foreground(BRUSH_LIGHT)
                            .with_child(main_grid),
                    )
                    .build(ctx),
                )
                .build(),
            popup,
            items: self.items,
            list_view: items_control,
            current,
            selection: self.selected,
            close_on_selection: self.close_on_selection,
            main_grid,
        });

        ctx.add_node(dropdown_list)
    }
}
