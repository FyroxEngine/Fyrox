// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
use fyrox::core::pool::NodeVariant;
use crate::{
    fyrox::{
        core::{
            algebra::Vector2, parking_lot::Mutex, pool::ErasedHandle, pool::Handle,
            reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*,
        },
        fxhash::FxHashSet,
        graph::{BaseSceneGraph, SceneGraph, SceneGraphNode},
        gui::{
            border::BorderBuilder,
            button::{ButtonBuilder, ButtonMessage},
            define_constructor, define_widget_deref,
            draw::DrawingContext,
            grid::{Column, GridBuilder, Row},
            message::{KeyCode, MessageDirection, OsEvent, UiMessage},
            scroll_viewer::ScrollViewerBuilder,
            scroll_viewer::ScrollViewerMessage,
            searchbar::{SearchBarBuilder, SearchBarMessage},
            stack_panel::StackPanelBuilder,
            style::{resource::StyleResourceExt, Style},
            text::TextBuilder,
            tree::{Tree, TreeBuilder, TreeRootBuilder, TreeRootMessage},
            widget::{Widget, WidgetBuilder, WidgetMessage},
            window::{Window, WindowBuilder, WindowMessage},
            BuildContext, Control, HorizontalAlignment, Orientation, Thickness, UiNode,
            UserInterface,
        },
    },
    utils::make_node_name,
};
use fyrox::gui::formatted_text::WrapMode;
use std::hash::{Hash, Hasher};
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::{mpsc::Sender, Arc},
};

#[derive(Eq, Clone, Debug, PartialEq)]
pub struct HierarchyNode {
    pub name: String,
    pub inner_type_name: String,
    pub handle: ErasedHandle,
    pub inner_type_id: TypeId,
    pub derived_type_ids: Vec<TypeId>,
    pub children: Vec<HierarchyNode>,
}

impl HierarchyNode {
    pub fn from_scene_node<G, N>(node_handle: Handle<N>, ignored_node: Handle<N>, graph: &G) -> Self
    where
        G: SceneGraph<Node = N>,
        N: SceneGraphNode<SceneGraph = G>,
    {
        let node = graph.node(node_handle);

        Self {
            name: node.name().to_string(),
            inner_type_name: graph
                .actual_type_name(node_handle)
                .unwrap_or_default()
                .to_string(),
            handle: node_handle.into(),
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
            inner_type_id: graph.actual_type_id(node_handle).unwrap(),
            derived_type_ids: graph.derived_type_ids(node_handle).unwrap(),
        }
    }

