use crate::fyrox::{
    asset::manager::ResourceManager,
    core::{
        algebra::Vector2, futures::executor::block_on, log::Log, math::Rect, pool::ErasedHandle,
        pool::Handle,
    },
    generic_animation::{Animation, AnimationContainer, RootMotionSettings},
    graph::{BaseSceneGraph, PrefabData, SceneGraph, SceneGraphNode},
    gui::{
        border::BorderBuilder,
        button::{Button, ButtonBuilder, ButtonMessage},
        check_box::{CheckBoxBuilder, CheckBoxMessage},
        dropdown_list::{DropdownList, DropdownListBuilder, DropdownListMessage},
        file_browser::{FileSelectorBuilder, FileSelectorMessage, Filter},
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        message::{MessageDirection, UiMessage},
        numeric::{NumericUpDownBuilder, NumericUpDownMessage},
        popup::{Placement, PopupBuilder, PopupMessage},
        stack_panel::StackPanelBuilder,
        text::{TextBuilder, TextMessage},
        text_box::{TextBox, TextBoxBuilder},
        utils::{make_cross, make_simple_tooltip},
        vector_image::{Primitive, VectorImageBuilder},
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
        VerticalAlignment, BRUSH_BRIGHT, BRUSH_LIGHT,
    },
    resource::model::AnimationSource,
};
use crate::{
    animation::{
        command::{
            AddAnimationCommand, RemoveAnimationCommand, ReplaceAnimationCommand,
            SetAnimationEnabledCommand, SetAnimationLoopingCommand, SetAnimationNameCommand,
            SetAnimationRootMotionSettingsCommand, SetAnimationSpeedCommand,
            SetAnimationTimeSliceCommand,
        },
        selection::AnimationSelection,
    },
    command::{Command, CommandGroup},
    gui::make_dropdown_list_option_universal,
    load_image,
    message::MessageSender,
    scene::{
        commands::ChangeSelectionCommand,
        selector::{HierarchyNode, NodeSelectorMessage, NodeSelectorWindowBuilder},
        Selection,
    },
    send_sync_message,
};
use std::path::Path;

enum ImportMode {
    Import,
    Reimport,
}

pub struct Toolbar {
    pub panel: Handle<UiNode>,
    pub play_pause: Handle<UiNode>,
    pub stop: Handle<UiNode>,
    pub speed: Handle<UiNode>,
    pub animations: Handle<UiNode>,
    pub add_animation: Handle<UiNode>,
    pub remove_current_animation: Handle<UiNode>,
    pub rename_current_animation: Handle<UiNode>,
    pub clone_current_animation: Handle<UiNode>,
    pub animation_name: Handle<UiNode>,
    pub preview: Handle<UiNode>,
    pub time_slice_start: Handle<UiNode>,
    pub time_slice_end: Handle<UiNode>,
    pub import: Handle<UiNode>,
    pub reimport: Handle<UiNode>,
    pub node_selector: Handle<UiNode>,
    pub import_file_selector: Handle<UiNode>,
    pub selected_import_root: ErasedHandle,
    pub looping: Handle<UiNode>,
    pub enabled: Handle<UiNode>,
    root_motion_dropdown_area: RootMotionDropdownArea,
    pub root_motion: Handle<UiNode>,
    import_mode: ImportMode,
}

struct RootMotionDropdownArea {
    popup: Handle<UiNode>,
    select_node: Handle<UiNode>,
    enabled: Handle<UiNode>,
    ignore_x: Handle<UiNode>,
    ignore_y: Handle<UiNode>,
    ignore_z: Handle<UiNode>,
    ignore_rotation: Handle<UiNode>,
    node_selector: Handle<UiNode>,
}

