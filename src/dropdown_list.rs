//! Drop-down list. This is control which shows currently selected item and provides drop-down
//! list to select its current item. It is build using composition with standard list view.

use crate::{
    list_view::ListViewBuilder,
    node::UINode,
    Control,
    widget::Widget,
    core::{
        pool::Handle,
        math::vec2::Vec2,
    },
    widget::WidgetBuilder,
    UserInterface,
    message::{
        UiMessage,
        UiMessageData,
        ListViewMessage,
        WidgetMessage,
    },
    popup::{
        PopupBuilder,
        Placement,
    },
    border::BorderBuilder,
    NodeHandleMapping,
};
use std::ops::{Deref, DerefMut};

pub struct DropdownList<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    popup: Handle<UINode<M, C>>,
    items: Vec<Handle<UINode<M, C>>>,
    list_view: Handle<UINode<M, C>>,
    current: Handle<UINode<M, C>>,
}

impl<M: 'static, C: 'static + Control<M, C>> Deref for DropdownList<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> DerefMut for DropdownList<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> Control<M, C> for DropdownList<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        UINode::DropdownList(Self {
            widget: self.widget.raw_copy(),
            popup: self.popup,
            items: self.items.clone(),
            list_view: self.list_view,
            current: self.current,
        })
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        self.popup = *node_map.get(&self.popup).unwrap();
        self.list_view = *node_map.get(&self.list_view).unwrap();

        for item in self.items.iter_mut() {
            *item = *node_map.get(item).unwrap();
        }
    }

    fn handle_routed_message(&mut self, self_handle: Handle<UINode<M, C>>, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_routed_message(self_handle, ui, message);

        match &message.data {
            UiMessageData::Widget(msg) => {
                if let WidgetMessage::MouseDown { .. } = msg {
                    if message.destination == self_handle || self.widget.has_descendant(message.destination, ui) {
                        if let UINode::Popup(popup) = ui.node_mut(self.popup) {
                            popup.set_width_mut(self.widget.actual_size().x);
                            let placement_position = self.widget.screen_position + Vec2::new(0.0, self.widget.actual_size().y);
                            popup.set_placement(Placement::Position(placement_position));
                            popup.open();
                        }
                    }
                }
            }
            UiMessageData::ListView(msg) => {
                match msg {
                    ListViewMessage::Items(items) => {
                        if message.destination == self_handle {
                            ui.post_message(UiMessage {
                                destination: self.list_view,
                                data: UiMessageData::ListView(ListViewMessage::Items(items.clone())),
                                ..Default::default()
                            });
                            self.items = items.clone();
                        }
                    }
                    &ListViewMessage::AddItem(item) => {
                        if message.destination == self_handle {
                            ui.post_message(UiMessage {
                                destination: self.list_view,
                                data: UiMessageData::ListView(ListViewMessage::AddItem(item)),
                                ..Default::default()
                            });
                            self.items.push(item);
                        }
                    }
                    _ => ()
                }
            }
            _ => {}
        }
    }

    fn preview_message(&mut self, _self_handle: Handle<UINode<M, C>>, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        if let UiMessageData::ListView(msg) = &message.data {
            if let ListViewMessage::SelectionChanged(selection) = msg {
                if message.destination == self.list_view {
                    // Copy node from current selection in items controls. This is not
                    // always suitable because if an item has some visual behaviour
                    // (change color on mouse hover, change something on click, etc)
                    // it will be also reflected in selected item.
                    if self.current.is_some() {
                        ui.remove_node(self.current)
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
                    // Post message again but from name of this drop-down list so user can catch
                    // message and respond properly.
                    self.post_message(UiMessage {
                        data: UiMessageData::ListView(ListViewMessage::SelectionChanged(*selection)),
                        ..Default::default()
                    })
                }
            }
        }
    }
}

impl<M: 'static, C: 'static + Control<M, C>> DropdownList<M, C> {
    pub fn set_items(&mut self, items: Vec<Handle<UINode<M, C>>>) {
        self.post_message(UiMessage {
            destination: self.list_view,
            data: UiMessageData::ListView(ListViewMessage::Items(items)),
            ..Default::default()
        })
    }
}

pub struct DropdownListBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    items: Vec<Handle<UINode<M, C>>>,
}

impl<M: 'static, C: 'static + Control<M, C>> DropdownListBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            items: Default::default(),
        }
    }

    pub fn with_items(mut self, items: Vec<Handle<UINode<M, C>>>) -> Self {
        self.items = items;
        self
    }

    pub fn build(self, ui: &mut UserInterface<M, C>) -> Handle<UINode<M, C>> where Self: Sized {
        let items_control = ListViewBuilder::new(WidgetBuilder::new()
            .with_max_size(Vec2::new(std::f32::INFINITY, 300.0)))
            .with_items(self.items.clone())
            .build(ui);

        let popup = PopupBuilder::new(WidgetBuilder::new())
            .with_content(items_control)
            .build(ui);

        let current =
            if let Some(first) = self.items.get(0) {
                ui.copy_node(*first)
            } else {
                Handle::NONE
            };

        let dropdown_list = UINode::DropdownList(DropdownList {
            widget: self.widget_builder
                .with_child(BorderBuilder::new(WidgetBuilder::new()
                    .with_child(current))
                    .build(ui))
                .build(),
            popup,
            items: self.items,
            list_view: items_control,
            current,
        });

        let handle = ui.add_node(dropdown_list);

        ui.flush_messages();

        handle
    }
}