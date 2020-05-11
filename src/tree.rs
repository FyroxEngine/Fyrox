use std::ops::{DerefMut, Deref};
use crate::{
    core::{
        pool::Handle,
        math::vec2::Vec2,
        color::Color,
    },
    grid::{GridBuilder, Row, Column},
    button::ButtonBuilder,
    message::{
        UiMessage,
        UiMessageData,
        ButtonMessage,
        WidgetMessage,
        WidgetProperty,
        TreeMessage,
        TreeRootMessage,
    },
    node::UINode,
    Control,
    UserInterface,
    Thickness,
    NodeHandleMapping,
    widget::{Widget, WidgetBuilder},
    border::BorderBuilder,
    brush::Brush,
    stack_panel::StackPanelBuilder,
};

pub struct Tree<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    expander: Handle<UINode<M, C>>,
    content: Handle<UINode<M, C>>,
    panel: Handle<UINode<M, C>>,
    is_expanded: bool,
    background: Handle<UINode<M, C>>,
    items: Vec<Handle<UINode<M, C>>>,
    is_selected: bool,
    selected_brush: Brush,
    hovered_brush: Brush,
    normal_brush: Brush,
    always_show_expander: bool
}

impl<M: 'static, C: 'static + Control<M, C>> Deref for Tree<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> DerefMut for Tree<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> Clone for Tree<M, C> {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.raw_copy(),
            expander: self.expander,
            content: self.content,
            panel: self.panel,
            is_expanded: self.is_expanded,
            background: self.background,
            items: self.items.to_vec(),
            is_selected: self.is_selected,
            selected_brush: self.selected_brush.clone(),
            hovered_brush: self.hovered_brush.clone(),
            normal_brush: self.normal_brush.clone(),
            always_show_expander: self.always_show_expander
        }
    }
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for Tree<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        UINode::Tree(self.clone())
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        if let Some(&content) = node_map.get(&self.content) {
            self.content = content;
        }
        self.expander = *node_map.get(&self.expander).unwrap();
        self.panel = *node_map.get(&self.panel).unwrap();
        self.background = *node_map.get(&self.background).unwrap();
    }

    fn arrange_override(&self, ui: &UserInterface<M, C>, final_size: Vec2) -> Vec2 {
        let size = self.widget.arrange_override(ui, final_size);

        if !self.always_show_expander {
            let expander_visibility = !self.items.is_empty();
            self.send_message(UiMessage {
                destination: self.expander,
                data: UiMessageData::Widget(WidgetMessage::Property(WidgetProperty::Visibility(expander_visibility))),
                handled: false,
            });
        }

        size
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_routed_message(ui, message);

        match &message.data {
            UiMessageData::Button(msg) => {
                if let ButtonMessage::Click = msg {
                    if message.destination == self.expander {
                        self.set_expanded(!self.is_expanded);
                    }
                }
            }
            UiMessageData::Widget(msg) => {
                match msg {
                    WidgetMessage::MouseDown { .. } => {
                        if !message.handled {
                            let root = ui.find_by_criteria_up(self.parent(), |n| {
                                if let UINode::TreeRoot(_) = n { true } else { false }
                            });
                            if root.is_some() {
                                if let UINode::TreeRoot(root) = ui.node_mut(root) {
                                    root.set_selected(self.handle);
                                }
                                message.handled = true;
                            }
                        }
                    }
                    WidgetMessage::MouseEnter => {
                        if !message.handled {
                            if !self.is_selected {
                                if let UINode::Border(background) = ui.node_mut(self.background) {
                                    background.set_background(self.hovered_brush.clone());
                                }
                            }
                            message.handled = true;
                        }
                    }
                    WidgetMessage::MouseLeave => {
                        if !message.handled {
                            if !self.is_selected {
                                if let UINode::Border(background) = ui.node_mut(self.background) {
                                    background.set_background(self.normal_brush.clone());
                                }
                            }
                            message.handled = true;
                        }
                    }
                    _ => {}
                }
            }
            UiMessageData::Tree(msg) => {
                if message.destination == self.handle {
                    match msg {
                        &TreeMessage::Expand(expand) => {
                            self.is_expanded = expand;
                            ui.node_mut(self.panel).set_visibility(self.is_expanded);
                        }
                        &TreeMessage::AddItem(item) => {
                            ui.link_nodes(item, self.panel);
                            self.items.push(item);
                            dbg!();
                        }
                        &TreeMessage::RemoveItem(item) => {
                            if let Some(pos) = self.items.iter().position(|&i| i == item) {
                                ui.remove_node(item);
                                self.items.remove(pos);
                            }
                        }
                        TreeMessage::SetItems(items) => {
                            for &item in self.items.iter() {
                                ui.remove_node(item);
                            }
                            for &item in items {
                                ui.link_nodes(item, self.panel);
                            }
                            self.items = items.clone();
                        }
                    }
                }
            }
            _ => ()
        }
    }

    fn remove_ref(&mut self, handle: Handle<UINode<M, C>>) {
        if self.expander == handle {
            self.expander = Default::default();
        }
        if self.content == handle {
            self.content = Default::default();
        }
        if self.panel == handle {
            self.panel = Default::default();
        }
        if self.background == handle {
            self.background = Default::default();
        }
    }
}

