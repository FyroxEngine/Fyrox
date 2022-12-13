use crate::{
    animation::{
        command::{
            AddAnimationCommand, RemoveAnimationCommand, SetAnimationNameCommand,
            SetAnimationSpeedCommand, SetAnimationTimeSliceCommand,
        },
        selection::AnimationSelection,
    },
    gui::make_dropdown_list_option_universal,
    load_image,
    scene::{
        commands::{ChangeSelectionCommand, CommandGroup, SceneCommand},
        selector::{HierarchyNode, NodeSelectorMessage, NodeSelectorWindowBuilder},
        EditorScene, Selection,
    },
    send_sync_message, Message,
};
use fyrox::{
    animation::Animation,
    core::{algebra::Vector2, futures::executor::block_on, math::Rect, pool::Handle},
    engine::resource_manager::ResourceManager,
    gui::{
        border::BorderBuilder,
        button::{ButtonBuilder, ButtonMessage},
        check_box::{CheckBoxBuilder, CheckBoxMessage},
        dropdown_list::{DropdownList, DropdownListBuilder, DropdownListMessage},
        file_browser::{FileSelectorBuilder, FileSelectorMessage, Filter},
        image::ImageBuilder,
        message::{MessageDirection, UiMessage},
        numeric::{NumericUpDownBuilder, NumericUpDownMessage},
        stack_panel::StackPanelBuilder,
        text::{TextBuilder, TextMessage},
        text_box::{TextBox, TextBoxBuilder},
        utils::{make_cross, make_simple_tooltip},
        vector_image::{Primitive, VectorImageBuilder},
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, Orientation, Thickness, UiNode, UserInterface, VerticalAlignment,
        BRUSH_BRIGHT, BRUSH_LIGHT,
    },
    scene::{animation::AnimationPlayer, node::Node, Scene},
    utils::log::Log,
};
use std::{path::Path, sync::mpsc::Sender};

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
    pub node_selector: Handle<UiNode>,
    pub file_selector: Handle<UiNode>,
    pub selected_import_root: Handle<Node>,
}

