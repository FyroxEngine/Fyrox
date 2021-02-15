use crate::core::algebra::Vector2;
use crate::{
    border::BorderBuilder,
    brush::Brush,
    button::ButtonBuilder,
    core::{color::Color, pool::Handle},
    decorator::DecoratorBuilder,
    grid::{Column, GridBuilder, Row},
    message::{
        ButtonMessage, DecoratorMessage, MessageData, MessageDirection, TextMessage, TreeMessage,
        TreeRootMessage, UiMessage, UiMessageData, WidgetMessage,
    },
    node::UINode,
    stack_panel::StackPanelBuilder,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, NodeHandleMapping, Thickness, UserInterface, BRUSH_DARK, BRUSH_DARKEST,
    BRUSH_LIGHT,
};
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone)]
pub struct Tree<M: MessageData, C: Control<M, C>> {
    widget: Widget<M, C>,
    expander: Handle<UINode<M, C>>,
    content: Handle<UINode<M, C>>,
    panel: Handle<UINode<M, C>>,
    is_expanded: bool,
    background: Handle<UINode<M, C>>,
    items: Vec<Handle<UINode<M, C>>>,
    is_selected: bool,
    always_show_expander: bool,
}

crate::define_widget_deref!(Tree<M, C>);

impl<M: MessageData, C: Control<M, C>> Control<M, C> for Tree<M, C> {
    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        node_map.resolve(&mut self.content);
        node_map.resolve(&mut self.expander);
        node_map.resolve(&mut self.panel);
        node_map.resolve(&mut self.background);
    }

    fn arrange_override(&self, ui: &UserInterface<M, C>, final_size: Vector2<f32>) -> Vector2<f32> {
        let size = self.widget.arrange_override(ui, final_size);

        if !self.always_show_expander {
            let expander_visibility = !self.items.is_empty();
            ui.send_message(WidgetMessage::visibility(
                self.expander,
                MessageDirection::ToWidget,
                expander_visibility,
            ));
        }

        size
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);

        match &message.data() {
            UiMessageData::Button(ButtonMessage::Click) => {
                if message.destination() == self.expander {
                    ui.send_message(TreeMessage::expand(
                        self.handle(),
                        MessageDirection::ToWidget,
                        !self.is_expanded,
                    ));
                }
            }
            UiMessageData::Widget(WidgetMessage::MouseDown { .. }) => {
                if !message.handled() {
                    let root =
                        ui.find_by_criteria_up(self.parent(), |n| matches!(n, UINode::TreeRoot(_)));
                    if root.is_some() {
                        if let UINode::TreeRoot(tree_root) = ui.node(root) {
                            let selection = if ui.keyboard_modifiers().control {
                                let mut selection = tree_root.selected.clone();
                                if let Some(existing) =
                                    selection.iter().position(|&h| h == self.handle)
                                {
                                    selection.remove(existing);
                                } else {
                                    selection.push(self.handle);
                                }
                                selection
                            } else {
                                vec![self.handle()]
                            };
                            ui.send_message(TreeRootMessage::select(
                                root,
                                MessageDirection::ToWidget,
                                selection,
                            ));
                            message.set_handled(true);
                        } else {
                            unreachable!();
                        }
                    }
                }
            }
            UiMessageData::Tree(msg) => {
                if message.destination() == self.handle() {
                    match msg {
                        &TreeMessage::Expand(expand) => {
                            self.is_expanded = expand;
                            ui.send_message(WidgetMessage::visibility(
                                self.panel,
                                MessageDirection::ToWidget,
                                self.is_expanded,
                            ));
                            if let UINode::Button(expander) = ui.node(self.expander) {
                                let content = expander.content();
                                let text = if expand { "-" } else { "+" };
                                ui.send_message(TextMessage::text(
                                    content,
                                    MessageDirection::ToWidget,
                                    text.to_owned(),
                                ));
                            }
                        }
                        &TreeMessage::AddItem(item) => {
                            ui.send_message(WidgetMessage::link(
                                item,
                                MessageDirection::ToWidget,
                                self.panel,
                            ));

                            self.items.push(item);
                        }
                        &TreeMessage::RemoveItem(item) => {
                            if let Some(pos) = self.items.iter().position(|&i| i == item) {
                                ui.send_message(WidgetMessage::remove(
                                    item,
                                    MessageDirection::ToWidget,
                                ));
                                self.items.remove(pos);
                            }
                        }
                        TreeMessage::SetItems(items) => {
                            for &item in self.items.iter() {
                                ui.send_message(WidgetMessage::remove(
                                    item,
                                    MessageDirection::ToWidget,
                                ));
                            }
                            for &item in items {
                                ui.send_message(WidgetMessage::link(
                                    item,
                                    MessageDirection::ToWidget,
                                    self.panel,
                                ));
                            }
                            self.items = items.clone();
                        }
                        &TreeMessage::Select(state) => {
                            if self.is_selected != state.0 {
                                self.is_selected = state.0;
                                ui.send_message(DecoratorMessage::select(
                                    self.background,
                                    MessageDirection::ToWidget,
                                    self.is_selected,
                                ));
                            }
                        }
                    }
                }
            }
            _ => (),
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

impl<M: MessageData, C: Control<M, C>> Tree<M, C> {
    pub fn content(&self) -> Handle<UINode<M, C>> {
        self.content
    }

    pub fn back(&self) -> Handle<UINode<M, C>> {
        self.background
    }

    pub fn items(&self) -> &[Handle<UINode<M, C>>] {
        &self.items
    }

    /// Adds new item to given tree. This method is meant to be used only on widget build stage,
    /// any runtime actions should be done via messages.
    pub fn add_item(
        tree: Handle<UINode<M, C>>,
        item: Handle<UINode<M, C>>,
        ctx: &mut BuildContext<M, C>,
    ) {
        if let UINode::Tree(tree) = &mut ctx[tree] {
            tree.items.push(item);
            let panel = tree.panel;
            ctx.link(item, panel);
        }
    }
}

pub struct TreeBuilder<M: MessageData, C: Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    items: Vec<Handle<UINode<M, C>>>,
    content: Handle<UINode<M, C>>,
    is_expanded: bool,
    always_show_expander: bool,
    back: Option<Handle<UINode<M, C>>>,
}

