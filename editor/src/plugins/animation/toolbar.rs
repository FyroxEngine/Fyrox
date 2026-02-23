// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::{
    command::{Command, CommandGroup},
    fyrox::{
        asset::manager::ResourceManager,
        core::{futures::executor::block_on, log::Log, pool::ErasedHandle, pool::Handle},
        generic_animation::{Animation, AnimationContainer, RootMotionSettings},
        graph::{PrefabData, SceneGraph, SceneGraphNode},
        gui::{
            border::BorderBuilder,
            button::{Button, ButtonBuilder, ButtonMessage},
            check_box::{CheckBox, CheckBoxBuilder, CheckBoxMessage},
            dropdown_list::{DropdownList, DropdownListBuilder, DropdownListMessage},
            file_browser::{
                FileSelector, FileSelectorBuilder, FileSelectorMessage, FileType, PathFilter,
            },
            grid::{Column, GridBuilder, Row},
            image::ImageBuilder,
            input::{InputBox, InputBoxBuilder, InputBoxMessage, InputBoxResult},
            message::{MessageDirection, UiMessage},
            numeric::{NumericUpDown, NumericUpDownBuilder, NumericUpDownMessage},
            popup::{Placement, Popup, PopupBuilder, PopupMessage},
            style::{resource::StyleResourceExt, Style},
            text::{Text, TextBuilder, TextMessage},
            toggle::{ToggleButton, ToggleButtonMessage},
            utils::{make_dropdown_list_option_universal, make_simple_tooltip},
            widget::{WidgetBuilder, WidgetMessage},
            window::{WindowAlignment, WindowBuilder, WindowMessage, WindowTitle},
            wrap_panel::WrapPanelBuilder,
            BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
            VerticalAlignment,
        },
        resource::model::AnimationSource,
    },
    load_image,
    message::MessageSender,
    plugins::animation::{
        command::{
            AddAnimationCommand, RemoveAnimationCommand, ReplaceAnimationCommand,
            SetAnimationEnabledCommand, SetAnimationLoopingCommand, SetAnimationNameCommand,
            SetAnimationRootMotionSettingsCommand, SetAnimationSpeedCommand,
            SetAnimationTimeSliceCommand,
        },
        selection::AnimationSelection,
    },
    scene::{
        commands::ChangeSelectionCommand,
        selector::NodeSelectorWindow,
        selector::{AllowedType, HierarchyNode, NodeSelectorMessage, NodeSelectorWindowBuilder},
        Selection,
    },
};
use fyrox::core::color::Color;
use fyrox::gui::utils::ImageButtonBuilder;
use std::any::TypeId;

enum ImportMode {
    Import,
    Reimport,
}

pub struct Toolbar {
    pub top_panel: Handle<UiNode>,
    pub bottom_panel: Handle<UiNode>,
    pub play_pause: Handle<Button>,
    pub stop: Handle<Button>,
    pub speed: Handle<NumericUpDown<f32>>,
    pub animations: Handle<DropdownList>,
    pub add_animation: Handle<Button>,
    pub remove_current_animation: Handle<Button>,
    pub rename_current_animation: Handle<Button>,
    pub rename_animation_input_box: Handle<InputBox>,
    pub clone_current_animation: Handle<Button>,
    pub animation_name_input_box: Handle<InputBox>,
    pub preview: Handle<ToggleButton>,
    pub time_slice_start: Handle<NumericUpDown<f32>>,
    pub time_slice_end: Handle<NumericUpDown<f32>>,
    pub import: Handle<Button>,
    pub reimport: Handle<Button>,
    pub node_selector: Handle<NodeSelectorWindow>,
    pub import_file_selector: Handle<FileSelector>,
    pub selected_import_root: ErasedHandle,
    pub looping: Handle<ToggleButton>,
    pub enabled: Handle<CheckBox>,
    root_motion_dropdown_area: RootMotionDropdownArea,
    pub root_motion: Handle<Button>,
    import_mode: ImportMode,
}

