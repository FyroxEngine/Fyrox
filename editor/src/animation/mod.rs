use crate::{
    animation::{
        command::{
            AddTrackCommand, AnimationCommand, AnimationCommandStack, AnimationEditorContext,
        },
        message::Message,
    },
    scene::{
        property::{
            object_to_property_tree, PropertySelectorMessage, PropertySelectorWindowBuilder,
        },
        selector::{HierarchyNode, NodeSelectorMessage, NodeSelectorWindowBuilder},
        EditorScene,
    },
};
use fyrox::{
    animation::{
        container::{TrackFramesContainer, TrackValueKind},
        definition::{AnimationDefinition, ResourceTrack},
        value::ValueBinding,
    },
    asset::{Resource, ResourceData, ResourceState},
    core::{
        futures::executor::block_on, pool::Handle, reflect::ResolvePath, uuid::Uuid,
        visitor::prelude::*,
    },
    engine::Engine,
    gui::{
        border::BorderBuilder,
        button::{ButtonBuilder, ButtonMessage},
        curve::CurveEditorBuilder,
        decorator::DecoratorBuilder,
        file_browser::{FileBrowserMode, FileSelectorBuilder, FileSelectorMessage, Filter},
        grid::{Column, GridBuilder, Row},
        list_view::{ListView, ListViewBuilder, ListViewMessage},
        menu::{MenuBuilder, MenuItemBuilder, MenuItemContent, MenuItemMessage},
        message::{MessageDirection, UiMessage},
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, Orientation, Thickness, UiNode, UserInterface, VerticalAlignment,
    },
    resource::animation::{AnimationResource, AnimationResourceState},
    scene::node::Node,
    utils::log::Log,
};
use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
    rc::Rc,
    sync::mpsc::{self, Receiver, Sender},
};

mod command;
mod message;

struct Menu {
    menu: Handle<UiNode>,
    new: Handle<UiNode>,
    load: Handle<UiNode>,
    save: Handle<UiNode>,
    save_as: Handle<UiNode>,
    exit: Handle<UiNode>,
    undo: Handle<UiNode>,
    redo: Handle<UiNode>,
    clear_command_stack: Handle<UiNode>,
    save_file_dialog: Handle<UiNode>,
    load_file_dialog: Handle<UiNode>,
}

pub fn make_file_dialog(
    title: &str,
    mode: FileBrowserMode,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    FileSelectorBuilder::new(
        WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
            .with_title(WindowTitle::text(title))
            .open(false),
    )
    .with_mode(mode)
    .with_path("./")
    .with_filter(Filter::new(|p: &Path| {
        if let Some(ext) = p.extension() {
            ext.to_string_lossy().as_ref() == "anim"
        } else {
            p.is_dir()
        }
    }))
    .build(ctx)
}

impl Menu {
    fn new(ctx: &mut BuildContext) -> Self {
        let save_file_dialog = make_file_dialog(
            "Save Animation As",
            FileBrowserMode::Save {
                default_file_name: PathBuf::from("unnamed.anim"),
            },
            ctx,
        );
        let load_file_dialog = make_file_dialog("Load Animation", FileBrowserMode::Open, ctx);

        let new;
        let load;
        let save;
        let save_as;
        let exit;
        let undo;
        let redo;
        let clear_command_stack;
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
                        {
                            clear_command_stack = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text_no_arrow("Redo"))
                                .build(ctx);
                            clear_command_stack
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
            clear_command_stack,
            save_file_dialog,
            load_file_dialog,
        }
    }
}

