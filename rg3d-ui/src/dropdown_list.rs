//! Drop-down list. This is control which shows currently selected item and provides drop-down
//! list to select its current item. It is build using composition with standard list view.

use crate::core::algebra::Vector2;
use crate::grid::{Column, GridBuilder, Row};
use crate::message::{MessageData, MessageDirection};
use crate::utils::{make_arrow, ArrowDirection};
use crate::{
    border::BorderBuilder,
    core::pool::Handle,
    list_view::ListViewBuilder,
    message::PopupMessage,
    message::{DropdownListMessage, ListViewMessage, UiMessage, UiMessageData, WidgetMessage},
    node::UINode,
    popup::{Placement, PopupBuilder},
    widget::Widget,
    widget::WidgetBuilder,
    BuildContext, Control, NodeHandleMapping, UserInterface, BRUSH_LIGHT,
};
use std::ops::{Deref, DerefMut};

#[derive(Clone)]
pub struct DropdownList<M: MessageData, C: Control<M, C>> {
    widget: Widget<M, C>,
    popup: Handle<UINode<M, C>>,
    items: Vec<Handle<UINode<M, C>>>,
    list_view: Handle<UINode<M, C>>,
    current: Handle<UINode<M, C>>,
    selection: Option<usize>,
    close_on_selection: bool,
    main_grid: Handle<UINode<M, C>>,
}

crate::define_widget_deref!(DropdownList<M, C>);

impl<M: MessageData, C: Control<M, C>> Control<M, C> for DropdownList<M, C> {
    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        node_map.resolve(&mut self.popup);
        node_map.resolve(&mut self.list_view);
        node_map.resolve(&mut self.current);
        node_map.resolve(&mut self.main_grid);
        node_map.resolve_slice(&mut self.items);
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);

        match &message.data() {
            UiMessageData::Widget(WidgetMessage::MouseDown { .. }) => {
                if message.destination() == self.handle()
                    || self.widget.has_descendant(message.destination(), ui)
                {
                    ui.send_message(WidgetMessage::width(
                        self.popup,
                        MessageDirection::ToWidget,
                        self.actual_size().x,
                    ));
                    let placement_position = self.widget.screen_position
                        + Vector2::new(0.0, self.widget.actual_size().y);
                    ui.send_message(PopupMessage::placement(
                        self.popup,
                        MessageDirection::ToWidget,
                        Placement::Position(placement_position),
                    ));
                    ui.send_message(PopupMessage::open(self.popup, MessageDirection::ToWidget));
                }
            }
            UiMessageData::DropdownList(msg)
                if message.destination() == self.handle()
                    && message.direction() == MessageDirection::ToWidget =>
            {
                match msg {
                    DropdownListMessage::Items(items) => {
                        ui.send_message(ListViewMessage::items(
                            self.list_view,
                            MessageDirection::ToWidget,
                            items.clone(),
                        ));
                        self.items = items.clone();
                    }
                    &DropdownListMessage::AddItem(item) => {
                        ListViewMessage::add_item(self.list_view, MessageDirection::ToWidget, item);
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
                            if let Some(index) = selection {
                                if let Some(item) = self.items.get(index) {
                                    self.current = ui.copy_node(*item);
                                    ui.send_message(WidgetMessage::link(
                                        self.current,
                                        MessageDirection::ToWidget,
                                        self.main_grid,
                                    ));
                                } else {
                                    self.current = Handle::NONE;
                                }
                            } else {
                                self.current = Handle::NONE;
                            }

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
            _ => {}
        }
    }

    fn preview_message(&self, ui: &UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        if let UiMessageData::ListView(msg) = &message.data() {
            if message.direction() == MessageDirection::FromWidget {
                if let ListViewMessage::SelectionChanged(selection) = msg {
                    if message.destination() == self.list_view && &self.selection != selection {
                        // Post message again but from name of this drop-down list so user can catch
                        // message and respond properly.
                        ui.send_message(DropdownListMessage::selection(
                            self.handle,
                            MessageDirection::ToWidget,
                            *selection,
                        ));
                    }
                }
            }
        }
    }
}

impl<M: MessageData, C: Control<M, C>> DropdownList<M, C> {
    pub fn selection(&self) -> Option<usize> {
        self.selection
    }

    pub fn close_on_selection(&self) -> bool {
        self.close_on_selection
    }

    pub fn items(&self) -> &[Handle<UINode<M, C>>] {
        &self.items
    }
}

pub struct DropdownListBuilder<M: MessageData, C: Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    items: Vec<Handle<UINode<M, C>>>,
    selected: Option<usize>,
    close_on_selection: bool,
}

impl<M: MessageData, C: Control<M, C>> DropdownListBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            items: Default::default(),
            selected: None,
            close_on_selection: false,
        }
    }

    pub fn with_items(mut self, items: Vec<Handle<UINode<M, C>>>) -> Self {
        self.items = items;
        self
    }

    pub fn with_selected(mut self, index: usize) -> Self {
        self.selected = Some(index);
        self
    }

    pub fn with_close_on_selection(mut self, value: bool) -> Self {
        self.close_on_selection = value;
        self
    }

    pub fn build(self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>>
    where
        Self: Sized,
    {
        let items_control = ListViewBuilder::new(
            WidgetBuilder::new().with_max_size(Vector2::new(std::f32::INFINITY, 200.0)),
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
        ctx[arrow].set_column(1);

        let main_grid =
            GridBuilder::new(WidgetBuilder::new().with_child(current).with_child(arrow))
                .add_row(Row::stretch())
                .add_column(Column::stretch())
                .add_column(Column::strict(20.0))
                .build(ctx);

        let dropdown_list = UINode::DropdownList(DropdownList {
            widget: self
                .widget_builder
                .with_child(
                    BorderBuilder::new(
                        WidgetBuilder::new()
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