impl<M: MessageData, C: Control<M, C>> TreeBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            items: Default::default(),
            content: Default::default(),
            is_expanded: true,
            always_show_expander: false,
            back: None,
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

    pub fn with_back(mut self, back: Handle<UINode<M, C>>) -> Self {
        self.back = Some(back);
        self
    }

    pub fn build_tree(self, ctx: &mut BuildContext<M, C>) -> Tree<M, C> {
        let expander = ButtonBuilder::new(
            WidgetBuilder::new()
                .with_width(20.0)
                .with_visibility(self.always_show_expander || !self.items.is_empty())
                .on_row(0)
                .on_column(0),
        )
        .with_text(if self.is_expanded { "-" } else { "+" })
        .build(ctx);

        if self.content.is_some() {
            ctx[self.content].set_row(0).set_column(1);
        };

        let internals = GridBuilder::new(
            WidgetBuilder::new()
                .on_column(0)
                .on_row(0)
                .with_margin(Thickness {
                    left: 1.0,
                    top: 1.0,
                    right: 0.0,
                    bottom: 1.0,
                })
                .with_child(expander)
                .with_child(self.content),
        )
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .add_row(Row::strict(20.0))
        .build(ctx);

        let item_background = self.back.unwrap_or_else(|| {
            DecoratorBuilder::new(BorderBuilder::new(
                WidgetBuilder::new()
                    .with_foreground(BRUSH_LIGHT)
                    .with_background(Brush::Solid(Color::TRANSPARENT)),
            ))
            .with_selected_brush(BRUSH_DARKEST)
            .with_hover_brush(BRUSH_DARK)
            .with_normal_brush(Brush::Solid(Color::TRANSPARENT))
            .with_pressed_brush(Brush::Solid(Color::TRANSPARENT))
            .with_pressable(false)
            .build(ctx)
        });

        ctx.link(internals, item_background);

        let panel;
        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(item_background)
                .with_child({
                    panel = StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .on_row(1)
                            .on_column(0)
                            .with_margin(Thickness::left(15.0))
                            .with_children(self.items.iter()),
                    )
                    .build(ctx);
                    panel
                }),
        )
        .add_column(Column::auto())
        .add_row(Row::strict(24.0))
        .add_row(Row::stretch())
        .build(ctx);

        Tree {
            widget: self
                .widget_builder
                .with_allow_drag(true)
                .with_allow_drop(true)
                .with_child(grid)
                .build(),
            content: self.content,
            panel,
            is_expanded: self.is_expanded,
            expander,
            background: item_background,
            items: self.items,
            is_selected: false,
            always_show_expander: self.always_show_expander,
        }
    }

    pub fn build(self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        let tree = self.build_tree(ctx);
        ctx.add_node(UINode::Tree(tree))
    }
}