impl RootMotionDropdownArea {
    fn new(ctx: &mut BuildContext) -> Self {
        fn text(text: &str, row: usize, ctx: &mut BuildContext) -> Handle<UiNode> {
            TextBuilder::new(
                WidgetBuilder::new()
                    .with_vertical_alignment(VerticalAlignment::Center)
                    .on_row(row)
                    .on_column(0),
            )
            .with_text(text)
            .build(ctx)
        }

        fn check_box(row: usize, ctx: &mut BuildContext) -> Handle<UiNode> {
            CheckBoxBuilder::new(
                WidgetBuilder::new()
                    .with_width(18.0)
                    .with_height(18.0)
                    .with_margin(Thickness::uniform(1.0))
                    .with_vertical_alignment(VerticalAlignment::Center)
                    .with_horizontal_alignment(HorizontalAlignment::Left)
                    .on_row(row)
                    .on_column(1),
            )
            .build(ctx)
        }

        let enabled = check_box(0, ctx);
        let select_node;
        let ignore_x = check_box(2, ctx);
        let ignore_y = check_box(3, ctx);
        let ignore_z = check_box(4, ctx);
        let ignore_rotation = check_box(5, ctx);
        let popup = PopupBuilder::new(
            WidgetBuilder::new()
                .with_width(220.0)
                .with_height(135.0)
                .with_visibility(false),
        )
        .stays_open(false)
        .with_content(
            GridBuilder::new(
                WidgetBuilder::new()
                    .with_margin(Thickness::uniform(2.0))
                    .with_child(text("Enabled", 0, ctx))
                    .with_child(enabled)
                    .with_child(text("Root", 1, ctx))
                    .with_child({
                        select_node = ButtonBuilder::new(
                            WidgetBuilder::new()
                                .with_margin(Thickness::uniform(1.0))
                                .on_row(1)
                                .on_column(1),
                        )
                        .with_text("<Unassigned>")
                        .build(ctx);
                        select_node
                    })
                    .with_child(text("Ignore X", 2, ctx))
                    .with_child(ignore_x)
                    .with_child(text("Ignore Y", 3, ctx))
                    .with_child(ignore_y)
                    .with_child(text("Ignore Z", 4, ctx))
                    .with_child(ignore_z)
                    .with_child(text("Ignore Rotation", 5, ctx))
                    .with_child(ignore_rotation),
            )
            .add_column(Column::strict(90.0))
            .add_column(Column::stretch())
            .add_row(Row::strict(22.0))
            .add_row(Row::strict(22.0))
            .add_row(Row::strict(22.0))
            .add_row(Row::strict(22.0))
            .add_row(Row::strict(22.0))
            .add_row(Row::strict(22.0))
            .add_row(Row::stretch())
            .build(ctx),
        )
        .build(ctx);

        Self {
            popup,
            select_node,
            enabled,
            ignore_x,
            ignore_y,
            ignore_z,
            ignore_rotation,
            node_selector: Default::default(),
        }
    }
}

