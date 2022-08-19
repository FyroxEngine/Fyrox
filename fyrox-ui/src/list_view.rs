use crate::{
    border::BorderBuilder,
    brush::Brush,
    core::{color::Color, pool::Handle},
    decorator::{Decorator, DecoratorMessage},
    define_constructor,
    draw::{CommandTexture, Draw, DrawingContext},
    message::{MessageDirection, UiMessage},
    scroll_viewer::{ScrollViewer, ScrollViewerBuilder, ScrollViewerMessage},
    stack_panel::StackPanelBuilder,
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, NodeHandleMapping, Thickness, UiNode, UserInterface, BRUSH_DARK,
    BRUSH_LIGHT,
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListViewMessage {
    SelectionChanged(Option<usize>),
    Items(Vec<Handle<UiNode>>),
    AddItem(Handle<UiNode>),
    RemoveItem(Handle<UiNode>),
    BringItemIntoView(Handle<UiNode>),
}

impl ListViewMessage {
    define_constructor!(ListViewMessage:SelectionChanged => fn selection(Option<usize>), layout: false);
    define_constructor!(ListViewMessage:Items => fn items(Vec<Handle<UiNode >>), layout: false);
    define_constructor!(ListViewMessage:AddItem => fn add_item(Handle<UiNode>), layout: false);
    define_constructor!(ListViewMessage:RemoveItem => fn remove_item(Handle<UiNode>), layout: false);
    define_constructor!(ListViewMessage:BringItemIntoView => fn bring_item_into_view(Handle<UiNode>), layout: false);
}

#[derive(Clone)]
pub struct ListView {
    pub widget: Widget,
    pub selected_index: Option<usize>,
    pub item_containers: Vec<Handle<UiNode>>,
    pub panel: Handle<UiNode>,
    pub items: Vec<Handle<UiNode>>,
    pub scroll_viewer: Handle<UiNode>,
}

crate::define_widget_deref!(ListView);

impl ListView {
    pub fn new(widget: Widget, items: Vec<Handle<UiNode>>) -> Self {
        Self {
            widget,
            selected_index: None,
            item_containers: items,
            panel: Default::default(),
            items: Default::default(),
            scroll_viewer: Default::default(),
        }
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected_index
    }

    pub fn item_containers(&self) -> &[Handle<UiNode>] {
        &self.item_containers
    }

    pub fn items(&self) -> &[Handle<UiNode>] {
        &self.items
    }

    pub fn scroll_viewer(&self) -> Handle<UiNode> {
        self.scroll_viewer
    }

    fn fix_selection(&self, ui: &UserInterface) {
        // Check if current selection is out-of-bounds.
        if let Some(selected_index) = self.selected_index {
            if selected_index >= self.items.len() {
                let new_selection = if self.items.is_empty() {
                    None
                } else {
                    Some(self.items.len() - 1)
                };

                ui.send_message(ListViewMessage::selection(
                    self.handle,
                    MessageDirection::ToWidget,
                    new_selection,
                ));
            }
        }
    }

    fn sync_decorators(&self, ui: &UserInterface) {
        for (i, &container) in self.item_containers.iter().enumerate() {
            let select = match self.selected_index {
                None => false,
                Some(selected_index) => i == selected_index,
            };
            if let Some(container) = ui.node(container).cast::<ListViewItem>() {
                let mut stack = container.children().to_vec();
                while let Some(handle) = stack.pop() {
                    let node = ui.node(handle);

                    if node.cast::<ListView>().is_some() {
                        // Do nothing.
                    } else if node.cast::<Decorator>().is_some() {
                        ui.send_message(DecoratorMessage::select(
                            handle,
                            MessageDirection::ToWidget,
                            select,
                        ));
                    } else {
                        stack.extend_from_slice(node.children())
                    }
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct ListViewItem {
    pub widget: Widget,
}

crate::define_widget_deref!(ListViewItem);

impl Control for ListViewItem {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        // Emit transparent geometry so item container can be picked by hit test.
        drawing_context.push_rect_filled(&self.widget.bounding_rect(), None);
        drawing_context.commit(
            self.clip_bounds(),
            Brush::Solid(Color::TRANSPARENT),
            CommandTexture::None,
            None,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        let parent_list_view =
            self.find_by_criteria_up(ui, |node| node.cast::<ListView>().is_some());

        if let Some(WidgetMessage::MouseUp { .. }) = message.data::<WidgetMessage>() {
            if !message.handled() {
                let self_index = ui
                    .node(parent_list_view)
                    .cast::<ListView>()
                    .expect("Parent of ListViewItem must be ListView!")
                    .item_containers
                    .iter()
                    .position(|c| *c == self.handle)
                    .expect("ListViewItem must be used as a child of ListView");

                // Explicitly set selection on parent items control. This will send
                // SelectionChanged message and all items will react.
                ui.send_message(ListViewMessage::selection(
                    parent_list_view,
                    MessageDirection::ToWidget,
                    Some(self_index),
                ));
                message.set_handled(true);
            }
        }
    }
}

impl Control for ListView {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        node_map.resolve(&mut self.panel);
        node_map.resolve_slice(&mut self.items);
        node_map.resolve_slice(&mut self.item_containers);
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<ListViewMessage>() {
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::ToWidget
            {
                match msg {
                    ListViewMessage::Items(items) => {
                        // Remove previous items.
                        for child in ui.node(self.panel).children() {
                            ui.send_message(WidgetMessage::remove(
                                *child,
                                MessageDirection::ToWidget,
                            ));
                        }

                        // Generate new items.
                        let item_containers = generate_item_containers(&mut ui.build_ctx(), items);

                        for item_container in item_containers.iter() {
                            ui.send_message(WidgetMessage::link(
                                *item_container,
                                MessageDirection::ToWidget,
                                self.panel,
                            ));
                        }

                        self.item_containers = item_containers;
                        self.items = items.clone();

                        self.fix_selection(ui);
                        self.sync_decorators(ui);
                    }
                    &ListViewMessage::AddItem(item) => {
                        let item_container = generate_item_container(&mut ui.build_ctx(), item);

                        ui.send_message(WidgetMessage::link(
                            item_container,
                            MessageDirection::ToWidget,
                            self.panel,
                        ));

                        self.item_containers.push(item_container);
                        self.items.push(item);
                    }
                    &ListViewMessage::SelectionChanged(selection) => {
                        if self.selected_index != selection {
                            self.selected_index = selection;
                            self.sync_decorators(ui);
                            ui.send_message(message.reverse());
                        }
                    }
                    &ListViewMessage::RemoveItem(item) => {
                        if let Some(item_position) = self.items.iter().position(|i| *i == item) {
                            self.items.remove(item_position);
                            self.item_containers.remove(item_position);

                            let container = ui.node(item).parent();

                            ui.send_message(WidgetMessage::remove(
                                container,
                                MessageDirection::ToWidget,
                            ));

                            self.fix_selection(ui);
                            self.sync_decorators(ui);
                        }
                    }
                    &ListViewMessage::BringItemIntoView(item) => {
                        if self.items.contains(&item) {
                            ui.send_message(ScrollViewerMessage::bring_into_view(
                                self.scroll_viewer,
                                MessageDirection::ToWidget,
                                item,
                            ));
                        }
                    }
                }
            }
        }
    }
}

pub struct ListViewBuilder {
    widget_builder: WidgetBuilder,
    items: Vec<Handle<UiNode>>,
    panel: Option<Handle<UiNode>>,
    scroll_viewer: Option<Handle<UiNode>>,
}

impl ListViewBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            items: Vec::new(),
            panel: None,
            scroll_viewer: None,
        }
    }

    pub fn with_items(mut self, items: Vec<Handle<UiNode>>) -> Self {
        self.items = items;
        self
    }

    pub fn with_items_panel(mut self, panel: Handle<UiNode>) -> Self {
        self.panel = Some(panel);
        self
    }

    pub fn with_scroll_viewer(mut self, sv: Handle<UiNode>) -> Self {
        self.scroll_viewer = Some(sv);
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let item_containers = generate_item_containers(ctx, &self.items);

        let panel = self.panel.unwrap_or_else(|| {
            StackPanelBuilder::new(
                WidgetBuilder::new().with_children(item_containers.iter().cloned()),
            )
            .build(ctx)
        });

        let back = BorderBuilder::new(
            WidgetBuilder::new()
                .with_background(BRUSH_DARK)
                .with_foreground(BRUSH_LIGHT),
        )
        .with_stroke_thickness(Thickness::uniform(1.0))
        .build(ctx);

        let scroll_viewer = self.scroll_viewer.unwrap_or_else(|| {
            ScrollViewerBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(3.0)))
                .build(ctx)
        });
        let scroll_viewer_ref = ctx[scroll_viewer]
            .cast_mut::<ScrollViewer>()
            .expect("ListView must have ScrollViewer");
        scroll_viewer_ref.set_content(panel);
        let content_presenter = scroll_viewer_ref.scroll_panel;
        ctx.link(panel, content_presenter);

        ctx.link(scroll_viewer, back);

        let list_box = ListView {
            widget: self.widget_builder.with_child(back).build(),
            selected_index: None,
            item_containers,
            items: self.items,
            panel,
            scroll_viewer,
        };

        ctx.add_node(UiNode::new(list_box))
    }
}

fn generate_item_container(ctx: &mut BuildContext, item: Handle<UiNode>) -> Handle<UiNode> {
    let item = ListViewItem {
        widget: WidgetBuilder::new().with_child(item).build(),
    };

    ctx.add_node(UiNode::new(item))
}

fn generate_item_containers(
    ctx: &mut BuildContext,
    items: &[Handle<UiNode>],
) -> Vec<Handle<UiNode>> {
    items
        .iter()
        .map(|&item| generate_item_container(ctx, item))
        .collect()
}