#[derive(Debug, Clone)]
pub struct TreeRoot<M: MessageData, C: Control<M, C>> {
    widget: Widget<M, C>,
    panel: Handle<UINode<M, C>>,
    items: Vec<Handle<UINode<M, C>>>,
    selected: Vec<Handle<UINode<M, C>>>,
}

crate::define_widget_deref!(TreeRoot<M, C>);

impl<M: MessageData, C: Control<M, C>> Control<M, C> for TreeRoot<M, C> {
    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        node_map.resolve(&mut self.panel);
        node_map.resolve_slice(&mut self.selected);
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);

        if let UiMessageData::TreeRoot(msg) = &message.data() {
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::ToWidget
            {
                match msg {
                    &TreeRootMessage::AddItem(item) => {
                        ui.send_message(WidgetMessage::link(
                            item,
                            MessageDirection::ToWidget,
                            self.panel,
                        ));

                        self.items.push(item);
                    }
                    &TreeRootMessage::RemoveItem(item) => {
                        if let Some(pos) = self.items.iter().position(|&i| i == item) {
                            ui.send_message(WidgetMessage::remove(
                                item,
                                MessageDirection::ToWidget,
                            ));
                            self.items.remove(pos);
                        }
                    }
                    TreeRootMessage::Items(items) => {
                        for &item in self.items.iter() {
                            ui.send_message(WidgetMessage::remove(
                                item,
                                MessageDirection::ToWidget,
                            ));
                        }
                        for &item in items {
                            ui.send_message(WidgetMessage::link(
                                item,
                                MessageDirection::ToWidget,
                                self.panel,
                            ));
                        }
                        self.items = items.to_vec();
                    }
                    TreeRootMessage::Selected(selected) => {
                        if &self.selected != selected {
                            let mut stack = self.children().to_vec();
                            while let Some(handle) = stack.pop() {
                                let node = ui.node(handle);
                                stack.extend_from_slice(node.children());
                                if selected.contains(&handle) {
                                    ui.send_message(TreeMessage::select(
                                        handle,
                                        MessageDirection::ToWidget,
                                        true,
                                    ));
                                } else {
                                    ui.send_message(TreeMessage::select(
                                        handle,
                                        MessageDirection::ToWidget,
                                        false,
                                    ));
                                }
                            }
                            self.selected = selected.clone();
                            ui.send_message(message.reverse());
                        }
                    }
                }
            }
        }
    }

    fn remove_ref(&mut self, handle: Handle<UINode<M, C>>) {
        if self.panel == handle {
            self.panel = Default::default();
        }
        if let Some(position) = self.selected.iter().position(|&s| s == handle) {
            self.selected.remove(position);
        }
    }
}

impl<M: MessageData, C: Control<M, C>> TreeRoot<M, C> {
    pub fn items(&self) -> &[Handle<UINode<M, C>>] {
        &self.items
    }
}

pub struct TreeRootBuilder<M: MessageData, C: Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    items: Vec<Handle<UINode<M, C>>>,
}

impl<M: MessageData, C: Control<M, C>> TreeRootBuilder<M, C> {
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

    pub fn build(self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        let panel = StackPanelBuilder::new(WidgetBuilder::new().with_children(self.items.iter()))
            .build(ctx);

        let tree = TreeRoot {
            widget: self.widget_builder.with_child(panel).build(),
            panel,
            items: self.items,
            selected: Default::default(),
        };

        ctx.add_node(UINode::TreeRoot(tree))
    }
}
