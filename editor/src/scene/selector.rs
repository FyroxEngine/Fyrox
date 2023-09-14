use crate::utils::make_node_name;
use fyrox::{
    core::{algebra::Vector2, pool::Handle},
    gui::{
        border::BorderBuilder,
        button::{ButtonBuilder, ButtonMessage},
        define_constructor, define_widget_deref,
        draw::DrawingContext,
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, OsEvent, UiMessage},
        scroll_viewer::ScrollViewerBuilder,
        searchbar::{SearchBarBuilder, SearchBarMessage},
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        tree::{Tree, TreeBuilder, TreeRootBuilder, TreeRootMessage},
        widget::{Widget, WidgetBuilder, WidgetMessage},
        window::{Window, WindowBuilder, WindowMessage},
        BuildContext, Control, HorizontalAlignment, NodeHandleMapping, Orientation, Thickness,
        UiNode, UserInterface,
    },
    scene::{graph::Graph, node::Node},
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::mpsc::Sender,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HierarchyNode {
    pub name: String,
    pub handle: Handle<Node>,
    pub children: Vec<HierarchyNode>,
}

impl HierarchyNode {
    pub fn from_scene_node(
        node_handle: Handle<Node>,
        ignored_node: Handle<Node>,
        graph: &Graph,
    ) -> Self {
        let node = &graph[node_handle];

        Self {
            name: node.name_owned(),
            handle: node_handle,
            children: node
                .children()
                .iter()
                .filter_map(|c| {
                    if *c == ignored_node {
                        None
                    } else {
                        Some(HierarchyNode::from_scene_node(*c, ignored_node, graph))
                    }
                })
                .collect(),
        }
    }

    #[allow(dead_code)]
    pub fn find_node(&mut self, node_handle: Handle<Node>) -> Option<&mut HierarchyNode> {
        if self.handle == node_handle {
            return Some(self);
        }

        for child in self.children.iter_mut() {
            if let Some(node) = child.find_node(node_handle) {
                return Some(node);
            }
        }

        None
    }

