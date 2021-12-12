use crate::{
    load_image,
    physics::{Collider, Joint, RigidBody},
    scene::{
        commands::{
            graph::LinkNodesCommand,
            physics::{
                LinkBodyCommand, SetJointBody1Command, SetJointBody2Command, UnlinkBodyCommand,
            },
            ChangeSelectionCommand, CommandGroup, SceneCommand,
        },
        EditorScene, Selection,
    },
    send_sync_message,
    world::{
        graph::{
            item::{SceneItem, SceneItemBuilder, SceneItemMessage},
            menu::ItemContextMenu,
            selection::GraphSelection,
        },
        link::{menu::LinkContextMenu, LinkItem, LinkItemBuilder, LinkItemMessage},
        physics::{
            menu::{DeletableSceneItemContextMenu, RigidBodyContextMenu},
            selection::{ColliderSelection, JointSelection, RigidBodySelection},
        },
        search::SearchBar,
        sound::selection::SoundSelection,
    },
    GameEngine, Message,
};
use rg3d::core::pool::ErasedHandle;
use rg3d::{
    core::{
        arrayvec::ArrayVec,
        color::Color,
        pool::{Handle, Pool},
        scope_profile,
    },
    engine::Engine,
    gui::{
        brush::Brush,
        button::{Button, ButtonBuilder, ButtonMessage},
        check_box::{CheckBoxBuilder, CheckBoxMessage},
        decorator::{Decorator, DecoratorMessage},
        grid::{Column, GridBuilder, Row},
        menu::MenuItemMessage,
        message::{MessageDirection, UiMessage},
        scroll_viewer::{ScrollViewerBuilder, ScrollViewerMessage},
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        tree::{
            Tree, TreeBuilder, TreeExpansionStrategy, TreeMessage, TreeRoot, TreeRootBuilder,
            TreeRootMessage,
        },
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowTitle},
        BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
        VerticalAlignment,
    },
    physics3d::desc::{ColliderShapeDesc, JointParamsDesc},
    scene::{graph::Graph, node::Node, Scene},
    sound::{context::SoundContext, source::SoundSource},
};
use std::any::TypeId;
use std::{cmp::Ordering, collections::HashMap, sync::mpsc::Sender};

pub mod graph;
pub mod link;
pub mod physics;
pub mod search;
pub mod sound;

pub struct WorldViewer {
    pub window: Handle<UiNode>,
    tree_root: Handle<UiNode>,
    graph_folder: Handle<UiNode>,
    rigid_bodies_folder: Handle<UiNode>,
    joints_folder: Handle<UiNode>,
    sounds_folder: Handle<UiNode>,
    sender: Sender<Message>,
    track_selection: Handle<UiNode>,
    track_selection_state: bool,
    search_bar: SearchBar,
    filter: String,
    stack: Vec<(Handle<UiNode>, Handle<Node>)>,
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
    item_context_menu: ItemContextMenu,
    link_context_menu: LinkContextMenu,
    rigid_body_context_menu: RigidBodyContextMenu,
    deletable_context_menu: DeletableSceneItemContextMenu,
    node_to_view_map: HashMap<Handle<Node>, Handle<UiNode>>,
    rigid_body_to_view_map: HashMap<Handle<RigidBody>, Handle<UiNode>>,
    joint_to_view_map: HashMap<Handle<Joint>, Handle<UiNode>>,
    sound_to_view_map: HashMap<Handle<SoundSource>, Handle<UiNode>>,
}

fn make_graph_node_item(
    node: &Node,
    handle: Handle<Node>,
    ctx: &mut BuildContext,
    context_menu: Handle<UiNode>,
) -> Handle<UiNode> {
    let icon = match node {
        Node::Light(_) => load_image(include_bytes!("../../resources/embed/light.png")),
        _ => load_image(include_bytes!("../../resources/embed/cube.png")),
    };

    SceneItemBuilder::new(TreeBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness {
                left: 1.0,
                top: 1.0,
                right: 0.0,
                bottom: 0.0,
            })
            .with_context_menu(context_menu),
    ))
    .with_text_brush(if node.resource().is_some() {
        Brush::Solid(Color::opaque(160, 160, 200))
    } else {
        Brush::Solid(rg3d::gui::COLOR_FOREGROUND)
    })
    .with_name(node.name().to_owned())
    .with_entity_handle(handle)
    .with_icon(icon)
    .build(ctx)
}

fn tree_node(ui: &UserInterface, tree: Handle<UiNode>) -> Handle<Node> {
    if let Some(item) = ui.node(tree).cast::<SceneItem<Node>>() {
        return item.entity_handle;
    }
    unreachable!()
}

