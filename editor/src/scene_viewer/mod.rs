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
    fyrox::{
        core::{color::Color, math::Rect, pool::Handle, uuid::Uuid},
        engine::Engine,
        fxhash::FxHashMap,
        graph::{BaseSceneGraph, SceneGraph, SceneGraphNode},
        graphics::PolygonFillMode,
        gui::{
            border::BorderBuilder,
            brush::Brush,
            button::{Button, ButtonBuilder, ButtonMessage},
            canvas::CanvasBuilder,
            check_box::{CheckBoxBuilder, CheckBoxMessage},
            decorator::DecoratorMessage,
            dropdown_list::{DropdownList, DropdownListMessage},
            dropdown_menu::DropdownMenuBuilder,
            formatted_text::WrapMode,
            grid::{Column, GridBuilder, Row},
            image::{ImageBuilder, ImageMessage},
            message::{MessageDirection, MouseButton, UiMessage},
            numeric::{NumericUpDownBuilder, NumericUpDownMessage},
            stack_panel::StackPanelBuilder,
            style::{resource::StyleResourceExt, Style},
            tab_control::{TabControl, TabControlBuilder, TabControlMessage, TabDefinition},
            text::{TextBuilder, TextMessage},
            utils::{
                make_dropdown_list_option, make_dropdown_list_option_universal,
                make_dropdown_list_option_with_height, make_image_button_with_tooltip,
                make_simple_tooltip,
            },
            widget::{WidgetBuilder, WidgetMessage},
            window::{WindowBuilder, WindowMessage, WindowTitle},
            BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
            VerticalAlignment,
        },
        resource::texture::TextureResource,
        scene::camera::Projection,
    },
    load_image,
    message::MessageSender,
    scene::container::EditorSceneEntry,
    scene_viewer::gizmo::{SceneGizmo, SceneGizmoAction},
    send_sync_message,
    settings::SettingsMessage,
    utils::enable_widget,
    DropdownListBuilder, GameScene, Message, Mode, SaveSceneConfirmationDialogAction,
    SceneContainer, Settings,
};
use fyrox::core::algebra::Vector2;
use std::{
    ops::Deref,
    sync::mpsc::{self, Receiver},
};
use strum::{IntoEnumIterator, VariantNames};
use strum_macros::{AsRefStr, EnumIter, EnumString, VariantNames};

mod gizmo;

#[derive(Default, Clone, Debug, EnumIter, AsRefStr, EnumString, VariantNames)]
pub enum GraphicsDebugSwitches {
    #[default]
    Shaded,
    Wireframe,
}

struct GridSnappingMenu {
    menu: Handle<UiNode>,
    button: Handle<UiNode>,
    enabled: Handle<UiNode>,
    x_step: Handle<UiNode>,
    y_step: Handle<UiNode>,
    z_step: Handle<UiNode>,
    receiver: Receiver<SettingsMessage>,
}