impl RootMotionDropdownArea {
    pub fn handle_ui_message<G, N>(
        &mut self,
        message: &UiMessage,
        graph: &G,
        sender: &MessageSender,
        ui: &mut UserInterface,
        animation: &Animation<Handle<N>>,
        root: Handle<N>,
        selection: &AnimationSelection<N>,
    ) where
        G: SceneGraph<Node = N>,
        N: SceneGraphNode<SceneGraph = G>,
    {
        let send_command = |settings: Option<RootMotionSettings<Handle<N>>>| {
            sender.do_command(SetAnimationRootMotionSettingsCommand {
                node_handle: selection.animation_player,
                animation_handle: selection.animation,
                value: settings,
            });
        };

        if let Some(CheckBoxMessage::Check(Some(value))) = message.data() {
            if message.direction() == MessageDirection::FromWidget {
                if message.destination() == self.enabled {
                    send_command(value.then(Default::default));
                } else if message.destination() == self.ignore_x {
                    if let Some(settings) = animation.root_motion_settings_ref() {
                        send_command(Some(RootMotionSettings {
                            ignore_x_movement: *value,
                            ..*settings
                        }));
                    }
                } else if message.destination() == self.ignore_y {
                    if let Some(settings) = animation.root_motion_settings_ref() {
                        send_command(Some(RootMotionSettings {
                            ignore_y_movement: *value,
                            ..*settings
                        }));
                    }
                } else if message.destination() == self.ignore_z {
                    if let Some(settings) = animation.root_motion_settings_ref() {
                        send_command(Some(RootMotionSettings {
                            ignore_z_movement: *value,
                            ..*settings
                        }));
                    }
                } else if message.destination() == self.ignore_rotation {
                    if let Some(settings) = animation.root_motion_settings_ref() {
                        send_command(Some(RootMotionSettings {
                            ignore_rotations: *value,
                            ..*settings
                        }));
                    }
                }
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if let Some(settings) = animation.root_motion_settings_ref() {
                if message.destination() == self.select_node {
                    self.node_selector = NodeSelectorWindowBuilder::new(
                        WindowBuilder::new(
                            WidgetBuilder::new().with_width(300.0).with_height(400.0),
                        )
                        .with_title(WindowTitle::text("Select a Root Node"))
                        .open(false),
                    )
                    .build(&mut ui.build_ctx());

                    ui.send_message(NodeSelectorMessage::hierarchy(
                        self.node_selector,
                        MessageDirection::ToWidget,
                        HierarchyNode::from_scene_node(root, Handle::NONE, graph),
                    ));

                    ui.send_message(NodeSelectorMessage::selection(
                        self.node_selector,
                        MessageDirection::ToWidget,
                        if settings.node.is_some() {
                            vec![settings.node.into()]
                        } else {
                            vec![]
                        },
                    ));

                    ui.send_message(WindowMessage::open_modal(
                        self.node_selector,
                        MessageDirection::ToWidget,
                        true,
                        true,
                    ));
                }
            }
        } else if let Some(NodeSelectorMessage::Selection(node_selection)) = message.data() {
            if message.destination() == self.node_selector
                && message.direction() == MessageDirection::FromWidget
            {
                if let Some(settings) = animation.root_motion_settings_ref() {
                    sender.do_command(SetAnimationRootMotionSettingsCommand {
                        node_handle: selection.animation_player,
                        animation_handle: selection.animation,
                        value: Some(RootMotionSettings {
                            node: node_selection
                                .first()
                                .cloned()
                                .map(Handle::from)
                                .unwrap_or_default(),
                            ..*settings
                        }),
                    });
                }
            }
        } else if let Some(WindowMessage::Close) = message.data() {
            if message.destination() == self.node_selector {
                ui.send_message(WidgetMessage::remove(
                    self.node_selector,
                    MessageDirection::ToWidget,
                ));
                self.node_selector = Handle::NONE;
            }
        }
    }

    pub fn sync_to_model<G, N>(
        &self,
        animation: &Animation<Handle<N>>,
        graph: &G,
        ui: &mut UserInterface,
    ) where
        G: SceneGraph<Node = N>,
        N: SceneGraphNode,
    {
        fn sync_checked(ui: &UserInterface, check_box: Handle<UiNode>, checked: bool) {
            send_sync_message(
                ui,
                CheckBoxMessage::checked(check_box, MessageDirection::ToWidget, Some(checked)),
            );
        }

        let root_motion_enabled = animation.root_motion_settings_ref().is_some();

        sync_checked(ui, self.enabled, root_motion_enabled);

        for widget in [
            self.select_node,
            self.ignore_x,
            self.ignore_y,
            self.ignore_z,
            self.ignore_rotation,
        ] {
            send_sync_message(
                ui,
                WidgetMessage::enabled(widget, MessageDirection::ToWidget, root_motion_enabled),
            );
        }

        if let Some(settings) = animation.root_motion_settings_ref() {
            send_sync_message(
                ui,
                TextMessage::text(
                    *ui.node(self.select_node)
                        .query_component::<Button>()
                        .unwrap()
                        .content,
                    MessageDirection::ToWidget,
                    graph
                        .try_get(settings.node)
                        .map(|n| n.name().to_owned())
                        .unwrap_or_else(|| String::from("<Unassigned>")),
                ),
            );

            sync_checked(ui, self.ignore_x, settings.ignore_x_movement);
            sync_checked(ui, self.ignore_y, settings.ignore_y_movement);
            sync_checked(ui, self.ignore_z, settings.ignore_z_movement);
            sync_checked(ui, self.ignore_rotation, settings.ignore_rotations);
        }
    }
}
#[must_use]
pub enum ToolbarAction {
    None,
    EnterPreviewMode,
    LeavePreviewMode,
    SelectAnimation(ErasedHandle),
    PlayPause,
    Stop,
}

impl Toolbar {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let play_pause;
        let stop;
        let speed;
        let animations;
        let add_animation;
        let remove_current_animation;
        let rename_current_animation;
        let clone_current_animation;
        let animation_name;
        let preview;
        let time_slice_start;
        let time_slice_end;
        let import;
        let reimport;
        let looping;
        let enabled;
        let root_motion;
        let panel = BorderBuilder::new(
            WidgetBuilder::new()
                .on_row(0)
                .with_foreground(BRUSH_LIGHT)
                .with_child(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .with_child({
                                animation_name = TextBoxBuilder::new(
                                    WidgetBuilder::new()
                                        .with_width(100.0)
                                        .with_margin(Thickness::uniform(1.0)),
                                )
                                .with_vertical_text_alignment(VerticalAlignment::Center)
                                .with_text("New Animation")
                                .build(ctx);
                                animation_name
                            })
                            .with_child({
                                add_animation = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_width(20.0)
                                        .with_height(20.0)
                                        .with_vertical_alignment(VerticalAlignment::Center)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_tooltip(make_simple_tooltip(
                                            ctx,
                                            "Add New Animation.\n\
                                            Adds new empty animation with the name at \
                                            the right text box.",
                                        )),
                                )
                                .with_text("+")
                                .build(ctx);
                                add_animation
                            })
                            .with_child({
                                import = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_tooltip(make_simple_tooltip(
                                            ctx,
                                            "Import Animation.\n\
                                            Imports an animation from external file (FBX) \
                                            and adds it to the animation player.",
                                        )),
                                )
                                .with_content(
                                    ImageBuilder::new(
                                        WidgetBuilder::new()
                                            .with_width(18.0)
                                            .with_height(18.0)
                                            .with_margin(Thickness::uniform(1.0))
                                            .with_background(BRUSH_BRIGHT),
                                    )
                                    .with_opt_texture(load_image(include_bytes!(
                                        "../../resources/import.png"
                                    )))
                                    .build(ctx),
                                )
                                .build(ctx);
                                import
                            })
                            .with_child({
                                reimport = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_tooltip(make_simple_tooltip(
                                            ctx,
                                            "Reimport Animation.\n\
                                            Imports an animation from external file (FBX) and \
                                            replaces content of the current animation. Use it \
                                            if you need to keep references to the animation valid \
                                            in some animation blending state machine, but just \
                                            replace animation with some other.",
                                        )),
                                )
                                .with_content(
                                    ImageBuilder::new(
                                        WidgetBuilder::new()
                                            .with_width(18.0)
                                            .with_height(18.0)
                                            .with_margin(Thickness::uniform(1.0))
                                            .with_background(BRUSH_BRIGHT),
                                    )
                                    .with_opt_texture(load_image(include_bytes!(
                                        "../../resources/reimport.png"
                                    )))
                                    .build(ctx),
                                )
                                .build(ctx);
                                reimport
                            })
                            .with_child({
                                rename_current_animation = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_enabled(false)
                                        .with_width(20.0)
                                        .with_height(20.0)
                                        .with_vertical_alignment(VerticalAlignment::Center)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_tooltip(make_simple_tooltip(
                                            ctx,
                                            "Rename Selected Animation",
                                        )),
                                )
                                .with_content(
                                    ImageBuilder::new(
                                        WidgetBuilder::new()
                                            .with_width(18.0)
                                            .with_height(18.0)
                                            .with_margin(Thickness::uniform(1.0))
                                            .with_background(BRUSH_BRIGHT),
                                    )
                                    .with_opt_texture(load_image(include_bytes!(
                                        "../../resources/rename.png"
                                    )))
                                    .build(ctx),
                                )
                                .build(ctx);
                                rename_current_animation
                            })
                            .with_child({
                                animations = DropdownListBuilder::new(
                                    WidgetBuilder::new()
                                        .with_width(120.0)
                                        .with_margin(Thickness::uniform(1.0)),
                                )
                                .build(ctx);
                                animations
                            })
                            .with_child({
                                remove_current_animation = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_enabled(false)
                                        .with_width(20.0)
                                        .with_height(20.0)
                                        .with_vertical_alignment(VerticalAlignment::Center)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_tooltip(make_simple_tooltip(
                                            ctx,
                                            "Remove Selected Animation",
                                        )),
                                )
                                .with_content(make_cross(ctx, 14.0, 2.0))
                                .build(ctx);
                                remove_current_animation
                            })
                            .with_child({
                                clone_current_animation = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_enabled(false)
                                        .with_width(20.0)
                                        .with_height(20.0)
                                        .with_vertical_alignment(VerticalAlignment::Center)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_tooltip(make_simple_tooltip(
                                            ctx,
                                            "Clone Selected Animation",
                                        )),
                                )
                                .with_content(
                                    ImageBuilder::new(
                                        WidgetBuilder::new()
                                            .with_width(18.0)
                                            .with_height(18.0)
                                            .with_margin(Thickness::uniform(1.0))
                                            .with_background(BRUSH_BRIGHT),
                                    )
                                    .with_opt_texture(load_image(include_bytes!(
                                        "../../resources/copy.png"
                                    )))
                                    .build(ctx),
                                )
                                .build(ctx);
                                clone_current_animation
                            })
                            .with_child({
                                looping = CheckBoxBuilder::new(
                                    WidgetBuilder::new()
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_tooltip(make_simple_tooltip(
                                        ctx,
                                        "Animation looping. Looped animation will play infinitely.",
                                    )),
                                )
                                .with_content(
                                    TextBuilder::new(
                                        WidgetBuilder::new()
                                            .with_vertical_alignment(VerticalAlignment::Center),
                                    )
                                    .with_text("Loop")
                                    .build(ctx),
                                )
                                .build(ctx);
                                looping
                            })
                            .with_child({
                                enabled = CheckBoxBuilder::new(
                                    WidgetBuilder::new()
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_tooltip(make_simple_tooltip(
                                            ctx,
                                            "Enables or disables the animation.",
                                        )),
                                )
                                .with_content(
                                    TextBuilder::new(
                                        WidgetBuilder::new()
                                            .with_vertical_alignment(VerticalAlignment::Center),
                                    )
                                    .with_text("Enabled")
                                    .build(ctx),
                                )
                                .build(ctx);
                                enabled
                            })
                            .with_child(
                                ImageBuilder::new(
                                    WidgetBuilder::new()
                                        .with_width(18.0)
                                        .with_height(18.0)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_background(BRUSH_BRIGHT),
                                )
                                .with_opt_texture(load_image(include_bytes!(
                                    "../../resources/speed.png"
                                )))
                                .build(ctx),
                            )
                            .with_child({
                                speed = NumericUpDownBuilder::<f32>::new(
                                    WidgetBuilder::new()
                                        .with_enabled(false)
                                        .with_width(50.0)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_tooltip(make_simple_tooltip(
                                            ctx,
                                            "Animation Playback Speed",
                                        )),
                                )
                                .with_value(1.0)
                                .build(ctx);
                                speed
                            })
                            .with_child(
                                ImageBuilder::new(
                                    WidgetBuilder::new()
                                        .with_width(18.0)
                                        .with_height(18.0)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_background(BRUSH_BRIGHT),
                                )
                                .with_opt_texture(load_image(include_bytes!(
                                    "../../resources/time.png"
                                )))
                                .build(ctx),
                            )
                            .with_child({
                                time_slice_start = NumericUpDownBuilder::<f32>::new(
                                    WidgetBuilder::new()
                                        .with_enabled(false)
                                        .with_width(50.0)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_tooltip(make_simple_tooltip(
                                            ctx,
                                            "Start Time of the Animation",
                                        )),
                                )
                                .with_min_value(0.0)
                                .with_value(0.0)
                                .build(ctx);
                                time_slice_start
                            })
                            .with_child({
                                time_slice_end = NumericUpDownBuilder::<f32>::new(
                                    WidgetBuilder::new()
                                        .with_enabled(false)
                                        .with_width(60.0)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_tooltip(make_simple_tooltip(
                                            ctx,
                                            "End Time of the Animation",
                                        )),
                                )
                                .with_min_value(0.0)
                                .with_value(1.0)
                                .build(ctx);
                                time_slice_end
                            })
                            .with_child({
                                root_motion =
                                    ButtonBuilder::new(WidgetBuilder::new().with_tooltip(
                                        make_simple_tooltip(ctx, "Root Motion Settings"),
                                    ))
                                    .with_text("RM")
                                    .build(ctx);
                                root_motion
                            })
                            .with_child({
                                preview = CheckBoxBuilder::new(
                                    WidgetBuilder::new().with_enabled(false).with_margin(
                                        Thickness {
                                            left: 10.0,
                                            top: 1.0,
                                            right: 5.0,
                                            bottom: 1.0,
                                        },
                                    ),
                                )
                                .with_content(
                                    TextBuilder::new(
                                        WidgetBuilder::new()
                                            .with_vertical_alignment(VerticalAlignment::Center),
                                    )
                                    .with_text("Preview")
                                    .build(ctx),
                                )
                                .checked(Some(false))
                                .build(ctx);
                                preview
                            })
                            .with_child({
                                play_pause = ButtonBuilder::new(
                                    WidgetBuilder::new().with_enabled(false).with_margin(
                                        Thickness {
                                            left: 1.0,
                                            top: 1.0,
                                            right: 1.0,
                                            bottom: 1.0,
                                        },
                                    ),
                                )
                                .with_content(
                                    VectorImageBuilder::new(
                                        WidgetBuilder::new()
                                            .with_foreground(BRUSH_BRIGHT)
                                            .with_tooltip(make_simple_tooltip(ctx, "Play/Pause")),
                                    )
                                    .with_primitives(vec![
                                        Primitive::Triangle {
                                            points: [
                                                Vector2::new(0.0, 0.0),
                                                Vector2::new(8.0, 8.0),
                                                Vector2::new(0.0, 16.0),
                                            ],
                                        },
                                        Primitive::RectangleFilled {
                                            rect: Rect::new(10.0, 0.0, 4.0, 16.0),
                                        },
                                        Primitive::RectangleFilled {
                                            rect: Rect::new(15.0, 0.0, 4.0, 16.0),
                                        },
                                    ])
                                    .build(ctx),
                                )
                                .build(ctx);
                                play_pause
                            })
                            .with_child({
                                stop = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_enabled(false)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_tooltip(make_simple_tooltip(ctx, "Stop Playback")),
                                )
                                .with_content(
                                    VectorImageBuilder::new(
                                        WidgetBuilder::new().with_foreground(BRUSH_BRIGHT),
                                    )
                                    .with_primitives(vec![Primitive::RectangleFilled {
                                        rect: Rect::new(0.0, 0.0, 16.0, 16.0),
                                    }])
                                    .build(ctx),
                                )
                                .build(ctx);
                                stop
                            }),
                    )
                    .with_orientation(Orientation::Horizontal)
                    .build(ctx),
                ),
        )
        .with_stroke_thickness(Thickness::uniform(1.0))
        .build(ctx);

        let node_selector = NodeSelectorWindowBuilder::new(
            WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
                .with_title(WindowTitle::text("Select a Target Node"))
                .open(false),
        )
        .build(ctx);

        let file_selector = FileSelectorBuilder::new(
            WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
                .open(false)
                .with_title(WindowTitle::text("Select Animation To Import")),
        )
        .with_filter(Filter::new(|p: &Path| {
            p.is_dir()
                || p.extension().map_or(false, |ext|
                // TODO: Here we allow importing only FBX files, but they can contain
                // multiple animations and it might be good to also add animation selector
                // that will be used to select a particular animation to import.
                ext.to_string_lossy().as_ref() == "fbx")
        }))
        .build(ctx);

        let root_motion_dropdown_area = RootMotionDropdownArea::new(ctx);

        Self {
            panel,
            play_pause,
            stop,
            speed,
            animations,
            add_animation,
            rename_current_animation,
            remove_current_animation,
            animation_name,
            preview,
            time_slice_start,
            time_slice_end,
            clone_current_animation,
            import,
            reimport,
            node_selector,
            import_file_selector: file_selector,
            selected_import_root: Default::default(),
            looping,
            enabled,
            root_motion,
            root_motion_dropdown_area,
            import_mode: ImportMode::Import,
        }
    }

    pub fn handle_ui_message<G, N>(
        &mut self,
        message: &UiMessage,
        sender: &MessageSender,
        graph: &G,
        ui: &mut UserInterface,
        animation_player_handle: Handle<N>,
        animations: &AnimationContainer<Handle<N>>,
        root: Handle<N>,
        selection: &AnimationSelection<N>,
    ) -> ToolbarAction
    where
        G: SceneGraph<Node = N>,
        N: SceneGraphNode<SceneGraph = G>,
    {
        if let Some(animation) = animations.try_get(selection.animation) {
            self.root_motion_dropdown_area
                .handle_ui_message(message, graph, sender, ui, animation, root, selection);
        }

        if let Some(DropdownListMessage::SelectionChanged(Some(index))) = message.data() {
            if message.destination() == self.animations
                && message.direction() == MessageDirection::FromWidget
            {
                let item = ui
                    .node(self.animations)
                    .query_component::<DropdownList>()
                    .unwrap()
                    .items[*index];
                let animation = ui
                    .node(item)
                    .user_data_cloned::<Handle<Animation<Handle<N>>>>()
                    .unwrap();
                sender.do_command(ChangeSelectionCommand::new(Selection::new(
                    AnimationSelection {
                        animation_player: animation_player_handle,
                        animation,
                        entities: vec![],
                    },
                )));
                return ToolbarAction::SelectAnimation(animation.into());
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.play_pause {
                return ToolbarAction::PlayPause;
            } else if message.destination() == self.stop {
                return ToolbarAction::Stop;
            } else if message.destination() == self.root_motion {
                ui.send_message(PopupMessage::placement(
                    self.root_motion_dropdown_area.popup,
                    MessageDirection::ToWidget,
                    Placement::LeftBottom(self.root_motion),
                ));
                ui.send_message(PopupMessage::open(
                    self.root_motion_dropdown_area.popup,
                    MessageDirection::ToWidget,
                ));
            } else if message.destination() == self.remove_current_animation {
                if animations.try_get(selection.animation).is_some() {
                    let group = vec![
                        Command::new(ChangeSelectionCommand::new(Selection::new(
                            AnimationSelection {
                                animation_player: animation_player_handle,
                                animation: Default::default(),
                                entities: vec![],
                            },
                        ))),
                        Command::new(RemoveAnimationCommand::new(
                            animation_player_handle,
                            selection.animation,
                        )),
                    ];

                    sender.do_command(CommandGroup::from(group));
                }
            } else if message.destination() == self.rename_current_animation {
                sender.do_command(SetAnimationNameCommand {
                    node_handle: animation_player_handle,
                    animation_handle: selection.animation,
                    value: ui
                        .node(self.animation_name)
                        .query_component::<TextBox>()
                        .unwrap()
                        .text(),
                });
            } else if message.destination() == self.add_animation {
                let mut animation = Animation::default();
                animation.set_name(
                    ui.node(self.animation_name)
                        .query_component::<TextBox>()
                        .unwrap()
                        .text(),
                );
                sender.do_command(AddAnimationCommand::new(animation_player_handle, animation));
            } else if message.destination() == self.clone_current_animation {
                if let Some(animation) = animations.try_get(selection.animation) {
                    let mut animation_clone = animation.clone();
                    animation_clone.set_name(format!("{} Copy", animation.name()));

                    sender.do_command(AddAnimationCommand::new(
                        animation_player_handle,
                        animation_clone,
                    ));
                }
            }
        } else if let Some(CheckBoxMessage::Check(Some(checked))) = message.data() {
            if message.direction() == MessageDirection::FromWidget {
                if message.destination() == self.preview {
                    return if *checked {
                        ToolbarAction::EnterPreviewMode
                    } else {
                        ToolbarAction::LeavePreviewMode
                    };
                } else if message.destination() == self.looping {
                    sender.do_command(SetAnimationLoopingCommand {
                        node_handle: animation_player_handle,
                        animation_handle: selection.animation,
                        value: *checked,
                    });
                } else if message.destination() == self.enabled {
                    sender.do_command(SetAnimationEnabledCommand {
                        node_handle: animation_player_handle,
                        animation_handle: selection.animation,
                        value: *checked,
                    });
                }
            }
        } else if let Some(NumericUpDownMessage::<f32>::Value(value)) = message.data() {
            if message.direction() == MessageDirection::FromWidget {
                if message.destination() == self.time_slice_start {
                    let mut time_slice = animations[selection.animation].time_slice();
                    time_slice.start = value.min(time_slice.end);
                    sender.do_command(SetAnimationTimeSliceCommand {
                        node_handle: animation_player_handle,
                        animation_handle: selection.animation,
                        value: time_slice,
                    });
                } else if message.destination() == self.time_slice_end {
                    let mut time_slice = animations[selection.animation].time_slice();
                    time_slice.end = value.max(time_slice.start);
                    sender.do_command(SetAnimationTimeSliceCommand {
                        node_handle: animation_player_handle,
                        animation_handle: selection.animation,
                        value: time_slice,
                    });
                } else if message.destination() == self.speed {
                    sender.do_command(SetAnimationSpeedCommand {
                        node_handle: animation_player_handle,
                        animation_handle: selection.animation,
                        value: *value,
                    });
                }
            }
        }

        ToolbarAction::None
    }

    pub fn post_handle_ui_message<P, G, N>(
        &mut self,
        message: &UiMessage,
        sender: &MessageSender,
        ui: &UserInterface,
        animation_player_handle: Handle<N>,
        graph: &G,
        root: Handle<N>,
        editor_selection: &Selection,
        resource_manager: &ResourceManager,
    ) where
        P: PrefabData<Graph = G> + AnimationSource<Node = N, SceneGraph = G, Prefab = P>,
        G: SceneGraph<Node = N, Prefab = P>,
        N: SceneGraphNode<SceneGraph = G, ResourceData = P>,
    {
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.import || message.destination() == self.reimport {
                ui.send_message(NodeSelectorMessage::hierarchy(
                    self.node_selector,
                    MessageDirection::ToWidget,
                    HierarchyNode::from_scene_node(root, Handle::NONE, graph),
                ));

                ui.send_message(WindowMessage::open_modal(
                    self.node_selector,
                    MessageDirection::ToWidget,
                    true,
                    true,
                ));

                if message.destination() == self.reimport {
                    self.import_mode = ImportMode::Reimport;
                } else {
                    self.import_mode = ImportMode::Import;
                }
            }
        } else if let Some(NodeSelectorMessage::Selection(selected_nodes)) = message.data() {
            if message.destination() == self.node_selector
                && message.direction() == MessageDirection::FromWidget
            {
                if let Some(first) = selected_nodes.first() {
                    self.selected_import_root = *first;

                    ui.send_message(WindowMessage::open_modal(
                        self.import_file_selector,
                        MessageDirection::ToWidget,
                        true,
                        true,
                    ));
                    ui.send_message(FileSelectorMessage::root(
                        self.import_file_selector,
                        MessageDirection::ToWidget,
                        Some(std::env::current_dir().unwrap()),
                    ));
                }
            }
        } else if let Some(FileSelectorMessage::Commit(path)) = message.data() {
            if message.destination() == self.import_file_selector {
                match block_on(resource_manager.request::<P>(path)) {
                    Ok(model) => {
                        let model_kind = model.kind();
                        let data = model.data_ref();
                        let mut animations = data.retarget_animations_directly(
                            self.selected_import_root.into(),
                            graph,
                            model_kind,
                        );

                        let file_stem = path
                            .file_stem()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|| "Unnamed".to_string());

                        for (i, animation) in animations.iter_mut().enumerate() {
                            animation.set_name(if i == 0 {
                                file_stem.clone()
                            } else {
                                format!("{} {}", file_stem, i)
                            });
                        }

                        match self.import_mode {
                            ImportMode::Import => {
                                let group = CommandGroup::from(
                                    animations
                                        .into_iter()
                                        .map(|a| {
                                            Command::new(AddAnimationCommand::new(
                                                animation_player_handle,
                                                a,
                                            ))
                                        })
                                        .collect::<Vec<_>>(),
                                );

                                sender.do_command(group);
                            }
                            ImportMode::Reimport => {
                                if let Some(selection) = editor_selection.as_animation() {
                                    if animations.len() > 1 {
                                        Log::warn("More than one animation found! Only first will be used");
                                    }

                                    if !animations.is_empty() {
                                        sender.do_command(ReplaceAnimationCommand {
                                            animation_player: selection.animation_player,
                                            animation_handle: selection.animation,
                                            animation: animations.into_iter().next().unwrap(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                    Err(err) => Log::err(format!(
                        "Failed to load {} animation file! Reason: {:?}",
                        path.display(),
                        err
                    )),
                }
            }
        }
    }

    pub fn clear(&mut self, ui: &UserInterface) {
        ui.send_message(DropdownListMessage::items(
            self.animations,
            MessageDirection::ToWidget,
            vec![],
        ));
    }

    pub fn on_preview_mode_changed(&self, ui: &UserInterface, in_preview_mode: bool) {
        for widget in [self.play_pause, self.stop] {
            ui.send_message(WidgetMessage::enabled(
                widget,
                MessageDirection::ToWidget,
                in_preview_mode,
            ));
        }
    }

    pub fn sync_to_model<G, N>(
        &self,
        animations: &AnimationContainer<Handle<N>>,
        selection: &AnimationSelection<N>,
        graph: &G,
        ui: &mut UserInterface,
        in_preview_mode: bool,
    ) where
        G: SceneGraph<Node = N>,
        N: SceneGraphNode,
    {
        let new_items = animations
            .pair_iter()
            .map(|(h, a)| {
                make_dropdown_list_option_universal(&mut ui.build_ctx(), a.name(), 22.0, h)
            })
            .collect();

        send_sync_message(
            ui,
            DropdownListMessage::items(self.animations, MessageDirection::ToWidget, new_items),
        );

        send_sync_message(
            ui,
            DropdownListMessage::selection(
                self.animations,
                MessageDirection::ToWidget,
                animations
                    .pair_iter()
                    .position(|(h, _)| h == selection.animation),
            ),
        );

        let mut selected_animation_valid = false;
        if let Some(animation) = animations.try_get(selection.animation) {
            self.root_motion_dropdown_area
                .sync_to_model(animation, graph, ui);

            selected_animation_valid = true;
            send_sync_message(
                ui,
                TextMessage::text(
                    self.animation_name,
                    MessageDirection::ToWidget,
                    animation.name().to_string(),
                ),
            );

            send_sync_message(
                ui,
                NumericUpDownMessage::value(
                    self.time_slice_start,
                    MessageDirection::ToWidget,
                    animation.time_slice().start,
                ),
            );
            send_sync_message(
                ui,
                NumericUpDownMessage::max_value(
                    self.time_slice_start,
                    MessageDirection::ToWidget,
                    animation.time_slice().end,
                ),
            );

            send_sync_message(
                ui,
                NumericUpDownMessage::value(
                    self.time_slice_end,
                    MessageDirection::ToWidget,
                    animation.time_slice().end,
                ),
            );
            send_sync_message(
                ui,
                NumericUpDownMessage::min_value(
                    self.time_slice_end,
                    MessageDirection::ToWidget,
                    animation.time_slice().start,
                ),
            );

            send_sync_message(
                ui,
                NumericUpDownMessage::value(
                    self.speed,
                    MessageDirection::ToWidget,
                    animation.speed(),
                ),
            );

            send_sync_message(
                ui,
                CheckBoxMessage::checked(
                    self.looping,
                    MessageDirection::ToWidget,
                    Some(animation.is_loop()),
                ),
            );

            send_sync_message(
                ui,
                CheckBoxMessage::checked(
                    self.enabled,
                    MessageDirection::ToWidget,
                    Some(animation.is_enabled()),
                ),
            );
        }

        for widget in [
            self.preview,
            self.speed,
            self.rename_current_animation,
            self.remove_current_animation,
            self.time_slice_start,
            self.time_slice_end,
            self.clone_current_animation,
            self.looping,
            self.enabled,
            self.root_motion,
        ] {
            send_sync_message(
                ui,
                WidgetMessage::enabled(
                    widget,
                    MessageDirection::ToWidget,
                    selected_animation_valid,
                ),
            );
        }

        for widget in [self.play_pause, self.stop] {
            send_sync_message(
                ui,
                WidgetMessage::enabled(
                    widget,
                    MessageDirection::ToWidget,
                    selected_animation_valid && in_preview_mode,
                ),
            );
        }
    }
}
