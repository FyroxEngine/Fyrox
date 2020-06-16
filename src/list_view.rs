use crate::{
    scroll_viewer::ScrollViewerBuilder,
    Thickness,
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
        ListViewMessage,
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
    BuildContext,
    message::DecoratorMessage,
};
use std::ops::{Deref, DerefMut};

pub struct ListView<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    selected_index: Option<usize>,
    item_containers: Vec<Handle<UINode<M, C>>>,
    panel: Handle<UINode<M, C>>,
    items: Vec<Handle<UINode<M, C>>>,
}

impl<M: 'static, C: 'static + Control<M, C>> Deref for ListView<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> DerefMut for ListView<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> ListView<M, C> {
    pub fn new(widget: Widget<M, C>, items: Vec<Handle<UINode<M, C>>>) -> Self {
        Self {
            widget,
            selected_index: None,
            item_containers: items,
            panel: Default::default(),
            items: Default::default(),
        }
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected_index
    }

    pub fn item_containers(&self) -> &[Handle<UINode<M, C>>] {
        &self.item_containers
    }

    pub fn items(&self) -> &[Handle<UINode<M, C>>] {
        &self.items
    }
}

pub struct ListViewItem<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    index: usize,
}

impl<M: 'static, C: 'static + Control<M, C>> Deref for ListViewItem<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> DerefMut for ListViewItem<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> ListViewItem<M, C> {
    pub fn index(&self) -> usize {
        self.index
    }
}

impl<M: 'static, C: 'static + Control<M, C>> Control<M, C> for ListViewItem<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        UINode::ListViewItem(Self {
            widget: self.widget.raw_copy(),
            index: self.index,
        })
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        // Emit transparent geometry so item container can be picked by hit test.
        drawing_context.push_rect_filled(&self.widget.screen_bounds(), None);
        drawing_context.commit(CommandKind::Geometry, Brush::Solid(Color::TRANSPARENT), CommandTexture::None);
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_routed_message(ui, message);

        let items_control = self.find_by_criteria_up(ui, |node| {
            if let UINode::ListView(_) = node { true } else { false }
        });

        if let UiMessageData::Widget(msg) = &message.data {
            if let WidgetMessage::MouseUp { .. } = msg {
                if !message.handled {
                    // Explicitly set selection on parent items control. This will send
                    // SelectionChanged message and all items will react.
                    ui.send_message(UiMessage {
                        handled: false,
                        data: UiMessageData::ListView(ListViewMessage::SelectionChanged(Some(self.index))),
                        destination: items_control,
                    });
                    message.handled = true;
                }
            }
        }
    }
}

impl<M: 'static, C: 'static + Control<M, C>> Control<M, C> for ListView<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        UINode::ListView(Self {
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

    fn handle_routed_message(&mut self, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_routed_message(ui, message);

        if let UiMessageData::ListView(msg) = &message.data {
            if message.destination == self.handle() {
                match msg {
                    ListViewMessage::Items(items) => {
                        // Remove previous items.
                        for child in ui.node(self.panel).children().to_vec() {
                            ui.send_message(WidgetMessage::remove(child));
                        }

                        // Generate new items.
                        let item_containers = generate_item_containers(&mut ui.build_ctx(), items);

                        for item_container in item_containers.iter() {
                            ui.link_nodes(*item_container, self.panel);
                        }

                        self.item_containers = item_containers;
                        self.items = items.clone();
                    }
                    &ListViewMessage::AddItem(item) => {
                        let item_container = generate_item_container(&mut ui.build_ctx(), item, self.items.len());

                        ui.link_nodes(item_container, self.panel);

                        self.item_containers.push(item_container);
                        self.items.push(item);
                    }
                    &ListViewMessage::SelectionChanged(selection) => {
                        for (i, &container) in self.item_containers.iter().enumerate() {
                            let select = selection.map_or(false, |k| k == i);
                            if let UINode::ListViewItem(container) = ui.node(container) {
                                let mut stack = container.children().to_vec();
                                while let Some(handle) = stack.pop() {
                                    let node = ui.node(handle);
                                    match node {
                                        UINode::ListView(_) => {}
                                        UINode::Decorator(_) => {
                                            ui.send_message(DecoratorMessage::select(handle, select));
                                        }
                                        _ => stack.extend_from_slice(node.children())
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn remove_ref(&mut self, handle: Handle<UINode<M, C>>) {
        self.item_containers.retain(|i| *i != handle);
    }
}

pub struct ListViewBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    items: Vec<Handle<UINode<M, C>>>,
    panel: Option<Handle<UINode<M, C>>>,
    scroll_viewer: Option<Handle<UINode<M, C>>>,
}

impl<M: 'static, C: 'static + Control<M, C>> ListViewBuilder<M, C> {
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

    pub fn build(self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        let item_containers = generate_item_containers(ctx, &self.items);

        let panel = self.panel.unwrap_or_else(|| {
            StackPanelBuilder::new(WidgetBuilder::new()
                .with_children(&item_containers))
                .build(ctx)
        });

        let scroll_viewer = self.scroll_viewer.unwrap_or_else(|| {
            ScrollViewerBuilder::new(WidgetBuilder::new()
                .with_margin(Thickness::uniform(3.0)))
                .build(ctx)
        });
        if let UINode::ScrollViewer(scroll_viewer) = &mut ctx[scroll_viewer] {
            scroll_viewer.set_content(panel);
            let content_presenter = scroll_viewer.content_presenter;
            ctx.link(panel, content_presenter);
        } else {
            panic!("must be scroll viewer!");
        }

        let list_box = ListView {
            widget: self.widget_builder
                .with_child(scroll_viewer)
                .build(),
            selected_index: None,
            item_containers,
            items: self.items,
            panel,
        };

        ctx.add_node(UINode::ListView(list_box))
    }
}

fn generate_item_container<M: 'static, C: 'static + Control<M, C>>(ctx: &mut BuildContext<M, C>, item: Handle<UINode<M, C>>, index: usize) -> Handle<UINode<M, C>> {
    let item = ListViewItem {
        widget: WidgetBuilder::new()
            .with_child(item)
            .build(),
        index,
    };

    ctx.add_node(UINode::ListViewItem(item))
}

fn generate_item_containers<M: 'static, C: 'static + Control<M, C>>(ctx: &mut BuildContext<M, C>, items: &[Handle<UINode<M, C>>]) -> Vec<Handle<UINode<M, C>>> {
    items.iter()
        .enumerate()
        .map(|(index, &item)| generate_item_container(ctx, item, index))
        .collect()
}