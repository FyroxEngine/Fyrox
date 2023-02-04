use crate::{
    audio::bus::{AudioBusView, AudioBusViewBuilder},
    scene::commands::effect::AddAudioBusCommand,
    send_sync_message,
    utils::window_content,
    ChangeSelectionCommand, EditorScene, GridBuilder, Message, MessageDirection, Mode,
    SceneCommand, Selection, UserInterface,
};
use fyrox::{
    core::pool::Handle,
    engine::Engine,
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        grid::{Column, Row},
        list_view::{ListView, ListViewBuilder, ListViewMessage},
        message::UiMessage,
        stack_panel::StackPanelBuilder,
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowTitle},
        Orientation, Thickness, UiNode,
    },
    scene::sound::AudioBus,
};
use std::{cmp::Ordering, sync::mpsc::Sender};

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
    audio_buses: Handle<UiNode>,
}

fn item_bus(item: Handle<UiNode>, ui: &UserInterface) -> Handle<AudioBus> {
    ui.node(item).query_component::<AudioBusView>().unwrap().bus
}

impl AudioPanel {
    pub fn new(engine: &mut Engine) -> Self {
        let ctx = &mut engine.user_interface.build_ctx();

        let add_bus;
        let buses;
        let window = WindowBuilder::new(WidgetBuilder::new())
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            buses = ListViewBuilder::new(WidgetBuilder::new().on_row(0))
                                .with_items_panel(
                                    StackPanelBuilder::new(WidgetBuilder::new())
                                        .with_orientation(Orientation::Horizontal)
                                        .build(ctx),
                                )
                                .build(ctx);
                            buses
                        })
                        .with_child(
                            StackPanelBuilder::new(WidgetBuilder::new().on_row(1).with_child({
                                add_bus = ButtonBuilder::new(
                                    WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                                )
                                .with_text("Add Bus")
                                .build(ctx);
                                add_bus
                            }))
                            .with_orientation(Orientation::Horizontal)
                            .build(ctx),
                        ),
                )
                .add_column(Column::stretch())
                .add_row(Row::stretch())
                .add_row(Row::strict(25.0))
                .build(ctx),
            )
            .with_title(WindowTitle::text("Audio Context"))
            .build(ctx);

        Self {
            window,
            audio_buses: buses,
            add_bus,
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_scene: &EditorScene,
        sender: &Sender<Message>,
        engine: &Engine,
    ) {
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.add_bus {
                sender
                    .send(Message::DoSceneCommand(SceneCommand::new(
                        AddAudioBusCommand::new(AudioBus::new("AudioBus".to_string())),
                    )))
                    .unwrap()
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

                sender
                    .send(Message::DoSceneCommand(SceneCommand::new(
                        ChangeSelectionCommand::new(
                            Selection::AudioBus(AudioBusSelection {
                                buses: vec![effect],
                            }),
                            editor_scene.selection.clone(),
                        ),
                    )))
                    .unwrap()
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

        match (context_state.bus_graph_ref().len() as usize).cmp(&items.len()) {
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
                                .with_width(80.0)
                                .with_margin(Thickness::uniform(1.0)),
                        )
                        .with_name(audio_bus.name())
                        .with_effect_names(
                            audio_bus
                                .effects()
                                .map(|e| AsRef::<str>::as_ref(&**e).to_owned())
                                .collect::<Vec<_>>(),
                        )
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

        if let Selection::AudioBus(ref selection) = editor_scene.selection {
            for (index, item) in items.into_iter().enumerate() {
                let bus_handle = item_bus(item, ui);

                if selection.buses.contains(&bus_handle) {
                    selection_index = Some(index);
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
    }

    pub fn on_mode_changed(&mut self, ui: &UserInterface, mode: &Mode) {
        ui.send_message(WidgetMessage::enabled(
            window_content(self.window, ui),
            MessageDirection::ToWidget,
            mode.is_edit(),
        ));
    }
}