struct RootMotionDropdownArea {
    popup: Handle<Popup>,
    select_node: Handle<Button>,
    enabled: Handle<CheckBox>,
    ignore_x: Handle<CheckBox>,
    ignore_y: Handle<CheckBox>,
    ignore_z: Handle<CheckBox>,
    ignore_rotation: Handle<CheckBox>,
    node_selector: Handle<NodeSelectorWindow>,
}

impl RootMotionDropdownArea {
    fn new(ctx: &mut BuildContext) -> Self {
        fn text(text: &str, row: usize, ctx: &mut BuildContext) -> Handle<Text> {
            TextBuilder::new(WidgetBuilder::new().on_row(row).on_column(0))
                .with_vertical_text_alignment(VerticalAlignment::Center)
                .with_text(text)
                .build(ctx)
        }

        fn check_box(row: usize, ctx: &mut BuildContext) -> Handle<CheckBox> {
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
                    .with_allowed_types(
                        [AllowedType {
                            id: TypeId::of::<N>(),
                            name: std::any::type_name::<N>().to_string(),
                        }]
                        .into_iter()
                        .collect(),
                    )
                    .build(&mut ui.build_ctx());

                    ui.send(
                        self.node_selector,
                        NodeSelectorMessage::Hierarchy(HierarchyNode::from_scene_node(
                            root,
                            Handle::NONE,
                            graph,
                        )),
                    );

                    ui.send(
                        self.node_selector,
                        NodeSelectorMessage::Selection(if settings.node.is_some() {
                            vec![settings.node.into()]
                        } else {
                            vec![]
                        }),
                    );

                    ui.send(
                        self.node_selector,
                        WindowMessage::Open {
                            alignment: WindowAlignment::Center,
                            modal: true,
                            focus_content: true,
                        },
                    );
                }
            }
        } else if let Some(NodeSelectorMessage::Selection(node_selection)) =
            message.data_from(self.node_selector)
        {
            if let Some(settings) = animation.root_motion_settings_ref() {
                sender.do_command(SetAnimationRootMotionSettingsCommand {
                    node_handle: selection.animation_player,
                    animation_handle: selection.animation,
                    value: Some(RootMotionSettings {
                        node: node_selection
                            .first()
                            .cloned()
                            .map(|selected| selected.handle.into())
                            .unwrap_or_default(),
                        ..*settings
                    }),
                });
            }
        } else if let Some(WindowMessage::Close) = message.data() {
            if message.destination() == self.node_selector {
                ui.send(self.node_selector, WidgetMessage::Remove);
                self.node_selector = Handle::NONE;
            }
        }
    }

    pub fn destroy(self, ui: &UserInterface) {
        ui.send(self.popup, WidgetMessage::Remove);
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
        fn sync_checked(ui: &UserInterface, check_box: Handle<CheckBox>, checked: bool) {
            ui.send_sync(check_box, CheckBoxMessage::Check(Some(checked)));
        }

        let root_motion_enabled = animation.root_motion_settings_ref().is_some();

        sync_checked(ui, self.enabled, root_motion_enabled);

        for widget in [
            self.select_node.to_base::<UiNode>(),
            self.ignore_x.to_base(),
            self.ignore_y.to_base(),
            self.ignore_z.to_base(),
            self.ignore_rotation.to_base(),
        ] {
            ui.send_sync(widget, WidgetMessage::Enabled(root_motion_enabled));
        }

        if let Some(settings) = animation.root_motion_settings_ref() {
            let content = *ui[self.select_node].content;
            ui.send_sync(
                content,
                TextMessage::Text(
                    graph
                        .try_get_node(settings.node)
                        .map(|n| n.name().to_owned())
                        .unwrap_or_else(|_| String::from("<Unassigned>")),
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
        let reimport_tooltip =
            "Reimport Animation.\nImports an animation from external file (FBX/GLTF) and replaces \
            content of the current animation. Use it if you need to keep references to the \
            animation valid in some animation blending state machine, but just replace animation \
            with some other.";
        let import_tooltip =
            "Import Animation.\nImports an animation from external file (FBX/GLTF) and adds it to \
            the animation player.";
        let rename_animation_tooltip = "Rename Selected Animation";
        let add_animation_tooltip = "Add New Animation";

        let play_pause;
        let stop;
        let speed;
        let animations;
        let add_animation;
        let remove_current_animation;
        let rename_current_animation;
        let clone_current_animation;
        let preview;
        let time_slice_start;
        let time_slice_end;
        let import;
        let reimport;
        let looping;
        let enabled;
        let root_motion;
        let top_panel = BorderBuilder::new(
            WidgetBuilder::new()
                .on_row(0)
                .with_foreground(ctx.style.property(Style::BRUSH_LIGHT))
                .with_child(
                    WrapPanelBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .with_child({
                                add_animation = ImageButtonBuilder::default()
                                    .with_image_color(Color::GREEN)
                                    .with_image(load_image!("../../../resources/add.png"))
                                    .with_tooltip(add_animation_tooltip)
                                    .build_button(ctx);
                                add_animation
                            })
                            .with_child({
                                import = ImageButtonBuilder::default()
                                    .with_image_color(Color::PALE_TURQUOISE)
                                    .with_image(load_image!("../../../resources/import.png"))
                                    .with_tooltip(import_tooltip)
                                    .build_button(ctx);
                                import
                            })
                            .with_child({
                                animations = DropdownListBuilder::new(
                                    WidgetBuilder::new()
                                        .with_width(120.0)
                                        .with_margin(Thickness {
                                            left: 10.0,
                                            top: 1.0,
                                            right: 1.0,
                                            bottom: 1.0,
                                        }),
                                )
                                .build(ctx);
                                animations
                            })
                            .with_child({
                                reimport = ImageButtonBuilder::default()
                                    .with_image_color(Color::DEEP_SKY_BLUE)
                                    .with_image(load_image!("../../../resources/reimport.png"))
                                    .with_tooltip(reimport_tooltip)
                                    .build_button(ctx);
                                reimport
                            })
                            .with_child({
                                rename_current_animation = ImageButtonBuilder::default()
                                    .with_image_color(Color::ORANGE)
                                    .with_image(load_image!("../../../resources/rename.png"))
                                    .with_tooltip(rename_animation_tooltip)
                                    .build_button(ctx);
                                rename_current_animation
                            })
                            .with_child({
                                remove_current_animation = ImageButtonBuilder::default()
                                    .with_image_color(Color::ORANGE_RED)
                                    .with_image(load_image!("../../../resources/cross.png"))
                                    .with_tooltip("Remove Selected Animation")
                                    .build_button(ctx);
                                remove_current_animation
                            })
                            .with_child({
                                clone_current_animation = ImageButtonBuilder::default()
                                    .with_image_color(Color::LIGHT_GOLDEN_ROD_YELLOW)
                                    .with_image(load_image!("../../../resources/copy.png"))
                                    .with_tooltip("Clone Selected Animation")
                                    .build_button(ctx);
                                clone_current_animation
                            })
                            .with_child({
                                root_motion = ImageButtonBuilder::default()
                                    .with_image(load_image!("../../../resources/root_motion.png"))
                                    .with_tooltip("Root Motion Settings")
                                    .build_button(ctx);
                                root_motion
                            })
                            .with_child({
                                looping = ImageButtonBuilder::default()
                                    .with_image(load_image!("../../../resources/loop.png"))
                                    .with_tooltip(
                                        "Animation looping. Looped animation will play infinitely.",
                                    )
                                    .build_toggle(ctx);
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
                                    TextBuilder::new(WidgetBuilder::new())
                                        .with_vertical_text_alignment(VerticalAlignment::Center)
                                        .with_text("Enabled")
                                        .build(ctx),
                                )
                                .build(ctx);
                                enabled
                            }),
                    )
                    .with_orientation(Orientation::Horizontal)
                    .build(ctx),
                ),
        )
        .with_corner_radius(3.0.into())
        .with_stroke_thickness(Thickness::uniform(1.0).into())
        .build(ctx)
        .to_base();

        let bottom_panel = BorderBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .with_foreground(ctx.style.property(Style::BRUSH_LIGHT))
                .with_child(
                    WrapPanelBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .with_child({
                                preview = ImageButtonBuilder::default()
                                    .with_image(load_image!("../../../resources/eye.png"))
                                    .with_tooltip("Preview")
                                    .build_toggle(ctx);
                                preview
                            })
                            .with_child({
                                play_pause = ImageButtonBuilder::default()
                                    .with_image_color(Color::GREEN)
                                    .with_image(load_image!("../../../resources/play_pause.png"))
                                    .with_tooltip("Play/Pause")
                                    .build_button(ctx);
                                play_pause
                            })
                            .with_child({
                                stop = ImageButtonBuilder::default()
                                    .with_image_color(Color::ORANGE_RED)
                                    .with_image(load_image!("../../../resources/stop.png"))
                                    .with_tooltip("Stop Playback")
                                    .build_button(ctx);
                                stop
                            })
                            .with_child(
                                ImageBuilder::new(
                                    WidgetBuilder::new()
                                        .with_width(18.0)
                                        .with_height(18.0)
                                        .with_margin(Thickness {
                                            left: 10.0,
                                            top: 1.0,
                                            right: 1.0,
                                            bottom: 1.0,
                                        })
                                        .with_background(ctx.style.property(Style::BRUSH_BRIGHT)),
                                )
                                .with_opt_texture(load_image!("../../../resources/speed.png"))
                                .build(ctx),
                            )
                            .with_child({
                                speed = NumericUpDownBuilder::<f32>::new(
                                    WidgetBuilder::new()
                                        .with_enabled(false)
                                        .with_width(60.0)
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
                                        .with_margin(Thickness {
                                            left: 10.0,
                                            top: 1.0,
                                            right: 1.0,
                                            bottom: 1.0,
                                        })
                                        .with_background(ctx.style.property(Style::BRUSH_BRIGHT)),
                                )
                                .with_opt_texture(load_image!("../../../resources/time.png"))
                                .build(ctx),
                            )
                            .with_child({
                                time_slice_start = NumericUpDownBuilder::<f32>::new(
                                    WidgetBuilder::new()
                                        .with_enabled(false)
                                        .with_width(60.0)
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
                            }),
                    )
                    .with_orientation(Orientation::Horizontal)
                    .build(ctx),
                ),
        )
        .with_corner_radius(3.0.into())
        .with_stroke_thickness(Thickness::uniform(1.0).into())
        .build(ctx)
        .to_base();

        let import_file_selector = FileSelectorBuilder::new(
            WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
                .open(false)
                .with_title(WindowTitle::text("Select Animation To Import")),
        )
        .with_filter(
            // TODO: Here we allow importing only FBX and GLTF files, but they can contain
            // multiple animations and it might be good to also add animation selector
            // that will be used to select a particular animation to import.
            PathFilter::new()
                .with_file_type(FileType::new_extension("fbx"))
                .with_file_type(FileType::new_extension("gltf"))
                .with_file_type(FileType::new_extension("glb")),
        )
        .build(ctx);

        let root_motion_dropdown_area = RootMotionDropdownArea::new(ctx);

        Self {
            top_panel,
            bottom_panel,
            play_pause,
            stop,
            speed,
            animations,
            add_animation,
            rename_current_animation,
            remove_current_animation,
            animation_name_input_box: Default::default(),
            preview,
            time_slice_start,
            time_slice_end,
            clone_current_animation,
            import,
            reimport,
            node_selector: Default::default(),
            import_file_selector,
            selected_import_root: Default::default(),
            looping,
            enabled,
            root_motion,
            root_motion_dropdown_area,
            import_mode: ImportMode::Import,
            rename_animation_input_box: Default::default(),
        }
    }

    pub fn destroy(self, ui: &UserInterface) {
        ui.send(self.node_selector, WidgetMessage::Remove);
        ui.send(self.import_file_selector, WidgetMessage::Remove);
        self.root_motion_dropdown_area.destroy(ui);
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
        if let Ok(animation) = animations.try_get(selection.animation) {
            self.root_motion_dropdown_area
                .handle_ui_message(message, graph, sender, ui, animation, root, selection);
        }

        if let Some(DropdownListMessage::Selection(Some(index))) =
            message.data_from(self.animations)
        {
            let item = ui[self.animations].items[*index];
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
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.play_pause {
                return ToolbarAction::PlayPause;
            } else if message.destination() == self.stop {
                return ToolbarAction::Stop;
            } else if message.destination() == self.root_motion {
                ui.send(
                    self.root_motion_dropdown_area.popup,
                    PopupMessage::Placement(Placement::LeftBottom(self.root_motion.to_base())),
                );
                ui.send(self.root_motion_dropdown_area.popup, PopupMessage::Open);
            } else if message.destination() == self.remove_current_animation {
                if animations.try_get(selection.animation).is_ok() {
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
                self.rename_animation_input_box = InputBoxBuilder::new(
                    WindowBuilder::new(WidgetBuilder::new().with_width(320.0).with_height(120.0))
                        .with_title(WindowTitle::text("Rename Animation"))
                        .open(false),
                )
                .with_text("Type the new name for the selected animation:")
                .with_value(
                    animations
                        .try_get(selection.animation)
                        .ok()
                        .map(|a| a.name().to_string())
                        .unwrap_or_else(|| "Animation".to_string()),
                )
                .build(&mut ui.build_ctx());
                ui.send(
                    self.rename_animation_input_box,
                    InputBoxMessage::open_as_is(),
                );
            } else if message.destination() == self.add_animation {
                self.animation_name_input_box = InputBoxBuilder::new(
                    WindowBuilder::new(WidgetBuilder::new().with_width(320.0).with_height(120.0))
                        .with_title(WindowTitle::text("New Animation Name"))
                        .open(false),
                )
                .with_text("Type the name for the new animation:")
                .with_value("Animation".to_string())
                .build(&mut ui.build_ctx());
                ui.send(self.animation_name_input_box, InputBoxMessage::open_as_is());
            } else if message.destination() == self.clone_current_animation {
                if let Ok(animation) = animations.try_get(selection.animation) {
                    let mut animation_clone = animation.clone();
                    animation_clone.set_name(format!("{} Copy", animation.name()));

                    sender.do_command(AddAnimationCommand::new(
                        animation_player_handle,
                        animation_clone,
                    ));
                }
            }
        } else if let Some(CheckBoxMessage::Check(Some(checked))) = message.data_from(self.enabled)
        {
            sender.do_command(SetAnimationEnabledCommand {
                node_handle: animation_player_handle,
                animation_handle: selection.animation,
                value: *checked,
            });
        } else if let Some(ToggleButtonMessage::Toggled(toggled)) = message.data_from(self.looping)
        {
            sender.do_command(SetAnimationLoopingCommand {
                node_handle: animation_player_handle,
                animation_handle: selection.animation,
                value: *toggled,
            });
        } else if let Some(ToggleButtonMessage::Toggled(toggled)) = message.data_from(self.preview)
        {
            return if *toggled {
                ToolbarAction::EnterPreviewMode
            } else {
                ToolbarAction::LeavePreviewMode
            };
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
        } else if let Some(InputBoxMessage::Close(InputBoxResult::Ok(name))) =
            message.data_from(self.rename_animation_input_box)
        {
            sender.do_command(SetAnimationNameCommand {
                node_handle: animation_player_handle,
                animation_handle: selection.animation,
                value: name.clone(),
            });
        } else if let Some(InputBoxMessage::Close(InputBoxResult::Ok(name))) =
            message.data_from(self.animation_name_input_box)
        {
            let mut animation = Animation::default();
            animation.set_name(name);
            sender.do_command(AddAnimationCommand::new(animation_player_handle, animation));
        }

        ToolbarAction::None
    }

    pub fn post_handle_ui_message<P, G, N>(
        &mut self,
        message: &UiMessage,
        sender: &MessageSender,
        ui: &mut UserInterface,
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
                self.node_selector = NodeSelectorWindowBuilder::new(
                    WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
                        .with_remove_on_close(true)
                        .with_title(WindowTitle::text("Select a Target Node"))
                        .open(false),
                )
                .with_allowed_types(
                    [AllowedType {
                        id: TypeId::of::<N>(),
                        name: std::any::type_name::<N>().to_string(),
                    }]
                    .into_iter()
                    .collect(),
                )
                .build(&mut ui.build_ctx());

                ui.send(
                    self.node_selector,
                    NodeSelectorMessage::Hierarchy(HierarchyNode::from_scene_node(
                        root,
                        Handle::NONE,
                        graph,
                    )),
                );

                ui.send(
                    self.node_selector,
                    WindowMessage::Open {
                        alignment: WindowAlignment::Center,
                        modal: true,
                        focus_content: true,
                    },
                );

                if message.destination() == self.reimport {
                    self.import_mode = ImportMode::Reimport;
                } else {
                    self.import_mode = ImportMode::Import;
                }
            }
        } else if let Some(NodeSelectorMessage::Selection(selected_nodes)) =
            message.data_from(self.node_selector)
        {
            if let Some(first) = selected_nodes.first() {
                self.selected_import_root = first.handle;

                ui.send(
                    self.import_file_selector,
                    WindowMessage::Open {
                        alignment: WindowAlignment::Center,
                        modal: true,
                        focus_content: true,
                    },
                );
                ui.send(
                    self.import_file_selector,
                    FileSelectorMessage::Root(Some(resource_manager.registry_folder())),
                );
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
                                format!("{file_stem} {i}")
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
        ui.send_sync(self.animations, DropdownListMessage::Items(vec![]));
    }

    pub fn on_preview_mode_changed(&self, ui: &UserInterface, in_preview_mode: bool) {
        for widget in [self.play_pause, self.stop] {
            ui.send(widget, WidgetMessage::Enabled(in_preview_mode));
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

        ui.send_sync(self.animations, DropdownListMessage::Items(new_items));
        ui.send_sync(
            self.animations,
            DropdownListMessage::Selection(
                animations
                    .pair_iter()
                    .position(|(h, _)| h == selection.animation),
            ),
        );

        let mut selected_animation_valid = false;
        if let Ok(animation) = animations.try_get(selection.animation) {
            self.root_motion_dropdown_area
                .sync_to_model(animation, graph, ui);

            selected_animation_valid = true;

            ui.send_sync(
                self.time_slice_start,
                NumericUpDownMessage::Value(animation.time_slice().start),
            );
            ui.send_sync(
                self.time_slice_start,
                NumericUpDownMessage::MaxValue(animation.time_slice().end),
            );

            ui.send_sync(
                self.time_slice_end,
                NumericUpDownMessage::Value(animation.time_slice().end),
            );
            ui.send_sync(
                self.time_slice_end,
                NumericUpDownMessage::MinValue(animation.time_slice().start),
            );

            ui.send_sync(self.speed, NumericUpDownMessage::Value(animation.speed()));
            ui.send_sync(
                self.looping,
                ToggleButtonMessage::Toggled(animation.is_loop()),
            );
            ui.send_sync(
                self.enabled,
                CheckBoxMessage::Check(Some(animation.is_enabled())),
            );
        }

        for widget in [
            self.preview.to_base::<UiNode>(),
            self.speed.to_base(),
            self.rename_current_animation.to_base(),
            self.remove_current_animation.to_base(),
            self.time_slice_start.to_base(),
            self.time_slice_end.to_base(),
            self.clone_current_animation.to_base(),
            self.looping.to_base(),
            self.enabled.to_base(),
            self.root_motion.to_base(),
        ] {
            ui.send_sync(widget, WidgetMessage::Enabled(selected_animation_valid));
        }

        for widget in [self.play_pause, self.stop] {
            ui.send_sync(
                widget,
                WidgetMessage::Enabled(selected_animation_valid && in_preview_mode),
            );
        }
    }
}
