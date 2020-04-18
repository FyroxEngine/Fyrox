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
        WidgetMessage,
    },
    Control,
    core::{
        pool::Handle,
        color::Color,
    },
    brush::Brush,
    NodeHandleMapping,
    draw::{DrawingContext, CommandTexture, CommandKind},
};
use std::ops::{Deref, DerefMut};

pub struct ItemsControl<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    selected_index: Option<usize>,
    item_containers: Vec<Handle<UINode<M, C>>>,
    panel: Handle<UINode<M, C>>,
    items: Vec<Handle<UINode<M, C>>>,
}

impl<M: 'static, C: 'static + Control<M, C>> Deref for ItemsControl<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> DerefMut for ItemsControl<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M, C: 'static + Control<M, C>> ItemsControl<M, C> {
    pub fn new(widget: Widget<M, C>, items: Vec<Handle<UINode<M, C>>>) -> Self {
        Self {
            widget,
            selected_index: None,
            item_containers: items,
            panel: Default::default(),
            items: Default::default(),
        }
    }

    pub fn set_selected(&mut self, new_index: Option<usize>) {
        let old_value = self.selected_index;

        self.selected_index = new_index;

        if old_value.is_none() && new_index.is_some() ||
            old_value.is_some() && new_index.is_none() ||
            old_value.unwrap() != new_index.unwrap() {
            self.widget.post_message(UiMessage::new(
                UiMessageData::ItemsControl(
                    ItemsControlMessage::SelectionChanged(self.selected_index))))
        }
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected_index
    }

    pub fn item_containers(&self) -> &[Handle<UINode<M, C>>] {
        &self.item_containers
    }
}

pub struct ItemContainer<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    index: usize,
}

impl<M: 'static, C: 'static + Control<M, C>> Deref for ItemContainer<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> DerefMut for ItemContainer<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M, C: 'static + Control<M, C>> ItemContainer<M, C> {
    pub fn index(&self) -> usize {
        self.index
    }
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for ItemContainer<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        UINode::ItemContainer(Self {
            widget: self.widget.raw_copy(),
            index: self.index,
        })
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        // Emit transparent geometry so item container can be picked by hit test.
        drawing_context.push_rect_filled(&self.widget.screen_bounds(), None);
        drawing_context.commit(CommandKind::Geometry, Brush::Solid(Color::TRANSPARENT), CommandTexture::None);
    }

    fn handle_message(&mut self, self_handle: Handle<UINode<M, C>>, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_message(self_handle, ui, message);

        let items_control = self.find_by_criteria_up(ui, |node| {
            if let UINode::ItemsControl(_) = node { true } else { false }
        });

        if let UiMessageData::Widget(msg) = &message.data {
            if message.source == self_handle || self.has_descendant(message.source, ui) {
                if let WidgetMessage::MouseUp { .. } = msg {
                    // Explicitly set selection on parent items control. This will send
                    // SelectionChanged message and all items will react.
                    if let UINode::ItemsControl(items_control) = ui.node_mut(items_control) {
                        items_control.set_selected(Some(self.index));
                    }
                }
            }
        }
    }
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for ItemsControl<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        UINode::ItemsControl(Self {
            widget: self.widget.raw_copy(),
            selected_index: self.selected_index,
            item_containers: self.item_containers.clone(),
            panel: self.panel,
            items: self.items.clone(),
        })
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        self.panel = *node_map.get(&self.panel).unwrap();
        for item_container in self.item_containers.iter_mut() {
            *item_container = *node_map.get(item_container).unwrap();
        }
        for item in self.items.iter_mut() {
            *item = *node_map.get(item).unwrap();
        }
    }

    fn handle_message(&mut self, self_handle: Handle<UINode<M, C>>, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_message(self_handle, ui, message);

        if let UiMessageData::ItemsControl(msg) = &message.data {
            if let ItemsControlMessage::Items(items) = msg {
                if message.target == self_handle {
                    // Remove previous items.
                    for child in ui.node(self.panel).children().to_vec() {
                        ui.remove_node(child);
                    }

                    // Generate new items.
                    let item_containers = generate_item_containers(ui, items);

                    for item_container in item_containers.iter() {
                        ui.link_nodes(*item_container, self.panel);
                    }

                    self.items = items.clone();
                }
            }
        }
    }

    fn remove_ref(&mut self, handle: Handle<UINode<M, C>>) {
        self.item_containers.retain(|i| *i != handle);
    }
}

pub struct ItemsControlBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    items: Vec<Handle<UINode<M, C>>>,
    panel: Option<Handle<UINode<M, C>>>,
    scroll_viewer: Option<Handle<UINode<M, C>>>,
}

impl<M, C: 'static + Control<M, C>> ItemsControlBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            items: Vec::new(),
            panel: None,
            scroll_viewer: None,
        }
    }

    pub fn with_items(mut self, items: Vec<Handle<UINode<M, C>>>) -> Self {
        self.items = items;
        self
    }

    pub fn with_items_panel(mut self, panel: Handle<UINode<M, C>>) -> Self {
        self.panel = Some(panel);
        self
    }

    pub fn with_scroll_viewer(mut self, sv: Handle<UINode<M, C>>) -> Self {
        self.scroll_viewer = Some(sv);
        self
    }

    pub fn build(self, ui: &mut UserInterface<M, C>) -> Handle<UINode<M, C>> {
        let item_containers = generate_item_containers(ui, &self.items);

        let panel = self.panel.unwrap_or_else(|| {
            StackPanelBuilder::new(WidgetBuilder::new())
                .build(ui)
        });

        for item_container in item_containers.iter() {
            ui.link_nodes(*item_container, panel);
        }

        let scroll_viewer = self.scroll_viewer.unwrap_or_else(|| {
            ScrollViewerBuilder::new(WidgetBuilder::new()
                .with_margin(Thickness::uniform(3.0)))
                .build(ui)
        });

        if let UINode::ScrollViewer(sv) = ui.node_mut(scroll_viewer) {
            sv.set_content(panel);
        }

        let list_box = ItemsControl {
            widget: self.widget_builder
                .with_child(BorderBuilder::new(WidgetBuilder::new()
                    .with_background(Brush::Solid(Color::opaque(100, 100, 100)))
                    .with_child(scroll_viewer))
                    .build(ui))
                .build(),
            selected_index: None,
            item_containers,
            items: self.items,
            panel,
        };

        let handle = ui.add_node(UINode::ItemsControl(list_box));

        ui.flush_messages();

        handle
    }
}

fn generate_item_containers<M: 'static, C: 'static + Control<M, C>>(ui: &mut UserInterface<M, C>, items: &[Handle<UINode<M, C>>]) -> Vec<Handle<UINode<M, C>>> {
    items.iter().enumerate().map(|(index, item)| {
        let item = ItemContainer {
            widget: WidgetBuilder::new()
                .with_child(*item)
                .build(),
            index,
        };

        ui.add_node(UINode::ItemContainer(item))
    }).collect()
}