    fn make_view(&self, ctx: &mut BuildContext) -> Handle<UiNode> {
        TreeBuilder::new(WidgetBuilder::new().with_user_data(Rc::new(TreeData {
            name: self.name.clone(),
            handle: self.handle,
        })))
        .with_items(self.children.iter().map(|c| c.make_view(ctx)).collect())
        .with_content(
            TextBuilder::new(WidgetBuilder::new())
                .with_text(make_node_name(&self.name, self.handle.into()))
                .build(ctx),
        )
        .build(ctx)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeSelectorMessage {
    #[allow(dead_code)] // Might be used in the future.
    Hierarchy(HierarchyNode),
    Selection(Vec<Handle<Node>>),
}

impl NodeSelectorMessage {
    define_constructor!(NodeSelectorMessage:Hierarchy => fn hierarchy(HierarchyNode), layout: false);
    define_constructor!(NodeSelectorMessage:Selection => fn selection(Vec<Handle<Node>>), layout: false);
}

struct TreeData {
    name: String,
    handle: Handle<Node>,
}

#[derive(Clone)]
pub struct NodeSelector {
    widget: Widget,
    tree_root: Handle<UiNode>,
    search_bar: Handle<UiNode>,
    selected: Vec<Handle<Node>>,
}

define_widget_deref!(NodeSelector);

fn apply_filter_recursive(node: Handle<UiNode>, filter: &str, ui: &UserInterface) -> bool {
    let node_ref = ui.node(node);

    let mut is_any_match = false;
    for &child in node_ref.children() {
        is_any_match |= apply_filter_recursive(child, filter, ui)
    }

    if let Some(data) = node_ref
        .query_component::<Tree>()
        .and_then(|n| n.user_data_ref::<TreeData>())
    {
        is_any_match |= data.name.to_lowercase().contains(filter);

        ui.send_message(WidgetMessage::visibility(
            node,
            MessageDirection::ToWidget,
            is_any_match,
        ));
    }

    is_any_match
}

impl Control for NodeSelector {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<NodeSelectorMessage>() {
            if message.destination() == self.handle
                && message.direction() == MessageDirection::ToWidget
            {
                match msg {
                    NodeSelectorMessage::Hierarchy(hierarchy) => {
                        let items = vec![hierarchy.make_view(&mut ui.build_ctx())];
                        ui.send_message(TreeRootMessage::items(
                            self.tree_root,
                            MessageDirection::ToWidget,
                            items,
                        ));
                    }
                    NodeSelectorMessage::Selection(selection) => {
                        if &self.selected != selection {
                            self.selected = selection.clone();

                            let mut stack = vec![self.tree_root];
                            let mut selected_trees = vec![];
                            while let Some(node_handle) = stack.pop() {
                                let node = ui.node(node_handle);

                                if let Some(tree) = node.query_component::<Tree>() {
                                    if self
                                        .selected
                                        .contains(&tree.user_data_ref::<TreeData>().unwrap().handle)
                                    {
                                        selected_trees.push(node_handle);
                                    }
                                }

                                stack.extend_from_slice(node.children());
                            }

                            ui.send_message(TreeRootMessage::select(
                                self.tree_root,
                                MessageDirection::ToWidget,
                                selected_trees,
                            ));

                            ui.send_message(message.reverse());
                        }
                    }
                }
            }
        } else if let Some(SearchBarMessage::Text(filter_text)) = message.data() {
            if message.destination() == self.search_bar
                && message.direction() == MessageDirection::FromWidget
            {
                apply_filter_recursive(self.tree_root, &filter_text.to_lowercase(), ui);
            }
        } else if let Some(TreeRootMessage::Selected(selection)) = message.data() {
            if message.destination() == self.tree_root
                && message.direction() == MessageDirection::FromWidget
            {
                ui.send_message(NodeSelectorMessage::selection(
                    self.handle,
                    MessageDirection::ToWidget,
                    selection
                        .iter()
                        .map(|s| ui.node(*s).user_data_ref::<TreeData>().unwrap().handle)
                        .collect(),
                ));
            }
        }
    }
}

pub struct NodeSelectorBuilder {
    widget_builder: WidgetBuilder,
    hierarchy: Option<HierarchyNode>,
}

impl NodeSelectorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            hierarchy: None,
        }
    }

    pub fn with_hierarchy(mut self, hierarchy: Option<HierarchyNode>) -> Self {
        self.hierarchy = hierarchy;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let items = self
            .hierarchy
            .map(|h| vec![h.make_view(ctx)])
            .unwrap_or_default();

        let tree_root = TreeRootBuilder::new(WidgetBuilder::new())
            .with_items(items)
            .build(ctx);
        let search_bar;
        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child({
                    search_bar = SearchBarBuilder::new(WidgetBuilder::new()).build(ctx);
                    search_bar
                })
                .with_child(
                    BorderBuilder::new(
                        WidgetBuilder::new()
                            .with_background(fyrox::gui::BRUSH_DARK)
                            .on_row(1)
                            .on_column(0)
                            .with_child(
                                ScrollViewerBuilder::new(
                                    WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                                )
                                .with_content(tree_root)
                                .build(ctx),
                            ),
                    )
                    .build(ctx),
                ),
        )
        .add_row(Row::strict(22.0))
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        let selector = NodeSelector {
            widget: self.widget_builder.with_child(content).build(),
            tree_root,
            search_bar,
            selected: Default::default(),
        };

        ctx.add_node(UiNode::new(selector))
    }
}

#[derive(Clone)]
pub struct NodeSelectorWindow {
    window: Window,
    selector: Handle<UiNode>,
    ok: Handle<UiNode>,
    cancel: Handle<UiNode>,
}

impl Deref for NodeSelectorWindow {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.window.widget
    }
}

impl DerefMut for NodeSelectorWindow {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.window.widget
    }
}