impl GridSnappingMenu {
    fn new(ctx: &mut BuildContext, settings: &mut Settings) -> Self {
        let (sender, receiver) = mpsc::channel();

        settings.subscribers.push(sender);

        let button;
        let enabled;
        let x_step;
        let y_step;
        let z_step;
        let grid_snap_menu = DropdownMenuBuilder::new(WidgetBuilder::new())
            .with_header({
                button = make_image_button_with_tooltip(
                    ctx,
                    20.0,
                    20.0,
                    load_image!("../../resources/grid_snapping.png"),
                    "Snapping Options",
                    None,
                );
                button
            })
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_margin(Thickness::uniform(2.0))
                        .with_child(
                            TextBuilder::new(WidgetBuilder::new().on_row(0).on_column(0))
                                .with_text("Grid Snapping")
                                .build(ctx),
                        )
                        .with_child({
                            enabled = CheckBoxBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(0)
                                    .on_column(1)
                                    .with_tab_index(Some(0)),
                            )
                            .checked(Some(settings.move_mode_settings.grid_snapping))
                            .build(ctx);
                            enabled
                        })
                        .with_child(
                            TextBuilder::new(WidgetBuilder::new().on_row(1).on_column(0))
                                .with_text("X Step")
                                .build(ctx),
                        )
                        .with_child({
                            x_step = NumericUpDownBuilder::<f32>::new(
                                WidgetBuilder::new()
                                    .on_row(1)
                                    .on_column(1)
                                    .with_tab_index(Some(1)),
                            )
                            .with_value(settings.move_mode_settings.x_snap_step)
                            .build(ctx);
                            x_step
                        })
                        .with_child(
                            TextBuilder::new(WidgetBuilder::new().on_row(2).on_column(0))
                                .with_text("Y Step")
                                .build(ctx),
                        )
                        .with_child({
                            y_step = NumericUpDownBuilder::<f32>::new(
                                WidgetBuilder::new()
                                    .on_row(2)
                                    .on_column(1)
                                    .with_tab_index(Some(2)),
                            )
                            .with_value(settings.move_mode_settings.y_snap_step)
                            .build(ctx);
                            y_step
                        })
                        .with_child(
                            TextBuilder::new(WidgetBuilder::new().on_row(3).on_column(0))
                                .with_text("Z Step")
                                .build(ctx),
                        )
                        .with_child({
                            z_step = NumericUpDownBuilder::<f32>::new(
                                WidgetBuilder::new()
                                    .on_row(3)
                                    .on_column(1)
                                    .with_tab_index(Some(3)),
                            )
                            .with_value(settings.move_mode_settings.z_snap_step)
                            .build(ctx);
                            z_step
                        }),
                )
                .add_row(Row::auto())
                .add_row(Row::auto())
                .add_row(Row::auto())
                .add_row(Row::auto())
                .add_column(Column::auto())
                .add_column(Column::auto())
                .build(ctx),
            )
            .build(ctx);

        Self {
            menu: grid_snap_menu,
            button,
            enabled,
            x_step,
            y_step,
            z_step,
            receiver,
        }
    }

    fn update(&self, settings: &Settings, ui: &UserInterface) {
        for message in self.receiver.try_iter() {
            match message {
                SettingsMessage::Changed => {
                    if let Ok(button) = ui.try_get_of_type::<Button>(self.button) {
                        ui.send_message(DecoratorMessage::selected_brush(
                            *button.decorator,
                            MessageDirection::ToWidget,
                            ui.style.property(Style::BRUSH_BRIGHT_BLUE),
                        ));

                        ui.send_message(DecoratorMessage::select(
                            *button.decorator,
                            MessageDirection::ToWidget,
                            settings.move_mode_settings.grid_snapping,
                        ));
                    }

                    ui.send_message(CheckBoxMessage::checked(
                        self.enabled,
                        MessageDirection::ToWidget,
                        Some(settings.move_mode_settings.grid_snapping),
                    ));

                    ui.send_message(NumericUpDownMessage::value(
                        self.x_step,
                        MessageDirection::ToWidget,
                        settings.move_mode_settings.x_snap_step,
                    ));
                    ui.send_message(NumericUpDownMessage::value(
                        self.y_step,
                        MessageDirection::ToWidget,
                        settings.move_mode_settings.y_snap_step,
                    ));
                    ui.send_message(NumericUpDownMessage::value(
                        self.z_step,
                        MessageDirection::ToWidget,
                        settings.move_mode_settings.z_snap_step,
                    ));
                }
            }
        }
    }

    fn handle_ui_message(&self, message: &UiMessage, settings: &mut Settings) {
        if message.direction() != MessageDirection::FromWidget {
            return;
        }

        if let Some(CheckBoxMessage::Check(Some(value))) = message.data() {
            if message.destination() == self.enabled {
                settings.move_mode_settings.grid_snapping = *value;
            }
        } else if let Some(NumericUpDownMessage::Value(value)) = message.data() {
            if message.destination() == self.x_step {
                settings.move_mode_settings.x_snap_step = *value;
            } else if message.destination() == self.y_step {
                settings.move_mode_settings.y_snap_step = *value;
            } else if message.destination() == self.z_step {
                settings.move_mode_settings.z_snap_step = *value;
            }
        }
    }
}

