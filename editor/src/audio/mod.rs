use crate::message::MessageSender;
use crate::{
    audio::bus::{AudioBusView, AudioBusViewBuilder, AudioBusViewMessage},
    gui::make_dropdown_list_option,
    scene::commands::{
        effect::{AddAudioBusCommand, LinkAudioBuses, RemoveAudioBusCommand},
        sound_context::{SetDistanceModelCommand, SetRendererCommand},
        CommandGroup,
    },
    send_sync_message,
    utils::window_content,
    ChangeSelectionCommand, EditorScene, GridBuilder, MessageDirection, Mode, SceneCommand,
    Selection, UserInterface,
};
use fyrox::{
    core::{log::Log, pool::Handle},
    engine::Engine,
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        dropdown_list::{DropdownListBuilder, DropdownListMessage},
        file_browser::{FileSelectorFieldBuilder, FileSelectorFieldMessage},
        grid::{Column, Row},
        list_view::{ListView, ListViewBuilder, ListViewMessage},
        message::UiMessage,
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        utils::make_simple_tooltip,
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowTitle},
        Orientation, Thickness, UiNode, VerticalAlignment,
    },
    scene::sound::{
        AudioBus, AudioBusGraph, DistanceModel, HrirSphere, HrtfRenderer, Renderer, SAMPLE_RATE,
    },
};
use std::cmp::Ordering;
use strum::VariantNames;

mod bus;
pub mod preview;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioBusSelection {
    pub buses: Vec<Handle<AudioBus>>,
}

impl AudioBusSelection {
    pub fn is_empty(&self) -> bool {
        self.buses.is_empty()
    }

    pub fn len(&self) -> usize {
        self.buses.len()
    }
}

pub struct AudioPanel {
    pub window: Handle<UiNode>,
    add_bus: Handle<UiNode>,
    remove_bus: Handle<UiNode>,
    audio_buses: Handle<UiNode>,
    distance_model: Handle<UiNode>,
    renderer: Handle<UiNode>,
    hrir_sphere_path: Handle<UiNode>,
}

fn item_bus(item: Handle<UiNode>, ui: &UserInterface) -> Handle<AudioBus> {
    ui.node(item).query_component::<AudioBusView>().unwrap().bus
}

fn fetch_possible_parent_buses(
    bus: Handle<AudioBus>,
    graph: &AudioBusGraph,
) -> Vec<(Handle<AudioBus>, String)> {
    let mut stack = vec![graph.primary_bus_handle()];
    let mut result = vec![];
    while let Some(other_bus) = stack.pop() {
        let other_bus_ref = graph.try_get_bus_ref(other_bus).expect("Malformed graph!");
        if other_bus != bus {
            result.push((other_bus, other_bus_ref.name().to_owned()));
            stack.extend_from_slice(other_bus_ref.children());
        }
    }
    result
}

fn audio_bus_effect_names(audio_bus: &AudioBus) -> Vec<String> {
    audio_bus
        .effects()
        .map(|e| AsRef::<str>::as_ref(e).to_owned())
        .collect::<Vec<_>>()
}

