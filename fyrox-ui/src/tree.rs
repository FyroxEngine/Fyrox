//! Tree widget allows you to create views for hierarchical data. See [`Tree`] docs for more info
//! and usage examples.

#![warn(missing_docs)]

use crate::message::KeyCode;
use crate::{
    border::BorderBuilder,
    brush::Brush,
    check_box::{CheckBoxBuilder, CheckBoxMessage},
    core::{
        algebra::Vector2, color::Color, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        visitor::prelude::*,
    },
    decorator::{DecoratorBuilder, DecoratorMessage},
    define_constructor,
    grid::{Column, GridBuilder, Row},
    message::{MessageDirection, UiMessage},
    stack_panel::StackPanelBuilder,
    utils::{make_arrow, ArrowDirection},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, MouseButton, Thickness, UiNode, UserInterface, VerticalAlignment,
    BRUSH_DARK, BRUSH_DIM_BLUE,
};
use fyrox_core::uuid_provider;
use fyrox_graph::{BaseSceneGraph, SceneGraph, SceneGraphNode};
use std::collections::VecDeque;
use std::ops::{Deref, DerefMut};

/// Opaque selection state of a tree.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SelectionState(pub(crate) bool);

/// Expansion strategy for a hierarchical structure.
#[derive(Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Hash, Debug)]
pub enum TreeExpansionStrategy {
    /// Expand a single item.
    Direct,
    /// Expand an item and its descendants.
    RecursiveDescendants,
    /// Expand an item and its ancestors (chain of parent trees).
    RecursiveAncestors,
}

/// A set of messages, that could be used to alternate the state of a [`Tree`] widget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeMessage {
    /// A message, that is used to expand a tree. Exact expansion behavior depends on the expansion
    /// strategy (see [`TreeExpansionStrategy`] docs for more info).
    Expand {
        /// Expand (`true`) or collapse (`false`) a tree.
        expand: bool,
        /// Expansion strategy.
        expansion_strategy: TreeExpansionStrategy,
    },
    /// A message, that is used to add an item to a tree.
    AddItem(Handle<UiNode>),
    /// A message, that is used to remove an item from a tree.
    RemoveItem(Handle<UiNode>),
    /// A message, that is used to prevent expander from being hidden when a tree does not have
    /// any child items.
    SetExpanderShown(bool),
    /// A message, that is use to specify a new set of children items of a tree.
    SetItems {
        /// A set of handles to new tree items.
        items: Vec<Handle<UiNode>>,
        /// A flag, that defines whether the previous items should be deleted or not. `false` is
        /// usually used to reorder existing items.
        remove_previous: bool,
    },
    // Private, do not use. For internal needs only. Use TreeRootMessage::Selected.
    #[doc(hidden)]
    Select(SelectionState),
}

impl TreeMessage {
    define_constructor!(
        /// Creates [`TreeMessage::Expand`] message.
        TreeMessage:Expand => fn expand(expand: bool, expansion_strategy: TreeExpansionStrategy), layout: false
    );
    define_constructor!(
        /// Creates [`TreeMessage::AddItem`] message.
        TreeMessage:AddItem => fn add_item(Handle<UiNode>), layout: false
    );
    define_constructor!(
        /// Creates [`TreeMessage::RemoveItem`] message.
        TreeMessage:RemoveItem => fn remove_item(Handle<UiNode>), layout: false
    );
    define_constructor!(
        /// Creates [`TreeMessage::SetExpanderShown`] message.
        TreeMessage:SetExpanderShown => fn set_expander_shown(bool), layout: false
    );
    define_constructor!(
        /// Creates [`TreeMessage::SetItems`] message.
        TreeMessage:SetItems => fn set_items(items: Vec<Handle<UiNode>>, remove_previous: bool), layout: false
    );
    define_constructor!(
        /// Creates [`TreeMessage::Select`] message.
        TreeMessage:Select => fn select(SelectionState), layout: false
    );
}

/// A set of messages, that could be used to alternate the state of a [`TreeRoot`] widget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeRootMessage {
    /// A message, that is used to add a child item to a tree root.
    AddItem(Handle<UiNode>),
    /// A message, that is used to remove a child item from a tree root.
    RemoveItem(Handle<UiNode>),
    /// A message, that is used to specify a new set of children items of a tree root.
    Items(Vec<Handle<UiNode>>),
    /// A message, that it is used to fetch or set current selection of a tree root.
    Selected(Vec<Handle<UiNode>>),
    /// A message, that is used to expand all descendant trees in the hierarchy.
    ExpandAll,
    /// A message, that is used to collapse all descendant trees in the hierarchy.
    CollapseAll,
    /// A message, that is used as a notification when tree root's items has changed.
    ItemsChanged,
}