pub struct SceneViewer {
    frame: Handle<UiNode>,
    window: Handle<UiNode>,
    selection_frame: Handle<UiNode>,
    interaction_modes: FxHashMap<Uuid, Handle<UiNode>>,
    camera_projection: Handle<UiNode>,
    play: Handle<UiNode>,
    build: Handle<UiNode>,
    stop: Handle<UiNode>,
    build_profile: Handle<UiNode>,
    sender: MessageSender,
    interaction_mode_panel: Handle<UiNode>,
    contextual_actions: Handle<UiNode>,
    no_scene_reminder: Handle<UiNode>,
    tab_control: Handle<UiNode>,
    scene_gizmo: SceneGizmo,
    scene_gizmo_image: Handle<UiNode>,
    debug_switches: Handle<UiNode>,
    grid_snap_menu: GridSnappingMenu,
}

impl SceneViewer {
    pub fn new(engine: &mut Engine, sender: MessageSender, settings: &mut Settings) -> Self {
        let scene_gizmo = SceneGizmo::new(engine);

        let ctx = &mut engine.user_interfaces.first_mut().build_ctx();

        let frame;
        let selection_frame;
        let camera_projection;
        let play;
        let build;
        let stop;
        let build_profile;

        let interaction_mode_panel = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .with_vertical_alignment(VerticalAlignment::Top)
                .with_horizontal_alignment(HorizontalAlignment::Left),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        let grid_snap_menu = GridSnappingMenu::new(ctx, settings);

        let debug_switches;
        let contextual_actions = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_height(25.0)
                .on_column(1)
                .with_horizontal_alignment(HorizontalAlignment::Right)
                .with_child({
                    camera_projection = DropdownListBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .with_width(40.0),
                    )
                    .with_items(vec![
                        make_dropdown_list_option_with_height(ctx, "3D", 22.0),
                        make_dropdown_list_option_with_height(ctx, "2D", 22.0),
                    ])
                    .with_close_on_selection(true)
                    .with_selected(0)
                    .build(ctx);
                    camera_projection
                })
                .with_child(grid_snap_menu.menu)
                .with_child({
                    debug_switches = DropdownListBuilder::new(
                        WidgetBuilder::new()
                            .with_width(90.0)
                            .with_margin(Thickness::uniform(1.0)),
                    )
                    .with_items(
                        GraphicsDebugSwitches::iter()
                            .zip(GraphicsDebugSwitches::VARIANTS.iter())
                            .map(|(variant, v)| {
                                make_dropdown_list_option_universal(ctx, v, 22.0, variant)
                            })
                            .collect::<Vec<_>>(),
                    )
                    .with_close_on_selection(true)
                    .with_selected(0)
                    .build(ctx);
                    debug_switches
                }),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        let build_tooltip = "Build Project.\nBuilds the game, but does not run it. Could be \
        useful for hot reloading - change the source code and then press this button. The new version \
        of the game DLL will be built and automagically loaded by the editor and the running game.";