impl AudioPanel {
    pub fn new(engine: &mut Engine) -> Self {
        let ctx = &mut engine.user_interface.build_ctx();

        let add_bus;
        let remove_bus;
        let buses;
        let distance_model;
        let renderer;
        let hrir_sphere_path;
        let window = WindowBuilder::new(WidgetBuilder::new().with_name("AudioPanel"))
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(
                            StackPanelBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(0)
                                    .with_child(
                                        TextBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_vertical_text_alignment(VerticalAlignment::Center)
                                        .with_text("DM")
                                        .build(ctx),
                                    )
                                    .with_child({
                                        distance_model = DropdownListBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0))
                                                .with_width(130.0)
                                                .with_tooltip(make_simple_tooltip(
                                                    ctx,
                                                    "Distance Model. Defines the method of \
                                                    calculating distance attenuation for sound \
                                                    sources.",
                                                )),
                                        )
                                        .with_items(
                                            DistanceModel::VARIANTS
                                                .iter()
                                                .map(|v| make_dropdown_list_option(ctx, v))
                                                .collect::<Vec<_>>(),
                                        )
                                        .build(ctx);
                                        distance_model
                                    })
                                    .with_child(
                                        TextBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_vertical_text_alignment(VerticalAlignment::Center)
                                        .with_text("Renderer")
                                        .build(ctx),
                                    )
                                    .with_child({
                                        renderer = DropdownListBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0))
                                                .with_width(100.0)
                                                .with_tooltip(make_simple_tooltip(ctx, "Renderer")),
                                        )
                                        .with_items(
                                            Renderer::VARIANTS
                                                .iter()
                                                .map(|v| make_dropdown_list_option(ctx, v))
                                                .collect::<Vec<_>>(),
                                        )
                                        .build(ctx);
                                        renderer
                                    })
                                    .with_child({
                                        hrir_sphere_path = FileSelectorFieldBuilder::new(
                                            WidgetBuilder::new()
                                                .with_tooltip(make_simple_tooltip(
                                                    ctx,
                                                    "A path to a HRIR sphere used \
                                                    by the HRTF renderer.",
                                                ))
                                                .with_width(120.0)
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .build(ctx);
                                        hrir_sphere_path
                                    }),
                            )
                            .with_orientation(Orientation::Horizontal)
                            .build(ctx),
                        )
                        .with_child({
                            buses = ListViewBuilder::new(WidgetBuilder::new().on_row(1))
                                .with_items_panel(
                                    StackPanelBuilder::new(WidgetBuilder::new())
                                        .with_orientation(Orientation::Horizontal)
                                        .build(ctx),
                                )
                                .build(ctx);
                            buses
                        })
                        .with_child(
                            StackPanelBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(2)
                                    .with_child({
                                        add_bus = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_width(100.0)
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_text("Add Bus")
                                        .build(ctx);
                                        add_bus
                                    })
                                    .with_child({
                                        remove_bus = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_width(100.0)
                                                .with_enabled(false)
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_text("Remove Bus")
                                        .build(ctx);
                                        remove_bus
                                    }),
                            )
                            .with_orientation(Orientation::Horizontal)
                            .build(ctx),
                        ),
                )
                .add_column(Column::stretch())
                .add_row(Row::strict(25.0))
                .add_row(Row::stretch())
                .add_row(Row::strict(25.0))
                .build(ctx),
            )
            .with_title(WindowTitle::text("Audio Context"))
            .build(ctx);

        Self {
            window,
            audio_buses: buses,
            distance_model,
            add_bus,
            remove_bus,
            renderer,
            hrir_sphere_path,
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_scene: &EditorScene,
        sender: &MessageSender,
        engine: &Engine,
    ) {
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.add_bus {
                sender.do_scene_command(AddAudioBusCommand::new(AudioBus::new(
                    "AudioBus".to_string(),
                )))
            } else if message.destination() == self.remove_bus {
                if let Selection::AudioBus(ref selection) = editor_scene.selection {
                    let mut commands = vec![SceneCommand::new(ChangeSelectionCommand::new(
                        Selection::None,
                        editor_scene.selection.clone(),
                    ))];

                    for &bus in &selection.buses {
                        commands.push(SceneCommand::new(RemoveAudioBusCommand::new(bus)));
                    }

                    sender.do_scene_command(CommandGroup::from(commands));
                }
            }
        } else if let Some(ListViewMessage::SelectionChanged(Some(effect_index))) = message.data() {
            if message.destination() == self.audio_buses
                && message.direction() == MessageDirection::FromWidget
            {
                let ui = &engine.user_interface;

                let effect = item_bus(
                    ui.node(self.audio_buses)
                        .cast::<ListView>()
                        .expect("Must be ListView")
                        .items()[*effect_index],
                    ui,
                );

                sender.do_scene_command(ChangeSelectionCommand::new(
                    Selection::AudioBus(AudioBusSelection {
                        buses: vec![effect],
                    }),
                    editor_scene.selection.clone(),
                ))
            }
        } else if let Some(AudioBusViewMessage::ChangeParent(new_parent)) = message.data() {
            if message.direction() == MessageDirection::FromWidget {
                let audio_bus_view_ref = engine
                    .user_interface
                    .node(message.destination())
                    .query_component::<AudioBusView>()
                    .unwrap();

                let child = audio_bus_view_ref.bus;

                sender.do_scene_command(LinkAudioBuses {
                    child,
                    parent: *new_parent,
                });
            }
        } else if let Some(DropdownListMessage::SelectionChanged(Some(index))) = message.data() {
            if message.direction() == MessageDirection::FromWidget {
                if message.destination() == self.renderer {
                    let renderer = match index {
                        0 => Renderer::Default,
                        1 => Renderer::HrtfRenderer(Default::default()),
                        _ => unreachable!(),
                    };

                    sender.do_scene_command(SetRendererCommand::new(renderer));
                } else if message.destination() == self.distance_model {
                    let distance_model = match index {
                        0 => DistanceModel::None,
                        1 => DistanceModel::InverseDistance,
                        2 => DistanceModel::LinearDistance,
                        3 => DistanceModel::ExponentDistance,
                        _ => unreachable!(),
                    };

                    sender.do_scene_command(SetDistanceModelCommand::new(distance_model));
                }
            }
        } else if let Some(FileSelectorFieldMessage::Path(path)) = message.data() {
            if message.destination() == self.hrir_sphere_path
                && message.direction() == MessageDirection::FromWidget
            {
                match HrirSphere::from_file(path, SAMPLE_RATE) {
                    Ok(hrir_sphere) => {
                        sender.do_scene_command(SetRendererCommand::new(Renderer::HrtfRenderer(
                            HrtfRenderer::new(hrir_sphere),
                        )));
                    }
                    Err(e) => {
                        Log::err(format!(
                            "Failed to load a HRIR sphere from {:?}. Reason: {:?}",
                            path, e
                        ));
                    }
                }
            }
        }
    }

    pub fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut Engine) {
        let context_state = engine.scenes[editor_scene.scene]
            .graph
            .sound_context
            .state();
        let ui = &mut engine.user_interface;

        let items = ui
            .node(self.audio_buses)
            .cast::<ListView>()
            .expect("Must be ListView!")
            .items()
            .to_vec();

        match (context_state.bus_graph_ref().len()).cmp(&items.len()) {
            Ordering::Less => {
                for &item in &items {
                    let bus_handle = item_bus(item, ui);
                    if context_state
                        .bus_graph_ref()
                        .buses_pair_iter()
                        .all(|(other_bus_handle, _)| other_bus_handle != bus_handle)
                    {
                        send_sync_message(
                            ui,
                            ListViewMessage::remove_item(
                                self.audio_buses,
                                MessageDirection::ToWidget,
                                item,
                            ),
                        );
                    }
                }
            }
            Ordering::Greater => {
                for (audio_bus_handle, audio_bus) in context_state.bus_graph_ref().buses_pair_iter()
                {
                    if items.iter().all(|i| item_bus(*i, ui) != audio_bus_handle) {
                        let item = AudioBusViewBuilder::new(
                            WidgetBuilder::new()
                                .with_width(100.0)
                                .with_margin(Thickness::uniform(1.0)),
                        )
                        .with_name(audio_bus.name())
                        .with_effect_names(audio_bus_effect_names(audio_bus))
                        .with_parent_bus(audio_bus.parent())
                        .with_possible_parent_buses(fetch_possible_parent_buses(
                            audio_bus_handle,
                            context_state.bus_graph_ref(),
                        ))
                        .with_audio_bus(audio_bus_handle)
                        .build(&mut ui.build_ctx());

                        send_sync_message(
                            ui,
                            ListViewMessage::add_item(
                                self.audio_buses,
                                MessageDirection::ToWidget,
                                item,
                            ),
                        );
                    }
                }
            }
            _ => (),
        }

        let mut selection_index = None;
        let mut is_primary_bus_selected = false;

        if let Selection::AudioBus(ref selection) = editor_scene.selection {
            for (index, item) in items.into_iter().enumerate() {
                let bus_handle = item_bus(item, ui);

                if selection.buses.contains(&bus_handle) {
                    selection_index = Some(index);

                    if context_state.bus_graph_ref().primary_bus_handle() == bus_handle {
                        is_primary_bus_selected = true;
                    }

                    break;
                }
            }
        }

        send_sync_message(
            ui,
            ListViewMessage::selection(
                self.audio_buses,
                MessageDirection::ToWidget,
                selection_index,
            ),
        );

        send_sync_message(
            ui,
            WidgetMessage::enabled(
                self.remove_bus,
                MessageDirection::ToWidget,
                selection_index.is_some() && !is_primary_bus_selected,
            ),
        );

        for audio_bus_view in ui
            .node(self.audio_buses)
            .cast::<ListView>()
            .expect("Must be ListView!")
            .items()
        {
            let audio_bus_view_ref = ui
                .node(*audio_bus_view)
                .query_component::<AudioBusView>()
                .unwrap();
            send_sync_message(
                ui,
                AudioBusViewMessage::possible_parent_buses(
                    *audio_bus_view,
                    MessageDirection::ToWidget,
                    fetch_possible_parent_buses(
                        audio_bus_view_ref.bus,
                        context_state.bus_graph_ref(),
                    ),
                ),
            );
            let audio_bus_ref = context_state
                .bus_graph_ref()
                .try_get_bus_ref(audio_bus_view_ref.bus)
                .unwrap();
            send_sync_message(
                ui,
                AudioBusViewMessage::effect_names(
                    *audio_bus_view,
                    MessageDirection::ToWidget,
                    audio_bus_effect_names(audio_bus_ref),
                ),
            );
            send_sync_message(
                ui,
                AudioBusViewMessage::name(
                    *audio_bus_view,
                    MessageDirection::ToWidget,
                    audio_bus_ref.name().to_owned(),
                ),
            );
        }

        send_sync_message(
            ui,
            DropdownListMessage::selection(
                self.distance_model,
                MessageDirection::ToWidget,
                Some(context_state.distance_model() as usize),
            ),
        );

        send_sync_message(
            ui,
            DropdownListMessage::selection(
                self.renderer,
                MessageDirection::ToWidget,
                Some(match context_state.renderer_ref() {
                    Renderer::Default => 0,
                    Renderer::HrtfRenderer(_) => 1,
                }),
            ),
        );

        if let Renderer::HrtfRenderer(hrtf) = context_state.renderer_ref() {
            send_sync_message(
                ui,
                WidgetMessage::visibility(self.hrir_sphere_path, MessageDirection::ToWidget, true),
            );
            send_sync_message(
                ui,
                FileSelectorFieldMessage::path(
                    self.hrir_sphere_path,
                    MessageDirection::ToWidget,
                    hrtf.hrir_sphere_path().to_path_buf(),
                ),
            );
        } else {
            send_sync_message(
                ui,
                WidgetMessage::visibility(self.hrir_sphere_path, MessageDirection::ToWidget, false),
            );
        }
    }

    pub fn on_mode_changed(&mut self, ui: &UserInterface, mode: &Mode) {
        ui.send_message(WidgetMessage::enabled(
            window_content(self.window, ui),
            MessageDirection::ToWidget,
            mode.is_edit(),
        ));
    }
}