#[must_use]
pub enum ToolbarAction {
    None,
    EnterPreviewMode,
    LeavePreviewMode,
    SelectAnimation(Handle<Animation>),
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
                                            "Add New Animation",
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
                                            Imports an animation from external file (FBX).",
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
                                        "../../resources/embed/import.png"
                                    )))
                                    .build(ctx),
                                )
                                .build(ctx);
                                import
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
                                        "../../resources/embed/rename.png"
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
                                        "../../resources/embed/copy.png"
                                    )))
                                    .build(ctx),
                                )
                                .build(ctx);
                                clone_current_animation
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
                                    "../../resources/embed/speed.png"
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
                                .with_min_value(0.0)
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
                                    "../../resources/embed/time.png"
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
            if let Some(ext) = p.extension() {
                // TODO: Here we allow importing only FBX files, but they can contain
                // multiple animations and it might be good to also add animation selector
                // that will be used to select a particular animation to import.
                ext.to_string_lossy().as_ref() == "fbx"
            } else {
                p.is_dir()
            }
        }))
        .build(ctx);

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
            node_selector,
            file_selector,
            selected_import_root: Default::default(),
        }
    }

    pub fn handle_ui_message(
        &self,
        message: &UiMessage,
        sender: &Sender<Message>,
        ui: &UserInterface,
        animation_player_handle: Handle<Node>,
        animation_player: &mut AnimationPlayer,
        editor_scene: &EditorScene,
        selection: &AnimationSelection,
    ) -> ToolbarAction {
        if let Some(DropdownListMessage::SelectionChanged(Some(index))) = message.data() {
            if message.destination() == self.animations
                && message.direction() == MessageDirection::FromWidget
            {
                let item = ui
                    .node(self.animations)
                    .query_component::<DropdownList>()
                    .unwrap()
                    .items()[*index];
                let animation = ui.node(item).user_data_ref::<Handle<Animation>>().unwrap();
                sender
                    .send(Message::do_scene_command(ChangeSelectionCommand::new(
                        Selection::Animation(AnimationSelection {
                            animation_player: animation_player_handle,
                            animation: *animation,
                            entities: vec![],
                        }),
                        editor_scene.selection.clone(),
                    )))
                    .unwrap();
                return ToolbarAction::SelectAnimation(*animation);
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.play_pause {
                return ToolbarAction::PlayPause;
            } else if message.destination() == self.stop {
                return ToolbarAction::Stop;
            } else if message.destination() == self.remove_current_animation {
                if animation_player
                    .animations()
                    .try_get(selection.animation)
                    .is_some()
                {
                    let group = vec![
                        SceneCommand::new(ChangeSelectionCommand::new(
                            Selection::Animation(AnimationSelection {
                                animation_player: animation_player_handle,
                                animation: Default::default(),
                                entities: vec![],
                            }),
                            editor_scene.selection.clone(),
                        )),
                        SceneCommand::new(RemoveAnimationCommand::new(
                            animation_player_handle,
                            selection.animation,
                        )),
                    ];

                    sender
                        .send(Message::do_scene_command(CommandGroup::from(group)))
                        .unwrap();
                }
            } else if message.destination() == self.rename_current_animation {
                sender
                    .send(Message::do_scene_command(SetAnimationNameCommand {
                        node_handle: animation_player_handle,
                        animation_handle: selection.animation,
                        value: ui
                            .node(self.animation_name)
                            .query_component::<TextBox>()
                            .unwrap()
                            .text(),
                    }))
                    .unwrap();
            } else if message.destination() == self.add_animation {
                let mut animation = Animation::default();
                animation.set_name(
                    ui.node(self.animation_name)
                        .query_component::<TextBox>()
                        .unwrap()
                        .text(),
                );
                sender
                    .send(Message::do_scene_command(AddAnimationCommand::new(
                        animation_player_handle,
                        animation,
                    )))
                    .unwrap();
            } else if message.destination() == self.clone_current_animation {
                if let Some(animation) = animation_player.animations().try_get(selection.animation)
                {
                    let mut animation_clone = animation.clone();
                    animation_clone.set_name(format!("{} Copy", animation.name()));

                    sender
                        .send(Message::do_scene_command(AddAnimationCommand::new(
                            animation_player_handle,
                            animation_clone,
                        )))
                        .unwrap();
                }
            }
        } else if let Some(CheckBoxMessage::Check(Some(checked))) = message.data() {
            if message.destination() == self.preview
                && message.direction() == MessageDirection::FromWidget
            {
                return if *checked {
                    ToolbarAction::EnterPreviewMode
                } else {
                    ToolbarAction::LeavePreviewMode
                };
            }
        } else if let Some(NumericUpDownMessage::<f32>::Value(value)) = message.data() {
            if message.direction() == MessageDirection::FromWidget {
                if message.destination() == self.time_slice_start {
                    let mut time_slice =
                        animation_player.animations()[selection.animation].time_slice();
                    time_slice.start = value.min(time_slice.end);
                    sender
                        .send(Message::do_scene_command(SetAnimationTimeSliceCommand {
                            node_handle: animation_player_handle,
                            animation_handle: selection.animation,
                            value: time_slice,
                        }))
                        .unwrap();
                } else if message.destination() == self.time_slice_end {
                    let mut time_slice =
                        animation_player.animations()[selection.animation].time_slice();
                    time_slice.end = value.max(time_slice.start);
                    sender
                        .send(Message::do_scene_command(SetAnimationTimeSliceCommand {
                            node_handle: animation_player_handle,
                            animation_handle: selection.animation,
                            value: time_slice,
                        }))
                        .unwrap();
                } else if message.destination() == self.speed {
                    sender
                        .send(Message::do_scene_command(SetAnimationSpeedCommand {
                            node_handle: animation_player_handle,
                            animation_handle: selection.animation,
                            value: *value,
                        }))
                        .unwrap();
                }
            }
        }

        ToolbarAction::None
    }

    pub fn post_handle_ui_message(
        &mut self,
        message: &UiMessage,
        sender: &Sender<Message>,
        ui: &UserInterface,
        animation_player_handle: Handle<Node>,
        scene: &Scene,
        editor_scene: &EditorScene,
        resource_manager: &ResourceManager,
    ) {
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.import {
                ui.send_message(NodeSelectorMessage::hierarchy(
                    self.node_selector,
                    MessageDirection::ToWidget,
                    HierarchyNode::from_scene_node(
                        scene.graph.get_root(),
                        editor_scene.editor_objects_root,
                        &scene.graph,
                    ),
                ));

                ui.send_message(WindowMessage::open_modal(
                    self.node_selector,
                    MessageDirection::ToWidget,
                    true,
                ));
            }
        } else if let Some(NodeSelectorMessage::Selection(selected_nodes)) = message.data() {
            if message.destination() == self.node_selector
                && message.direction() == MessageDirection::FromWidget
            {
                if let Some(first) = selected_nodes.first() {
                    self.selected_import_root = *first;

                    ui.send_message(WindowMessage::open_modal(
                        self.file_selector,
                        MessageDirection::ToWidget,
                        true,
                    ));
                    ui.send_message(FileSelectorMessage::root(
                        self.file_selector,
                        MessageDirection::ToWidget,
                        Some(std::env::current_dir().unwrap()),
                    ));
                }
            }
        } else if let Some(FileSelectorMessage::Commit(path)) = message.data() {
            if message.destination() == self.file_selector {
                match block_on(resource_manager.request_model(path)) {
                    Ok(model) => {
                        let mut animations = model
                            .retarget_animations_directly(self.selected_import_root, &scene.graph);

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

                            animation.set_enabled(false);
                        }

                        let group = CommandGroup::from(
                            animations
                                .into_iter()
                                .map(|a| {
                                    SceneCommand::new(AddAnimationCommand::new(
                                        animation_player_handle,
                                        a,
                                    ))
                                })
                                .collect::<Vec<_>>(),
                        );

                        sender.send(Message::do_scene_command(group)).unwrap();
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

    pub fn sync_to_model(
        &self,
        animation_player: &AnimationPlayer,
        selection: &AnimationSelection,
        ui: &mut UserInterface,
        in_preview_mode: bool,
    ) {
        let new_items = animation_player
            .animations()
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
                animation_player
                    .animations()
                    .pair_iter()
                    .position(|(h, _)| h == selection.animation),
            ),
        );

        let mut selected_animation_valid = false;
        if let Some(animation) = animation_player.animations().try_get(selection.animation) {
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
        }

        for widget in [
            self.preview,
            self.speed,
            self.rename_current_animation,
            self.remove_current_animation,
            self.time_slice_start,
            self.time_slice_end,
            self.clone_current_animation,
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
