use crate::{
    scene::commands::effect::AddAudioBusCommand, utils::window_content, ChangeSelectionCommand,
    EditorScene, GridBuilder, Message, MessageDirection, Mode, SceneCommand, Selection,
    UserInterface,
};
use fyrox::scene::sound::AudioBus;
use fyrox::{
    core::pool::Handle,
    engine::Engine,
    gui::{
        border::BorderBuilder,
        button::{ButtonBuilder, ButtonMessage},
        decorator::DecoratorBuilder,
        grid::{Column, Row},
        list_view::{ListView, ListViewBuilder, ListViewMessage},
        message::UiMessage,
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowTitle},
        Orientation, Thickness, UiNode,
    },
};
use std::{cmp::Ordering, rc::Rc, sync::mpsc::Sender};

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
    effects: Handle<UiNode>,
}

fn item_bus(item: Handle<UiNode>, ui: &UserInterface) -> Handle<AudioBus> {
    *ui.node(item)
        .user_data_ref::<Handle<AudioBus>>()
        .expect("Must be Handle<AudioBus>")
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
                            buses = ListViewBuilder::new(WidgetBuilder::new().on_row(0)).build(ctx);
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
            effects: buses,
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
            if message.destination() == self.effects
                && message.direction() == MessageDirection::FromWidget
            {
                let ui = &engine.user_interface;

                let effect = item_bus(
                    ui.node(self.effects)
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
            .node(self.effects)
            .cast::<ListView>()
            .expect("Must be ListView!")
            .items()
            .to_vec();

        match (context_state.bus_graph_ref().len() as usize).cmp(&items.len()) {
            Ordering::Less => {
                for item in items {
                    let bus_handle = item_bus(item, ui);
                    if context_state
                        .bus_graph_ref()
                        .buses_pair_iter()
                        .all(|(other_bus_handle, _)| other_bus_handle != bus_handle)
                    {
                        ui.send_message(ListViewMessage::remove_item(
                            self.effects,
                            MessageDirection::ToWidget,
                            item,
                        ));
                    }
                }
            }
            Ordering::Greater => {
                for (effect_handle, effect) in context_state.bus_graph_ref().buses_pair_iter() {
                    if items.iter().all(|i| item_bus(*i, ui) != effect_handle) {
                        let item = DecoratorBuilder::new(BorderBuilder::new(
                            WidgetBuilder::new()
                                .with_user_data(Rc::new(effect_handle))
                                .with_child(
                                    TextBuilder::new(WidgetBuilder::new())
                                        .with_text(effect.name())
                                        .build(&mut ui.build_ctx()),
                                ),
                        ))
                        .build(&mut ui.build_ctx());

                        ui.send_message(ListViewMessage::add_item(
                            self.effects,
                            MessageDirection::ToWidget,
                            item,
                        ));
                    }
                }
            }
            _ => (),
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
