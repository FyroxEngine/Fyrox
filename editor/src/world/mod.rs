use crate::{
    gui::make_image_button_with_tooltip,
    load_image,
    message::MessageSender,
    send_sync_message,
    utils::window_content,
    world::graph::item::{SceneItem, SceneItemBuilder, SceneItemMessage},
    Mode, Settings,
};
use fyrox::{
    core::{
        color::Color,
        pool::{ErasedHandle, Handle},
        scope_profile,
    },
    engine::Engine,
    gui::{
        border::BorderBuilder,
        brush::Brush,
        button::{ButtonBuilder, ButtonMessage},
        check_box::{CheckBoxBuilder, CheckBoxMessage},
        decorator::{Decorator, DecoratorBuilder, DecoratorMessage},
        draw::SharedTexture,
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessage},
        scroll_viewer::{ScrollViewerBuilder, ScrollViewerMessage},
        searchbar::{SearchBarBuilder, SearchBarMessage},
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        tree::{
            TreeBuilder, TreeExpansionStrategy, TreeMessage, TreeRoot, TreeRootBuilder,
            TreeRootMessage,
        },
        ttf::{FontBuilder, SharedFont},
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowTitle},
        wrap_panel::WrapPanelBuilder,
        BuildContext, Orientation, RcUiNodeHandle, Thickness, UiNode, UserInterface,
        VerticalAlignment, BRUSH_BRIGHT_BLUE, BRUSH_PRIMARY,
    },
};
use rust_fuzzy_search::fuzzy_compare;
use std::{cell::RefCell, cmp::Ordering, collections::HashMap, path::Path, rc::Rc};

pub mod graph;

pub trait WorldViewerDataProvider {
    fn root_node(&self) -> ErasedHandle;

    fn path(&self) -> Option<&Path>;

    fn children_of(&self, node: ErasedHandle) -> Vec<ErasedHandle>;

    fn child_count_of(&self, node: ErasedHandle) -> usize;

    fn is_node_has_child(&self, node: ErasedHandle, child: ErasedHandle) -> bool;

    fn parent_of(&self, node: ErasedHandle) -> ErasedHandle;

    fn name_of(&self, node: ErasedHandle) -> Option<&str>;

    fn is_valid_handle(&self, node: ErasedHandle) -> bool;

    fn icon_of(&self, node: ErasedHandle) -> Option<SharedTexture>;

    fn is_instance(&self, node: ErasedHandle) -> bool;

    fn selection(&self) -> Vec<ErasedHandle>;

    fn on_drop(&self, child: ErasedHandle, parent: ErasedHandle);

    fn validate(&self) -> Vec<(ErasedHandle, Result<(), String>)>;

    fn on_selection_changed(&self, new_selection: &[ErasedHandle]);
}

pub trait WorldViewerItemContextMenu {
    fn menu(&self) -> RcUiNodeHandle;
}

pub struct WorldViewer {
    pub window: Handle<UiNode>,
    tree_root: Handle<UiNode>,
    sender: MessageSender,
    track_selection: Handle<UiNode>,
    search_bar: Handle<UiNode>,
    filter: String,
    stack: Vec<(Handle<UiNode>, ErasedHandle)>,
    /// Hack. Due to delayed execution of UI code we can't sync immediately after we
    /// did sync_to_model, instead we defer selection syncing to post_update() - at
    /// this moment UI is completely built and we can do syncing.
    pub sync_selection: bool,
    node_path: Handle<UiNode>,
    breadcrumbs: HashMap<Handle<UiNode>, Handle<UiNode>>,
    collapse_all: Handle<UiNode>,
    expand_all: Handle<UiNode>,
    locate_selection: Handle<UiNode>,
    scroll_view: Handle<UiNode>,
    item_context_menu: Rc<RefCell<dyn WorldViewerItemContextMenu>>,
    node_to_view_map: HashMap<ErasedHandle, Handle<UiNode>>,
}

