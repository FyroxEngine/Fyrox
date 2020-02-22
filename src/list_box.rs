use crate::{
    scroll_viewer::ScrollViewerBuilder,
    Thickness,
    border::BorderBuilder,
    widget::{
        Widget,
        WidgetBuilder,
    },
    UINode,
    UserInterface,
    stack_panel::StackPanelBuilder,
    message::{
        UiMessageData,
        UiMessage,
        ItemsControlMessage,
        WidgetMessage
    },
    Control,
    core::{
        pool::Handle,
        color::Color,
    },
    brush::Brush,
};

pub struct ListBox<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    selected_index: Option<usize>,
    items: Vec<Handle<UINode<M, C>>>,
}

impl<M, C: 'static + Control<M, C>> ListBox<M, C> {
    pub fn new(widget: Widget<M, C>, items: Vec<Handle<UINode<M, C>>>) -> Self {
        Self {
            widget,
            selected_index: None,
            items,
        }
    }

    pub fn set_selected(&mut self, new_index: Option<usize>) {
        let old_value = self.selected_index;

        self.selected_index = new_index;

        if old_value.is_none() && new_index.is_some() ||
            old_value.is_some() && new_index.is_none() ||
            old_value.unwrap() != new_index.unwrap() {
            self.widget
                .outgoing_messages
                .borrow_mut()
                .push_back(UiMessage::new(UiMessageData::ItemsControl(ItemsControlMessage::SelectionChanged(self.selected_index))))
        }
    }

    pub fn get_selected(&self) -> Option<usize> {
        self.selected_index
    }

    pub fn get_items(&self) -> &[Handle<UINode<M, C>>] {
        &self.items
    }
}

pub struct ListBoxItem<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    body: Handle<UINode<M, C>>,
    index: usize,
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for ListBoxItem<M, C> {
    fn widget(&self) -> &Widget<M, C> {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget<M, C> {
        &mut self.widget
    }

    fn handle_message(&mut self, self_handle: Handle<UINode<M, C>>, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_message(self_handle, ui, message);

        let list_box = self.widget().find_by_criteria_up(ui, |node| {
            if let UINode::ListBox(_) = node { true } else { false }
        });

        match &message.data {
            UiMessageData::Widget(msg) => {
                if self.body.is_some() && (message.source == self_handle || self.widget().has_descendant(message.source, ui)) {
                    let body = ui.node_mut(self.body).widget_mut();
                    match msg {
                        WidgetMessage::MouseLeave => {
                            body.set_background(Brush::Solid(Color::opaque(100, 100, 100)));
                        }
                        WidgetMessage::MouseEnter => {
                            body.set_background(Brush::Solid(Color::opaque(130, 130, 130)));
                        }
                        WidgetMessage::MouseDown { .. } => {
                            // Explicitly set selection on parent list box. This will send
                            // SelectionChanged event and all items will react.
                            if let UINode::ListBox(list_box) = ui.node_mut(list_box) {
                                list_box.set_selected(Some(self.index));
                            }
                        }
                        _ => ()
                    }
                }
            }
            UiMessageData::ItemsControl(msg) => {
                if let UINode::Border(border) = ui.node_mut(self.body) {
                    if message.source == list_box && self.body.is_some() {
                        if let ItemsControlMessage::SelectionChanged(new_value) = msg {
                            // We know now that selection has changed in parent list box,
                            // check at which index and keep visual state according to it.
                            if let Some(new_value) = *new_value {
                                if new_value == self.index {
                                    border.widget_mut().set_foreground(Brush::Solid(Color::opaque(0, 0, 0)));
                                    border.set_stroke_thickness(Thickness::uniform(2.0));
                                    return;
                                }
                            }
                            border.widget_mut().set_foreground(Brush::Solid(Color::opaque(80, 80, 80)));
                            border.set_stroke_thickness(Thickness::uniform(1.0));
                        }
                    }
                }
            }
            _ => ()
        }
    }

    fn remove_ref(&mut self, handle: Handle<UINode<M, C>>) {
        if self.body == handle {
            self.body = Handle::NONE;
        }
    }
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for ListBox<M, C> {
    fn widget(&self) -> &Widget<M, C> {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget<M, C> {
        &mut self.widget
    }


    fn remove_ref(&mut self, handle: Handle<UINode<M, C>>) {
        self.items.retain(|i| *i != handle);
    }
}

pub struct ListBoxBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    items: Vec<Handle<UINode<M, C>>>,
}

impl<M, C: 'static + Control<M, C>> ListBoxBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            items: Vec::new(),
        }
    }

    pub fn with_items(mut self, items: Vec<Handle<UINode<M, C>>>) -> Self {
        self.items = items;
        self
    }

    pub fn build(self, ui: &mut UserInterface<M, C>) -> Handle<UINode<M, C>> {
        // Wrap each item into container which will have selection behaviour
        let items: Vec<Handle<UINode<M, C>>> = self.items.iter().enumerate().map(|(index, item)| {
            let body = BorderBuilder::new(WidgetBuilder::new()
                .with_foreground(Brush::Solid(Color::opaque(60, 60, 60)))
                .with_background(Brush::Solid(Color::opaque(80, 80, 80)))
                .with_child(*item))
                .with_stroke_thickness(Thickness::uniform(1.0))
                .build(ui);

            let item = ListBoxItem {
                widget: WidgetBuilder::new()
                    .with_child(body)
                    .build(),
                body,
                index,
            };

            ui.add_node(UINode::ListBoxItem(item))
        }).collect();

        let panel = StackPanelBuilder::new(WidgetBuilder::new()
            .with_children(&items))
            .build(ui);

        let scroll_viewer = ScrollViewerBuilder::new(WidgetBuilder::new()
            .with_margin(Thickness::uniform(3.0)))
            .with_content(panel)
            .build(ui);

        let list_box = ListBox {
            widget: self.widget_builder
                .with_child(BorderBuilder::new(WidgetBuilder::new()
                    .with_background(Brush::Solid(Color::opaque(100, 100, 100)))
                    .with_child(scroll_viewer))
                    .build(ui))
                .build(),
            selected_index: None,
            items,
        };

        let handle = ui.add_node(UINode::ListBox(list_box));

        ui.flush_messages();

        handle
    }
}