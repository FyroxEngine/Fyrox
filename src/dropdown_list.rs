//! Drop-down list. This is control which shows currently selected item and provides drop-down
//! list to select its current item. It is build using composition with standard list view.

use crate::{
    border::BorderBuilder,
    core::{math::vec2::Vec2, pool::Handle},
    list_view::ListViewBuilder,
    message::PopupMessage,
    message::{DropdownListMessage, ListViewMessage, UiMessage, UiMessageData, WidgetMessage},
    node::UINode,
    popup::{Placement, PopupBuilder},
    widget::Widget,
    widget::WidgetBuilder,
    BuildContext, Control, NodeHandleMapping, UserInterface,
};
use std::ops::{Deref, DerefMut};

#[derive(Clone)]
pub struct DropdownList<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    popup: Handle<UINode<M, C>>,
    items: Vec<Handle<UINode<M, C>>>,
    list_view: Handle<UINode<M, C>>,
    current: Handle<UINode<M, C>>,
    selection: Option<usize>,
}

impl<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> Deref
    for DropdownList<M, C>
{
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> DerefMut
    for DropdownList<M, C>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> Control<M, C>
    for DropdownList<M, C>
{
    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        self.popup = *node_map.get(&self.popup).unwrap();
        self.list_view = *node_map.get(&self.list_view).unwrap();

        for item in self.items.iter_mut() {
            *item = *node_map.get(item).unwrap();
        }
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);

        match &message.data {
            UiMessageData::Widget(msg) => {
                if let WidgetMessage::MouseDown { .. } = msg {
                    if message.destination == self.handle()
                        || self.widget.has_descendant(message.destination, ui)
                    {
                        ui.send_message(WidgetMessage::width(self.popup, self.actual_size().x));
                        let placement_position = self.widget.screen_position
                            + Vec2::new(0.0, self.widget.actual_size().y);
                        ui.send_message(UiMessage {
                            handled: false,
                            data: UiMessageData::Popup(PopupMessage::Placement(
                                Placement::Position(placement_position),
                            )),
                            destination: self.popup,
                        });
                        ui.send_message(UiMessage {
                            handled: false,
                            data: UiMessageData::Popup(PopupMessage::Open),
                            destination: self.popup,
                        });
                    }
                }
            }
            UiMessageData::DropdownList(msg) if message.destination == self.handle() => match msg {
                DropdownListMessage::Items(items) => {
                    ListViewMessage::items(self.list_view, items.clone());
                    self.items = items.clone();
                }
                &DropdownListMessage::AddItem(item) => {
                    ListViewMessage::add_item(self.list_view, item);
                    self.items.push(item);
                }
                &DropdownListMessage::SelectionChanged(selection) => {
                    if selection != self.selection {
                        self.selection = selection.clone();
                        ui.send_message(ListViewMessage::selection(self.list_view, selection));
                    }
                }
            },
            _ => {}
        }
    }

    fn preview_message(&mut self, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        if let UiMessageData::ListView(msg) = &message.data {
            if let ListViewMessage::SelectionChanged(selection) = msg {
                if message.destination == self.list_view {
                    // Copy node from current selection in list view. This is not
                    // always suitable because if an item has some visual behaviour
                    // (change color on mouse hover, change something on click, etc)
                    // it will be also reflected in selected item.
                    if self.current.is_some() {
                        ui.send_message(WidgetMessage::remove(self.current));
                    }
                    if let Some(index) = selection {
                        if let Some(item) = self.items.get(*index) {
                            self.current = ui.copy_node(*item);
                            let body = self.widget.children()[0];
                            ui.link_nodes(self.current, body);
                        } else {
                            self.current = Handle::NONE;
                        }
                    } else {
                        self.current = Handle::NONE;
                    }

                    if &self.selection != selection {
                        // Post message again but from name of this drop-down list so user can catch
                        // message and respond properly.
                        ui.send_message(DropdownListMessage::selection(
                            self.handle,
                            selection.clone(),
                        ));
                    }
                }
            }
        }
    }
}

pub struct DropdownListBuilder<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    items: Vec<Handle<UINode<M, C>>>,
    selected: usize,
}

impl<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> DropdownListBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            items: Default::default(),
            selected: 0,
        }
    }

    pub fn with_items(mut self, items: Vec<Handle<UINode<M, C>>>) -> Self {
        self.items = items;
        self
    }

    pub fn with_selected(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    pub fn build(self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>>
    where
        Self: Sized,
    {
        let items_control = ListViewBuilder::new(
            WidgetBuilder::new().with_max_size(Vec2::new(std::f32::INFINITY, 300.0)),
        )
        .with_items(self.items.clone())
        .build(ctx);

        let popup = PopupBuilder::new(WidgetBuilder::new())
            .with_content(items_control)
            .build(ctx);

        let current = self
            .items
            .get(self.selected)
            .map_or(Handle::NONE, |&f| ctx.copy(f));

        let dropdown_list = UINode::DropdownList(DropdownList {
            widget: self
                .widget_builder
                .with_child(BorderBuilder::new(WidgetBuilder::new().with_child(current)).build(ctx))
                .build(),
            popup,
            items: self.items,
            list_view: items_control,
            current,
            selection: Some(self.selected),
        });

        ctx.add_node(dropdown_list)
    }
}
