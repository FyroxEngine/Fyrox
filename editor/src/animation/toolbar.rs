use crate::{
    animation::{
        command::{AddAnimationCommand, RemoveAnimationCommand},
        selection::AnimationSelection,
    },
    gui::make_dropdown_list_option_universal,
    scene::{
        commands::{ChangeSelectionCommand, CommandGroup, SceneCommand},
        EditorScene, Selection,
    },
    Message,
};
use fyrox::utils::log::Log;
use fyrox::{
    animation::Animation,
    core::{algebra::Vector2, math::Rect, pool::Handle},
    gui::{
        border::BorderBuilder,
        button::{ButtonBuilder, ButtonMessage},
        check_box::{CheckBoxBuilder, CheckBoxMessage},
        dropdown_list::{DropdownList, DropdownListBuilder, DropdownListMessage},
        message::{MessageDirection, UiMessage},
        numeric::NumericUpDownBuilder,
        stack_panel::StackPanelBuilder,
        text::{TextBuilder, TextMessage},
        text_box::{TextBox, TextBoxBuilder},
        utils::{make_arrow, make_cross, make_simple_tooltip, ArrowDirection},
        vector_image::{Primitive, VectorImageBuilder},
        widget::{WidgetBuilder, WidgetMessage},
        BuildContext, Orientation, Thickness, UiNode, UserInterface, VerticalAlignment,
        BRUSH_BRIGHT, BRUSH_LIGHT,
    },
    scene::{animation::AnimationPlayer, node::Node},
};
use std::sync::mpsc::Sender;

pub struct Toolbar {
    pub panel: Handle<UiNode>,
    pub play_pause: Handle<UiNode>,
    pub stop: Handle<UiNode>,
    pub speed: Handle<UiNode>,
    pub animations: Handle<UiNode>,
    pub add_animation: Handle<UiNode>,
    pub remove_current_animation: Handle<UiNode>,
    pub rename_current_animation: Handle<UiNode>,
    pub animation_name: Handle<UiNode>,
    pub preview: Handle<UiNode>,
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
        let animation_name;
        let preview;
        let panel = BorderBuilder::new(
            WidgetBuilder::new()
                .on_row(0)
                .with_foreground(BRUSH_LIGHT)
                .with_child(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .with_child({
                                preview = CheckBoxBuilder::new(
                                    WidgetBuilder::new().with_enabled(false).with_margin(
                                        Thickness {
                                            left: 1.0,
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
                                .checked(Some(true))
                                .build(ctx);
                                preview
                            })
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
                                .with_content(make_arrow(ctx, ArrowDirection::Right, 14.0))
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
                                play_pause = ButtonBuilder::new(
                                    WidgetBuilder::new().with_enabled(false).with_margin(
                                        Thickness {
                                            left: 16.0,
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
                            })
                            .with_child({
                                speed = NumericUpDownBuilder::<f32>::new(
                                    WidgetBuilder::new()
                                        .with_enabled(false)
                                        .with_width(80.0)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_tooltip(make_simple_tooltip(
                                            ctx,
                                            "Preview Playback Speed",
                                        )),
                                )
                                .with_value(1.0)
                                .build(ctx);
                                speed
                            }),
                    )
                    .with_orientation(Orientation::Horizontal)
                    .build(ctx),
                ),
        )
        .with_stroke_thickness(Thickness::uniform(1.0))
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
    ) {
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
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.play_pause {
                Log::warn("Implement playback!");
            } else if message.destination() == self.stop {
                Log::warn("Implement playback stopping!");
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
                // TODO
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
            }
        } else if let Some(CheckBoxMessage::Check(Some(checked))) = message.data() {
            if message.destination() == self.preview
                && message.direction() == MessageDirection::FromWidget
            {
                if let Some(animation) = animation_player
                    .animations_mut()
                    .try_get_mut(selection.animation)
                {
                    animation.set_enabled(*checked);
                }
            }
        }
    }

    pub fn sync_to_model(
        &self,
        animation_player: &AnimationPlayer,
        selection: &AnimationSelection,
        ui: &mut UserInterface,
    ) {
        let new_items = animation_player
            .animations()
            .pair_iter()
            .map(|(h, a)| {
                make_dropdown_list_option_universal(&mut ui.build_ctx(), a.name(), 22.0, h)
            })
            .collect();

        ui.send_message(DropdownListMessage::items(
            self.animations,
            MessageDirection::ToWidget,
            new_items,
        ));

        let mut selected_animation_valid = false;
        if let Some(animation) = animation_player.animations().try_get(selection.animation) {
            selected_animation_valid = true;
            ui.send_message(TextMessage::text(
                self.animation_name,
                MessageDirection::ToWidget,
                animation.name().to_string(),
            ));
        }

        for widget in [
            self.preview,
            self.play_pause,
            self.stop,
            self.speed,
            self.rename_current_animation,
            self.remove_current_animation,
        ] {
            ui.send_message(WidgetMessage::enabled(
                widget,
                MessageDirection::ToWidget,
                selected_animation_valid,
            ));
        }
    }
}