fn make_graph_node_item(
    name: &str,
    is_instance: bool,
    icon: Option<SharedTexture>,
    handle: ErasedHandle,
    ctx: &mut BuildContext,
    context_menu: RcUiNodeHandle,
    sender: MessageSender,
    is_expanded: bool,
) -> Handle<UiNode> {
    SceneItemBuilder::new(
        TreeBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness {
                    left: 1.0,
                    top: 1.0,
                    right: 0.0,
                    bottom: 0.0,
                })
                .with_context_menu(context_menu),
        )
        .with_expanded(is_expanded),
    )
    .with_text_brush(if is_instance {
        Brush::Solid(Color::opaque(160, 160, 200))
    } else {
        Brush::Solid(fyrox::gui::COLOR_FOREGROUND)
    })
    .with_name(name.to_owned())
    .with_entity_handle(handle)
    .with_icon(icon)
    .build(ctx, sender)
}

fn tree_node(ui: &UserInterface, tree: Handle<UiNode>) -> ErasedHandle {
    ui.node(tree)
        .cast::<SceneItem>()
        .expect("Malformed scene item!")
        .entity_handle
}

fn colorize(handle: Handle<UiNode>, ui: &UserInterface, index: &mut usize) {
    let node = ui.node(handle);

    if let Some(decorator) = node.cast::<Decorator>() {
        if node.parent().is_some() {
            let new_brush = Brush::Solid(if *index % 2 == 0 {
                Color::opaque(50, 50, 50)
            } else {
                Color::opaque(60, 60, 60)
            });

            if decorator.normal_brush != new_brush {
                ui.send_message(DecoratorMessage::normal_brush(
                    handle,
                    MessageDirection::ToWidget,
                    new_brush,
                ));
            }

            *index += 1;
        }
    }

    for &item in node.children() {
        colorize(item, ui, index);
    }
}

fn fetch_expanded_state(
    node: ErasedHandle,
    data_provider: &dyn WorldViewerDataProvider,
    settings: &Settings,
) -> bool {
    data_provider
        .path()
        .as_ref()
        .and_then(|p| settings.scene_settings.get(*p))
        .and_then(|s| s.node_infos.get(&node))
        .map_or(true, |i| i.is_expanded)
}

