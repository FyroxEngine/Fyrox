//! List view is used to display lists with arbitrary items. It supports single-selection and by default, it stacks the items
//! vertically.  

#![warn(missing_docs)]

use crate::{
    border::BorderBuilder,
    brush::Brush,
    core::{
        color::Color, pool::Handle, reflect::prelude::*, type_traits::prelude::*, uuid_provider,
        variable::InheritableVariable, visitor::prelude::*,
    },
    decorator::{Decorator, DecoratorMessage},
    define_constructor,
    draw::{CommandTexture, Draw, DrawingContext},
    message::{KeyCode, MessageDirection, UiMessage},
    scroll_viewer::{ScrollViewer, ScrollViewerBuilder, ScrollViewerMessage},
    stack_panel::StackPanelBuilder,
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, Thickness, UiNode, UserInterface, BRUSH_DARK, BRUSH_LIGHT,
};
use fyrox_graph::BaseSceneGraph;
use std::ops::{Deref, DerefMut};

/// A set of messages that can be used to modify/fetch the state of a [`ListView`] widget at runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListViewMessage {
    /// A message, that is used to either fetch or modify current selection of a [`ListView`] widget.
    SelectionChanged(Vec<usize>),
    /// A message, that is used to set new items of a list view.
    Items(Vec<Handle<UiNode>>),
    /// A message, that is used to add an item to a list view.
    AddItem(Handle<UiNode>),
    /// A message, that is used to remove an item from a list view.
    RemoveItem(Handle<UiNode>),
    /// A message, that is used to bring an item into view.
    BringItemIntoView(Handle<UiNode>),
}

impl ListViewMessage {
    define_constructor!(
        /// Creates [`ListViewMessage::SelectionChanged`] message.
        ListViewMessage:SelectionChanged => fn selection(Vec<usize>), layout: false
    );
    define_constructor!(
        /// Creates [`ListViewMessage::Items`] message.
        ListViewMessage:Items => fn items(Vec<Handle<UiNode >>), layout: false
    );
    define_constructor!(
        /// Creates [`ListViewMessage::AddItem`] message.
        ListViewMessage:AddItem => fn add_item(Handle<UiNode>), layout: false
    );
    define_constructor!(
        /// Creates [`ListViewMessage::RemoveItem`] message.
        ListViewMessage:RemoveItem => fn remove_item(Handle<UiNode>), layout: false
    );
    define_constructor!(
        /// Creates [`ListViewMessage::BringItemIntoView`] message.
        ListViewMessage:BringItemIntoView => fn bring_item_into_view(Handle<UiNode>), layout: false
    );
}