        let top_ribbon = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(interaction_mode_panel)
                .with_child({
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .with_height(25.0)
                            .with_horizontal_alignment(HorizontalAlignment::Right)
                            .with_child({
                                build_profile = DropdownListBuilder::new(
                                    WidgetBuilder::new()
                                        .with_tooltip(make_simple_tooltip(
                                            ctx,
                                            "Current Build Profile\nYou can configure \
                                            build profiles in editor settings.",
                                        ))
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_width(90.0),
                                )
                                .with_items(
                                    settings
                                        .build
                                        .profiles
                                        .iter()
                                        .map(|p| make_dropdown_list_option(ctx, &p.name))
                                        .collect::<Vec<_>>(),
                                )
                                .with_close_on_selection(true)
                                .with_selected(settings.build.selected_profile)
                                .build(ctx);
                                build_profile
                            })
                            .with_child({
                                play = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_width(26.0)
                                        .with_tooltip(make_simple_tooltip(ctx, "Play")),
                                )
                                .with_content(
                                    ImageBuilder::new(
                                        WidgetBuilder::new()
                                            .with_width(16.0)
                                            .with_height(16.0)
                                            .with_margin(Thickness::uniform(4.0))
                                            .with_background(
                                                Brush::Solid(Color::opaque(0, 200, 0)).into(),
                                            ),
                                    )
                                    .with_opt_texture(load_image!("../../resources/play.png"))
                                    .build(ctx),
                                )
                                .build(ctx);
                                play
                            })
                            .with_child({
                                build = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_width(26.0)
                                        .with_tooltip(make_simple_tooltip(ctx, build_tooltip)),
                                )
                                .with_content(
                                    ImageBuilder::new(
                                        WidgetBuilder::new()
                                            .with_width(16.0)
                                            .with_height(16.0)
                                            .with_margin(Thickness::uniform(4.0))
                                            .with_background(
                                                Brush::Solid(Color::opaque(0, 200, 0)).into(),
                                            ),
                                    )
                                    .with_opt_texture(load_image!("../../resources/hammer.png"))
                                    .build(ctx),
                                )
                                .build(ctx);
                                build
                            })
                            .with_child({
                                stop = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_enabled(false)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_width(26.0),
                                )
                                .with_content(
                                    ImageBuilder::new(
                                        WidgetBuilder::new()
                                            .with_width(16.0)
                                            .with_height(16.0)
                                            .with_margin(Thickness::uniform(4.0))
                                            .with_background(
                                                Brush::Solid(Color::opaque(200, 0, 0)).into(),
                                            ),
                                    )
                                    .with_opt_texture(load_image!("../../resources/stop.png"))
                                    .build(ctx),
                                )
                                .build(ctx);
                                stop
                            }),
                    )
                    .with_orientation(Orientation::Horizontal)
                    .build(ctx)
                })
                .with_child(contextual_actions),
        )
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .add_column(Column::auto())
        .add_row(Row::stretch())
        .build(ctx);

        let no_scene_reminder = TextBuilder::new(
            WidgetBuilder::new()
                .with_hit_test_visibility(false)
                .with_foreground(ctx.style.property(Style::BRUSH_DARKEST)),
        )
        .with_text("No scene loaded. Create a new scene (File -> New Scene) or load existing (File -> Load Scene)")
        .with_vertical_text_alignment(VerticalAlignment::Center)
        .with_horizontal_text_alignment(HorizontalAlignment::Center)
        .with_wrap(WrapMode::Word)
        .build(ctx);

        let scene_gizmo_image = ImageBuilder::new(
            WidgetBuilder::new()
                .with_width(85.0)
                .with_height(85.0)
                .with_horizontal_alignment(HorizontalAlignment::Right)
                .with_vertical_alignment(VerticalAlignment::Top)
                .with_margin(Thickness::uniform(1.0)),
        )
        .with_flip(true)
        .with_texture(scene_gizmo.render_target.clone())
        .build(ctx);

        let tab_control;
        let window = WindowBuilder::new(WidgetBuilder::new().with_name("SceneViewer"))
            .can_close(false)
            .can_minimize(false)
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_min_size(Vector2::new(0.0, 21.0))
                        .on_row(0)
                        .with_child(top_ribbon)
                        .with_child({
                            tab_control = TabControlBuilder::new(WidgetBuilder::new().on_row(1))
                                .with_tab_drag(true)
                                .build(ctx);
                            tab_control
                        })
                        .with_child(
                            GridBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(2)
                                    .with_child({
                                        frame = ImageBuilder::new(
                                            WidgetBuilder::new()
                                                .with_child(no_scene_reminder)
                                                .with_child(scene_gizmo_image)
                                                .with_allow_drop(true),
                                        )
                                        .with_flip(true)
                                        .build(ctx);
                                        frame
                                    })
                                    .with_child(
                                        CanvasBuilder::new(WidgetBuilder::new().with_child({
                                            selection_frame = BorderBuilder::new(
                                                WidgetBuilder::new()
                                                    .with_visibility(false)
                                                    .with_background(
                                                        Brush::Solid(Color::from_rgba(
                                                            255, 255, 255, 40,
                                                        ))
                                                        .into(),
                                                    )
                                                    .with_foreground(
                                                        Brush::Solid(Color::opaque(0, 255, 0))
                                                            .into(),
                                                    ),
                                            )
                                            .with_stroke_thickness(Thickness::uniform(1.0).into())
                                            .build(ctx);
                                            selection_frame
                                        }))
                                        .build(ctx),
                                    ),
                            )
                            .add_row(Row::stretch())
                            .add_column(Column::stretch())
                            .build(ctx),
                        ),
                )
                .add_row(Row::strict(30.0))
                .add_row(Row::auto())
                .add_row(Row::stretch())
                .add_column(Column::stretch())
                .build(ctx),
            )
            .with_title(WindowTitle::text("Scene Preview"))
            .with_tab_label("Scene")
            .build(ctx);
        Self {
            sender,
            window,
            frame,
            interaction_modes: Default::default(),
            selection_frame,
            camera_projection,
            play,
            interaction_mode_panel,
            contextual_actions,
            build_profile,
            stop,
            no_scene_reminder,
            tab_control,
            scene_gizmo,
            scene_gizmo_image,
            debug_switches,
            grid_snap_menu,
            build,
        }
    }
}

