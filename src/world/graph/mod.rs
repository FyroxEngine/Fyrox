use crate::{
    gui::SceneItemMessage,
    load_image,
    physics::{Joint, RigidBody},
    scene::{
        commands::{graph::LinkNodesCommand, ChangeSelectionCommand},
        EditorScene, GraphSelection, Selection,
    },
    send_sync_message,
    world::{
        graph::{
            item::{GraphNodeItem, SceneItemBuilder},
            menu::ItemContextMenu,
        },
        physics::{
            fetch_name,
            item::{PhysicsItem, PhysicsItemBuilder, PhysicsItemMessage},
            selection::{JointSelection, RigidBodySelection},
        },
        sound::{SoundItem, SoundItemBuilder, SoundSelection},
    },
    GameEngine, Message,
};
use rg3d::{
    core::{
        color::Color,
        pool::{Handle, Pool},
        scope_profile,
    },
    engine::resource_manager::ResourceManager,
    gui::{
        brush::Brush,
        button::ButtonBuilder,
        grid::{Column, GridBuilder, Row},
        message::{
            ButtonMessage, MessageDirection, ScrollViewerMessage, TreeExpansionStrategy,
            TreeMessage, TreeRootMessage, UiMessage, UiMessageData, WidgetMessage,
        },
        scroll_viewer::ScrollViewerBuilder,
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        tree::{Tree, TreeBuilder, TreeRoot, TreeRootBuilder},
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        BuildContext, Control, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
        VerticalAlignment,
    },
    scene::{graph::Graph, node::Node},
    sound::context::SoundContext,
};
use std::{cmp::Ordering, collections::HashMap, marker::PhantomData, sync::mpsc::Sender};

pub mod item;
pub mod menu;

pub struct WorldViewer {
    pub window: Handle<UiNode>,
    tree_root: Handle<UiNode>,
    graph_folder: Handle<UiNode>,
    rigid_bodies_folder: Handle<UiNode>,
    joints_folder: Handle<UiNode>,
    sounds_folder: Handle<UiNode>,
    sender: Sender<Message>,
    stack: Vec<(Handle<UiNode>, Handle<Node>)>,
    /// Hack. Due to delayed execution of UI code we can't sync immediately after we
    /// did sync_to_model, instead we defer selection syncing to post_update() - at
    /// this moment UI is completely built and we can do syncing.
    pub sync_selection: bool,
    node_path: Handle<UiNode>,
    breadcrumbs: HashMap<Handle<UiNode>, Handle<Node>>,
    collapse_all: Handle<UiNode>,
    expand_all: Handle<UiNode>,
    locate_selection: Handle<UiNode>,
    scroll_view: Handle<UiNode>,
    item_context_menu: ItemContextMenu,
}

fn make_tree(
    node: &Node,
    handle: Handle<Node>,
    ctx: &mut BuildContext,
    sender: Sender<Message>,
    resource_manager: ResourceManager,
    context_menu: Handle<UiNode>,
) -> Handle<UiNode> {
    let icon = match node {
        Node::Light(_) => load_image(include_bytes!("../../../resources/embed/light.png")),
        _ => load_image(include_bytes!("../../../resources/embed/cube.png")),
    };

    SceneItemBuilder::new()
        .with_name(node.name().to_owned())
        .with_node(handle)
        .with_visibility(node.visibility())
        .with_icon(icon)
        .with_context_menu(context_menu)
        .build(ctx, sender, resource_manager, node)
}

fn tree_node(ui: &UserInterface, tree: Handle<UiNode>) -> Handle<Node> {
    if let Some(item) = ui.node(tree).cast::<GraphNodeItem>() {
        return item.node;
    }
    unreachable!()
}

fn colorize(tree: Handle<UiNode>, ui: &UserInterface, index: &mut usize) {
    let node = ui.node(tree);

    if let Some(i) = node.cast::<GraphNodeItem>() {
        ui.send_message(UiMessage::user(
            tree,
            MessageDirection::ToWidget,
            Box::new(SceneItemMessage::Order(*index % 2 == 0)),
        ));

        *index += 1;

        for &item in i.tree.items() {
            colorize(item, ui, index);
        }
    } else if let Some(root) = node.cast::<TreeRoot>() {
        for &item in root.items() {
            colorize(item, ui, index);
        }
    }
}