/// List view is used to display lists with arbitrary items. It supports single-selection and by default, it stacks the items
/// vertically.  
///
/// ## Example
///
/// [`ListView`] can be created using [`ListViewBuilder`]:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle, list_view::ListViewBuilder, text::TextBuilder, widget::WidgetBuilder,
/// #     BuildContext, UiNode,
/// # };
/// #
/// fn create_list(ctx: &mut BuildContext) -> Handle<UiNode> {
///     ListViewBuilder::new(WidgetBuilder::new())
///         .with_items(vec![
///             TextBuilder::new(WidgetBuilder::new())
///                 .with_text("Item0")
///                 .build(ctx),
///             TextBuilder::new(WidgetBuilder::new())
///                 .with_text("Item1")
///                 .build(ctx),
///         ])
///         .build(ctx)
/// }
/// ```
///
/// Keep in mind, that the items of the list view can be pretty much any other widget. They also don't have to be the same
/// type, you can mix any type of widgets.
///
/// ## Custom Items Panel
///
/// By default, list view creates inner [`crate::stack_panel::StackPanel`] to arrange its items. It is enough for most cases,
/// however in rare cases you might want to use something else. For example, you could use [`crate::wrap_panel::WrapPanel`]
/// to create list view with selectable "tiles":
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle, list_view::ListViewBuilder, text::TextBuilder, widget::WidgetBuilder,
/// #     wrap_panel::WrapPanelBuilder, BuildContext, UiNode,
/// # };
/// fn create_list(ctx: &mut BuildContext) -> Handle<UiNode> {
///     ListViewBuilder::new(WidgetBuilder::new())
///         // Using WrapPanel instead of StackPanel:
///         .with_items_panel(WrapPanelBuilder::new(WidgetBuilder::new()).build(ctx))
///         .with_items(vec![
///             TextBuilder::new(WidgetBuilder::new())
///                 .with_text("Item0")
///                 .build(ctx),
///             TextBuilder::new(WidgetBuilder::new())
///                 .with_text("Item1")
///                 .build(ctx),
///         ])
///         .build(ctx)
/// }
/// ```
///
/// ## Selection
///
/// List view support single selection only, you can change it at runtime by sending [`ListViewMessage::SelectionChanged`]
/// message with [`MessageDirection::ToWidget`] like so:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle, list_view::ListViewMessage, message::MessageDirection, UiNode,
/// #     UserInterface,
/// # };
/// fn change_selection(my_list_view: Handle<UiNode>, ui: &UserInterface) {
///     ui.send_message(ListViewMessage::selection(
///         my_list_view,
///         MessageDirection::ToWidget,
///         vec![1],
///     ));
/// }
/// ```
///
/// It is also possible to not have selected item at all, to do this you need to send [`None`] as a selection.
///
/// To catch the moment when selection has changed (either by a user or by the [`ListViewMessage::SelectionChanged`],) you need
/// to listen to the same message but with opposite direction, like so:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle, list_view::ListViewMessage, message::MessageDirection,
/// #     message::UiMessage, UiNode,
/// # };
/// #
/// fn do_something(my_list_view: Handle<UiNode>, message: &UiMessage) {
///     if let Some(ListViewMessage::SelectionChanged(selection)) = message.data() {
///         if message.destination() == my_list_view
///             && message.direction() == MessageDirection::FromWidget
///         {
///             println!("New selection is: {:?}", selection);
///         }
///     }
/// }
/// ```
///
/// ## Adding/removing items
///
/// To change items of the list view you can use the variety of following messages: [`ListViewMessage::AddItem`], [`ListViewMessage::RemoveItem`],
/// [`ListViewMessage::Items`]. To decide which one to use, is very simple - if you adding/removing a few items, use [`ListViewMessage::AddItem`]
/// and [`ListViewMessage::RemoveItem`], otherwise use [`ListViewMessage::Items`], which changes the items at once.
///
/// ```rust
/// use fyrox_ui::{
///     core::pool::Handle, list_view::ListViewMessage, message::MessageDirection,
///     text::TextBuilder, widget::WidgetBuilder, UiNode, UserInterface,
/// };
/// fn change_items(my_list_view: Handle<UiNode>, ui: &mut UserInterface) {
///     let ctx = &mut ui.build_ctx();
///
///     // Build new items first.
///     let items = vec![
///         TextBuilder::new(WidgetBuilder::new())
///             .with_text("Item0")
///             .build(ctx),
///         TextBuilder::new(WidgetBuilder::new())
///             .with_text("Item1")
///             .build(ctx),
///     ];
///
///     // Then send the message with their handles to the list view.
///     ui.send_message(ListViewMessage::items(
///         my_list_view,
///         MessageDirection::ToWidget,
///         items,
///     ));
/// }
/// ```
///
/// ## Bringing a particular item into view
///
/// It is possible to bring a particular item into view, which is useful when you have hundreds or thousands of items and you
/// want to bring only particular item into view. It could be done by sending a [`ListViewMessage::BringItemIntoView`] message:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle, list_view::ListViewMessage, message::MessageDirection, UiNode,
/// #     UserInterface,
/// # };
/// fn bring_item_into_view(
///     my_list_view: Handle<UiNode>,
///     my_item: Handle<UiNode>,
///     ui: &UserInterface,
/// ) {
///     ui.send_message(ListViewMessage::bring_item_into_view(
///         my_list_view,
///         MessageDirection::ToWidget,
///         my_item,
///     ));
/// }
/// ```
#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider)]
pub struct ListView {
    /// Base widget of the list view.
    pub widget: Widget,
    /// Current selection.
    pub selection: Vec<usize>,
    /// An array of handle of item containers, which wraps the actual items.
    pub item_containers: InheritableVariable<Vec<Handle<UiNode>>>,
    /// Current panel widget that is used to arrange the items.
    pub panel: InheritableVariable<Handle<UiNode>>,
    /// Current items of the list view.
    pub items: InheritableVariable<Vec<Handle<UiNode>>>,
    /// Current scroll viewer instance that is used to provide scrolling functionality, when items does
    /// not fit in the view entirely.
    pub scroll_viewer: InheritableVariable<Handle<UiNode>>,
}