impl SceneViewer {
    pub fn window(&self) -> Handle<UiNode> {
        self.window
    }

    pub fn frame(&self) -> Handle<UiNode> {
        self.frame
    }

    pub fn selection_frame(&self) -> Handle<UiNode> {
        self.selection_frame
    }

    pub fn handle_message(&mut self, message: &Message, engine: &mut Engine) {
        if let Message::SetInteractionMode(mode) = message {
            if let Some(&active_button) = self.interaction_modes.get(mode) {
                for &mode_button in self.interaction_modes.values() {
                    let decorator = *engine
                        .user_interfaces
                        .first_mut()
                        .node(mode_button)
                        .query_component::<Button>()
                        .unwrap()
                        .decorator;

                    engine
                        .user_interfaces
                        .first_mut()
                        .send_message(DecoratorMessage::select(
                            decorator,
                            MessageDirection::ToWidget,
                            mode_button == active_button,
                        ));
                }
            }
        }
    }

    pub fn sync_interaction_modes(
        &mut self,
        scene: Option<&mut EditorSceneEntry>,
        ui: &mut UserInterface,
    ) {
        // Remove interaction mode buttons first.
        for (_, button) in self.interaction_modes.drain() {
            ui.send_message(WidgetMessage::remove(button, MessageDirection::ToWidget));
        }

        // Create new buttons for each mode.
        if let Some(scene_entry) = scene {
            for mode in scene_entry.interaction_modes.iter_mut() {
                let button = mode.make_button(
                    &mut ui.build_ctx(),
                    scene_entry.current_interaction_mode.unwrap_or_default() == mode.uuid(),
                );
                ui.send_message(WidgetMessage::link(
                    button,
                    MessageDirection::ToWidget,
                    self.interaction_mode_panel,
                ));
                self.interaction_modes.insert(mode.uuid(), button);
            }
        }
    }

    pub fn on_current_scene_changed(
        &mut self,
        new_scene: Option<&mut EditorSceneEntry>,
        engine: &mut Engine,
    ) {
        let ui = engine.user_interfaces.first_mut();
        let index = new_scene
            .as_ref()
            .and_then(|entry| entry.controller.downcast_ref::<GameScene>())
            .map(|game_scene| {
                let scene = &engine.scenes[game_scene.scene];
                match scene.graph[game_scene.camera_controller.camera]
                    .as_camera()
                    .projection()
                {
                    Projection::Perspective(_) => 0,
                    Projection::Orthographic(_) => 1,
                }
            });
        ui.send_message(DropdownListMessage::selection(
            self.camera_projection,
            MessageDirection::ToWidget,
            index,
        ));
        let debug_mode_index = new_scene
            .as_ref()
            .and_then(|s| s.controller.downcast_ref::<GameScene>())
            .map(|s| {
                engine.scenes[s.scene]
                    .rendering_options
                    .polygon_rasterization_mode
            })
            .map(|s| match s {
                PolygonFillMode::Fill => 0,
                PolygonFillMode::Line => 1,
                _ => 0,
            });
        if let Some(debug_mode_index) = debug_mode_index {
            ui.send_message(DropdownListMessage::selection(
                self.debug_switches,
                MessageDirection::ToWidget,
                Some(debug_mode_index),
            ));
        }
        self.sync_interaction_modes(new_scene, ui)
    }

