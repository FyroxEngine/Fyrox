use crate::{
    items_control::ItemsControlBuilder,
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
        ItemsControlMessage,
        WidgetMessage,
    },
    popup::{
        PopupBuilder,
        Placement,
    },
    border::BorderBuilder,
    NodeHandleMapping,
};

pub struct ComboBox<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    popup: Handle<UINode<M, C>>,
    items: Vec<Handle<UINode<M, C>>>,
    items_control: Handle<UINode<M, C>>,
    current: Handle<UINode<M, C>>,
}

impl<M: 'static, C: 'static + Control<M, C>> Control<M, C> for ComboBox<M, C> {
    fn widget(&self) -> &Widget<M, C> {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget<M, C> {
        &mut self.widget
    }

    fn raw_copy(&self) -> UINode<M, C> {
        UINode::ComboBox(Self {
            widget: self.widget.raw_copy(),
            popup: self.popup,
            items: self.items.clone(),
            items_control: self.items_control,
            current: self.current,
        })
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        self.popup = *node_map.get(&self.popup).unwrap();
        self.items_control = *node_map.get(&self.items_control).unwrap();

        for item in self.items.iter_mut() {
            *item = *node_map.get(item).unwrap();
        }
    }

    fn handle_message(&mut self, self_handle: Handle<UINode<M, C>>, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        match &message.data {
            UiMessageData::Widget(msg) => {
                if let WidgetMessage::MouseDown { .. } = msg {
                    if message.source == self_handle || self.widget.has_descendant(message.source, ui) {
                        if let UINode::Popup(popup) = ui.node_mut(self.popup) {
                            popup.widget_mut()
                                .set_width_mut(self.widget.actual_size().x);
                            let placement_position = self.widget.screen_position + Vec2::new(0.0, self.widget.actual_size().y);
                            popup.set_placement(Placement::Position(placement_position));
                            popup.open();
                        }
                    }
                }
            }
            UiMessageData::ItemsControl(msg) => {
                match msg {
                    ItemsControlMessage::Items(items) => {
                        if message.target == self_handle {
                            ui.post_message(UiMessage::targeted(
                                self.items_control,
                                UiMessageData::ItemsControl(
                                    ItemsControlMessage::Items(items.clone()))));
                            self.items = items.clone();
                        }
                    }
                    ItemsControlMessage::SelectionChanged(selection) => {
                        if message.source == self.items_control {
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
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

impl<M: 'static, C: 'static + Control<M, C>> ComboBox<M, C> {
    pub fn set_items(&mut self, items: Vec<Handle<UINode<M, C>>>) {
        self.widget
            .outgoing_messages
            .borrow_mut()
            .push_back(UiMessage::targeted(
                self.items_control,
                UiMessageData::ItemsControl(
                    ItemsControlMessage::Items(items))))
    }
}

pub struct ComboBoxBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    items: Vec<Handle<UINode<M, C>>>,
}

impl<M: 'static, C: 'static + Control<M, C>> ComboBoxBuilder<M, C> {
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
        let items_control = ItemsControlBuilder::new(WidgetBuilder::new())
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

        let combobox = UINode::ComboBox(ComboBox {
            widget: self.widget_builder
                .with_child(BorderBuilder::new(WidgetBuilder::new()
                    .with_child(current))
                    .build(ui))
                .build(),
            popup,
            items: self.items,
            items_control,
            current,
        });

        let handle = ui.add_node(combobox);

        ui.flush_messages();

        handle
    }
}