impl Control for NodeSelectorWindow {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        self.window.query_component(type_id).or_else(|| {
            if type_id == TypeId::of::<Self>() {
                Some(self)
            } else {
                None
            }
        })
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        self.window.resolve(node_map);
    }

    fn on_remove(&self, sender: &Sender<UiMessage>) {
        self.window.on_remove(sender);
    }

    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        self.window.measure_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        self.window.arrange_override(ui, final_size)
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        self.window.draw(drawing_context)
    }

    fn update(&mut self, dt: f32, sender: &Sender<UiMessage>) {
        self.window.update(dt, sender)
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.window.handle_routed_message(ui, message);

        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.ok {
                ui.send_message(NodeSelectorMessage::selection(
                    self.handle,
                    MessageDirection::FromWidget,
                    ui.node(self.selector)
                        .query_component::<NodeSelector>()
                        .unwrap()
                        .selected
                        .clone(),
                ));

                ui.send_message(WindowMessage::close(
                    self.handle,
                    MessageDirection::ToWidget,
                ));
            } else if message.destination() == self.cancel {
                ui.send_message(WindowMessage::close(
                    self.handle,
                    MessageDirection::ToWidget,
                ));
            }
        } else if let Some(msg) = message.data::<NodeSelectorMessage>() {
            if message.destination() == self.handle
                && message.direction() == MessageDirection::ToWidget
            {
                // Dispatch to inner selector.
                let mut msg = message.clone();
                msg.destination = self.selector;
                ui.send_message(msg);
            } else if message.destination() == self.selector
                && message.direction() == MessageDirection::FromWidget
            {
                // Enable "ok" button if selection is valid.
                if let NodeSelectorMessage::Selection(selection) = msg {
                    ui.send_message(WidgetMessage::enabled(
                        self.ok,
                        MessageDirection::ToWidget,
                        !selection.is_empty(),
                    ));
                }
            }
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        self.window.preview_message(ui, message)
    }

    fn handle_os_event(
        &mut self,
        self_handle: Handle<UiNode>,
        ui: &mut UserInterface,
        event: &OsEvent,
    ) {
        self.window.handle_os_event(self_handle, ui, event);
    }
}

pub struct NodeSelectorWindowBuilder {
    window_builder: WindowBuilder,
    hierarchy: Option<HierarchyNode>,
}

impl NodeSelectorWindowBuilder {
    pub fn new(window_builder: WindowBuilder) -> Self {
        Self {
            window_builder,
            hierarchy: None,
        }
    }

    pub fn with_hierarchy(mut self, hierarchy: HierarchyNode) -> Self {
        self.hierarchy = Some(hierarchy);
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let ok;
        let cancel;
        let selector;
        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child({
                    selector = NodeSelectorBuilder::new(WidgetBuilder::new())
                        .with_hierarchy(self.hierarchy)
                        .build(ctx);
                    selector
                })
                .with_child(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(2.0))
                            .on_row(1)
                            .on_column(0)
                            .with_horizontal_alignment(HorizontalAlignment::Right)
                            .with_child({
                                ok = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_enabled(false)
                                        .with_width(100.0)
                                        .with_margin(Thickness::uniform(1.0)),
                                )
                                .with_text("OK")
                                .build(ctx);
                                ok
                            })
                            .with_child({
                                cancel = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_width(100.0)
                                        .with_margin(Thickness::uniform(1.0)),
                                )
                                .with_text("Cancel")
                                .build(ctx);
                                cancel
                            }),
                    )
                    .with_orientation(Orientation::Horizontal)
                    .build(ctx),
                ),
        )
        .add_column(Column::stretch())
        .add_row(Row::stretch())
        .add_row(Row::strict(27.0))
        .build(ctx);

        let window = NodeSelectorWindow {
            window: self
                .window_builder
                .with_content(content)
                .open(false)
                .build_window(ctx),
            ok,
            cancel,
            selector,
        };

        ctx.add_node(UiNode::new(window))
    }
}