impl Menu {
    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        ui: &UserInterface,
        sender: &Sender<Message>,
        data_model: Option<&DataModel>,
    ) {
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.new {
                sender.send(Message::NewAnimation).unwrap();
            } else if message.destination() == self.load {
                self.open_load_file_dialog(ui);
            } else if message.destination() == self.save {
                if let Some(data_model) = data_model {
                    if !data_model.saved
                        && data_model.resource.data_ref().path() == PathBuf::default()
                    {
                        self.open_save_file_dialog(ui);
                    } else {
                        sender
                            .send(Message::Save(
                                data_model.resource.data_ref().path().to_path_buf(),
                            ))
                            .unwrap();
                    }
                }
            } else if message.destination() == self.save_as {
                self.open_save_file_dialog(ui);
            } else if message.destination() == self.exit {
                sender.send(Message::Exit).unwrap();
            } else if message.destination() == self.undo {
                sender.send(Message::Undo).unwrap();
            } else if message.destination() == self.redo {
                sender.send(Message::Redo).unwrap();
            } else if message.destination() == self.clear_command_stack {
                sender.send(Message::ClearCommandStack).unwrap();
            }
        } else if let Some(FileSelectorMessage::Commit(path)) = message.data() {
            if message.destination() == self.save_file_dialog {
                sender.send(Message::Save(path.clone())).unwrap();
            } else if message.destination() == self.load_file_dialog {
                sender.send(Message::Load(path.clone())).unwrap();
            }
        }
    }

    pub fn open_load_file_dialog(&self, ui: &UserInterface) {
        ui.send_message(FileSelectorMessage::root(
            self.load_file_dialog,
            MessageDirection::ToWidget,
            std::env::current_dir().ok(),
        ));
        ui.send_message(WindowMessage::open_modal(
            self.load_file_dialog,
            MessageDirection::ToWidget,
            true,
        ));
    }

    pub fn open_save_file_dialog(&self, ui: &UserInterface) {
        ui.send_message(FileSelectorMessage::root(
            self.save_file_dialog,
            MessageDirection::ToWidget,
            std::env::current_dir().ok(),
        ));
        ui.send_message(WindowMessage::open_modal(
            self.save_file_dialog,
            MessageDirection::ToWidget,
            true,
        ));
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

struct TrackViewData {
    id: Uuid,
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
        sender: &Sender<Message>,
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
                            match node.as_reflect().resolve_path(property_path) {
                                Ok(_property) => {
                                    // TODO: Check property type.
                                    let mut track =
                                        ResourceTrack::new(TrackFramesContainer::with_n_curves(
                                            TrackValueKind::Vector3,
                                            3,
                                        ));
                                    track
                                        .set_binding(ValueBinding::Property(property_path.clone()));

                                    sender
                                        .send(Message::DoCommand(AnimationCommand::new(
                                            AddTrackCommand::new(track),
                                        )))
                                        .unwrap();
                                }
                                Err(e) => {
                                    Log::err(format!(
                                        "Invalid property path {}. Error: {:?}!",
                                        property_path, e
                                    ));
                                }
                            }
                        }
                    } else {
                        Log::err("Invalid node handle!");
                    }
                }
            }
        }
    }

    pub fn sync_to_model(&mut self, engine: &mut Engine, definition: &AnimationDefinition) {
        let ui = &mut engine.user_interface;
        let track_views = ui
            .node(self.list)
            .query_component::<ListView>()
            .unwrap()
            .items
            .clone();
        match definition.tracks().len().cmp(&track_views.len()) {
            Ordering::Less => {
                for track_view in track_views.iter() {
                    let track_view_ref = ui.node(*track_view);
                    let track_view_data = track_view_ref.user_data_ref::<TrackViewData>().unwrap();
                    if definition
                        .tracks()
                        .iter()
                        .all(|t| t.id() != track_view_data.id)
                    {
                        ui.send_message(ListViewMessage::remove_item(
                            self.list,
                            MessageDirection::ToWidget,
                            *track_view,
                        ));
                    }
                }
            }
            Ordering::Equal => {
                // Nothing to do.
            }
            Ordering::Greater => {
                for model_track in definition.tracks().iter() {
                    if track_views
                        .iter()
                        .map(|v| ui.node(*v))
                        .all(|v| v.user_data_ref::<TrackViewData>().unwrap().id != model_track.id())
                    {
                        let track_view = DecoratorBuilder::new(BorderBuilder::new(
                            WidgetBuilder::new()
                                .with_user_data(Rc::new(TrackViewData {
                                    id: model_track.id(),
                                }))
                                .with_height(18.0)
                                .with_margin(Thickness::uniform(1.0))
                                .with_child(
                                    TextBuilder::new(WidgetBuilder::new())
                                        .with_text(format!("{}", model_track.binding()))
                                        .with_vertical_text_alignment(VerticalAlignment::Center)
                                        .build(&mut ui.build_ctx()),
                                ),
                        ))
                        .build(&mut ui.build_ctx());

                        ui.send_message(ListViewMessage::add_item(
                            self.list,
                            MessageDirection::ToWidget,
                            track_view,
                        ))
                    }
                }
            }
        }
    }
}