impl WorldViewer {
    pub fn new(
        ctx: &mut BuildContext,
        sender: MessageSender,
        settings: &Settings,
        item_context_menu: Rc<RefCell<dyn WorldViewerItemContextMenu>>,
    ) -> Self {
        let tree_root;
        let node_path;
        let collapse_all;
        let expand_all;
        let locate_selection;
        let scroll_view;
        let track_selection;
        let search_bar = SearchBarBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .with_margin(Thickness::uniform(1.0)),
        )
        .build(ctx);
        let size = 15.0;
        let window = WindowBuilder::new(WidgetBuilder::new().with_name("WorldOutliner"))
            .can_minimize(false)
            .with_title(WindowTitle::text("World Viewer"))
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(
                            StackPanelBuilder::new(
                                WidgetBuilder::new()
                                    .with_margin(Thickness::uniform(1.0))
                                    .on_row(0)
                                    .with_child({
                                        collapse_all = make_image_button_with_tooltip(
                                            ctx,
                                            size,
                                            size,
                                            load_image(include_bytes!(
                                                "../../resources/collapse.png"
                                            )),
                                            "Collapse Everything",
                                        );
                                        collapse_all
                                    })
                                    .with_child({
                                        expand_all = make_image_button_with_tooltip(
                                            ctx,
                                            size,
                                            size,
                                            load_image(include_bytes!(
                                                "../../resources/expand.png"
                                            )),
                                            "Expand Everything",
                                        );
                                        expand_all
                                    })
                                    .with_child({
                                        locate_selection = make_image_button_with_tooltip(
                                            ctx,
                                            size,
                                            size,
                                            load_image(include_bytes!(
                                                "../../resources/locate.png"
                                            )),
                                            "Locate Selection",
                                        );
                                        locate_selection
                                    })
                                    .with_child({
                                        track_selection = CheckBoxBuilder::new(
                                            WidgetBuilder::new()
                                                .with_vertical_alignment(VerticalAlignment::Center)
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_content(
                                            TextBuilder::new(WidgetBuilder::new())
                                                .with_vertical_text_alignment(
                                                    VerticalAlignment::Center,
                                                )
                                                .with_text("Track Selection")
                                                .build(ctx),
                                        )
                                        .checked(Some(settings.selection.track_selection))
                                        .build(ctx);
                                        track_selection
                                    }),
                            )
                            .with_orientation(Orientation::Horizontal)
                            .build(ctx),
                        )
                        .with_child(search_bar)
                        .with_child({
                            scroll_view = ScrollViewerBuilder::new(WidgetBuilder::new().on_row(2))
                                .with_content({
                                    tree_root =
                                        TreeRootBuilder::new(WidgetBuilder::new()).build(ctx);
                                    tree_root
                                })
                                .build(ctx);
                            scroll_view
                        })
                        .with_child({
                            node_path = WrapPanelBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(3)
                                    .with_vertical_alignment(VerticalAlignment::Top),
                            )
                            .with_orientation(Orientation::Horizontal)
                            .build(ctx);
                            node_path
                        }),
                )
                .add_column(Column::stretch())
                .add_row(Row::strict(25.0))
                .add_row(Row::strict(22.0))
                .add_row(Row::stretch())
                .add_row(Row::auto())
                .build(ctx),
            )
            .build(ctx);

        Self {
            search_bar,
            track_selection,
            window,
            sender,
            tree_root,
            node_path,
            stack: Default::default(),
            sync_selection: false,
            breadcrumbs: Default::default(),
            locate_selection,
            collapse_all,
            expand_all,
            scroll_view,
            item_context_menu,
            node_to_view_map: Default::default(),
            filter: Default::default(),
        }
    }

    pub fn sync_to_model(
        &mut self,
        data_provider: &dyn WorldViewerDataProvider,
        ui: &mut UserInterface,
        settings: &Settings,
    ) {
        self.sync_graph(ui, data_provider, settings);
        self.validate(data_provider, ui);
    }

    fn build_breadcrumb(
        &mut self,
        name: &str,
        associated_item: Handle<UiNode>,
        ui: &mut UserInterface,
    ) {
        let ctx = &mut ui.build_ctx();

        let element = ButtonBuilder::new(WidgetBuilder::new().with_height(16.0))
            .with_back(
                DecoratorBuilder::new(BorderBuilder::new(
                    WidgetBuilder::new().with_foreground(BRUSH_PRIMARY),
                ))
                .with_normal_brush(BRUSH_PRIMARY)
                .with_hover_brush(BRUSH_BRIGHT_BLUE)
                .build(ctx),
            )
            .with_content(
                TextBuilder::new(WidgetBuilder::new())
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .with_text(if self.breadcrumbs.is_empty() {
                        name.to_owned()
                    } else {
                        format!("{} >", name)
                    })
                    .with_height(11.0)
                    .build(ctx),
            )
            .build(ctx);

        send_sync_message(
            ui,
            WidgetMessage::link_reverse(element, MessageDirection::ToWidget, self.node_path),
        );

        self.breadcrumbs.insert(element, associated_item);
    }

    fn clear_breadcrumbs(&mut self, ui: &UserInterface) {
        self.breadcrumbs.clear();
        for &child in ui.node(self.node_path).children() {
            send_sync_message(ui, WidgetMessage::remove(child, MessageDirection::ToWidget));
        }
    }

    fn update_breadcrumbs(
        &mut self,
        ui: &mut UserInterface,
        data_provider: &dyn WorldViewerDataProvider,
    ) {
        // Update breadcrumbs.
        self.clear_breadcrumbs(ui);

        if let Some(&first_selected) = data_provider.selection().first() {
            let mut node_handle = first_selected;
            while node_handle.is_some() && node_handle != data_provider.root_node() {
                let view = ui.find_by_criteria_down(self.tree_root, &|n| {
                    n.cast::<SceneItem>()
                        .map(|i| i.entity_handle == node_handle)
                        .unwrap_or_default()
                });
                assert!(view.is_some());
                self.build_breadcrumb(
                    &format!(
                        "{}({})",
                        data_provider.name_of(node_handle).unwrap_or_default(),
                        node_handle
                    ),
                    view,
                    ui,
                );

                node_handle = data_provider.parent_of(node_handle);
            }
        }
    }

    fn sync_graph(
        &mut self,
        ui: &mut UserInterface,
        data_provider: &dyn WorldViewerDataProvider,
        settings: &Settings,
    ) {
        // Sync tree structure with graph structure.
        self.stack.clear();
        self.stack.push((self.tree_root, data_provider.root_node()));
        while let Some((tree_handle, node_handle)) = self.stack.pop() {
            let ui_node = ui.node(tree_handle);

            if let Some(item) = ui_node.cast::<SceneItem>() {
                let child_count = data_provider.child_count_of(node_handle);
                let items = item.tree.items.clone();

                match child_count.cmp(&items.len()) {
                    Ordering::Less => {
                        for &item in items.iter() {
                            let child_node = tree_node(ui, item);
                            if !data_provider.is_node_has_child(node_handle, child_node) {
                                send_sync_message(
                                    ui,
                                    TreeMessage::remove_item(
                                        tree_handle,
                                        MessageDirection::ToWidget,
                                        item,
                                    ),
                                );
                                if let Some(existing_view) = self.node_to_view_map.get(&child_node)
                                {
                                    if *existing_view == item {
                                        self.node_to_view_map.remove(&child_node);
                                    }
                                }
                            } else {
                                self.stack.push((item, child_node));
                            }
                        }
                    }
                    Ordering::Equal => {
                        for &tree in items.iter() {
                            let child = tree_node(ui, tree);
                            self.stack.push((tree, child));
                        }
                    }
                    Ordering::Greater => {
                        for child_handle in data_provider.children_of(node_handle) {
                            let mut found = false;
                            for &item in items.iter() {
                                let tree_node_handle = tree_node(ui, item);
                                if tree_node_handle == child_handle {
                                    self.stack.push((item, child_handle));
                                    found = true;
                                    break;
                                }
                            }
                            if !found {
                                let graph_node_item = make_graph_node_item(
                                    data_provider.name_of(child_handle).unwrap_or(""),
                                    data_provider.is_instance(child_handle),
                                    data_provider.icon_of(child_handle),
                                    child_handle,
                                    &mut ui.build_ctx(),
                                    self.item_context_menu.borrow().menu(),
                                    self.sender.clone(),
                                    fetch_expanded_state(child_handle, data_provider, settings),
                                );
                                send_sync_message(
                                    ui,
                                    TreeMessage::add_item(
                                        tree_handle,
                                        MessageDirection::ToWidget,
                                        graph_node_item,
                                    ),
                                );
                                self.node_to_view_map.insert(child_handle, graph_node_item);
                                self.stack.push((graph_node_item, child_handle));
                            }
                        }
                    }
                }
            } else if let Some(tree_root) = ui_node.cast::<TreeRoot>() {
                if tree_root.items.is_empty()
                    || tree_node(ui, tree_root.items[0]) != data_provider.root_node()
                {
                    let new_root_item = make_graph_node_item(
                        data_provider.name_of(node_handle).unwrap_or(""),
                        data_provider.is_instance(node_handle),
                        data_provider.icon_of(node_handle),
                        node_handle,
                        &mut ui.build_ctx(),
                        self.item_context_menu.borrow().menu(),
                        self.sender.clone(),
                        fetch_expanded_state(node_handle, data_provider, settings),
                    );
                    send_sync_message(
                        ui,
                        TreeRootMessage::items(
                            tree_handle,
                            MessageDirection::ToWidget,
                            vec![new_root_item],
                        ),
                    );
                    self.node_to_view_map.insert(node_handle, new_root_item);
                    self.stack.push((new_root_item, node_handle));
                } else {
                    self.stack.push((tree_root.items[0], node_handle));
                }
            }
        }

        // Sync items data.
        let mut stack = vec![self.tree_root];
        while let Some(handle) = stack.pop() {
            let ui_node = ui.node(handle);

            if let Some(item) = ui_node.cast::<SceneItem>() {
                if let Some(name) = data_provider.name_of(item.entity_handle) {
                    if item.name() != name {
                        send_sync_message(
                            ui,
                            SceneItemMessage::name(
                                handle,
                                MessageDirection::ToWidget,
                                name.to_owned(),
                            ),
                        );
                    }

                    stack.extend_from_slice(&item.tree.items);
                }
            } else if let Some(root) = ui_node.cast::<TreeRoot>() {
                stack.extend_from_slice(&root.items)
            }
        }

        self.colorize(ui);

        self.node_to_view_map
            .retain(|k, v| data_provider.is_valid_handle(*k) && ui.try_get_node(*v).is_some());
    }

    pub fn colorize(&mut self, ui: &UserInterface) {
        let mut index = 0;
        colorize(self.tree_root, ui, &mut index);
    }

    fn apply_filter(&self, data_provider: &dyn WorldViewerDataProvider, ui: &UserInterface) {
        fn apply_filter_recursive(node: Handle<UiNode>, filter: &str, ui: &UserInterface) -> bool {
            let node_ref = ui.node(node);

            let mut is_any_match = false;
            for &child in node_ref.children() {
                is_any_match |= apply_filter_recursive(child, filter, ui)
            }

            let name = node_ref.cast::<SceneItem>().map(|i| i.name());

            if let Some(name) = name {
                is_any_match |= name.to_lowercase().contains(filter)
                    || fuzzy_compare(filter, name.to_lowercase().as_str()) >= 0.33;

                ui.send_message(WidgetMessage::visibility(
                    node,
                    MessageDirection::ToWidget,
                    is_any_match,
                ));
            }

            is_any_match
        }

        apply_filter_recursive(self.tree_root, &self.filter.to_lowercase(), ui);

        if self.filter.is_empty() {
            if let Some(first) = data_provider.selection().first() {
                if let Some(view) = self.node_to_view_map.get(first) {
                    ui.send_message(ScrollViewerMessage::bring_into_view(
                        self.scroll_view,
                        MessageDirection::ToWidget,
                        *view,
                    ));
                }
            }
        }
    }

    pub fn set_filter(
        &mut self,
        filter: String,
        data_provider: &dyn WorldViewerDataProvider,
        ui: &UserInterface,
    ) {
        self.filter = filter;
        self.apply_filter(data_provider, ui)
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        data_provider: &dyn WorldViewerDataProvider,
        engine: &Engine,
        settings: &mut Settings,
    ) {
        scope_profile!();

        if let Some(TreeRootMessage::Selected(selection)) = message.data::<TreeRootMessage>() {
            if message.destination() == self.tree_root
                && message.direction() == MessageDirection::FromWidget
            {
                self.handle_selection(selection, data_provider, engine);
            }
        } else if let Some(&WidgetMessage::Drop(node)) = message.data::<WidgetMessage>() {
            self.handle_drop(engine, data_provider, message.destination(), node);
        } else if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if let Some(&view) = self.breadcrumbs.get(&message.destination()) {
                if let Some(graph_node) = engine
                    .user_interface
                    .try_get_node(view)
                    .and_then(|n| n.cast::<SceneItem>())
                {
                    data_provider.on_selection_changed(&[graph_node.entity_handle]);
                }
            } else if message.destination() == self.collapse_all {
                engine
                    .user_interface
                    .send_message(TreeRootMessage::collapse_all(
                        self.tree_root,
                        MessageDirection::ToWidget,
                    ));
            } else if message.destination() == self.expand_all {
                engine
                    .user_interface
                    .send_message(TreeRootMessage::expand_all(
                        self.tree_root,
                        MessageDirection::ToWidget,
                    ));
            } else if message.destination() == self.locate_selection {
                self.locate_selection(&data_provider.selection(), &engine.user_interface)
            }
        } else if let Some(CheckBoxMessage::Check(Some(value))) = message.data::<CheckBoxMessage>()
        {
            if message.destination() == self.track_selection {
                settings.selection.track_selection = *value;
                if *value {
                    self.locate_selection(&data_provider.selection(), &engine.user_interface);
                }
            }
        } else if let Some(SearchBarMessage::Text(text)) = message.data() {
            if message.destination() == self.search_bar
                && message.direction == MessageDirection::FromWidget
            {
                self.set_filter(text.clone(), data_provider, &engine.user_interface);
            }
        } else if let Some(TreeMessage::Expand { expand, .. }) = message.data() {
            if let Some(scene_view_item) = engine
                .user_interface
                .node(message.destination())
                .query_component::<SceneItem>()
            {
                if let Some(path) = data_provider.path() {
                    settings
                        .scene_settings
                        .entry(path.to_owned())
                        .or_default()
                        .node_infos
                        .entry(scene_view_item.entity_handle)
                        .or_default()
                        .is_expanded = *expand;
                }
            }
        }
    }

    pub fn try_locate_object(&self, handle: ErasedHandle, ui: &UserInterface) {
        self.locate_selection(&[handle], ui)
    }

    fn locate_selection(&self, selection: &[ErasedHandle], ui: &UserInterface) {
        let tree_to_focus = self.map_selection(selection, ui);

        if let Some(tree_to_focus) = tree_to_focus.first() {
            ui.send_message(TreeMessage::expand(
                *tree_to_focus,
                MessageDirection::ToWidget,
                true,
                TreeExpansionStrategy::RecursiveAncestors,
            ));

            ui.send_message(ScrollViewerMessage::bring_into_view(
                self.scroll_view,
                MessageDirection::ToWidget,
                *tree_to_focus,
            ));
        }
    }

    fn handle_selection(
        &self,
        selection: &[Handle<UiNode>],
        data_provider: &dyn WorldViewerDataProvider,
        engine: &Engine,
    ) {
        data_provider.on_selection_changed(
            &selection
                .iter()
                .map(|selected_item| {
                    engine
                        .user_interface
                        .node(*selected_item)
                        .cast::<SceneItem>()
                        .unwrap()
                        .entity_handle
                })
                .collect::<Vec<_>>(),
        );
    }

    /// `target` - is a node at which `dropped` was dropped.
    /// `dropped` - is a node which was dropped at `target`.
    fn handle_drop(
        &self,
        engine: &Engine,
        data_provider: &dyn WorldViewerDataProvider,
        target: Handle<UiNode>,
        dropped: Handle<UiNode>,
    ) {
        let ui = &engine.user_interface;

        if ui.is_node_child_of(dropped, self.tree_root)
            && ui.is_node_child_of(target, self.tree_root)
            && dropped != target
        {
            if let (Some(child), Some(parent)) = (
                ui.node(dropped).cast::<SceneItem>(),
                ui.node(target).cast::<SceneItem>(),
            ) {
                data_provider.on_drop(child.entity_handle, parent.entity_handle)
            }
        }
    }

    fn map_selection(&self, selection: &[ErasedHandle], ui: &UserInterface) -> Vec<Handle<UiNode>> {
        map_selection(selection, self.tree_root, ui)
    }

    pub fn post_update(
        &mut self,
        data_provider: &dyn WorldViewerDataProvider,
        ui: &mut UserInterface,
        settings: &Settings,
    ) {
        // Hack. See `self.sync_selection` for details.
        if self.sync_selection {
            let trees = self.map_selection(&data_provider.selection(), ui);

            send_sync_message(
                ui,
                TreeRootMessage::select(self.tree_root, MessageDirection::ToWidget, trees),
            );

            self.update_breadcrumbs(ui, data_provider);
            if settings.selection.track_selection {
                self.locate_selection(&data_provider.selection(), ui);
            }

            self.sync_selection = false;
        }
    }

    pub fn clear(&mut self, ui: &UserInterface) {
        self.node_to_view_map.clear();
        self.clear_breadcrumbs(ui);
        ui.send_message(TreeRootMessage::items(
            self.tree_root,
            MessageDirection::ToWidget,
            vec![],
        ));
    }

    pub fn on_configure(&self, ui: &UserInterface, settings: &Settings) {
        ui.send_message(CheckBoxMessage::checked(
            self.track_selection,
            MessageDirection::ToWidget,
            Some(settings.selection.track_selection),
        ));
    }

    pub fn on_mode_changed(&mut self, ui: &UserInterface, mode: &Mode) {
        ui.send_message(WidgetMessage::enabled(
            window_content(self.window, ui),
            MessageDirection::ToWidget,
            mode.is_edit(),
        ));
    }

    pub fn validate(&self, data_provider: &dyn WorldViewerDataProvider, ui: &UserInterface) {
        for (node_handle, result) in data_provider.validate() {
            if let Some(view) = self.node_to_view_map.get(&node_handle) {
                let view_ref = ui.node(*view).query_component::<SceneItem>().unwrap();

                if view_ref.warning_icon.is_none() && result.is_err()
                    || view_ref.warning_icon.is_some() && result.is_ok()
                {
                    send_sync_message(
                        ui,
                        SceneItemMessage::validate(*view, MessageDirection::ToWidget, result),
                    );
                }
            }
        }
    }
}

fn map_selection(
    selection: &[ErasedHandle],
    root_node: Handle<UiNode>,
    ui: &UserInterface,
) -> Vec<Handle<UiNode>> {
    selection
        .iter()
        .filter_map(|&handle| {
            let item = ui.find_by_criteria_down(root_node, &|n| {
                n.cast::<SceneItem>()
                    .map(|n| n.entity_handle == handle)
                    .unwrap_or_default()
            });
            if item.is_some() {
                Some(item)
            } else {
                None
            }
        })
        .collect()
}
