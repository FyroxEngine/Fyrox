#![allow(dead_code)] // TODO

use crate::{
    animation::command::AnimationCommandStack,
    scene::{
        property::{
            object_to_property_tree, PropertySelectorMessage, PropertySelectorWindowBuilder,
        },
        selector::{HierarchyNode, NodeSelectorMessage, NodeSelectorWindowBuilder},
        EditorScene,
    },
};
use fyrox::{
    core::{pool::Handle, reflect::ResolvePath},
    engine::Engine,
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        curve::CurveEditorBuilder,
        grid::{Column, GridBuilder, Row},
        list_view::ListViewBuilder,
        menu::{MenuBuilder, MenuItemBuilder, MenuItemContent, MenuItemMessage},
        message::{MessageDirection, UiMessage},
        stack_panel::StackPanelBuilder,
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, Orientation, Thickness, UiNode, UserInterface,
    },
    resource::animation::AnimationResource,
    scene::node::Node,
};

pub mod command;

struct Menu {
    menu: Handle<UiNode>,
    new: Handle<UiNode>,
    load: Handle<UiNode>,
    save: Handle<UiNode>,
    save_as: Handle<UiNode>,
    exit: Handle<UiNode>,
    undo: Handle<UiNode>,
    redo: Handle<UiNode>,
}

impl Menu {
    fn new(ctx: &mut BuildContext) -> Self {
        let new;
        let load;
        let save;
        let save_as;
        let exit;
        let undo;
        let redo;
        let menu = MenuBuilder::new(WidgetBuilder::new().on_row(0).on_column(0))
            .with_items(vec![
                MenuItemBuilder::new(WidgetBuilder::new())
                    .with_content(MenuItemContent::text_no_arrow("File"))
                    .with_items(vec![
                        {
                            new = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text_no_arrow("New"))
                                .build(ctx);
                            new
                        },
                        {
                            load = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text_no_arrow("Load..."))
                                .build(ctx);
                            load
                        },
                        {
                            save = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text_no_arrow("Save"))
                                .build(ctx);
                            save
                        },
                        {
                            save_as = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text_no_arrow("Save As..."))
                                .build(ctx);
                            save_as
                        },
                        {
                            exit = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text_no_arrow("Exit"))
                                .build(ctx);
                            exit
                        },
                    ])
                    .build(ctx),
                MenuItemBuilder::new(WidgetBuilder::new())
                    .with_content(MenuItemContent::text_no_arrow("Edit"))
                    .with_items(vec![
                        {
                            undo = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text_no_arrow("Undo"))
                                .build(ctx);
                            undo
                        },
                        {
                            redo = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text_no_arrow("Redo"))
                                .build(ctx);
                            redo
                        },
                    ])
                    .build(ctx),
            ])
            .build(ctx);

        Self {
            menu,
            new,
            load,
            save,
            save_as,
            exit,
            undo,
            redo,
        }
    }
}

struct TrackList {
    panel: Handle<UiNode>,
    list: Handle<UiNode>,
    add_track: Handle<UiNode>,
    node_selector: Handle<UiNode>,
    property_selector: Handle<UiNode>,
    selected_node: Handle<Node>,
}

impl TrackList {
    fn new(ctx: &mut BuildContext) -> Self {
        let list;
        let add_track;

        let panel = GridBuilder::new(
            WidgetBuilder::new()
                .with_child({
                    list = ListViewBuilder::new(
                        WidgetBuilder::new()
                            .on_row(0)
                            .on_column(0)
                            .with_margin(Thickness::uniform(1.0)),
                    )
                    .build(ctx);
                    list
                })
                .with_child(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .on_row(1)
                            .on_column(0)
                            .with_margin(Thickness::uniform(1.0))
                            .with_child({
                                add_track = ButtonBuilder::new(WidgetBuilder::new())
                                    .with_text("Add Track..")
                                    .build(ctx);
                                add_track
                            }),
                    )
                    .with_orientation(Orientation::Horizontal)
                    .build(ctx),
                ),
        )
        .add_row(Row::stretch())
        .add_row(Row::strict(22.0))
        .add_column(Column::stretch())
        .build(ctx);