crate::define_widget_deref!(ListView);

impl ListView {
    /// Returns a slice with current items.
    pub fn items(&self) -> &[Handle<UiNode>] {
        &self.items
    }

    fn fix_selection(&self, ui: &UserInterface) {
        // Check if current selection is out-of-bounds.
        let mut fixed_selection = Vec::with_capacity(self.selection.len());

        for &selected_index in self.selection.iter() {
            if selected_index >= self.items.len() {
                if !self.items.is_empty() {
                    fixed_selection.push(self.items.len() - 1);
                }
            } else {
                fixed_selection.push(selected_index);
            }
        }

        if self.selection != fixed_selection {
            ui.send_message(ListViewMessage::selection(
                self.handle,
                MessageDirection::ToWidget,
                fixed_selection,
            ));
        }
    }

    fn largest_selection_index(&self) -> Option<usize> {
        self.selection.iter().max().cloned()
    }

    fn smallest_selection_index(&self) -> Option<usize> {
        self.selection.iter().min().cloned()
    }

    fn sync_decorators(&self, ui: &UserInterface) {
        for (i, &container) in self.item_containers.iter().enumerate() {
            let select = self.selection.contains(&i);
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

/// A wrapper for list view items, that is used to add selection functionality to arbitrary items.
#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider)]
pub struct ListViewItem {
    /// Base widget of the list view item.
    pub widget: Widget,
}

crate::define_widget_deref!(ListViewItem);

uuid_provider!(ListViewItem = "02f21415-5843-42f5-a3e4-b4a21e7739ad");

impl Control for ListViewItem {
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
                let list_view = ui
                    .node(parent_list_view)
                    .cast::<ListView>()
                    .expect("Parent of ListViewItem must be ListView!");

                let self_index = list_view
                    .item_containers
                    .iter()
                    .position(|c| *c == self.handle)
                    .expect("ListViewItem must be used as a child of ListView");

                let new_selection = if ui.keyboard_modifiers.control {
                    let mut selection = list_view.selection.clone();
                    selection.push(self_index);
                    selection
                } else {
                    vec![self_index]
                };

                // Explicitly set selection on parent items control. This will send
                // SelectionChanged message and all items will react.
                ui.send_message(ListViewMessage::selection(
                    parent_list_view,
                    MessageDirection::ToWidget,
                    new_selection,
                ));
                message.set_handled(true);
            }
        }
    }
}

uuid_provider!(ListView = "5832a643-5bf9-4d84-8358-b4c45bb440e8");

