//! Tree widget allows you to create views for hierarchical data.
//!
//! ## Built-in controls
//!
//! Selection works on all mouse buttons, not just left.
//!
//! `Ctrl+Click` - enables multi-selection.
//! `Alt+Click` - prevents selection allowing you to use drag'n'drop.

use crate::{
    border::BorderBuilder,
    brush::Brush,
    check_box::{CheckBoxBuilder, CheckBoxMessage},
    core::{algebra::Vector2, color::Color, pool::Handle},
    decorator::{DecoratorBuilder, DecoratorMessage},
    define_constructor,
    grid::{Column, GridBuilder, Row},
    message::{MessageDirection, UiMessage},
    stack_panel::StackPanelBuilder,
    utils::{make_arrow, ArrowDirection},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, NodeHandleMapping, Thickness, UiNode, UserInterface, VerticalAlignment,
    BRUSH_DARK, BRUSH_DARKEST, BRUSH_LIGHT,
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SelectionState(pub(in crate) bool);

#[derive(Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Hash, Debug)]
pub enum TreeExpansionStrategy {
    /// Expand a single item.
    Direct,
    /// Expand an item and its descendants.
    RecursiveDescendants,
    /// Expand an item and its ancestors (chain of parent trees).
    RecursiveAncestors,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TreeMessage {
    Expand {
        expand: bool,
        expansion_strategy: TreeExpansionStrategy,
    },
    AddItem(Handle<UiNode>),
    RemoveItem(Handle<UiNode>),
    SetExpanderShown(bool),
    SetItems(Vec<Handle<UiNode>>),
    // Private, do not use. For internal needs only. Use TreeRootMessage::Selected.
    Select(SelectionState),
}

impl TreeMessage {
    define_constructor!(TreeMessage:Expand => fn expand(expand: bool, expansion_strategy: TreeExpansionStrategy), layout: false);
    define_constructor!(TreeMessage:AddItem => fn add_item(Handle<UiNode>), layout: false);
    define_constructor!(TreeMessage:RemoveItem => fn remove_item(Handle<UiNode>), layout: false);
    define_constructor!(TreeMessage:SetExpanderShown => fn set_expander_shown(bool), layout: false);
    define_constructor!(TreeMessage:SetItems => fn set_items(Vec<Handle<UiNode >>), layout: false);
    define_constructor!(TreeMessage:Select => fn select(SelectionState), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum TreeRootMessage {
    AddItem(Handle<UiNode>),
    RemoveItem(Handle<UiNode>),
    Items(Vec<Handle<UiNode>>),
    Selected(Vec<Handle<UiNode>>),
    ExpandAll,
    CollapseAll,
}

impl TreeRootMessage {
    define_constructor!(TreeRootMessage:AddItem => fn add_item(Handle<UiNode>), layout: false);
    define_constructor!(TreeRootMessage:RemoveItem=> fn remove_item(Handle<UiNode>), layout: false);
    define_constructor!(TreeRootMessage:Items => fn items(Vec<Handle<UiNode >>), layout: false);
    define_constructor!(TreeRootMessage:Selected => fn select(Vec<Handle<UiNode >>), layout: false);
    define_constructor!(TreeRootMessage:ExpandAll => fn expand_all(), layout: false);
    define_constructor!(TreeRootMessage:CollapseAll => fn collapse_all(), layout: false);
}

#[derive(Debug, Clone)]
pub struct Tree {
    widget: Widget,
    expander: Handle<UiNode>,
    content: Handle<UiNode>,
    panel: Handle<UiNode>,
    is_expanded: bool,
    background: Handle<UiNode>,
    items: Vec<Handle<UiNode>>,
    is_selected: bool,
    always_show_expander: bool,
}

crate::define_widget_deref!(Tree);

impl Control for Tree {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        node_map.resolve(&mut self.content);
        node_map.resolve(&mut self.expander);
        node_map.resolve(&mut self.panel);
        node_map.resolve(&mut self.background);
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        let size = self.widget.arrange_override(ui, final_size);

        let expander_visibility = !self.items.is_empty() || self.always_show_expander;
        ui.send_message(WidgetMessage::visibility(
            self.expander,
            MessageDirection::ToWidget,
            expander_visibility,
        ));

        size
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(CheckBoxMessage::Check(Some(expanded))) = message.data() {
            if message.destination() == self.expander
                && message.direction == MessageDirection::FromWidget
            {
                ui.send_message(TreeMessage::expand(
                    self.handle(),
                    MessageDirection::ToWidget,
                    *expanded,
                    TreeExpansionStrategy::Direct,
                ));
            }
        } else if let Some(WidgetMessage::MouseDown { .. }) = message.data::<WidgetMessage>() {
            if !message.handled() {
                let keyboard_modifiers = ui.keyboard_modifiers();
                // Prevent selection changes by Alt+Click to be able to drag'n'drop tree items.
                if !keyboard_modifiers.alt {
                    if let Some((tree_root_handle, tree_root)) =
                        ui.try_borrow_by_type_up::<TreeRoot>(self.parent())
                    {
                        let selection = if keyboard_modifiers.control {
                            let mut selection = tree_root.selected.clone();
                            if let Some(existing) = selection.iter().position(|&h| h == self.handle)
                            {
                                selection.remove(existing);
                            } else {
                                selection.push(self.handle);
                            }
                            Some(selection)
                        } else if !self.is_selected {
                            Some(vec![self.handle()])
                        } else {
                            None
                        };
                        if let Some(selection) = selection {
                            ui.send_message(TreeRootMessage::select(
                                tree_root_handle,
                                MessageDirection::ToWidget,
                                selection,
                            ));
                        }
                        message.set_handled(true);
                    }
                }
            }
        } else if let Some(msg) = message.data::<TreeMessage>() {
            if message.destination() == self.handle() {
                match msg {
                    &TreeMessage::Expand {
                        expand,
                        expansion_strategy,
                    } => {
                        self.is_expanded = expand;

                        ui.send_message(WidgetMessage::visibility(
                            self.panel,
                            MessageDirection::ToWidget,
                            self.is_expanded,
                        ));

                        ui.send_message(CheckBoxMessage::checked(
                            self.expander,
                            MessageDirection::ToWidget,
                            Some(expand),
                        ));

                        match expansion_strategy {
                            TreeExpansionStrategy::RecursiveDescendants => {
                                for &item in self.items() {
                                    ui.send_message(TreeMessage::expand(
                                        item,
                                        MessageDirection::ToWidget,
                                        expand,
                                        expansion_strategy,
                                    ));
                                }
                            }
                            TreeExpansionStrategy::RecursiveAncestors => {
                                // CAVEAT: This may lead to potential false expansions when there are
                                // trees inside trees (this is insane, but possible) because we're searching
                                // up on visual tree and don't care about search bounds, ideally we should
                                // stop search if we're found TreeRoot.
                                let parent_tree =
                                    self.find_by_criteria_up(ui, |n| n.cast::<Tree>().is_some());
                                if parent_tree.is_some() {
                                    ui.send_message(TreeMessage::expand(
                                        parent_tree,
                                        MessageDirection::ToWidget,
                                        expand,
                                        expansion_strategy,
                                    ));
                                }
                            }
                            TreeExpansionStrategy::Direct => {
                                // Handle this variant too instead of using _ => (),
                                // to force compiler to notify if new strategy is added.
                            }
                        }
                    }
                    &TreeMessage::SetExpanderShown(show) => {
                        self.always_show_expander = show;
                        self.invalidate_arrange();
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
    }
}

impl Tree {
    pub fn content(&self) -> Handle<UiNode> {
        self.content
    }

    pub fn back(&self) -> Handle<UiNode> {
        self.background
    }

    pub fn items(&self) -> &[Handle<UiNode>] {
        &self.items
    }

    /// Adds new item to given tree. This method is meant to be used only on widget build stage,
    /// any runtime actions should be done via messages.
    pub fn add_item(tree: Handle<UiNode>, item: Handle<UiNode>, ctx: &mut BuildContext) {
        if let Some(tree) = ctx[tree].cast_mut::<Tree>() {
            tree.items.push(item);
            let panel = tree.panel;
            ctx.link(item, panel);
        }
    }

    pub fn expanded(&self) -> bool {
        self.is_expanded
    }

    pub fn expander_shown(&self) -> bool {
        self.always_show_expander
    }
}

pub struct TreeBuilder {
    widget_builder: WidgetBuilder,
    items: Vec<Handle<UiNode>>,
    content: Handle<UiNode>,
    is_expanded: bool,
    always_show_expander: bool,
    back: Option<Handle<UiNode>>,
}

impl TreeBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            items: Default::default(),
            content: Default::default(),
            is_expanded: true,
            always_show_expander: false,
            back: None,
        }
    }

    pub fn with_items(mut self, items: Vec<Handle<UiNode>>) -> Self {
        self.items = items;
        self
    }

    pub fn with_content(mut self, content: Handle<UiNode>) -> Self {
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

    pub fn with_back(mut self, back: Handle<UiNode>) -> Self {
        self.back = Some(back);
        self
    }

    pub fn build_tree(self, ctx: &mut BuildContext) -> Tree {
        let expander = build_expander(
            self.always_show_expander,
            !self.items.is_empty(),
            self.is_expanded,
            ctx,
        );

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
                    .with_foreground(Brush::Solid(Color::TRANSPARENT))
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
                            .with_visibility(self.is_expanded)
                            .with_children(self.items.iter().cloned()),
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

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let tree = self.build_tree(ctx);
        ctx.add_node(UiNode::new(tree))
    }
}

fn build_expander(
    always_show_expander: bool,
    items_populated: bool,
    is_expanded: bool,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    let down_arrow = make_arrow(ctx, ArrowDirection::Bottom, 8.0);
    ctx[down_arrow].set_vertical_alignment(VerticalAlignment::Center);

    let right_arrow = make_arrow(ctx, ArrowDirection::Right, 8.0);
    ctx[right_arrow].set_vertical_alignment(VerticalAlignment::Center);

    CheckBoxBuilder::new(
        WidgetBuilder::new()
            .on_row(0)
            .on_column(0)
            .with_visibility(always_show_expander || items_populated),
    )
    .with_background(
        BorderBuilder::new(
            WidgetBuilder::new()
                .with_background(Brush::Solid(Color::TRANSPARENT))
                .with_min_size(Vector2::new(10.0, 4.0)),
        )
        .with_stroke_thickness(Thickness::zero())
        .build(ctx),
    )
    .checked(Some(is_expanded))
    .with_check_mark(down_arrow)
    .with_uncheck_mark(right_arrow)
    .build(ctx)
}

#[derive(Debug, Clone)]
pub struct TreeRoot {
    widget: Widget,
    panel: Handle<UiNode>,
    items: Vec<Handle<UiNode>>,
    selected: Vec<Handle<UiNode>>,
}

crate::define_widget_deref!(TreeRoot);

impl Control for TreeRoot {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        node_map.resolve(&mut self.panel);
        node_map.resolve_slice(&mut self.selected);
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<TreeRootMessage>() {
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

                                let new_selection_state = if selected.contains(&handle) {
                                    SelectionState(true)
                                } else {
                                    SelectionState(false)
                                };

                                if let Some(tree_ref) = node
                                    .query_component(TypeId::of::<Tree>())
                                    .and_then(|tree_ref| tree_ref.downcast_ref::<Tree>())
                                {
                                    if tree_ref.is_selected != new_selection_state.0 {
                                        ui.send_message(TreeMessage::select(
                                            handle,
                                            MessageDirection::ToWidget,
                                            new_selection_state,
                                        ));
                                    }
                                }
                            }
                            self.selected = selected.clone();
                            ui.send_message(message.reverse());
                        }
                    }
                    TreeRootMessage::CollapseAll => {
                        self.expand_all(ui, false);
                    }
                    TreeRootMessage::ExpandAll => {
                        self.expand_all(ui, true);
                    }
                }
            }
        }
    }
}

impl TreeRoot {
    pub fn items(&self) -> &[Handle<UiNode>] {
        &self.items
    }

    fn expand_all(&self, ui: &UserInterface, expand: bool) {
        for &item in self.items.iter() {
            ui.send_message(TreeMessage::expand(
                item,
                MessageDirection::ToWidget,
                expand,
                TreeExpansionStrategy::RecursiveDescendants,
            ));
        }
    }
}

pub struct TreeRootBuilder {
    widget_builder: WidgetBuilder,
    items: Vec<Handle<UiNode>>,
}

impl TreeRootBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            items: Default::default(),
        }
    }

    pub fn with_items(mut self, items: Vec<Handle<UiNode>>) -> Self {
        self.items = items;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let panel =
            StackPanelBuilder::new(WidgetBuilder::new().with_children(self.items.iter().cloned()))
                .build(ctx);

        let tree = TreeRoot {
            widget: self.widget_builder.with_child(panel).build(),
            panel,
            items: self.items,
            selected: Default::default(),
        };

        ctx.add_node(UiNode::new(tree))
    }
}