    #[allow(dead_code)]
    pub fn find_node(&mut self, node_handle: ErasedHandle) -> Option<&mut HierarchyNode> {
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

    fn make_view(
        &self,
        allowed_types: &FxHashSet<AllowedType>,
        ctx: &mut BuildContext,
    ) -> Handle<UiNode> {
        let brush = if allowed_types.contains(&AllowedType::unnamed(self.inner_type_id))
            || self
                .derived_type_ids
                .iter()
                .any(|derived| allowed_types.contains(&AllowedType::unnamed(*derived)))
        {
            ctx.style.property(Style::BRUSH_TEXT)
        } else {
            ctx.style.property(Style::BRUSH_LIGHT)
        };

        TreeBuilder::new(
            WidgetBuilder::new().with_user_data(Arc::new(Mutex::new(TreeData {
                name: self.name.clone(),
                handle: self.handle,
                inner_type_id: self.inner_type_id,
                derived_type_ids: self.derived_type_ids.clone(),
            }))),
        )
        .with_items(
            self.children
                .iter()
                .map(|c| c.make_view(allowed_types, ctx))
                .collect(),
        )
        .with_content(
            TextBuilder::new(WidgetBuilder::new().with_foreground(brush))
                .with_text(make_node_name(&self.name, self.handle) + " - " + &self.inner_type_name)
                .build(ctx),
        )
        .build(ctx)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Visit, Reflect)]
pub struct SelectedHandle {
    #[visit(skip)]
    #[reflect(hidden)]
    pub inner_type_id: TypeId,
    #[visit(skip)]
    #[reflect(hidden)]
    pub derived_type_ids: Vec<TypeId>,
    pub handle: ErasedHandle,
}

impl<T: Reflect> From<Handle<T>> for SelectedHandle {
    fn from(value: Handle<T>) -> Self {
        Self {
            inner_type_id: TypeId::of::<T>(),
            derived_type_ids: T::derived_types().to_vec(),
            handle: value.into(),
        }
    }
}

impl Default for SelectedHandle {
    fn default() -> Self {
        Self {
            inner_type_id: ().type_id(),
            derived_type_ids: Default::default(),
            handle: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeSelectorMessage {
    #[allow(dead_code)] // Might be used in the future.
    Hierarchy(HierarchyNode),
    Selection(Vec<SelectedHandle>),
    ChooseFocus,
}

impl NodeSelectorMessage {
    define_constructor!(NodeSelectorMessage:Hierarchy => fn hierarchy(HierarchyNode), layout: false);
    define_constructor!(NodeSelectorMessage:Selection => fn selection(Vec<SelectedHandle>), layout: false);
    define_constructor!(NodeSelectorMessage:ChooseFocus => fn choose_focus(), layout: false);
}

#[derive(Clone)]
struct TreeData {
    name: String,
    handle: ErasedHandle,
    inner_type_id: TypeId,
    pub derived_type_ids: Vec<TypeId>,
}

#[derive(Debug, Clone, Visit, Reflect, TypeUuidProvider, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
#[type_uuid(id = "1d718f90-323c-492d-b057-98d47495900a")]
pub struct NodeSelector {
    widget: Widget,
    tree_root: Handle<UiNode>,
    search_bar: Handle<UiNode>,
    selected: Vec<SelectedHandle>,
    scroll_viewer: Handle<UiNode>,
    #[visit(skip)]
    #[reflect(hidden)]
    allowed_types: FxHashSet<AllowedType>,
}

impl NodeVariant<UiNode> for NodeSelector {}

define_widget_deref!(NodeSelector);

fn apply_filter_recursive(node: Handle<UiNode>, filter: &str, ui: &UserInterface) -> bool {
    let node_ref = ui.node(node);

    let mut is_any_match = false;
    for &child in node_ref.children() {
        is_any_match |= apply_filter_recursive(child, filter, ui)
    }

    if let Some(data) = node_ref
        .query_component::<Tree>()
        .and_then(|n| n.user_data_cloned::<TreeData>())
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
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<NodeSelectorMessage>() {
            if message.destination() == self.handle
                && message.direction() == MessageDirection::ToWidget
            {
                match msg {
                    NodeSelectorMessage::Hierarchy(hierarchy) => {
                        let items =
                            vec![hierarchy.make_view(&self.allowed_types, &mut ui.build_ctx())];
                        ui.send_message(TreeRootMessage::items(
                            self.tree_root,
                            MessageDirection::ToWidget,
                            items,
                        ));
                    }
                    NodeSelectorMessage::Selection(selection) => {
                        if &self.selected != selection {
                            self.selected.clone_from(selection);

                            self.sync_selection(ui);

                            ui.send_message(message.reverse());
                        }
                    }
                    NodeSelectorMessage::ChooseFocus => {
                        ui.send_message(WidgetMessage::focus(
                            self.search_bar,
                            MessageDirection::ToWidget,
                        ));
                        self.sync_selection(ui);
                    }
                }
            }
        } else if let Some(SearchBarMessage::Text(filter_text)) = message.data() {
            if message.destination() == self.search_bar
                && message.direction() == MessageDirection::FromWidget
            {
                apply_filter_recursive(self.tree_root, &filter_text.to_lowercase(), ui);

                // Bring first item of current selection in the view when clearing the filter.
                if filter_text.is_empty() {
                    let selected_trees = self.find_selected_tree_items(ui);

                    if let Some(first) = selected_trees.first() {
                        ui.send_message(ScrollViewerMessage::bring_into_view(
                            self.scroll_viewer,
                            MessageDirection::ToWidget,
                            *first,
                        ));
                    }
                }
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
                        .map(|s| {
                            let tree_data = ui.node(*s).user_data_cloned::<TreeData>().unwrap();

                            SelectedHandle {
                                handle: tree_data.handle,
                                inner_type_id: tree_data.inner_type_id,
                                derived_type_ids: tree_data.derived_type_ids,
                            }
                        })
                        .collect(),
                ));
            }
        } else if let Some(TreeRootMessage::ItemsChanged) = message.data() {
            if message.destination == self.tree_root
                && message.direction() == MessageDirection::FromWidget
            {
                self.sync_selection(ui);
            }
        }
    }
}

impl NodeSelector {
    fn find_selected_tree_items(&self, ui: &UserInterface) -> Vec<Handle<UiNode>> {
        let mut stack = vec![self.tree_root];
        let mut selected_trees = Vec::new();

        while let Some(node_handle) = stack.pop() {
            let node = ui.node(node_handle);

            if let Some(tree) = node.query_component::<Tree>() {
                if self.selected.iter().any(|selected| {
                    let tree_data = tree.user_data_cloned::<TreeData>().unwrap();
                    tree_data.handle == selected.handle
                        && tree_data.inner_type_id == selected.inner_type_id
                }) {
                    selected_trees.push(node_handle);
                }
            }

            stack.extend_from_slice(node.children());
        }

        selected_trees
    }