struct DataModel {
    saved: bool,
    resource: AnimationResource,
}

impl DataModel {
    pub fn save(&mut self, path: PathBuf) {
        if !self.saved {
            self.resource.data_ref().set_path(path.clone());
            if let ResourceState::Ok(ref mut state) = *self.resource.state() {
                let mut visitor = Visitor::new();
                state
                    .animation_definition
                    .visit("Definition", &mut visitor)
                    .unwrap();
                visitor.save_binary(&path).unwrap();
            }
            self.saved = true;
        }
    }
}

pub struct AnimationEditor {
    pub window: Handle<UiNode>,
    track_list: TrackList,
    #[allow(dead_code)] // TODO
    curve_editor: Handle<UiNode>,
    data_model: Option<DataModel>,
    menu: Menu,
    command_stack: AnimationCommandStack,
    message_sender: Sender<Message>,
    message_receiver: Receiver<Message>,
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

        let (message_sender, message_receiver) = mpsc::channel();

        Self {
            window,
            track_list,
            curve_editor,
            data_model: None,
            menu,
            command_stack: AnimationCommandStack::new(false),
            message_sender,
            message_receiver,
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
            .handle_ui_message(message, editor_scene, engine, &self.message_sender);
        self.menu.handle_ui_message(
            message,
            &engine.user_interface,
            &self.message_sender,
            self.data_model.as_ref(),
        );
    }

    pub fn update(&mut self, engine: &mut Engine) {
        let mut need_sync = false;
        while let Ok(message) = self.message_receiver.try_recv() {
            match message {
                Message::DoCommand(command) => {
                    if let Some(data_model) = self.data_model.as_mut() {
                        let resource = data_model.resource.data_ref();
                        self.command_stack
                            .do_command(command.0, AnimationEditorContext { resource });
                        data_model.saved = false;
                        need_sync = true;
                    }
                }
                Message::Undo => {
                    if let Some(data_model) = self.data_model.as_mut() {
                        let resource = data_model.resource.data_ref();
                        self.command_stack.redo(AnimationEditorContext { resource });
                        data_model.saved = false;
                        need_sync = true;
                    }
                }
                Message::Redo => {
                    if let Some(data_model) = self.data_model.as_mut() {
                        let resource = data_model.resource.data_ref();
                        self.command_stack.undo(AnimationEditorContext { resource });
                        data_model.saved = false;
                        need_sync = true;
                    }
                }
                Message::ClearCommandStack => {
                    if let Some(data_model) = self.data_model.as_ref() {
                        let resource = data_model.resource.data_ref();
                        self.command_stack
                            .clear(AnimationEditorContext { resource });
                    }
                }
                Message::Exit => {
                    engine.user_interface.send_message(WindowMessage::close(
                        self.window,
                        MessageDirection::ToWidget,
                    ));
                }
                Message::NewAnimation => {
                    self.data_model = Some(DataModel {
                        resource: AnimationResource(Resource::new(ResourceState::Ok(
                            AnimationResourceState::default(),
                        ))),
                        saved: false,
                    });
                    need_sync = true;
                }
                Message::Save(path) => {
                    if let Some(data_model) = self.data_model.as_mut() {
                        data_model.save(path);
                    }
                }
                Message::Load(path) => {
                    if let Ok(animation) = block_on(engine.resource_manager.request_animation(path))
                    {
                        self.data_model = Some(DataModel {
                            saved: true,
                            resource: animation,
                        });
                        need_sync = true;
                    }
                }
            }
        }

        if need_sync {
            self.sync_to_model(engine)
        }
    }

    fn sync_to_model(&mut self, engine: &mut Engine) {
        if let Some(resource) = self.data_model.as_ref() {
            let resource = resource.resource.data_ref();
            self.track_list
                .sync_to_model(engine, &resource.animation_definition);
        }
    }
}