impl TreeRootMessage {
    define_constructor!(
        /// Creates [`TreeRootMessage::AddItem`] message.
        TreeRootMessage:AddItem => fn add_item(Handle<UiNode>), layout: false
    );
    define_constructor!(
        /// Creates [`TreeRootMessage::RemoveItem`] message.
        TreeRootMessage:RemoveItem=> fn remove_item(Handle<UiNode>), layout: false
    );
    define_constructor!(
        /// Creates [`TreeRootMessage::Items`] message.
        TreeRootMessage:Items => fn items(Vec<Handle<UiNode >>), layout: false
    );
    define_constructor!(
        /// Creates [`TreeRootMessage::Selected`] message.
        TreeRootMessage:Selected => fn select(Vec<Handle<UiNode >>), layout: false
    );
    define_constructor!(
        /// Creates [`TreeRootMessage::ExpandAll`] message.
        TreeRootMessage:ExpandAll => fn expand_all(), layout: false
    );
    define_constructor!(
        /// Creates [`TreeRootMessage::CollapseAll`] message.
        TreeRootMessage:CollapseAll => fn collapse_all(), layout: false
    );
    define_constructor!(
        /// Creates [`TreeRootMessage::ItemsChanged`] message.
        TreeRootMessage:ItemsChanged => fn items_changed(), layout: false
    );
}

/// Tree widget allows you to create views for hierarchical data. It could be used to show file
/// system entries, graphs, and anything else that could be represented as a tree.
///
/// ## Examples
///
/// A simple tree with one root and two children items could be created like so:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle,
/// #     text::TextBuilder,
/// #     tree::{TreeBuilder, TreeRootBuilder},
/// #     widget::WidgetBuilder,
/// #     BuildContext, UiNode,
/// # };
/// #
/// fn create_tree(ctx: &mut BuildContext) -> Handle<UiNode> {
///     // Note, that `TreeRoot` widget is mandatory here. Otherwise some functionality of
///     // descendant trees won't work.
///     TreeRootBuilder::new(WidgetBuilder::new())
///         .with_items(vec![TreeBuilder::new(WidgetBuilder::new())
///             .with_content(
///                 TextBuilder::new(WidgetBuilder::new())
///                     .with_text("Root Item 0")
///                     .build(ctx),
///             )
///             .with_items(vec![
///                 TreeBuilder::new(WidgetBuilder::new())
///                     .with_content(
///                         TextBuilder::new(WidgetBuilder::new())
///                             .with_text("Child Item 0")
///                             .build(ctx),
///                     )
///                     .build(ctx),
///                 TreeBuilder::new(WidgetBuilder::new())
///                     .with_content(
///                         TextBuilder::new(WidgetBuilder::new())
///                             .with_text("Child Item 1")
///                             .build(ctx),
///                     )
///                     .build(ctx),
///             ])
///             .build(ctx)])
///         .build(ctx)
/// }
/// ```
///
/// Note, that `TreeRoot` widget is mandatory here. Otherwise some functionality of descendant trees
/// won't work (primarily - selection). See [`TreeRoot`] docs for more detailed explanation.
///
/// ## Built-in controls
///
/// Tree widget is a rich control element, which has its own set of controls:
///
/// `Any Mouse Button` - select.
/// `Ctrl+Click` - enables multi-selection.
/// `Alt+Click` - prevents selection allowing you to use drag'n'drop.
/// `Shift+Click` - selects a span of items.
#[derive(Default, Debug, Clone, Visit, Reflect, ComponentProvider)]
pub struct Tree {
    /// Base widget of the tree.
    pub widget: Widget,
    /// Current expander of the tree. Usually, it is just a handle of CheckBox widget.
    pub expander: Handle<UiNode>,
    /// Current content of the tree.
    pub content: Handle<UiNode>,
    /// Current layout panel, that used to arrange children items.
    pub panel: Handle<UiNode>,
    /// A flag, that indicates whether the tree is expanded or not.
    pub is_expanded: bool,
    /// Current background widget of the tree.
    pub background: Handle<UiNode>,
    /// Current set of items of the tree.
    pub items: Vec<Handle<UiNode>>,
    /// A flag, that defines whether the tree is selected or not.
    pub is_selected: bool,
    /// A flag, that defines whether the tree should always show its expander, even if there's no
    /// children elements, or not.
    pub always_show_expander: bool,
}