fn make_folder(ctx: &mut BuildContext, name: &str) -> Handle<UiNode> {
    TreeBuilder::new(WidgetBuilder::new())
        .with_content(
            TextBuilder::new(
                WidgetBuilder::new()
                    .with_margin(Thickness::left(5.0))
                    .with_foreground(Brush::Solid(Color::opaque(153, 217, 234))),
            )
            .with_vertical_text_alignment(VerticalAlignment::Center)
            .with_text(name)
            .build(ctx),
        )
        .build(ctx)
}

pub fn sync_pool<T, N, M, F>(
    folder: Handle<UiNode>,
    pool: &Pool<T>,
    ui: &mut UserInterface,
    selection: Option<&[Handle<T>]>,
    mut make_view: M,
    mut make_name: N,
    mut fetch_entity: F,
) -> Vec<Handle<UiNode>>
where
    T: 'static,
    N: FnMut(Handle<T>) -> String,
    M: FnMut(&mut UserInterface, Handle<T>, &T) -> Handle<UiNode>,
    F: FnMut(Handle<UiNode>, &UserInterface) -> Handle<T>,
{
    let folder_items = ui.node(folder).cast::<Tree>().unwrap().items().to_vec();

    match pool.alive_count().cmp(&folder_items.len()) {
        Ordering::Less => {
            // An entity was removed.
            for &item in folder_items.iter() {
                let associated_source = (fetch_entity)(item, ui);

                if pool.pair_iter().all(|(h, _)| h != associated_source) {
                    send_sync_message(
                        ui,
                        TreeMessage::remove_item(folder, MessageDirection::ToWidget, item),
                    );
                }
            }
        }
        Ordering::Greater => {
            // An entity was added.
            for (handle, elem) in pool.pair_iter() {
                if folder_items
                    .iter()
                    .all(|i| (fetch_entity)(*i, ui) != handle)
                {
                    let item = (make_view)(ui, handle, elem);
                    send_sync_message(
                        ui,
                        TreeMessage::add_item(folder, MessageDirection::ToWidget, item),
                    );
                }
            }
        }
        _ => (),
    }

    let mut selected_items = Vec::new();

    // Sync selection.
    if let Some(selection) = selection {
        for selected in selection {
            if let Some(associated_item) = ui
                .node(folder)
                .cast::<Tree>()
                .unwrap()
                .items()
                .iter()
                .find(|i| (fetch_entity)(**i, ui) == *selected)
            {
                selected_items.push(*associated_item)
            }
        }
    }

    // Sync names. Since rigid body cannot have a name, we just take the name of an associated
    // scene node (if any), or a placeholder "Rigid Body" if there is no associated scene node.
    for item in ui.node(folder).cast::<Tree>().unwrap().items() {
        let rigid_body = (fetch_entity)(*item, ui);
        ui.send_message(UiMessage::user(
            *item,
            MessageDirection::ToWidget,
            Box::new(PhysicsItemMessage::Name((make_name)(rigid_body))),
        ));
    }

    selected_items
}