impl Control for ListView {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<ListViewMessage>() {
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::ToWidget
            {
                match msg {
                    ListViewMessage::Items(items) => {
                        // Remove previous items.
                        for child in ui.node(*self.panel).children() {
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
                                *self.panel,
                            ));
                        }

                        self.item_containers
                            .set_value_and_mark_modified(item_containers);
                        self.items.set_value_and_mark_modified(items.clone());

                        self.fix_selection(ui);
                        self.sync_decorators(ui);
                    }
                    &ListViewMessage::AddItem(item) => {
                        let item_container = generate_item_container(&mut ui.build_ctx(), item);

                        ui.send_message(WidgetMessage::link(
                            item_container,
                            MessageDirection::ToWidget,
                            *self.panel,
                        ));

                        self.item_containers.push(item_container);
                        self.items.push(item);
                    }
                    ListViewMessage::SelectionChanged(selection) => {
                        if &self.selection != selection {
                            self.selection.clone_from(selection);
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
                                *self.scroll_viewer,
                                MessageDirection::ToWidget,
                                item,
                            ));
                        }
                    }
                }
            }
        } else if let Some(WidgetMessage::KeyDown(key_code)) = message.data() {
            if !message.handled() {
                let new_selection = if *key_code == KeyCode::ArrowDown {
                    match self.largest_selection_index() {
                        Some(i) => Some(i.saturating_add(1) % self.items.len()),
                        None => {
                            if self.items.is_empty() {
                                None
                            } else {
                                Some(0)
                            }
                        }
                    }
                } else if *key_code == KeyCode::ArrowUp {
                    match self.smallest_selection_index() {
                        Some(i) => {
                            let mut index = (i as isize).saturating_sub(1);
                            let count = self.items.len() as isize;
                            if index < 0 {
                                index += count;
                            }
                            Some((index % count) as usize)
                        }
                        None => {
                            if self.items.is_empty() {
                                None
                            } else {
                                Some(0)
                            }
                        }
                    }
                } else {
                    None
                };

                if let Some(new_selection) = new_selection {
                    ui.send_message(ListViewMessage::selection(
                        self.handle,
                        MessageDirection::ToWidget,
                        vec![new_selection],
                    ));

                    message.set_handled(true);
                }
            }
        }
    }
}

/// List view builder is used to create [`ListView`] widget instances and add them to a user interface.
pub struct ListViewBuilder {
    widget_builder: WidgetBuilder,
    items: Vec<Handle<UiNode>>,
    panel: Option<Handle<UiNode>>,
    scroll_viewer: Option<Handle<UiNode>>,
}

impl ListViewBuilder {
    /// Creates new list view builder.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            items: Vec::new(),
            panel: None,
            scroll_viewer: None,
        }
    }

    /// Sets an array of handle of desired items for the list view.
    pub fn with_items(mut self, items: Vec<Handle<UiNode>>) -> Self {
        self.items = items;
        self
    }

    /// Sets the desired item panel that will be used to arrange the items.
    pub fn with_items_panel(mut self, panel: Handle<UiNode>) -> Self {
        self.panel = Some(panel);
        self
    }

    /// Sets the desired scroll viewer.
    pub fn with_scroll_viewer(mut self, sv: Handle<UiNode>) -> Self {
        self.scroll_viewer = Some(sv);
        self
    }

    /// Finishes list view building and adds it to the user interface.
    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let item_containers = generate_item_containers(ctx, &self.items);

        let panel = self
            .panel
            .unwrap_or_else(|| StackPanelBuilder::new(WidgetBuilder::new()).build(ctx));

        for &item_container in item_containers.iter() {
            ctx.link(item_container, panel);
        }

        let back = BorderBuilder::new(
            WidgetBuilder::new()
                .with_background(BRUSH_DARK)
                .with_foreground(BRUSH_LIGHT),
        )
        .with_stroke_thickness(Thickness::uniform(1.0))
        .build(ctx);

        let scroll_viewer = self.scroll_viewer.unwrap_or_else(|| {
            ScrollViewerBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(0.0)))
                .build(ctx)
        });
        let scroll_viewer_ref = ctx[scroll_viewer]
            .cast_mut::<ScrollViewer>()
            .expect("ListView must have ScrollViewer");
        scroll_viewer_ref.content = panel;
        let content_presenter = scroll_viewer_ref.scroll_panel;
        ctx.link(panel, content_presenter);

        ctx.link(scroll_viewer, back);

        let list_box = ListView {
            widget: self
                .widget_builder
                .with_accepts_input(true)
                .with_child(back)
                .build(),
            selection: Default::default(),
            item_containers: item_containers.into(),
            items: self.items.into(),
            panel: panel.into(),
            scroll_viewer: scroll_viewer.into(),
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
