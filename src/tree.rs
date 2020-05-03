use std::ops::{DerefMut, Deref};
use crate::{
    core::{
        pool::Handle,
        math::vec2::Vec2
    },
    grid::{GridBuilder, Row, Column},
    button::ButtonBuilder,
    message::{
        UiMessage,
        UiMessageData,
        ItemsControlMessage,
        ButtonMessage,
        WidgetMessage,
        WidgetProperty,
        TreeMessage
    },
    node::UINode,
    Control,
    UserInterface,
    Thickness,
    NodeHandleMapping,
    widget::{Widget, WidgetBuilder},
    items_control::ItemsControlBuilder,
};

pub struct Tree<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    expander: Handle<UINode<M, C>>,
    content: Handle<UINode<M, C>>,
    items_control: Handle<UINode<M, C>>,
    is_expanded: bool
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
            items_control: self.items_control,
            is_expanded: self.is_expanded
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
        self.items_control = *node_map.get(&self.items_control).unwrap();
    }

    fn arrange_override(&self, ui: &UserInterface<M, C>, final_size: Vec2) -> Vec2 {
        let size = self.widget.arrange_override(ui, final_size);

        if let UINode::ItemsControl(items_control) = ui.node(self.items_control) {
            let expander_visibility = !items_control.items().is_empty();
            self.post_message(UiMessage::targeted(self.expander, UiMessageData::Widget(
                WidgetMessage::Property(WidgetProperty::Visibility(expander_visibility)))));
        }

        size
    }

    fn handle_message(&mut self, self_handle: Handle<UINode<M, C>>, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_message(self_handle, ui, message);

        match &message.data {
            // Re-cast items control messages sent to this tree to inner items control.
            UiMessageData::ItemsControl(msg) => {
                if message.source == self_handle || message.target == self_handle {
                    match msg {
                        &ItemsControlMessage::SelectionChanged(selection) => {
                            ui.post_message(UiMessage::targeted(
                                self.items_control,
                                UiMessageData::ItemsControl(
                                    ItemsControlMessage::SelectionChanged(selection))));
                        }
                        ItemsControlMessage::Items(items) => {
                            ui.post_message(UiMessage::targeted(
                                self.items_control,
                                UiMessageData::ItemsControl(
                                    ItemsControlMessage::Items(items.clone()))));
                        }
                        &ItemsControlMessage::AddItem(item) => {
                            ui.post_message(UiMessage::targeted(
                                self.items_control,
                                UiMessageData::ItemsControl(
                                    ItemsControlMessage::AddItem(item))));
                        }
                    }
                }
            }
            UiMessageData::Button(msg) => {
                match msg {
                    ButtonMessage::Click => {
                        if message.source == self.expander {
                            self.set_expanded(!self.is_expanded);
                        }
                    },
                    _ => ()
                }
            }
            UiMessageData::Tree(msg) => {
                match msg {
                    &TreeMessage::Expand(expand) => {
                        if message.target == self_handle || message.source == self_handle {
                            self.is_expanded = expand;
                            ui.node_mut(self.items_control).set_visibility(self.is_expanded);
                        }
                    },
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
        if self.items_control == handle {
            self.items_control = Default::default();
        }
    }
}

impl<M, C: 'static + Control<M, C>> Tree<M, C> {
    pub fn content(&self) -> Handle<UINode<M, C>> {
        self.content
    }

    pub fn add_item(&mut self, item: Handle<UINode<M, C>>) {
        self.post_message(UiMessage::new(
            UiMessageData::ItemsControl(ItemsControlMessage::AddItem(item))));
    }

    pub fn set_expanded(&mut self, expand: bool) {
        self.post_message(UiMessage::new(UiMessageData::Tree(TreeMessage::Expand(expand))));
    }
}

pub struct TreeBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    items: Vec<Handle<UINode<M, C>>>,
    content: Handle<UINode<M, C>>,
    is_expanded: bool
}

impl<M, C: 'static + Control<M, C>> TreeBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            items: Default::default(),
            content: Default::default(),
            is_expanded: true
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

        let items_control;
        let grid = GridBuilder::new(WidgetBuilder::new()
            .with_child(GridBuilder::new(WidgetBuilder::new()
                .on_column(0)
                .on_row(0)
                .with_child(expander)
                .with_child(self.content))
                .add_column(Column::auto())
                .add_column(Column::stretch())
                .add_row(Row::strict(20.0))
                .build(ui))
            .with_child({
                items_control = ItemsControlBuilder::new(WidgetBuilder::new()
                    .with_margin(Thickness::left(15.0))
                    .on_column(0)
                    .on_row(1))
                    .with_items(self.items)
                    .build(ui);
                items_control
            }))
            .add_column(Column::auto())
            .add_row(Row::strict(20.0))
            .add_row(Row::stretch())
            .build(ui);

        let tree = Tree {
            widget: self.widget_builder
                .with_child(grid)
                .build(),
            content: self.content,
            items_control,
            is_expanded: self.is_expanded,
            expander,
        };

        let handle = ui.add_node(UINode::Tree(tree));

        ui.flush_messages();

        handle
    }
}