crate::define_widget_deref!(Tree);

uuid_provider!(Tree = "e090e913-393a-4192-a220-e1d87e272170");

impl Control for Tree {
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
        } else if let Some(msg) = message.data::<WidgetMessage>() {
            if !message.handled() {
                match msg {
                    WidgetMessage::MouseDown { .. } => {
                        let keyboard_modifiers = ui.keyboard_modifiers();
                        // Prevent selection changes by Alt+Click to be able to drag'n'drop tree items.
                        if !keyboard_modifiers.alt {
                            if let Some((tree_root_handle, tree_root)) =
                                ui.find_component_up::<TreeRoot>(self.parent())
                            {
                                let selection = if keyboard_modifiers.control {
                                    let mut selection = tree_root.selected.clone();
                                    if let Some(existing) =
                                        selection.iter().position(|&h| h == self.handle)
                                    {
                                        selection.remove(existing);
                                    } else {
                                        selection.push(self.handle);
                                    }
                                    Some(selection)
                                } else if keyboard_modifiers.shift {
                                    // Select range.
                                    let mut first_position = None;
                                    let mut this_position = None;
                                    let mut flat_hierarchy = Vec::new();

                                    fn visit_widget(
                                        this_tree: &Tree,
                                        handle: Handle<UiNode>,
                                        ui: &UserInterface,
                                        selection: &[Handle<UiNode>],
                                        hierarchy: &mut Vec<Handle<UiNode>>,
                                        first_position: &mut Option<usize>,
                                        this_position: &mut Option<usize>,
                                    ) {
                                        let node = if handle == this_tree.handle {
                                            *this_position = Some(hierarchy.len());

                                            hierarchy.push(handle);

                                            &this_tree.widget
                                        } else {
                                            let node = ui.node(handle);

                                            if let Some(first) = selection.first() {
                                                if *first == handle {
                                                    *first_position = Some(hierarchy.len());
                                                }
                                            }

                                            if node.query_component::<Tree>().is_some() {
                                                hierarchy.push(handle);
                                            }

                                            node
                                        };

                                        for &child in node.children() {
                                            visit_widget(
                                                this_tree,
                                                child,
                                                ui,
                                                selection,
                                                hierarchy,
                                                first_position,
                                                this_position,
                                            );
                                        }
                                    }

                                    visit_widget(
                                        self,
                                        tree_root_handle,
                                        ui,
                                        &tree_root.selected,
                                        &mut flat_hierarchy,
                                        &mut first_position,
                                        &mut this_position,
                                    );

                                    if let (Some(this_position), Some(first_position)) =
                                        (this_position, first_position)
                                    {
                                        Some(if first_position < this_position {
                                            flat_hierarchy[first_position..=this_position].to_vec()
                                        } else {
                                            flat_hierarchy[this_position..=first_position].to_vec()
                                        })
                                    } else {
                                        Some(vec![])
                                    }
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
                    WidgetMessage::DoubleClick { button } => {
                        if *button == MouseButton::Left {
                            // Mimic click on expander button to have uniform behavior.
                            ui.send_message(CheckBoxMessage::checked(
                                self.expander,
                                MessageDirection::ToWidget,
                                Some(!self.is_expanded),
                            ));

                            message.set_handled(true);
                        }
                    }
                    _ => (),
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
                                for &item in &self.items {
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
                    TreeMessage::SetItems {
                        items,
                        remove_previous,
                    } => {
                        if *remove_previous {
                            for &item in self.items.iter() {
                                ui.send_message(WidgetMessage::remove(
                                    item,
                                    MessageDirection::ToWidget,
                                ));
                            }
                        }
                        for &item in items {
                            ui.send_message(WidgetMessage::link(
                                item,
                                MessageDirection::ToWidget,
                                self.panel,
                            ));
                        }
                        self.items.clone_from(items);
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
    /// Adds new item to given tree. This method is meant to be used only on widget build stage,
    /// any runtime actions should be done via messages.
    pub fn add_item(tree: Handle<UiNode>, item: Handle<UiNode>, ctx: &mut BuildContext) {
        if let Some(tree) = ctx[tree].cast_mut::<Tree>() {
            tree.items.push(item);
            let panel = tree.panel;
            ctx.link(item, panel);
        }
    }
}

/// Tree builder creates [`Tree`] widget instances and adds them to the user interface.
pub struct TreeBuilder {
    widget_builder: WidgetBuilder,
    items: Vec<Handle<UiNode>>,
    content: Handle<UiNode>,
    is_expanded: bool,
    always_show_expander: bool,
    back: Option<Handle<UiNode>>,
}

impl TreeBuilder {
    /// Creates a new tree builder instance.
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

    /// Sets the desired children items of the tree.
    pub fn with_items(mut self, items: Vec<Handle<UiNode>>) -> Self {
        self.items = items;
        self
    }

    /// Sets the desired content of the tree.
    pub fn with_content(mut self, content: Handle<UiNode>) -> Self {
        self.content = content;
        self
    }

    /// Sets the desired expansion state of the tree.
    pub fn with_expanded(mut self, expanded: bool) -> Self {
        self.is_expanded = expanded;
        self
    }

    /// Sets whether the tree should always show its expander, no matter if has children items or
    /// not.
    pub fn with_always_show_expander(mut self, state: bool) -> Self {
        self.always_show_expander = state;
        self
    }

    /// Sets the desired background of the tree.
    pub fn with_back(mut self, back: Handle<UiNode>) -> Self {
        self.back = Some(back);
        self
    }

    /// Builds the tree widget, but does not add it to user interface.
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
        .add_column(Column::strict(11.0))
        .add_column(Column::stretch())
        .add_row(Row::strict(20.0))
        .build(ctx);

        let item_background = self.back.unwrap_or_else(|| {
            DecoratorBuilder::new(BorderBuilder::new(
                WidgetBuilder::new()
                    .with_foreground(Brush::Solid(Color::TRANSPARENT))
                    .with_background(Brush::Solid(Color::TRANSPARENT)),
            ))
            .with_selected_brush(BRUSH_DIM_BLUE)
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

    /// Finishes widget building and adds it to the user interface, returning a handle to the new
    /// instance.
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

/// Tree root is special widget that handles the entire hierarchy of descendant [`Tree`] widgets. Its
/// main purpose is to handle selection of descendant [`Tree`] widgets. Tree root cannot have a
/// content and it only could have children tree items. See docs for [`Tree`] for usage examples.
#[derive(Default, Debug, Clone, Visit, Reflect, ComponentProvider)]
pub struct TreeRoot {
    /// Base widget of the tree root.
    pub widget: Widget,
    /// Current layout panel of the tree root, that is used to arrange children trees.
    pub panel: Handle<UiNode>,
    /// Current items of the tree root.
    pub items: Vec<Handle<UiNode>>,
    /// Selected items of the tree root.
    pub selected: Vec<Handle<UiNode>>,
}

crate::define_widget_deref!(TreeRoot);

uuid_provider!(TreeRoot = "cf7c0476-f779-4e4b-8b7e-01a23ff51a72");

impl Control for TreeRoot {
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
                        ui.send_message(TreeRootMessage::items_changed(
                            self.handle,
                            MessageDirection::FromWidget,
                        ));
                    }
                    &TreeRootMessage::RemoveItem(item) => {
                        if let Some(pos) = self.items.iter().position(|&i| i == item) {
                            ui.send_message(WidgetMessage::remove(
                                item,
                                MessageDirection::ToWidget,
                            ));

                            self.items.remove(pos);
                            ui.send_message(TreeRootMessage::items_changed(
                                self.handle,
                                MessageDirection::FromWidget,
                            ));
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
                        ui.send_message(TreeRootMessage::items_changed(
                            self.handle,
                            MessageDirection::FromWidget,
                        ));
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

                                if let Some(tree_ref) = node.query_component::<Tree>() {
                                    if tree_ref.is_selected != new_selection_state.0 {
                                        ui.send_message(TreeMessage::select(
                                            handle,
                                            MessageDirection::ToWidget,
                                            new_selection_state,
                                        ));
                                    }
                                }
                            }
                            self.selected.clone_from(selected);
                            ui.send_message(message.reverse());
                        }
                    }
                    TreeRootMessage::CollapseAll => {
                        self.expand_all(ui, false);
                    }
                    TreeRootMessage::ExpandAll => {
                        self.expand_all(ui, true);
                    }
                    TreeRootMessage::ItemsChanged => {
                        // Do nothing.
                    }
                }
            }
        } else if let Some(WidgetMessage::KeyDown(key_code)) = message.data() {
            if !message.handled() {
                match *key_code {
                    KeyCode::ArrowRight => {
                        self.move_selection(ui, Direction::Down, true);
                        message.set_handled(true);
                    }
                    KeyCode::ArrowLeft => {
                        if let Some(selection) = self.selected.first() {
                            if let Some(item) = ui
                                .try_get(*selection)
                                .and_then(|n| n.component_ref::<Tree>())
                            {
                                if item.is_expanded {
                                    ui.send_message(TreeMessage::expand(
                                        *selection,
                                        MessageDirection::ToWidget,
                                        false,
                                        TreeExpansionStrategy::Direct,
                                    ));
                                    message.set_handled(true);
                                } else if let Some((parent_handle, _)) =
                                    ui.find_component_up::<Tree>(item.parent())
                                {
                                    ui.send_message(TreeRootMessage::select(
                                        self.handle,
                                        MessageDirection::ToWidget,
                                        vec![parent_handle],
                                    ));
                                    message.set_handled(true);
                                }
                            }
                        }
                    }
                    KeyCode::ArrowUp => {
                        self.move_selection(ui, Direction::Up, false);
                        message.set_handled(true);
                    }
                    KeyCode::ArrowDown => {
                        self.move_selection(ui, Direction::Down, false);
                        message.set_handled(true);
                    }
                    _ => (),
                }
            }
        }
    }
}

enum Direction {
    Up,
    Down,
}

impl TreeRoot {
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

    fn select(&self, ui: &UserInterface, item: Handle<UiNode>) {
        ui.send_message(TreeRootMessage::select(
            self.handle,
            MessageDirection::ToWidget,
            vec![item],
        ));
    }

    fn move_selection(&self, ui: &UserInterface, direction: Direction, expand: bool) {
        if let Some(selected_item) = self.selected.first() {
            let Some(item) = ui
                .try_get(*selected_item)
                .and_then(|n| n.component_ref::<Tree>())
            else {
                return;
            };

            if !item.is_expanded && expand {
                ui.send_message(TreeMessage::expand(
                    *selected_item,
                    MessageDirection::ToWidget,
                    true,
                    TreeExpansionStrategy::Direct,
                ));
                return;
            }

            let Some((parent_handle, parent)) = ui.find_component_up::<Tree>(item.parent()) else {
                return;
            };

            let Some(selected_item_position) =
                parent.items.iter().position(|c| *c == *selected_item)
            else {
                return;
            };

            match direction {
                Direction::Up => {
                    if let Some(prev) = selected_item_position
                        .checked_sub(1)
                        .and_then(|prev| parent.items.get(prev))
                    {
                        let mut last_descendant_item = None;
                        let mut queue = VecDeque::new();
                        queue.push_back(*prev);
                        while let Some(item) = queue.pop_front() {
                            if let Some(item_ref) = ui.node(item).component_ref::<Tree>() {
                                if item_ref.is_expanded {
                                    queue.extend(item_ref.items.iter());
                                }
                                last_descendant_item = Some(item);
                            }
                        }

                        if let Some(last_descendant_item) = last_descendant_item {
                            self.select(ui, last_descendant_item);
                        }
                    } else {
                        self.select(ui, parent_handle);
                    }
                }
                Direction::Down => {
                    if let (Some(first_item), true) = (item.items.first(), item.is_expanded) {
                        self.select(ui, *first_item);
                    } else if let Some(next) =
                        parent.items.get(selected_item_position.saturating_add(1))
                    {
                        self.select(ui, *next);
                    } else {
                        let mut current_ancestor = parent_handle;
                        let mut current_ancestor_parent = parent.parent();
                        while let Some((ancestor_handle, ancestor)) =
                            ui.find_component_up::<Tree>(current_ancestor_parent)
                        {
                            if ancestor.is_expanded {
                                if let Some(current_ancestor_position) =
                                    ancestor.items.iter().position(|c| *c == current_ancestor)
                                {
                                    if let Some(next) = ancestor
                                        .items
                                        .get(current_ancestor_position.saturating_add(1))
                                    {
                                        self.select(ui, *next);
                                        break;
                                    }
                                }
                            }

                            current_ancestor_parent = ancestor.parent();
                            current_ancestor = ancestor_handle;
                        }
                    }
                }
            }
        } else if let Some(first_item) = self.items.first() {
            self.select(ui, *first_item);
        }
    }
}

/// Tree root builder creates [`TreeRoot`] instances and adds them to the user interface.
pub struct TreeRootBuilder {
    widget_builder: WidgetBuilder,
    items: Vec<Handle<UiNode>>,
}

impl TreeRootBuilder {
    /// Creates new tree root builder.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            items: Default::default(),
        }
    }

    /// Sets the desired items of the tree root.
    pub fn with_items(mut self, items: Vec<Handle<UiNode>>) -> Self {
        self.items = items;
        self
    }

    /// Finishes widget building and adds the new instance to the user interface, returning its handle.
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
