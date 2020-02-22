// SEMI-COMPLETE DO NOT USE

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
};

pub struct ComboBox<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    popup: Handle<UINode<M, C>>,
    items: Vec<Handle<UINode<M, C>>>,
    items_control: Handle<UINode<M, C>>,
}

impl<M: 'static, C: 'static + Control<M, C>> Control<M, C> for ComboBox<M, C> {
    fn widget(&self) -> &Widget<M, C> {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget<M, C> {
        &mut self.widget
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
                if let ItemsControlMessage::Items(items) = msg {
                    if message.target == self_handle {
                        ui.post_message(UiMessage::targeted(
                            self.items_control,
                            UiMessageData::ItemsControl(
                                ItemsControlMessage::Items(items.clone()))));
                        self.items = items.clone();
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
    panel: Handle<UINode<M, C>>,
    items: Vec<Handle<UINode<M, C>>>,
}

impl<M: 'static, C: 'static + Control<M, C>> ComboBoxBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            panel: Default::default(),
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

        let combobox = UINode::ComboBox(ComboBox {
            widget: self.widget_builder
                .with_child(BorderBuilder::new(WidgetBuilder::new())
                    .build(ui))
                .build(),
            popup,
            items: self.items,
            items_control,
        });

        let handle = ui.add_node(combobox);

        ui.flush_messages();

        handle
    }
}