        Self {
            panel,
            list,
            add_track,
            node_selector: Default::default(),
            property_selector: Default::default(),
            selected_node: Default::default(),
        }
    }

    fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_scene: Option<&EditorScene>,
        engine: &mut Engine,
    ) {
        let ui = &mut engine.user_interface;

        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.add_track {
                if let Some(editor_scene) = editor_scene {
                    let scene = &engine.scenes[editor_scene.scene];

                    self.node_selector = NodeSelectorWindowBuilder::new(
                        WindowBuilder::new(
                            WidgetBuilder::new().with_width(300.0).with_height(400.0),
                        )
                        .with_title(WindowTitle::text("Select a Node")),
                    )
                    .with_hierarchy(HierarchyNode::from_scene_node(
                        scene.graph.get_root(),
                        editor_scene.editor_objects_root,
                        &scene.graph,
                    ))
                    .build(&mut ui.build_ctx());

                    ui.send_message(WindowMessage::open_modal(
                        self.node_selector,
                        MessageDirection::ToWidget,
                        true,
                    ));
                }
            }
        } else if let Some(WindowMessage::Close) = message.data() {
            if message.destination() == self.node_selector
                || message.destination() == self.property_selector
            {
                ui.send_message(WidgetMessage::remove(
                    message.destination(),
                    MessageDirection::ToWidget,
                ));
            }
        } else if let Some(NodeSelectorMessage::Selection(selection)) = message.data() {
            if message.destination() == self.node_selector {
                if let Some(editor_scene) = editor_scene {
                    let scene = &engine.scenes[editor_scene.scene];
                    if let Some(first) = selection.first() {
                        self.selected_node = *first;

                        self.property_selector = PropertySelectorWindowBuilder::new(
                            WindowBuilder::new(
                                WidgetBuilder::new().with_width(300.0).with_height(400.0),
                            )
                            .open(false),
                        )
                        .with_property_descriptors(object_to_property_tree(
                            "",
                            scene.graph[*first].as_reflect(),
                        ))
                        .build(&mut ui.build_ctx());

                        ui.send_message(WindowMessage::open_modal(
                            self.property_selector,
                            MessageDirection::ToWidget,
                            true,
                        ));
                    }
                }
            }
        } else if let Some(PropertySelectorMessage::Selection(selected_properties)) = message.data()
        {
            if message.destination() == self.property_selector
                && message.direction() == MessageDirection::FromWidget
            {
                if let Some(editor_scene) = editor_scene {
                    let scene = &engine.scenes[editor_scene.scene];
                    if let Some(node) = scene.graph.try_get(self.selected_node) {
                        for property_path in selected_properties {
                            if let Ok(_property) = node.as_reflect().resolve_path(property_path) {
                                // TODO: Add tracks.
                            }
                        }
                    }
                }
            }
        }
    }
}

pub struct AnimationEditor {
    pub window: Handle<UiNode>,
    track_list: TrackList,
    curve_editor: Handle<UiNode>,
    resource: Option<AnimationResource>,
    menu: Menu,
    command_stack: AnimationCommandStack,
}

impl AnimationEditor {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let curve_editor;

        let menu = Menu::new(ctx);
        let track_list = TrackList::new(ctx);

        let payload = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .on_column(0)
                .with_child(track_list.panel)
                .with_child({
                    curve_editor = CurveEditorBuilder::new(
                        WidgetBuilder::new()
                            .on_row(0)
                            .on_column(1)
                            .with_margin(Thickness::uniform(1.0)),
                    )
                    .build(ctx);
                    curve_editor
                }),
        )
        .add_row(Row::stretch())
        .add_column(Column::strict(250.0))
        .add_column(Column::stretch())
        .build(ctx);

        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(menu.menu)
                .with_child(payload),
        )
        .add_row(Row::strict(22.0))
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(600.0).with_height(500.0))
            .with_content(content)
            .open(false)
            .with_title(WindowTitle::text("Animation Editor"))
            .build(ctx);

        Self {
            window,
            track_list,
            curve_editor,
            resource: None,
            menu,
            command_stack: AnimationCommandStack::new(false),
        }
    }

    pub fn open(&self, ui: &UserInterface) {
        ui.send_message(WindowMessage::open(
            self.window,
            MessageDirection::ToWidget,
            true,
        ));
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_scene: Option<&EditorScene>,
        engine: &mut Engine,
    ) {
        self.track_list
            .handle_ui_message(message, editor_scene, engine);

        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.menu.exit {
                engine.user_interface.send_message(WindowMessage::close(
                    self.window,
                    MessageDirection::ToWidget,
                ));
            }
        }
    }
}