    pub fn handle_ui_message(
        &mut self,
        message: &mut UiMessage,
        engine: &mut Engine,
        scenes: &mut SceneContainer,
        settings: &mut Settings,
        mode: &Mode,
    ) {
        self.grid_snap_menu.handle_ui_message(message, settings);

        let ui = engine.user_interfaces.first_mut();

        if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            for (mode_id, mode_button) in self.interaction_modes.iter() {
                if message.destination() == *mode_button {
                    self.sender.send(Message::SetInteractionMode(*mode_id));
                }
            }

            if message.destination() == self.play {
                self.sender.send(Message::SwitchToBuildMode {
                    play_after_build: true,
                });
            } else if message.destination() == self.build {
                self.sender.send(Message::SwitchToBuildMode {
                    play_after_build: false,
                });
            } else if message.destination() == self.stop {
                self.sender.send(Message::SwitchToEditMode);
            }
        } else if let Some(DropdownListMessage::SelectionChanged(Some(index))) = message.data() {
            if message.direction == MessageDirection::FromWidget {
                if message.destination() == self.camera_projection {
                    if *index == 0 {
                        self.sender.send(Message::SetEditorCameraProjection(
                            Projection::Perspective(Default::default()),
                        ))
                    } else {
                        self.sender.send(Message::SetEditorCameraProjection(
                            Projection::Orthographic(Default::default()),
                        ))
                    }
                } else if message.destination() == self.build_profile {
                    settings.build.selected_profile = *index;
                } else if message.destination() == self.debug_switches {
                    let items = ui
                        .node(self.debug_switches)
                        .component_ref::<DropdownList>()
                        .unwrap()
                        .items
                        .deref();
                    if let Some(item) = items.get(*index) {
                        if let Some(variant) =
                            ui.node(*item).user_data_cloned::<GraphicsDebugSwitches>()
                        {
                            if let Some(entry) = scenes.current_scene_entry_mut() {
                                if let Some(game_scene) =
                                    entry.controller.downcast_ref::<GameScene>()
                                {
                                    let scene = &mut engine.scenes[game_scene.scene];
                                    match variant {
                                        GraphicsDebugSwitches::Shaded => {
                                            scene.rendering_options.polygon_rasterization_mode =
                                                PolygonFillMode::Fill;
                                        }
                                        GraphicsDebugSwitches::Wireframe => {
                                            scene.rendering_options.polygon_rasterization_mode =
                                                PolygonFillMode::Line;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else if let Some(msg) = message.data::<TabControlMessage>() {
            if message.destination() == self.tab_control
                && message.direction() == MessageDirection::FromWidget
            {
                match msg {
                    TabControlMessage::CloseTabByUuid(uuid) => {
                        if let Some(entry) = scenes.entry_by_scene_id(*uuid) {
                            if entry.need_save() {
                                self.sender.send(Message::OpenSaveSceneConfirmationDialog {
                                    id: entry.id,
                                    action: SaveSceneConfirmationDialogAction::CloseScene(entry.id),
                                });
                            } else {
                                self.sender.send(Message::CloseScene(entry.id));
                            }
                        }
                    }
                    TabControlMessage::ActiveTabUuid(Some(uuid)) => {
                        if let Some(entry) = scenes.entry_by_scene_id(*uuid) {
                            self.sender.send(Message::SetCurrentScene(entry.id));
                        }
                    }
                    _ => (),
                }
            }
        }

        if let Some(entry) = scenes.current_scene_entry_mut() {
            if let (Some(msg), Mode::Edit) = (message.data::<WidgetMessage>(), mode) {
                if message.destination() == self.frame() {
                    let screen_bounds = self.frame_bounds(engine.user_interfaces.first());
                    match *msg {
                        WidgetMessage::MouseDown { button, pos, .. } => {
                            engine
                                .user_interfaces
                                .first_mut()
                                .capture_mouse(self.frame());

                            entry.on_mouse_down(button, pos, screen_bounds, engine, settings)
                        }
                        WidgetMessage::MouseUp { button, pos, .. } => {
                            engine.user_interfaces.first_mut().release_mouse_capture();
                            entry.on_mouse_up(button, pos, screen_bounds, engine, settings)
                        }
                        WidgetMessage::MouseWheel { amount, .. } => {
                            entry.on_mouse_wheel(amount, engine, settings);
                        }
                        WidgetMessage::MouseMove { pos, .. } => {
                            entry.on_mouse_move(pos, screen_bounds, engine, settings);
                        }
                        WidgetMessage::KeyUp(key) => {
                            if entry.on_key_up(key, engine, &settings.key_bindings) {
                                message.set_handled(true);
                            }
                        }
                        WidgetMessage::KeyDown(key) => {
                            if entry.on_key_down(key, engine, &settings.key_bindings) {
                                message.set_handled(true);
                            }
                        }
                        WidgetMessage::MouseEnter => {
                            entry.on_mouse_enter(screen_bounds, engine, settings);
                        }
                        WidgetMessage::MouseLeave => {
                            entry.on_mouse_leave(screen_bounds, engine, settings);
                        }
                        WidgetMessage::DragOver(handle) => {
                            entry.on_drag_over(handle, screen_bounds, engine, settings);
                        }
                        WidgetMessage::Drop(handle) => {
                            entry.on_drop(handle, screen_bounds, engine, settings);
                        }
                        _ => {}
                    }
                } else if message.destination() == self.scene_gizmo_image {
                    if let Some(game_scene) = entry.controller.downcast_mut::<GameScene>() {
                        match *msg {
                            WidgetMessage::MouseDown { button, pos, .. } => {
                                if button == MouseButton::Left {
                                    let rel_pos =
                                        pos - ui.node(self.scene_gizmo_image).screen_position();
                                    self.scene_gizmo.drag_context = Some(gizmo::DragContext {
                                        initial_click_pos: rel_pos,
                                        initial_rotation: gizmo::CameraRotation {
                                            pitch: game_scene.camera_controller.pitch(),
                                            yaw: game_scene.camera_controller.yaw(),
                                        },
                                    });
                                    ui.capture_mouse(self.scene_gizmo_image);
                                }
                            }
                            WidgetMessage::MouseUp { pos, button } => {
                                if button == MouseButton::Left {
                                    self.scene_gizmo.drag_context = None;
                                    ui.release_mouse_capture();
                                }
                                let rel_pos =
                                    pos - ui.node(self.scene_gizmo_image).screen_position();
                                if let Some(action) =
                                    self.scene_gizmo.on_click(rel_pos, &engine.scenes)
                                {
                                    match action {
                                        SceneGizmoAction::Rotate(rotation) => {
                                            game_scene.camera_controller.set_pitch(rotation.pitch);
                                            game_scene.camera_controller.set_yaw(rotation.yaw);
                                        }
                                        SceneGizmoAction::SwitchProjection => {
                                            let graph = &engine.scenes[game_scene.scene].graph;
                                            match graph[game_scene.camera_controller.camera]
                                                .as_camera()
                                                .projection()
                                            {
                                                Projection::Perspective(_) => {
                                                    ui.send_message(
                                                        DropdownListMessage::selection(
                                                            self.camera_projection,
                                                            MessageDirection::ToWidget,
                                                            Some(1),
                                                        ),
                                                    );
                                                }
                                                Projection::Orthographic(_) => {
                                                    ui.send_message(
                                                        DropdownListMessage::selection(
                                                            self.camera_projection,
                                                            MessageDirection::ToWidget,
                                                            Some(0),
                                                        ),
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            WidgetMessage::MouseMove { pos, .. } => {
                                let rel_pos = pos
                                    - engine
                                        .user_interfaces
                                        .first()
                                        .node(self.scene_gizmo_image)
                                        .screen_position();
                                self.scene_gizmo.on_mouse_move(
                                    rel_pos,
                                    engine,
                                    &mut game_scene.camera_controller,
                                );
                            }
                            _ => (),
                        }
                    }
                }
            }
        }
    }

    pub fn sync_to_model(&self, scenes: &SceneContainer, engine: &mut Engine) {
        // Sync tabs first.

        let tabs = engine
            .user_interfaces
            .first_mut()
            .node(self.tab_control)
            .query_component::<TabControl>()
            .expect("Must be TabControl!")
            .tabs
            .clone();
        // Remove any excess tabs.
        for tab in tabs.iter() {
            if scenes.iter().all(|s| tab.uuid != s.id) {
                send_sync_message(
                    engine.user_interfaces.first(),
                    TabControlMessage::remove_tab_by_uuid(
                        self.tab_control,
                        MessageDirection::ToWidget,
                        tab.uuid,
                    ),
                );
            }
        }
        // Add any missing tabs.
        for entry in scenes.iter() {
            if tabs.iter().all(|tab| tab.uuid != entry.id) {
                let header = TextBuilder::new(WidgetBuilder::new().with_margin(Thickness {
                    left: 4.0,
                    top: 2.0,
                    right: 4.0,
                    bottom: 2.0,
                }))
                .with_text(entry.name())
                .build(&mut engine.user_interfaces.first_mut().build_ctx());

                send_sync_message(
                    engine.user_interfaces.first(),
                    TabControlMessage::add_tab_with_uuid(
                        self.tab_control,
                        MessageDirection::ToWidget,
                        entry.id,
                        TabDefinition {
                            header,
                            content: Default::default(),
                            can_be_closed: true,
                            user_data: None,
                        },
                    ),
                );
            }
        }
        for tab in tabs.iter() {
            if let Some(scene) = scenes.entry_by_scene_id(tab.uuid) {
                engine
                    .user_interfaces
                    .first_mut()
                    .send_message(TextMessage::text(
                        tab.header_content,
                        MessageDirection::ToWidget,
                        format!(
                            "{}{}",
                            scene.name(),
                            if scene.need_save() { "*" } else { "" }
                        ),
                    ));
            }
        }

        send_sync_message(
            engine.user_interfaces.first(),
            TabControlMessage::active_tab_uuid(
                self.tab_control,
                MessageDirection::ToWidget,
                scenes.current_scene_entry_ref().map(|e| e.id),
            ),
        );

        // Then sync to the current scene.
        if let Some(entry) = scenes.current_scene_entry_ref() {
            self.set_title(
                engine.user_interfaces.first(),
                format!(
                    "Scene Preview - {}",
                    entry
                        .path
                        .as_ref()
                        .map_or("Unnamed Scene".to_string(), |p| p
                            .to_string_lossy()
                            .to_string())
                ),
            );

            self.set_render_target(
                engine.user_interfaces.first(),
                entry.controller.render_target(engine),
            );

            send_sync_message(
                engine.user_interfaces.first(),
                WidgetMessage::visibility(
                    self.scene_gizmo_image,
                    MessageDirection::ToWidget,
                    entry.controller.downcast_ref::<GameScene>().is_some(),
                ),
            );
        }

        send_sync_message(
            engine.user_interfaces.first(),
            WidgetMessage::visibility(
                self.no_scene_reminder,
                MessageDirection::ToWidget,
                scenes.current_scene_controller_ref().is_none(),
            ),
        );
    }

    pub fn on_mode_changed(&self, ui: &UserInterface, mode: &Mode) {
        let enabled = mode.is_edit();
        for widget in [self.interaction_mode_panel, self.contextual_actions] {
            enable_widget(widget, enabled, ui);
        }

        ui.send_message(WidgetMessage::enabled(
            self.play,
            MessageDirection::ToWidget,
            mode.is_edit(),
        ));
        ui.send_message(WidgetMessage::enabled(
            self.stop,
            MessageDirection::ToWidget,
            !mode.is_edit(),
        ));
    }

    pub fn set_render_target(&self, ui: &UserInterface, render_target: Option<TextureResource>) {
        ui.send_message(ImageMessage::texture(
            self.frame,
            MessageDirection::ToWidget,
            render_target,
        ));
    }

    pub fn set_title(&self, ui: &UserInterface, title: String) {
        ui.send_message(WindowMessage::title(
            self.window,
            MessageDirection::ToWidget,
            WindowTitle::text(title),
        ));
    }

    pub fn reset_camera_projection(&self, ui: &UserInterface) {
        // Default camera projection is Perspective.
        ui.send_message(DropdownListMessage::selection(
            self.camera_projection,
            MessageDirection::ToWidget,
            Some(0),
        ));
    }

    pub fn frame_bounds(&self, ui: &UserInterface) -> Rect<f32> {
        ui.node(self.frame).screen_bounds()
    }

    pub fn pre_update(&self, settings: &Settings, engine: &mut Engine) {
        self.grid_snap_menu
            .update(settings, engine.user_interfaces.first());
    }

    pub fn update(&self, game_scene: &GameScene, engine: &mut Engine) {
        self.scene_gizmo.sync_rotations(game_scene, engine);
    }
}