impl<M, C: 'static + Control<M, C>> Tree<M, C> {
    pub fn content(&self) -> Handle<UINode<M, C>> {
        self.content
    }

    pub fn add_item(&mut self, item: Handle<UINode<M, C>>) {
        self.send_message(UiMessage {
            data: UiMessageData::Tree(TreeMessage::AddItem(item)),
            destination: self.handle,
            handled: false,
        });
    }

    /// Removes specified item from Tree.
    pub fn remove_item(&mut self, item: Handle<UINode<M, C>>) {
        self.send_message(UiMessage {
            data: UiMessageData::Tree(TreeMessage::RemoveItem(item)),
            destination: self.handle,
            handled: false,
        });
    }

    pub fn set_items(&mut self, items: Vec<Handle<UINode<M, C>>>) {
        self.send_message(UiMessage {
            data: UiMessageData::Tree(TreeMessage::SetItems(items)),
            destination: self.handle,
            handled: false,
        });
    }

    pub fn set_expanded(&mut self, expand: bool) {
        if self.is_expanded != expand {
            self.send_message(UiMessage {
                data: UiMessageData::Tree(TreeMessage::Expand(expand)),
                destination: self.handle,
                handled: false,
            });
        }
    }

    pub fn items(&self) -> &[Handle<UINode<M, C>>] {
        &self.items
    }
}

pub struct TreeBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    items: Vec<Handle<UINode<M, C>>>,
    content: Handle<UINode<M, C>>,
    is_expanded: bool,
    selected_brush: Brush,
    hovered_brush: Brush,
    normal_brush: Brush,
    always_show_expander: bool
}

impl<M, C: 'static + Control<M, C>> TreeBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            items: Default::default(),
            content: Default::default(),
            is_expanded: true,
            selected_brush: Brush::Solid(Color::opaque(140, 140, 140)),
            hovered_brush: Brush::Solid(Color::opaque(100, 100, 100)),
            normal_brush: Brush::Solid(Color::TRANSPARENT),
            always_show_expander: false
        }
    }

    pub fn with_items(mut self, items: Vec<Handle<UINode<M, C>>>) -> Self {
        self.items = items;
        self
    }

    pub fn with_content(mut self, content: Handle<UINode<M, C>>) -> Self {
        self.content = content;
        self
    }

    pub fn with_expanded(mut self, expanded: bool) -> Self {
        self.is_expanded = expanded;
        self
    }

    pub fn with_always_show_expander(mut self, state: bool) -> Self {
        self.always_show_expander = state;
        self
    }

    pub fn build(self, ui: &mut UserInterface<M, C>) -> Handle<UINode<M, C>> {
        let expander = ButtonBuilder::new(WidgetBuilder::new()
            .with_width(20.0)
            .on_row(0)
            .on_column(0))
            .with_text("+")
            .build(ui);

        ui.node_mut(self.content)
            .set_row(0)
            .set_column(1);

        let item_background;
        let panel;
        let grid = GridBuilder::new(WidgetBuilder::new()
            .with_child({
                item_background = BorderBuilder::new(WidgetBuilder::new()
                    .with_background(self.normal_brush.clone())
                    .with_child(GridBuilder::new(WidgetBuilder::new()
                        .on_column(0)
                        .on_row(0)
                        .with_margin(Thickness {
                            left: 1.0,
                            top: 1.0,
                            right: 0.0,
                            bottom: 1.0,
                        })
                        .with_child(expander)
                        .with_child(self.content))
                        .add_column(Column::auto())
                        .add_column(Column::stretch())
                        .add_row(Row::strict(20.0))
                        .build(ui)))
                    .build(ui);
                item_background
            })
            .with_child({
                panel = StackPanelBuilder::new(WidgetBuilder::new()
                    .on_row(1)
                    .on_column(0)
                    .with_margin(Thickness::left(15.0))
                    .with_children(&self.items))
                    .build(ui);
                panel
            }))
            .add_column(Column::auto())
            .add_row(Row::strict(24.0))
            .add_row(Row::stretch())
            .build(ui);

        let tree = Tree {
            widget: self.widget_builder
                .with_allow_drag(true)
                .with_allow_drop(true)
                .with_child(grid)
                .build(ui.sender()),
            content: self.content,
            panel,
            is_expanded: self.is_expanded,
            expander,
            background: item_background,
            items: self.items,
            is_selected: false,
            selected_brush: self.selected_brush,
            hovered_brush: self.hovered_brush,
            normal_brush: self.normal_brush,
            always_show_expander: self.always_show_expander
        };

        let handle = ui.add_node(UINode::Tree(tree));

        ui.flush_messages();

        handle
    }
}