    fn sync_selection(&self, ui: &UserInterface) {
        let selected_trees = self.find_selected_tree_items(ui);

        if let Some(first) = selected_trees.first() {
            ui.send_message(ScrollViewerMessage::bring_into_view(
                self.scroll_viewer,
                MessageDirection::ToWidget,
                *first,
            ))
        }

        ui.send_message(TreeRootMessage::select(
            self.tree_root,
            MessageDirection::ToWidget,
            selected_trees,
        ));
    }
}

pub struct NodeSelectorBuilder {
    widget_builder: WidgetBuilder,
    hierarchy: Option<HierarchyNode>,
    allowed_types: FxHashSet<AllowedType>,
}

impl NodeSelectorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            hierarchy: None,
            allowed_types: Default::default(),
        }
    }

    pub fn with_hierarchy(mut self, hierarchy: Option<HierarchyNode>) -> Self {
        self.hierarchy = hierarchy;
        self
    }

    pub fn with_allowed_types(mut self, allowed_types: FxHashSet<AllowedType>) -> Self {
        self.allowed_types = allowed_types;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let items = self
            .hierarchy
            .map(|h| vec![h.make_view(&self.allowed_types, ctx)])
            .unwrap_or_default();

        let tree_root = TreeRootBuilder::new(WidgetBuilder::new().with_tab_index(Some(1)))
            .with_items(items)
            .build(ctx);
        let search_bar;
        let scroll_viewer;
        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child({
                    search_bar =
                        SearchBarBuilder::new(WidgetBuilder::new().with_tab_index(Some(0)))
                            .build(ctx);
                    search_bar
                })
                .with_child(
                    BorderBuilder::new(
                        WidgetBuilder::new()
                            .with_background(ctx.style.property(Style::BRUSH_DARK))
                            .on_row(1)
                            .on_column(0)
                            .with_child({
                                scroll_viewer = ScrollViewerBuilder::new(
                                    WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                                )
                                .with_content(tree_root)
                                .build(ctx);
                                scroll_viewer
                            }),
                    )
                    .build(ctx),
                ),
        )
        .add_row(Row::strict(22.0))
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        let selector = NodeSelector {
            widget: self.widget_builder.with_child(content).build(ctx),
            tree_root,
            search_bar,
            selected: Default::default(),
            scroll_viewer,
            allowed_types: self.allowed_types,
        };

        ctx.add_node(UiNode::new(selector))
    }
}

#[derive(Debug, Clone, Visit, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "5bb00f15-d6ec-4f0e-af7e-9472b0e290b4")]
#[reflect(derived_type = "UiNode")]
pub struct NodeSelectorWindow {
    #[component(include)]
    window: Window,
    selector: Handle<UiNode>,
    ok: Handle<UiNode>,
    cancel: Handle<UiNode>,
    #[visit(skip)]
    #[reflect(hidden)]
    allowed_types: FxHashSet<AllowedType>,
}

impl NodeVariant<UiNode> for NodeSelectorWindow {}

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

impl NodeSelectorWindow {
    fn confirm(&self, ui: &UserInterface) {
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
    }
}

impl Control for NodeSelectorWindow {
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