fn colorize(handle: Handle<UiNode>, ui: &UserInterface, index: &mut usize) {
    let node = ui.node(handle);

    if let Some(decorator) = node.cast::<Decorator>() {
        if node.parent().is_some() && ui.node(node.parent()).cast::<Button>().is_none() {
            let new_brush = Brush::Solid(if *index % 2 == 0 {
                Color::opaque(50, 50, 50)
            } else {
                Color::opaque(60, 60, 60)
            });

            if decorator.normal_brush() != &new_brush {
                ui.send_message(DecoratorMessage::normal_brush(
                    handle,
                    MessageDirection::ToWidget,
                    new_brush,
                ));
            }
        }
    }

    *index += 1;

    for &item in node.children() {
        colorize(item, ui, index);
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

pub fn sync_pool<T, N, M>(
    folder: Handle<UiNode>,
    pool: &Pool<T>,
    ui: &mut UserInterface,
    selection: Option<&[Handle<T>]>,
    view_map: &mut HashMap<Handle<T>, Handle<UiNode>>,
    mut make_view: M,
    mut make_name: N,
) -> Vec<Handle<UiNode>>
where
    T: 'static,
    N: FnMut(Handle<T>) -> String,
    M: FnMut(&mut UserInterface, Handle<T>, &T) -> Handle<UiNode>,
{
    let folder_items = ui
        .node(folder)
        .cast::<Tree>()
        .unwrap()
        .items()
        .iter()
        .cloned()
        .filter(|i| ui.node(*i).cast::<SceneItem<T>>().is_some())
        .collect::<Vec<_>>();

    let pool_count = usize::try_from(pool.alive_count()).expect("overflowed usize");
    match pool_count.cmp(&folder_items.len()) {
        Ordering::Less => {
            // An entity was removed.
            for &item in folder_items.iter() {
                let entity_handle = ui.node(item).cast::<SceneItem<T>>().unwrap().entity_handle;

                if pool.pair_iter().all(|(h, _)| h != entity_handle) {
                    let removed = view_map.remove(&entity_handle);

                    assert!(removed.is_some());

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
                    .all(|i| ui.node(*i).cast::<SceneItem<T>>().unwrap().entity_handle != handle)
                {
                    let view = (make_view)(ui, handle, elem);

                    let previous = view_map.insert(handle, view);

                    assert!(previous.is_none());

                    send_sync_message(
                        ui,
                        TreeMessage::add_item(folder, MessageDirection::ToWidget, view),
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
                .cloned()
                .find(|i| ui.node(*i).cast::<SceneItem<T>>().unwrap().entity_handle == *selected)
            {
                selected_items.push(associated_item)
            }
        }
    }

    // Sync names.
    for item in ui.node(folder).cast::<Tree>().unwrap().items() {
        let item_ref = ui.node(*item).cast::<SceneItem<T>>().unwrap();
        let entity_handle = item_ref.entity_handle;
        if pool.is_valid_handle(entity_handle) {
            let new_name = (make_name)(entity_handle);
            if new_name != item_ref.name() {
                ui.send_message(SceneItemMessage::name(
                    *item,
                    MessageDirection::ToWidget,
                    new_name,
                ));
            }
        }
    }

    selected_items
}

impl WorldViewer {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let track_selection_state = true;
        let tree_root;
        let node_path;
        let collapse_all;
        let expand_all;
        let locate_selection;
        let scroll_view;
        let track_selection;
        let search_bar = SearchBar::new(ctx);
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
                                    })
                                    .with_child({
                                        track_selection = CheckBoxBuilder::new(
                                            WidgetBuilder::new()
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
                                        .checked(Some(track_selection_state))
                                        .build(ctx);
                                        track_selection
                                    }),
                            )
                            .with_orientation(Orientation::Horizontal)
                            .build(ctx),
                        )
                        .with_child(search_bar.container)
                        .with_child(
                            TextBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(2)
                                    .on_column(0)
                                    .with_opacity(Some(0.4)),
                            )
                            .with_text("Breadcrumbs")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .with_horizontal_text_alignment(HorizontalAlignment::Center)
                            .build(ctx),
                        )
                        .with_child(
                            ScrollViewerBuilder::new(WidgetBuilder::new().on_row(2))
                                .with_content({
                                    node_path = StackPanelBuilder::new(WidgetBuilder::new())
                                        .with_orientation(Orientation::Horizontal)
                                        .build(ctx);
                                    node_path
                                })
                                .build(ctx),
                        )
                        .with_child({
                            scroll_view = ScrollViewerBuilder::new(WidgetBuilder::new().on_row(3))
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
                .add_row(Row::strict(24.0))
                .add_row(Row::stretch())
                .build(ctx),
            )
            .build(ctx);

        let item_context_menu = ItemContextMenu::new(ctx);
        let link_context_menu = LinkContextMenu::new(ctx);
        let rigid_body_context_menu = RigidBodyContextMenu::new(ctx);
        let deletable_context_menu = DeletableSceneItemContextMenu::new(ctx);

        Self {
            search_bar,
            track_selection,
            track_selection_state,
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
            link_context_menu,
            rigid_body_context_menu,
            deletable_context_menu,
            node_to_view_map: Default::default(),
            rigid_body_to_view_map: Default::default(),
            joint_to_view_map: Default::default(),
            sound_to_view_map: Default::default(),
            filter: Default::default(),
        }
    }

    pub fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        scope_profile!();

        let scene = &mut engine.scenes[editor_scene.scene];
        let graph = &mut scene.graph;
        let ui = &mut engine.user_interface;

        let mut selected_items = Vec::new();

        selected_items.extend(self.sync_graph(ui, editor_scene, graph));
        selected_items.extend(self.sync_rigid_bodies(ui, editor_scene));
        selected_items.extend(self.sync_colliders(ui, editor_scene));
        selected_items.extend(self.sync_joints(ui, editor_scene));
        selected_items.extend(self.sync_sounds(ui, editor_scene, scene.sound_context.clone()));

        if !selected_items.is_empty() {
            send_sync_message(
                ui,
                TreeRootMessage::select(self.tree_root, MessageDirection::ToWidget, selected_items),
            );
        }

        self.sync_links(editor_scene, engine);
    }

    fn build_breadcrumb(
        &mut self,
        name: &str,
        associated_item: Handle<UiNode>,
        ui: &mut UserInterface,
    ) {
        let element = ButtonBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
            .with_text(name)
            .build(&mut ui.build_ctx());

        send_sync_message(
            ui,
            WidgetMessage::link_reverse(element, MessageDirection::ToWidget, self.node_path),
        );

        self.breadcrumbs.insert(element, associated_item);
    }

    fn update_breadcrumbs(
        &mut self,
        ui: &mut UserInterface,
        editor_scene: &EditorScene,
        scene: &Scene,
    ) {
        // Update breadcrumbs.
        self.breadcrumbs.clear();
        for &child in ui.node(self.node_path).children() {
            send_sync_message(ui, WidgetMessage::remove(child, MessageDirection::ToWidget));
        }

        match &editor_scene.selection {
            Selection::Graph(selection) => {
                if let Some(&first_selected) = selection.nodes().first() {
                    let mut item = first_selected;
                    while item.is_some() {
                        let node = &scene.graph[item];

                        let view = ui.find_by_criteria_down(self.graph_folder, &|n| {
                            n.cast::<SceneItem<Node>>()
                                .map(|i| i.entity_handle == item)
                                .unwrap_or_default()
                        });
                        assert!(view.is_some());
                        self.build_breadcrumb(node.name(), view, ui);

                        item = node.parent();
                    }
                }
            }
            Selection::Navmesh(_) => {
                // TODO
            }
            Selection::Sound(selection) => {
                if let Some(&first_selected) = selection.sources().first() {
                    let view = ui.find_by_criteria_down(self.sounds_folder, &|n| {
                        n.cast::<SceneItem<SoundSource>>()
                            .map(|i| i.entity_handle == first_selected)
                            .unwrap_or_default()
                    });
                    assert!(view.is_some());
                    self.build_breadcrumb(
                        scene.sound_context.state().source(first_selected).name(),
                        view,
                        ui,
                    );
                }
            }
            Selection::RigidBody(selection) => {
                if let Some(&first_selected) = selection.bodies().first() {
                    let view = ui.find_by_criteria_down(self.rigid_bodies_folder, &|n| {
                        n.cast::<SceneItem<RigidBody>>()
                            .map(|i| i.entity_handle == first_selected)
                            .unwrap_or_default()
                    });
                    assert!(view.is_some());
                    self.build_breadcrumb("Rigid Body", view, ui);
                }
            }
            Selection::Joint(selection) => {
                if let Some(&first_selected) = selection.joints().first() {
                    let view = ui.find_by_criteria_down(self.joints_folder, &|n| {
                        n.cast::<SceneItem<Joint>>()
                            .map(|i| i.entity_handle == first_selected)
                            .unwrap_or_default()
                    });
                    assert!(view.is_some());
                    self.build_breadcrumb("Joint", view, ui);
                }
            }
            Selection::Collider(selection) => {
                if let Some(&first_selected) = selection.colliders().first() {
                    let view = ui.find_by_criteria_down(self.rigid_bodies_folder, &|n| {
                        n.cast::<SceneItem<Collider>>()
                            .map(|i| i.entity_handle == first_selected)
                            .unwrap_or_default()
                    });
                    assert!(view.is_some());
                    self.build_breadcrumb("Collider", view, ui);
                }
            }
            Selection::None => {}
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
            &mut self.sound_to_view_map,
            |ui, handle, _| {
                SceneItemBuilder::<SoundSource>::new(TreeBuilder::new(WidgetBuilder::new()))
                    .with_name(ctx.source(handle).name_owned())
                    .with_icon(load_image(include_bytes!(
                        "../../resources/embed/sound_source.png"
                    )))
                    .with_entity_handle(handle)
                    .build(&mut ui.build_ctx())
            },
            |s| ctx.source(s).name_owned(),
        )
    }

    fn sync_joints(
        &mut self,
        ui: &mut UserInterface,
        editor_scene: &EditorScene,
    ) -> Vec<Handle<UiNode>> {
        let make_name = |j: Handle<Joint>| match editor_scene.physics.joints[j].params {
            JointParamsDesc::BallJoint(_) => "Ball Joint".to_owned(),
            JointParamsDesc::FixedJoint(_) => "Fixed Joint".to_owned(),
            JointParamsDesc::PrismaticJoint(_) => "Prismatic Joint".to_owned(),
            JointParamsDesc::RevoluteJoint(_) => "Revolute Joint".to_owned(),
        };

        let context_menu = self.deletable_context_menu.menu;
        sync_pool(
            self.joints_folder,
            &editor_scene.physics.joints,
            ui,
            if let Selection::Joint(ref s) = editor_scene.selection {
                Some(&s.joints)
            } else {
                None
            },
            &mut self.joint_to_view_map,
            |ui, handle, _| {
                SceneItemBuilder::<Joint>::new(TreeBuilder::new(
                    WidgetBuilder::new().with_context_menu(context_menu),
                ))
                .with_name(make_name(handle))
                .with_icon(load_image(include_bytes!(
                    "../../resources/embed/joint.png"
                )))
                .with_entity_handle(handle)
                .build(&mut ui.build_ctx())
            },
            make_name,
        )
    }

    fn sync_rigid_bodies(
        &mut self,
        ui: &mut UserInterface,
        editor_scene: &EditorScene,
    ) -> Vec<Handle<UiNode>> {
        let context_menu = self.rigid_body_context_menu.menu;
        sync_pool(
            self.rigid_bodies_folder,
            &editor_scene.physics.bodies,
            ui,
            if let Selection::RigidBody(ref s) = editor_scene.selection {
                Some(&s.bodies)
            } else {
                None
            },
            &mut self.rigid_body_to_view_map,
            |ui, handle, _| {
                SceneItemBuilder::<RigidBody>::new(TreeBuilder::new(
                    WidgetBuilder::new().with_context_menu(context_menu),
                ))
                .with_icon(load_image(include_bytes!(
                    "../../resources/embed/rigid_body.png"
                )))
                .with_name("Rigid Body".to_owned())
                .with_entity_handle(handle)
                .build(&mut ui.build_ctx())
            },
            |_| "Rigid Body".to_owned(),
        )
    }

    fn sync_graph(
        &mut self,
        ui: &mut UserInterface,
        editor_scene: &EditorScene,
        graph: &Graph,
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

            if let Some(item) = ui_node.cast::<SceneItem<Node>>() {
                // Since we are filtering out editor stuff from world viewer, we must
                // correctly count children, excluding editor nodes.
                let mut child_count = 0;
                for &child in node.children() {
                    if child != editor_scene.root {
                        child_count += 1;
                    }
                }

                // We're interested only scene graph child items.
                // Such filtering is needed because we can have links as children in UI.
                let items = item
                    .tree
                    .items()
                    .iter()
                    .cloned()
                    .filter(|i| ui.node(*i).cast::<SceneItem<Node>>().is_some())
                    .collect::<Vec<_>>();

                match child_count.cmp(&items.len()) {
                    Ordering::Less => {
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
                                self.node_to_view_map.remove(&child_node);
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
                                let graph_node_item = make_graph_node_item(
                                    &graph[child_handle],
                                    child_handle,
                                    &mut ui.build_ctx(),
                                    self.item_context_menu.menu,
                                );
                                send_sync_message(
                                    ui,
                                    TreeMessage::add_item(
                                        tree_handle,
                                        MessageDirection::ToWidget,
                                        graph_node_item,
                                    ),
                                );
                                if let Selection::Graph(selection) = &editor_scene.selection {
                                    if selection.contains(child_handle) {
                                        selected_items.push(graph_node_item);
                                    }
                                }
                                self.node_to_view_map.insert(child_handle, graph_node_item);
                                self.stack.push((graph_node_item, child_handle));
                            }
                        }
                    }
                }
            } else if let Some(folder) = ui_node.cast::<Tree>() {
                if folder.items().is_empty() {
                    let graph_node_item = make_graph_node_item(
                        node,
                        node_handle,
                        &mut ui.build_ctx(),
                        self.item_context_menu.menu,
                    );
                    send_sync_message(
                        ui,
                        TreeMessage::add_item(
                            tree_handle,
                            MessageDirection::ToWidget,
                            graph_node_item,
                        ),
                    );
                    self.node_to_view_map.insert(node_handle, graph_node_item);
                    self.stack.push((graph_node_item, node_handle));
                } else {
                    self.stack.push((folder.items()[0], node_handle));
                }
            }
        }

        // Sync items data.
        let mut stack = vec![self.tree_root];
        while let Some(handle) = stack.pop() {
            let ui_node = ui.node(handle);

            if let Some(item) = ui_node.cast::<SceneItem<Node>>() {
                if graph.is_valid_handle(item.entity_handle) {
                    let node = &graph[item.entity_handle];

                    if item.name() != node.name() {
                        send_sync_message(
                            ui,
                            SceneItemMessage::name(
                                handle,
                                MessageDirection::ToWidget,
                                node.name().to_owned(),
                            ),
                        );
                    }

                    stack.extend_from_slice(item.tree.items());
                }
            } else if let Some(root) = ui_node.cast::<TreeRoot>() {
                stack.extend_from_slice(root.items())
            }
        }

        self.colorize(ui);

        self.node_to_view_map
            .retain(|k, v| graph.is_valid_handle(*k) && ui.try_get_node(*v).is_some());

        selected_items
    }

    fn sync_node_rigid_body_links(&mut self, editor_scene: &EditorScene, engine: &mut Engine) {
        let ui = &mut engine.user_interface;

        for (&node, &view) in self.node_to_view_map.iter() {
            let node_view_ref = ui
                .node(view)
                .cast::<SceneItem<Node>>()
                .expect("Must be GraphNodeItem");

            let rigid_body_links = node_view_ref
                .tree
                .items()
                .iter()
                .cloned()
                .filter(|i| ui.node(*i).cast::<LinkItem<RigidBody, Node>>().is_some())
                .collect::<Vec<_>>();

            let linked_body = editor_scene.physics.binder.forward_map().get(&node);

            if rigid_body_links.is_empty() {
                if let Some(linked_body) = linked_body {
                    let link = LinkItemBuilder::new(TreeBuilder::new(
                        WidgetBuilder::new().with_context_menu(self.link_context_menu.menu),
                    ))
                    .with_name("Linked Rigid Body")
                    .with_source(*linked_body)
                    .with_dest(node)
                    .build(&mut ui.build_ctx());

                    ui.send_message(TreeMessage::add_item(
                        view,
                        MessageDirection::ToWidget,
                        link,
                    ));
                }
            } else if linked_body.is_none() {
                assert_eq!(rigid_body_links.len(), 1);

                // Remove link.
                ui.send_message(TreeMessage::remove_item(
                    view,
                    MessageDirection::ToWidget,
                    rigid_body_links[0],
                ));
            }
        }
    }

    fn sync_rigid_body_node_links(&mut self, editor_scene: &EditorScene, engine: &mut Engine) {
        let ui = &mut engine.user_interface;

        let graph = &engine.scenes[editor_scene.scene].graph;

        for (&rigid_body, &view) in self.rigid_body_to_view_map.iter() {
            let rigid_body_view_ref = ui
                .node(view)
                .cast::<SceneItem<RigidBody>>()
                .expect("Must be SceneItem<RigidBody>");

            let node_links = rigid_body_view_ref
                .tree
                .items()
                .iter()
                .cloned()
                .filter(|i| ui.node(*i).cast::<LinkItem<Node, RigidBody>>().is_some())
                .collect::<Vec<_>>();

            let linked_node = editor_scene.physics.binder.backward_map().get(&rigid_body);

            if node_links.is_empty() {
                if let Some(linked_node) = linked_node {
                    let link = LinkItemBuilder::<Node, RigidBody>::new(TreeBuilder::new(
                        WidgetBuilder::new().with_context_menu(self.link_context_menu.menu),
                    ))
                    .with_name(format!("Linked Node {}", graph[*linked_node].name()))
                    .with_source(*linked_node)
                    .with_dest(rigid_body)
                    .build(&mut ui.build_ctx());

                    ui.send_message(TreeMessage::add_item(
                        view,
                        MessageDirection::ToWidget,
                        link,
                    ));
                }
            } else if linked_node.is_none() {
                assert_eq!(node_links.len(), 1);

                // Remove link.
                ui.send_message(TreeMessage::remove_item(
                    view,
                    MessageDirection::ToWidget,
                    node_links[0],
                ));
            }

            for (node_link, node_link_ref) in node_links
                .into_iter()
                .map(|h| (h, ui.node(h).cast::<LinkItem<Node, RigidBody>>().unwrap()))
            {
                ui.send_message(LinkItemMessage::name(
                    node_link,
                    MessageDirection::ToWidget,
                    format!("Linked Node {}", graph[node_link_ref.source].name()),
                ));
            }
        }
    }

    fn sync_links(&mut self, editor_scene: &EditorScene, engine: &mut Engine) {
        self.sync_node_rigid_body_links(editor_scene, engine);
        self.sync_rigid_body_node_links(editor_scene, engine);
        self.sync_joint_body_links(editor_scene, engine);
    }

    fn sync_joint_body_links(&mut self, editor_scene: &EditorScene, engine: &mut Engine) {
        let ui = &mut engine.user_interface;

        for (&joint, &view) in self.joint_to_view_map.iter() {
            let joint_view_ref = ui
                .node(view)
                .cast::<SceneItem<Joint>>()
                .expect("Must be SceneItem<Joint>");

            let rigid_body_links = joint_view_ref
                .tree
                .items()
                .iter()
                .cloned()
                .filter(|i| ui.node(*i).cast::<LinkItem<RigidBody, Joint>>().is_some())
                .collect::<Vec<_>>();

            let joint_ref = &editor_scene.physics.joints[joint];

            let linked_bodies = ArrayVec::<Handle<RigidBody>, 2>::from_iter(
                [joint_ref.body1, joint_ref.body2].iter().filter_map(|&j| {
                    if j.is_none() {
                        None
                    } else {
                        Some(Handle::<RigidBody>::from(j))
                    }
                }),
            );

            match linked_bodies.len().cmp(&rigid_body_links.len()) {
                Ordering::Less => {
                    for rigid_body_link in rigid_body_links.iter() {
                        if linked_bodies.iter().all(|b| {
                            ui.node(*rigid_body_link)
                                .cast::<LinkItem<RigidBody, Joint>>()
                                .unwrap()
                                .source
                                != *b
                        }) {
                            // Remove link.
                            ui.send_message(TreeMessage::remove_item(
                                view,
                                MessageDirection::ToWidget,
                                *rigid_body_link,
                            ));
                        }
                    }
                }
                Ordering::Greater => {
                    for linked_body in linked_bodies.iter() {
                        if rigid_body_links.iter().all(|l| {
                            ui.node(*l)
                                .cast::<LinkItem<RigidBody, Joint>>()
                                .unwrap()
                                .source
                                != *linked_body
                        }) {
                            let link = LinkItemBuilder::new(TreeBuilder::new(
                                WidgetBuilder::new().with_context_menu(self.link_context_menu.menu),
                            ))
                            .with_name("Linked Rigid Body")
                            .with_source(*linked_body)
                            .with_dest(joint)
                            .build(&mut ui.build_ctx());

                            ui.send_message(TreeMessage::add_item(
                                view,
                                MessageDirection::ToWidget,
                                link,
                            ));
                        }
                    }
                }
                _ => (),
            }
        }
    }

    pub fn sync_colliders(
        &mut self,
        ui: &mut UserInterface,
        editor_scene: &EditorScene,
    ) -> Vec<Handle<UiNode>> {
        let mut selected_colliders = Vec::new();

        for (&rigid_body_handle, &rigid_body_view) in self.rigid_body_to_view_map.iter() {
            let rigid_body_view_ref = ui
                .node(rigid_body_view)
                .cast::<SceneItem<RigidBody>>()
                .expect("Must be a SceneItem<RigidBody>");

            let collider_views = rigid_body_view_ref
                .tree
                .items()
                .iter()
                .cloned()
                .filter(|i| ui.node(*i).cast::<SceneItem<Collider>>().is_some())
                .collect::<Vec<_>>();

            let rigid_body = &editor_scene.physics.bodies[rigid_body_handle];

            match rigid_body.colliders.len().cmp(&collider_views.len()) {
                Ordering::Less => {
                    // A collider was removed.
                    for (&collider_view, collider_view_ref) in collider_views.iter().map(|v| {
                        (
                            v,
                            ui.node(*v)
                                .cast::<SceneItem<Collider>>()
                                .expect("Must be a SceneItem<Collider>!"),
                        )
                    }) {
                        if rigid_body
                            .colliders
                            .iter()
                            .map(|&c| Handle::<Collider>::from(c))
                            .all(|c| c != collider_view_ref.entity_handle)
                        {
                            ui.send_message(TreeMessage::remove_item(
                                rigid_body_view,
                                MessageDirection::ToWidget,
                                collider_view,
                            ));
                        }
                    }
                }
                Ordering::Greater => {
                    // A collider was added.
                    for collider_handle in rigid_body
                        .colliders
                        .iter()
                        .map(|&h| Handle::<Collider>::from(h))
                    {
                        if collider_views.iter().all(|v| {
                            ui.node(*v)
                                .cast::<SceneItem<Collider>>()
                                .expect("Must be a SceneItem<Collider>")
                                .entity_handle
                                != collider_handle
                        }) {
                            let collider_ref = &editor_scene.physics.colliders[collider_handle];

                            let name = match &collider_ref.shape {
                                ColliderShapeDesc::Ball(_) => "Ball Collider",
                                ColliderShapeDesc::Cylinder(_) => "Cylinder Collider",
                                ColliderShapeDesc::RoundCylinder(_) => "Round Cylinder Collider",
                                ColliderShapeDesc::Cone(_) => "Cone  Collider",
                                ColliderShapeDesc::Cuboid(_) => "Cuboid Collider",
                                ColliderShapeDesc::Capsule(_) => "Capsule Collider",
                                ColliderShapeDesc::Segment(_) => "Segment Collider",
                                ColliderShapeDesc::Triangle(_) => "Triangle Collider",
                                ColliderShapeDesc::Trimesh(_) => "Triangle Mesh Collider",
                                ColliderShapeDesc::Heightfield(_) => "Height Field Collider",
                            };

                            let view = SceneItemBuilder::<Collider>::new(TreeBuilder::new(
                                WidgetBuilder::new()
                                    .with_context_menu(self.deletable_context_menu.menu),
                            ))
                            .with_name(name.to_owned())
                            .with_icon(load_image(include_bytes!(
                                "../../resources/embed/collider.png"
                            )))
                            .with_entity_handle(collider_handle)
                            .build(&mut ui.build_ctx());

                            ui.send_message(TreeMessage::add_item(
                                rigid_body_view,
                                MessageDirection::ToWidget,
                                view,
                            ));
                        }
                    }
                }
                Ordering::Equal => {
                    // Do nothing.
                }
            }
        }

        // Sync selection.
        if let Selection::Collider(ref selection) = editor_scene.selection {
            for rigid_body_view_ref in self.rigid_body_to_view_map.values().map(|v| {
                ui.node(*v)
                    .cast::<SceneItem<RigidBody>>()
                    .expect("Must be SceneItem<RigidBody>!")
            }) {
                for (collider_view, collider_view_ref) in rigid_body_view_ref
                    .tree
                    .items()
                    .iter()
                    .filter_map(|i| ui.node(*i).cast::<SceneItem<Collider>>().map(|c| (*i, c)))
                {
                    if selection
                        .colliders
                        .contains(&collider_view_ref.entity_handle)
                    {
                        selected_colliders.push(collider_view);
                    }
                }
            }
        }

        selected_colliders
    }

    pub fn colorize(&mut self, ui: &UserInterface) {
        let mut index = 0;
        colorize(self.tree_root, ui, &mut index);
    }

    fn apply_filter(&self, ui: &UserInterface) {
        fn apply_filter_recursive(node: Handle<UiNode>, filter: &str, ui: &UserInterface) -> bool {
            let node_ref = ui.node(node);

            let mut is_any_match = false;
            for &child in node_ref.children() {
                is_any_match |= apply_filter_recursive(child, filter, ui)
            }

            // TODO: It is very easy to forget to add a new condition here if a new type
            // of a scene item is added. Find a way of doing this in a better way.
            // Also due to very simple RTTI in Rust, it becomes boilerplate-ish very quick.
            let name = node_ref
                .cast::<SceneItem<Node>>()
                .map(|i| i.name())
                .or_else(|| node_ref.cast::<SceneItem<RigidBody>>().map(|i| i.name()))
                .or_else(|| node_ref.cast::<SceneItem<Joint>>().map(|i| i.name()))
                .or_else(|| node_ref.cast::<SceneItem<Collider>>().map(|i| i.name()));

            if let Some(name) = name {
                is_any_match |= name.contains(filter);

                ui.send_message(WidgetMessage::visibility(
                    node,
                    MessageDirection::ToWidget,
                    is_any_match,
                ));
            }

            is_any_match
        }

        apply_filter_recursive(self.tree_root, &self.filter, ui);
    }

    pub fn set_filter(&mut self, filter: String, ui: &UserInterface) {
        self.filter = filter;
        self.apply_filter(ui)
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
        self.link_context_menu.handle_ui_message(message);
        self.rigid_body_context_menu.handle_ui_message(
            message,
            &self.sender,
            editor_scene,
            &engine.user_interface,
        );
        self.deletable_context_menu.handle_ui_message(
            message,
            &engine.user_interface,
            &self.sender,
            editor_scene,
        );
        self.search_bar
            .handle_ui_message(message, &engine.user_interface, &self.sender);

        if let Some(TreeRootMessage::Selected(selection)) = message.data::<TreeRootMessage>() {
            if message.destination() == self.tree_root
                && message.direction() == MessageDirection::FromWidget
            {
                self.handle_selection(selection, editor_scene, engine);
            }
        } else if let Some(&WidgetMessage::Drop(node)) = message.data::<WidgetMessage>() {
            self.handle_drop(engine, editor_scene, message.destination(), node);
        } else if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if let Some(&view) = self.breadcrumbs.get(&message.destination()) {
                if let Some(graph_node) = engine
                    .user_interface
                    .try_get_node(view)
                    .and_then(|n| n.cast::<SceneItem<Node>>())
                {
                    self.sender
                        .send(Message::do_scene_command(ChangeSelectionCommand::new(
                            Selection::Graph(GraphSelection::single_or_empty(
                                graph_node.entity_handle,
                            )),
                            editor_scene.selection.clone(),
                        )))
                        .unwrap();
                } else {
                    // Rest are not handled intentionally because other entities cannot have
                    // hierarchy and thus there is no need to change selection when we already
                    // have it selected.
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
                self.locate_selection(&editor_scene.selection, engine)
            }
        } else if let Some(CheckBoxMessage::Check(Some(value))) = message.data::<CheckBoxMessage>()
        {
            if message.destination() == self.track_selection {
                self.track_selection_state = *value;
                if *value {
                    self.locate_selection(&editor_scene.selection, engine);
                }
            }
        } else if let Some(MenuItemMessage::Click) = message.data::<MenuItemMessage>() {
            if message.destination() == self.link_context_menu.unlink {
                self.handle_unlink(&engine.user_interface, editor_scene);
            } else if message.destination() == self.link_context_menu.select_target {
                self.select_link_target(&engine.user_interface, editor_scene)
            }
        }
    }

    pub fn try_locate_object(&self, type_id: TypeId, handle: ErasedHandle, engine: &Engine) {
        if type_id == TypeId::of::<Node>() {
            let selection = Selection::Graph(GraphSelection::single_or_empty(handle.into()));
            self.locate_selection(&selection, engine)
        } else {
            // TODO: Add more types here.
        }
    }

    fn locate_selection(&self, selection: &Selection, engine: &Engine) {
        let tree_to_focus = self.map_selection(selection, engine);

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

    fn select_link_target(&self, ui: &UserInterface, editor_scene: &EditorScene) {
        assert!(self.link_context_menu.target.is_some());

        if let Some(rigid_body_link) = ui
            .try_get_node(self.link_context_menu.target)
            .and_then(|n| n.cast::<LinkItem<RigidBody, Node>>())
        {
            self.sender
                .send(Message::do_scene_command(ChangeSelectionCommand::new(
                    Selection::RigidBody(RigidBodySelection {
                        bodies: vec![rigid_body_link.source],
                    }),
                    editor_scene.selection.clone(),
                )))
                .unwrap();
        } else if let Some(node_link) = ui
            .try_get_node(self.link_context_menu.target)
            .and_then(|n| n.cast::<LinkItem<Node, RigidBody>>())
        {
            self.sender
                .send(Message::do_scene_command(ChangeSelectionCommand::new(
                    Selection::Graph(GraphSelection::single_or_empty(node_link.source)),
                    editor_scene.selection.clone(),
                )))
                .unwrap();
        } else if let Some(joint_link) = ui
            .try_get_node(self.link_context_menu.target)
            .and_then(|n| n.cast::<LinkItem<RigidBody, Joint>>())
        {
            self.sender
                .send(Message::do_scene_command(ChangeSelectionCommand::new(
                    Selection::RigidBody(RigidBodySelection {
                        bodies: vec![joint_link.source],
                    }),
                    editor_scene.selection.clone(),
                )))
                .unwrap();
        }
    }

    fn handle_selection(
        &self,
        selection: &[Handle<UiNode>],
        editor_scene: &EditorScene,
        engine: &Engine,
    ) {
        let mut new_selection = Selection::None;
        for selected_item in selection {
            let selected_item_ref = engine.user_interface.node(*selected_item);

            if let Some(graph_node) = selected_item_ref.cast::<SceneItem<Node>>() {
                match new_selection {
                    Selection::None => {
                        new_selection = Selection::Graph(GraphSelection::single_or_empty(
                            graph_node.entity_handle,
                        ));
                    }
                    Selection::Graph(ref mut selection) => {
                        selection.insert_or_exclude(graph_node.entity_handle)
                    }
                    _ => (),
                }
            } else if let Some(rigid_body) = selected_item_ref.cast::<SceneItem<RigidBody>>() {
                match new_selection {
                    Selection::None => {
                        new_selection = Selection::RigidBody(RigidBodySelection {
                            bodies: vec![rigid_body.entity_handle],
                        });
                    }
                    Selection::RigidBody(ref mut selection) => {
                        selection.bodies.push(rigid_body.entity_handle)
                    }
                    _ => (),
                }
            } else if let Some(joint) = selected_item_ref.cast::<SceneItem<Joint>>() {
                match new_selection {
                    Selection::None => {
                        new_selection = Selection::Joint(JointSelection {
                            joints: vec![joint.entity_handle],
                        });
                    }
                    Selection::Joint(ref mut selection) => {
                        selection.joints.push(joint.entity_handle)
                    }
                    _ => (),
                }
            } else if let Some(collider) = selected_item_ref.cast::<SceneItem<Collider>>() {
                match new_selection {
                    Selection::None => {
                        new_selection = Selection::Collider(ColliderSelection {
                            colliders: vec![collider.entity_handle],
                        });
                    }
                    Selection::Collider(ref mut selection) => {
                        selection.colliders.push(collider.entity_handle)
                    }
                    _ => (),
                }
            } else if let Some(sound) = selected_item_ref.cast::<SceneItem<SoundSource>>() {
                match new_selection {
                    Selection::None => {
                        new_selection = Selection::Sound(SoundSelection {
                            sources: vec![sound.entity_handle],
                        });
                    }
                    Selection::Sound(ref mut selection) => {
                        selection.sources.push(sound.entity_handle)
                    }
                    _ => (),
                }
            } else {
                return;
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

    fn handle_unlink(&self, ui: &UserInterface, editor_scene: &EditorScene) {
        assert!(self.link_context_menu.target.is_some());

        if let Some(rigid_body_link) = ui
            .try_get_node(self.link_context_menu.target)
            .and_then(|n| n.cast::<LinkItem<RigidBody, Node>>())
        {
            self.sender
                .send(Message::do_scene_command(UnlinkBodyCommand {
                    node: rigid_body_link.dest,
                    handle: rigid_body_link.source,
                }))
                .unwrap();
        } else if let Some(node_link) = ui
            .try_get_node(self.link_context_menu.target)
            .and_then(|n| n.cast::<LinkItem<Node, RigidBody>>())
        {
            self.sender
                .send(Message::do_scene_command(UnlinkBodyCommand {
                    node: node_link.source,
                    handle: node_link.dest,
                }))
                .unwrap();
        } else if let Some(joint_link) = ui
            .try_get_node(self.link_context_menu.target)
            .and_then(|n| n.cast::<LinkItem<RigidBody, Joint>>())
        {
            let joint_ref = &editor_scene.physics.joints[joint_link.dest];
            if joint_ref.body1 == joint_link.source.into() {
                self.sender
                    .send(Message::do_scene_command(SetJointBody1Command::new(
                        joint_link.dest,
                        Default::default(),
                    )))
                    .unwrap();
            } else if joint_ref.body2 == joint_link.source.into() {
                self.sender
                    .send(Message::do_scene_command(SetJointBody2Command::new(
                        joint_link.dest,
                        Default::default(),
                    )))
                    .unwrap();
            }
        }
    }

    /// `target` - is a node at which `dropped` was dropped.
    /// `dropped` - is a node which was dropped at `target`.
    fn handle_drop(
        &self,
        engine: &Engine,
        editor_scene: &EditorScene,
        target: Handle<UiNode>,
        dropped: Handle<UiNode>,
    ) {
        let ui = &engine.user_interface;

        if ui.is_node_child_of(dropped, self.tree_root)
            && ui.is_node_child_of(target, self.tree_root)
            && dropped != target
        {
            if let (Some(child), Some(parent)) = (
                ui.node(dropped).cast::<SceneItem<Node>>(),
                ui.node(target).cast::<SceneItem<Node>>(),
            ) {
                // Make sure we won't create any loops - child must not have parent in its
                // descendants.
                let mut attach = true;
                let graph = &engine.scenes[editor_scene.scene].graph;
                let mut p = parent.entity_handle;
                while p.is_some() {
                    if p == child.entity_handle {
                        attach = false;
                        break;
                    }
                    p = graph[p].parent();
                }

                if attach {
                    self.sender
                        .send(Message::do_scene_command(LinkNodesCommand::new(
                            child.entity_handle,
                            parent.entity_handle,
                        )))
                        .unwrap();
                }
            } else if let (Some(rigid_body), Some(node)) = (
                ui.node(dropped).cast::<SceneItem<RigidBody>>(),
                ui.node(target).cast::<SceneItem<Node>>(),
            ) {
                let already_linked = editor_scene
                    .physics
                    .binder
                    .forward_map()
                    .iter()
                    .any(|(n, b)| node.entity_handle == *n && rigid_body.entity_handle == *b);

                if !already_linked {
                    let mut group = Vec::new();

                    if let Some(linked_body) = editor_scene
                        .physics
                        .binder
                        .forward_map()
                        .get(&node.entity_handle)
                    {
                        group.push(SceneCommand::new(UnlinkBodyCommand {
                            node: node.entity_handle,
                            handle: *linked_body,
                        }));
                    }

                    group.push(SceneCommand::new(LinkBodyCommand {
                        node: node.entity_handle,
                        handle: rigid_body.entity_handle,
                    }));

                    self.sender
                        .send(Message::do_scene_command(CommandGroup::from(group)))
                        .unwrap();
                }
            } else if let (Some(rigid_body), Some(joint)) = (
                ui.node(dropped).cast::<SceneItem<RigidBody>>(),
                ui.node(target).cast::<SceneItem<Joint>>(),
            ) {
                let joint_ref = &editor_scene.physics.joints[joint.entity_handle];

                if joint_ref.body1.is_none() {
                    self.sender
                        .send(Message::do_scene_command(SetJointBody1Command::new(
                            joint.entity_handle,
                            rigid_body.entity_handle.into(),
                        )))
                        .unwrap();
                } else if joint_ref.body2.is_none() {
                    self.sender
                        .send(Message::do_scene_command(SetJointBody2Command::new(
                            joint.entity_handle,
                            rigid_body.entity_handle.into(),
                        )))
                        .unwrap();
                }
            }
        }
    }

    fn map_selection(&self, selection: &Selection, engine: &GameEngine) -> Vec<Handle<UiNode>> {
        let ui = &engine.user_interface;
        match selection {
            Selection::Graph(selection) => {
                map_selection(selection.nodes(), self.graph_folder, &engine.user_interface)
            }
            Selection::Sound(selection) => {
                map_selection(selection.sources(), self.sounds_folder, ui)
            }
            Selection::RigidBody(selection) => {
                map_selection(selection.bodies(), self.rigid_bodies_folder, ui)
            }
            Selection::Joint(selection) => {
                map_selection(selection.joints(), self.joints_folder, ui)
            }
            Selection::Collider(selection) => {
                // Collider views stored as rigid body child.
                map_selection(selection.colliders(), self.rigid_bodies_folder, ui)
            }
            Selection::None | Selection::Navmesh(_) => Default::default(),
        }
    }

    pub fn post_update(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        // Hack. See `self.sync_selection` for details.
        if self.sync_selection {
            let trees = self.map_selection(&editor_scene.selection, engine);

            let ui = &mut engine.user_interface;
            send_sync_message(
                ui,
                TreeRootMessage::select(self.tree_root, MessageDirection::ToWidget, trees),
            );

            self.update_breadcrumbs(ui, editor_scene, &engine.scenes[editor_scene.scene]);
            if self.track_selection_state {
                self.locate_selection(&editor_scene.selection, engine);
            }

            self.sync_selection = false;
        }
    }

    pub fn clear(&mut self, ui: &mut UserInterface) {
        self.node_to_view_map.clear();
        self.rigid_body_to_view_map.clear();
        self.joint_to_view_map.clear();
        self.sound_to_view_map.clear();

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

fn map_selection<T>(
    selection: &[Handle<T>],
    folder: Handle<UiNode>,
    ui: &UserInterface,
) -> Vec<Handle<UiNode>>
where
    T: 'static,
{
    selection
        .iter()
        .filter_map(|&handle| {
            let item = ui.find_by_criteria_down(folder, &|n| {
                n.cast::<SceneItem<T>>()
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