pub struct TreeRoot<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    panel: Handle<UINode<M, C>>,
    items: Vec<Handle<UINode<M, C>>>,
    selected: Handle<UINode<M, C>>
}

impl<M: 'static, C: 'static + Control<M, C>> Deref for TreeRoot<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> DerefMut for TreeRoot<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> Clone for TreeRoot<M, C> {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.raw_copy(),
            panel: self.panel,
            items: self.items.clone(),
            selected: self.selected
        }
    }
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for TreeRoot<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        UINode::TreeRoot(self.clone())
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        self.panel = *node_map.get(&self.panel).unwrap();
        if self.selected.is_some() {
            self.selected = *node_map.get(&self.selected).unwrap();
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_routed_message(ui, message);

        match &message.data {
            UiMessageData::TreeRoot(msg) => {
                if message.destination == self.handle {
                    match msg {
                        &TreeRootMessage::AddItem(item) => {
                            ui.link_nodes(item, self.panel);
                            self.items.push(item);
                        }
                        &TreeRootMessage::RemoveItem(item) => {
                            if let Some(pos) = self.items.iter().position(|&i| i == item) {
                                ui.remove_node(item);
                                self.items.remove(pos);
                            }
                        }
                        TreeRootMessage::SetItems(items) => {
                            for &item in self.items.iter() {
                                ui.remove_node(item);
                            }
                            for &item in items {
                                ui.link_nodes(item, self.panel);
                            }
                            self.items = items.to_vec();
                        }
                        &TreeRootMessage::SetSelected(selected) => {
                            if self.selected != selected {
                                let mut stack = self.children().to_vec();
                                while let Some(handle) = stack.pop() {
                                    let node = ui.node_mut(handle);
                                    for &child_handle in node.children() {
                                        stack.push(child_handle);
                                    }
                                    if let UINode::Tree(tree) = node {
                                        let (select, brush) = if handle == selected {
                                            (true, tree.selected_brush.clone())
                                        } else {
                                            (false, tree.normal_brush.clone())
                                        };
                                        tree.is_selected = select;
                                        if select {
                                            self.selected = selected;
                                        }
                                        let background_handle = tree.background;
                                        if let UINode::Border(background) = ui.node_mut(background_handle) {
                                            background.set_background(brush);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn remove_ref(&mut self, handle: Handle<UINode<M, C>>) {
        if self.panel == handle {
            self.panel = Default::default();
        }
        if self.selected == handle {
            self.selected = Default::default();
        }
    }
}

impl<M: 'static, C: 'static + Control<M, C>> TreeRoot<M, C> {
    /// Sets new items to tree root. Item should be handle to instance of Tree.
    /// This method has deferred execution.
    pub fn add_item(&mut self, item: Handle<UINode<M, C>>) {
        self.send_message(UiMessage {
            data: UiMessageData::TreeRoot(TreeRootMessage::AddItem(item)),
            destination: self.handle,
            handled: false,
        });
    }

    /// Removes specified item from TreeRoot.
    pub fn remove_item(&mut self, item: Handle<UINode<M, C>>) {
        self.send_message(UiMessage {
            data: UiMessageData::TreeRoot(TreeRootMessage::RemoveItem(item)),
            destination: self.handle,
            handled: false,
        });
    }

    /// Sets new items to tree root. Item should be handle to instance of Tree.
    /// This method has deferred execution.
    pub fn set_items(&mut self, items: Vec<Handle<UINode<M, C>>>) {
        self.send_message(UiMessage {
            data: UiMessageData::TreeRoot(TreeRootMessage::SetItems(items)),
            destination: self.handle,
            handled: false,
        });
    }

    /// Makes desired node selected. Pass Handle::NONE to deselect all nodes.
    /// This method has deferred execution.
    pub fn set_selected(&mut self, selected: Handle<UINode<M, C>>) {
        if self.selected != selected {
            self.send_message(UiMessage {
                data: UiMessageData::TreeRoot(TreeRootMessage::SetSelected(selected)),
                destination: self.handle,
                handled: false,
            });
        }
    }

    pub fn items(&self) -> &[Handle<UINode<M, C>>] {
        &self.items
    }
}

pub struct TreeRootBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    items: Vec<Handle<UINode<M, C>>>,
}

impl<M, C: 'static + Control<M, C>> TreeRootBuilder<M, C> {
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

    pub fn build(self, ui: &mut UserInterface<M, C>) -> Handle<UINode<M, C>> {
        let panel = StackPanelBuilder::new(WidgetBuilder::new()
            .with_children(&self.items))
            .build(ui);

        let tree = TreeRoot {
            widget: self.widget_builder
                .with_child(panel)
                .build(ui.sender()),
            panel,
            items: self.items,
            selected: Default::default()
        };

        let handle = ui.add_node(UINode::TreeRoot(tree));

        ui.flush_messages();

        handle
    }
}