impl WorldViewer {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let tree_root;
        let node_path;
        let collapse_all;
        let expand_all;
        let locate_selection;
        let scroll_view;
        let graph_folder = make_folder(ctx, "Scene Graph");
        let rigid_bodies_folder = make_folder(ctx, "Rigid Bodies");
        let joints_folder = make_folder(ctx, "Joints");
        let sounds_folder = make_folder(ctx, "Sounds");
        let window = WindowBuilder::new(WidgetBuilder::new())
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
                                        collapse_all = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_text("Collapse All")
                                        .build(ctx);
                                        collapse_all
                                    })
                                    .with_child({
                                        expand_all = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_text("Expand All")
                                        .build(ctx);
                                        expand_all
                                    })
                                    .with_child({
                                        locate_selection = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_text("Locate Selection")
                                        .build(ctx);
                                        locate_selection
                                    }),
                            )
                            .with_orientation(Orientation::Horizontal)
                            .build(ctx),
                        )
                        .with_child(
                            TextBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(1)
                                    .on_column(0)
                                    .with_opacity(0.4),
                            )
                            .with_text("Breadcrumbs")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .with_horizontal_text_alignment(HorizontalAlignment::Center)
                            .build(ctx),
                        )
                        .with_child(
                            ScrollViewerBuilder::new(WidgetBuilder::new().on_row(1))
                                .with_content({
                                    node_path = StackPanelBuilder::new(WidgetBuilder::new())
                                        .with_orientation(Orientation::Horizontal)
                                        .build(ctx);
                                    node_path
                                })
                                .build(ctx),
                        )
                        .with_child({
                            scroll_view = ScrollViewerBuilder::new(WidgetBuilder::new().on_row(2))
                                .with_content({
                                    tree_root = TreeRootBuilder::new(WidgetBuilder::new())
                                        .with_items(vec![
                                            graph_folder,
                                            rigid_bodies_folder,
                                            joints_folder,
                                            sounds_folder,
                                        ])
                                        .build(ctx);
                                    tree_root
                                })
                                .build(ctx);
                            scroll_view
                        }),
                )
                .add_column(Column::stretch())
                .add_row(Row::strict(24.0))
                .add_row(Row::strict(24.0))
                .add_row(Row::stretch())
                .build(ctx),
            )
            .build(ctx);

        let item_context_menu = ItemContextMenu::new(ctx);

        Self {
            window,
            sender,
            tree_root,
            graph_folder,
            rigid_bodies_folder,
            joints_folder,
            node_path,
            stack: Default::default(),
            sync_selection: false,
            breadcrumbs: Default::default(),
            locate_selection,
            collapse_all,
            expand_all,
            scroll_view,
            item_context_menu,
            sounds_folder,
        }
    }

    pub fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        scope_profile!();

        let scene = &mut engine.scenes[editor_scene.scene];
        let graph = &mut scene.graph;
        let ui = &mut engine.user_interface;

        let mut selected_items = Vec::new();

        selected_items.extend(self.sync_graph(
            ui,
            editor_scene,
            graph,
            engine.resource_manager.clone(),
        ));
        selected_items.extend(self.sync_rigid_bodies(ui, editor_scene, graph));
        selected_items.extend(self.sync_joints(ui, editor_scene));
        selected_items.extend(self.sync_sounds(ui, editor_scene, scene.sound_context.clone()));

        if !selected_items.is_empty() {
            send_sync_message(
                ui,
                TreeRootMessage::select(self.tree_root, MessageDirection::ToWidget, selected_items),
            );
        }

        self.update_breadcrumbs(ui, editor_scene, graph);
    }

    fn update_breadcrumbs(
        &mut self,
        ui: &mut UserInterface,
        editor_scene: &EditorScene,
        graph: &Graph,
    ) {
        // Update breadcrumbs.
        self.breadcrumbs.clear();
        for &child in ui.node(self.node_path).children() {
            send_sync_message(ui, WidgetMessage::remove(child, MessageDirection::ToWidget));
        }
        if let Selection::Graph(selection) = &editor_scene.selection {
            if let Some(&first_selected) = selection.nodes().first() {
                let mut item = first_selected;
                while item.is_some() {
                    let node = &graph[item];

                    let element = ButtonBuilder::new(
                        WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                    )
                    .with_text(node.name())
                    .build(&mut ui.build_ctx());

                    send_sync_message(
                        ui,
                        WidgetMessage::link_reverse(
                            element,
                            MessageDirection::ToWidget,
                            self.node_path,
                        ),
                    );

                    self.breadcrumbs.insert(element, item);

                    item = node.parent();
                }
            }
        }
    }

    fn sync_sounds(
        &mut self,
        ui: &mut UserInterface,
        editor_scene: &EditorScene,
        ctx: SoundContext,
    ) -> Vec<Handle<UiNode>> {
        let ctx = ctx.state();

        sync_pool(
            self.sounds_folder,
            ctx.sources(),
            ui,
            if let Selection::Sound(ref s) = editor_scene.selection {
                Some(&s.sources)
            } else {
                None
            },
            |ui, handle, _| {
                SoundItemBuilder::new(TreeBuilder::new(WidgetBuilder::new()))
                    .with_name(ctx.source(handle).name_owned())
                    .with_sound_source(handle)
                    .build(&mut ui.build_ctx())
            },
            |s| ctx.source(s).name_owned(),
            |s, ui| ui.node(s).cast::<SoundItem>().unwrap().sound_source,
        )
    }

    fn sync_joints(
        &mut self,
        ui: &mut UserInterface,
        editor_scene: &EditorScene,
    ) -> Vec<Handle<UiNode>> {
        sync_pool(
            self.joints_folder,
            &editor_scene.physics.joints,
            ui,
            if let Selection::Joint(ref s) = editor_scene.selection {
                Some(&s.joints)
            } else {
                None
            },
            |ui, handle, _| {
                PhysicsItemBuilder::<Joint>::new(TreeBuilder::new(WidgetBuilder::new()))
                    .with_name("Joint".to_owned())
                    .with_physics_entity(handle)
                    .build(&mut ui.build_ctx())
            },
            |_| "Joint".to_owned(),
            |s, ui| {
                ui.node(s)
                    .cast::<PhysicsItem<Joint>>()
                    .unwrap()
                    .physics_entity
            },
        )
    }

    fn sync_rigid_bodies(
        &mut self,
        ui: &mut UserInterface,
        editor_scene: &EditorScene,
        graph: &Graph,
    ) -> Vec<Handle<UiNode>> {
        sync_pool(
            self.rigid_bodies_folder,
            &editor_scene.physics.bodies,
            ui,
            if let Selection::RigidBody(ref s) = editor_scene.selection {
                Some(&s.bodies)
            } else {
                None
            },
            |ui, handle, _| {
                PhysicsItemBuilder::<RigidBody>::new(TreeBuilder::new(WidgetBuilder::new()))
                    .with_name(fetch_name(handle, editor_scene, graph))
                    .with_physics_entity(handle)
                    .build(&mut ui.build_ctx())
            },
            |b| fetch_name(b, editor_scene, graph),
            |s, ui| {
                ui.node(s)
                    .cast::<PhysicsItem<RigidBody>>()
                    .unwrap()
                    .physics_entity
            },
        )
    }

    fn sync_graph(
        &mut self,
        ui: &mut UserInterface,
        editor_scene: &EditorScene,
        graph: &Graph,
        resource_manager: ResourceManager,
    ) -> Vec<Handle<UiNode>> {
        let mut selected_items = Vec::new();

        // Sync tree structure with graph structure.
        self.stack.clear();
        self.stack.push((self.graph_folder, graph.get_root()));
        while let Some((tree_handle, node_handle)) = self.stack.pop() {
            // Hide all editor nodes.
            if node_handle == editor_scene.root {
                continue;
            }
            let node = &graph[node_handle];
            let ui_node = ui.node(tree_handle);

            if let Some(item) = ui_node.cast::<GraphNodeItem>() {
                // Since we are filtering out editor stuff from world outliner, we must
                // correctly count children, excluding editor nodes.
                let mut child_count = 0;
                for &child in node.children() {
                    if child != editor_scene.root {
                        child_count += 1;
                    }
                }
                let items = item.tree.items().to_vec();
                if child_count < items.len() {
                    for &item in items.iter() {
                        let child_node = tree_node(ui, item);
                        if !node.children().contains(&child_node) {
                            send_sync_message(
                                ui,
                                TreeMessage::remove_item(
                                    tree_handle,
                                    MessageDirection::ToWidget,
                                    item,
                                ),
                            );
                        } else {
                            self.stack.push((item, child_node));
                        }
                    }
                } else if child_count > item.tree.items().len() {
                    for &child_handle in node.children() {
                        // Hide all editor nodes.
                        if child_handle == editor_scene.root {
                            continue;
                        }
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
                            let tree = make_tree(
                                &graph[child_handle],
                                child_handle,
                                &mut ui.build_ctx(),
                                self.sender.clone(),
                                resource_manager.clone(),
                                self.item_context_menu.menu,
                            );
                            send_sync_message(
                                ui,
                                TreeMessage::add_item(
                                    tree_handle,
                                    MessageDirection::ToWidget,
                                    tree,
                                ),
                            );
                            if let Selection::Graph(selection) = &editor_scene.selection {
                                if selection.contains(child_handle) {
                                    selected_items.push(tree);
                                }
                            }
                            self.stack.push((tree, child_handle));
                        }
                    }
                } else {
                    for &tree in items.iter() {
                        let child = tree_node(ui, tree);
                        self.stack.push((tree, child));
                    }
                }
            } else if let Some(folder) = ui_node.cast::<Tree>() {
                if folder.items().is_empty() {
                    let tree = make_tree(
                        node,
                        node_handle,
                        &mut ui.build_ctx(),
                        self.sender.clone(),
                        resource_manager.clone(),
                        self.item_context_menu.menu,
                    );
                    send_sync_message(
                        ui,
                        TreeMessage::add_item(tree_handle, MessageDirection::ToWidget, tree),
                    );
                    self.stack.push((tree, node_handle));
                } else {
                    self.stack.push((folder.items()[0], node_handle));
                }
            }
        }

        // Sync items data.
        let mut stack = vec![self.tree_root];
        while let Some(handle) = stack.pop() {
            let ui_node = ui.node(handle);

            if let Some(item) = ui_node.cast::<GraphNodeItem>() {
                if graph.is_valid_handle(item.node) {
                    let node = &graph[item.node];
                    send_sync_message(
                        ui,
                        SceneItemMessage::node_visibility(handle, node.visibility()),
                    );
                    send_sync_message(ui, SceneItemMessage::name(handle, node.name().to_owned()));
                    stack.extend_from_slice(item.tree.items());
                }
            } else if let Some(root) = ui_node.cast::<TreeRoot>() {
                stack.extend_from_slice(root.items())
            }
        }

        self.colorize(ui);

        selected_items
    }

    pub fn colorize(&mut self, ui: &UserInterface) {
        let mut index = 0;
        colorize(self.tree_root, ui, &mut index);
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_scene: &mut EditorScene,
        engine: &GameEngine,
    ) {
        scope_profile!();

        self.item_context_menu
            .handle_ui_message(message, editor_scene, engine, &self.sender);

        match message.data() {
            UiMessageData::TreeRoot(msg) => {
                if message.destination() == self.tree_root
                    && message.direction() == MessageDirection::FromWidget
                {
                    if let TreeRootMessage::Selected(selection) = msg {
                        let mut new_selection = Selection::None;
                        for selected_item in selection {
                            let selected_item_ref = engine.user_interface.node(*selected_item);

                            if let Some(graph_node) = selected_item_ref.cast::<GraphNodeItem>() {
                                match new_selection {
                                    Selection::None => {
                                        new_selection = Selection::Graph(
                                            GraphSelection::single_or_empty(graph_node.node),
                                        );
                                    }
                                    Selection::Graph(ref mut selection) => {
                                        selection.insert_or_exclude(graph_node.node)
                                    }
                                    _ => (),
                                }
                            } else if let Some(rigid_body) =
                                selected_item_ref.cast::<PhysicsItem<RigidBody>>()
                            {
                                match new_selection {
                                    Selection::None => {
                                        new_selection = Selection::RigidBody(RigidBodySelection {
                                            bodies: vec![rigid_body.physics_entity],
                                        });
                                    }
                                    Selection::RigidBody(ref mut selection) => {
                                        selection.bodies.push(rigid_body.physics_entity)
                                    }
                                    _ => (),
                                }
                            } else if let Some(joint) =
                                selected_item_ref.cast::<PhysicsItem<Joint>>()
                            {
                                match new_selection {
                                    Selection::None => {
                                        new_selection = Selection::Joint(JointSelection {
                                            joints: vec![joint.physics_entity],
                                        });
                                    }
                                    Selection::Joint(ref mut selection) => {
                                        selection.joints.push(joint.physics_entity)
                                    }
                                    _ => (),
                                }
                            } else if let Some(sound) = selected_item_ref.cast::<SoundItem>() {
                                match new_selection {
                                    Selection::None => {
                                        new_selection = Selection::Sound(SoundSelection {
                                            sources: vec![sound.sound_source],
                                        });
                                    }
                                    Selection::Sound(ref mut selection) => {
                                        selection.sources.push(sound.sound_source)
                                    }
                                    _ => (),
                                }
                            }
                        }

                        if new_selection != editor_scene.selection {
                            self.sender
                                .send(Message::do_scene_command(ChangeSelectionCommand::new(
                                    new_selection,
                                    editor_scene.selection.clone(),
                                )))
                                .unwrap();
                        }
                    }
                }
            }
            &UiMessageData::Widget(WidgetMessage::Drop(node)) => {
                if engine.user_interface.is_node_child_of(node, self.tree_root)
                    && engine
                        .user_interface
                        .is_node_child_of(message.destination(), self.tree_root)
                    && node != message.destination()
                {
                    if let (Some(child), Some(parent)) = (
                        engine.user_interface.node(node).cast::<GraphNodeItem>(),
                        engine
                            .user_interface
                            .node(message.destination())
                            .cast::<GraphNodeItem>(),
                    ) {
                        // Make sure we won't create any loops - child must not have parent in its
                        // descendants.
                        let mut attach = true;
                        let graph = &engine.scenes[editor_scene.scene].graph;
                        let mut p = parent.node;
                        while p.is_some() {
                            if p == child.node {
                                attach = false;
                                break;
                            }
                            p = graph[p].parent();
                        }

                        if attach {
                            self.sender
                                .send(Message::do_scene_command(LinkNodesCommand::new(
                                    child.node,
                                    parent.node,
                                )))
                                .unwrap();
                        }
                    }
                }
            }
            UiMessageData::Button(ButtonMessage::Click) => {
                if let Some(&node) = self.breadcrumbs.get(&message.destination()) {
                    self.sender
                        .send(Message::do_scene_command(ChangeSelectionCommand::new(
                            Selection::Graph(GraphSelection::single_or_empty(node)),
                            editor_scene.selection.clone(),
                        )))
                        .unwrap();
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
                    let tree_to_focus = self.map_selection(editor_scene, engine);

                    if let Some(tree_to_focus) = tree_to_focus.first() {
                        engine.user_interface.send_message(TreeMessage::expand(
                            *tree_to_focus,
                            MessageDirection::ToWidget,
                            true,
                            TreeExpansionStrategy::RecursiveAncestors,
                        ));

                        engine
                            .user_interface
                            .send_message(ScrollViewerMessage::bring_into_view(
                                self.scroll_view,
                                MessageDirection::ToWidget,
                                *tree_to_focus,
                            ));
                    }
                }
            }
            _ => {}
        }
    }

    fn map_selection(
        &self,
        editor_scene: &EditorScene,
        engine: &GameEngine,
    ) -> Vec<Handle<UiNode>> {
        match &editor_scene.selection {
            Selection::Graph(selection) => map_selection(
                selection.nodes(),
                self.graph_folder,
                &engine.user_interface,
                |n, handle| n.node == handle,
                PhantomData::<GraphNodeItem>,
            ),
            Selection::Sound(selection) => map_selection(
                selection.sources(),
                self.sounds_folder,
                &engine.user_interface,
                |n, handle| n.sound_source == handle,
                PhantomData::<SoundItem>,
            ),
            Selection::RigidBody(selection) => map_selection(
                selection.bodies(),
                self.rigid_bodies_folder,
                &engine.user_interface,
                |n, handle| n.physics_entity == handle,
                PhantomData::<PhysicsItem<RigidBody>>,
            ),
            Selection::Joint(selection) => map_selection(
                selection.joints(),
                self.joints_folder,
                &engine.user_interface,
                |n, handle| n.physics_entity == handle,
                PhantomData::<PhysicsItem<Joint>>,
            ),
            Selection::None | Selection::Navmesh(_) => Default::default(),
        }
    }

    pub fn post_update(&mut self, editor_scene: &EditorScene, engine: &GameEngine) {
        // Hack. See `self.sync_selection` for details.
        if self.sync_selection {
            let ui = &engine.user_interface;

            let trees = self.map_selection(editor_scene, engine);

            send_sync_message(
                ui,
                TreeRootMessage::select(self.tree_root, MessageDirection::ToWidget, trees),
            );

            self.sync_selection = false;
        }
    }

    pub fn clear(&mut self, ui: &mut UserInterface) {
        for folder in [
            self.graph_folder,
            self.rigid_bodies_folder,
            self.joints_folder,
            self.sounds_folder,
        ] {
            ui.send_message(TreeMessage::set_items(
                folder,
                MessageDirection::ToWidget,
                vec![],
            ));
        }
    }
}

fn map_selection<T, V, C>(
    selection: &[Handle<T>],
    folder: Handle<UiNode>,
    ui: &UserInterface,
    cmp: C,
    _phantom: PhantomData<V>,
) -> Vec<Handle<UiNode>>
where
    C: Fn(&V, Handle<T>) -> bool,
    V: Control,
{
    selection
        .iter()
        .filter_map(|&handle| {
            let item = ui.find_by_criteria_down(folder, &|n| {
                n.cast::<V>().map(|n| (cmp)(n, handle)).unwrap_or_default()
            });
            if item.is_some() {
                Some(item)
            } else {
                None
            }
        })
        .collect()
}