    fn update(&mut self, dt: f32, ui: &mut UserInterface) {
        self.window.update(dt, ui)
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.window.handle_routed_message(ui, message);

        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.ok {
                self.confirm(ui);
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
                        !selection.is_empty()
                            && selection.iter().all(|h| {
                                self.allowed_types
                                    .contains(&AllowedType::unnamed(h.inner_type_id))
                                    || h.derived_type_ids.iter().any(|derived| {
                                        self.allowed_types.contains(&AllowedType::unnamed(*derived))
                                    })
                            }),
                    ));
                }
            }
        } else if let Some(WindowMessage::Open { .. })
        | Some(WindowMessage::OpenAt { .. })
        | Some(WindowMessage::OpenModal { .. })
        | Some(WindowMessage::OpenAndAlign { .. }) = message.data()
        {
            ui.send_message(NodeSelectorMessage::choose_focus(
                self.selector,
                MessageDirection::ToWidget,
            ));
        } else if let Some(WidgetMessage::KeyDown(KeyCode::Enter | KeyCode::NumpadEnter)) =
            message.data()
        {
            if !message.handled() {
                self.confirm(ui);
                message.set_handled(true);
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

#[derive(Clone, Eq, Debug)]
pub struct AllowedType {
    pub id: TypeId,
    pub name: String,
}

impl AllowedType {
    pub fn unnamed(id: TypeId) -> Self {
        Self {
            id,
            name: Default::default(),
        }
    }
}

impl Hash for AllowedType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

impl PartialEq for AllowedType {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

pub struct NodeSelectorWindowBuilder {
    window_builder: WindowBuilder,
    hierarchy: Option<HierarchyNode>,
    allowed_types: FxHashSet<AllowedType>,
}

impl NodeSelectorWindowBuilder {
    pub fn new(window_builder: WindowBuilder) -> Self {
        Self {
            window_builder,
            hierarchy: None,
            allowed_types: Default::default(),
        }
    }

    pub fn with_hierarchy(mut self, hierarchy: HierarchyNode) -> Self {
        self.hierarchy = Some(hierarchy);
        self
    }

    pub fn with_allowed_types(mut self, allowed_types: FxHashSet<AllowedType>) -> Self {
        self.allowed_types = allowed_types;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let ok;
        let cancel;
        let selector;
        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(
                    TextBuilder::new(
                        WidgetBuilder::new()
                            .with_visibility(!self.allowed_types.is_empty())
                            .with_margin(Thickness::uniform(2.0)),
                    )
                    .with_text(
                        "Select a node of the following type(s):\n".to_string()
                            + &self
                                .allowed_types
                                .iter()
                                .map(|ty| ty.name.clone())
                                .collect::<Vec<_>>()
                                .join("\n"),
                    )
                    .with_wrap(WrapMode::Letter)
                    .build(ctx),
                )
                .with_child({
                    selector = NodeSelectorBuilder::new(WidgetBuilder::new().on_row(1))
                        .with_hierarchy(self.hierarchy)
                        .with_allowed_types(self.allowed_types.clone())
                        .build(ctx);
                    selector
                })
                .with_child(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(2.0))
                            .on_row(2)
                            .on_column(0)
                            .with_horizontal_alignment(HorizontalAlignment::Right)
                            .with_child({
                                ok = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_enabled(false)
                                        .with_width(100.0)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_tab_index(Some(2)),
                                )
                                .with_text("OK")
                                .build(ctx);
                                ok
                            })
                            .with_child({
                                cancel = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_width(100.0)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_tab_index(Some(3)),
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
        .add_row(Row::auto())
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
            allowed_types: self.allowed_types,
        };

        ctx.add_node(UiNode::new(window))
    }
}

#[cfg(test)]
mod test {
    use crate::scene::selector::{NodeSelectorBuilder, NodeSelectorWindowBuilder};
    use fyrox::gui::window::WindowBuilder;
    use fyrox::{gui::test::test_widget_deletion, gui::widget::WidgetBuilder};

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| NodeSelectorBuilder::new(WidgetBuilder::new()).build(ctx));
        test_widget_deletion(|ctx| {
            NodeSelectorWindowBuilder::new(WindowBuilder::new(WidgetBuilder::new())).build(ctx)
        });
    }
}
