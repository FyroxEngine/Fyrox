use crate::scene::commands::{ChangeSelectionCommand, SceneCommand};
use crate::{
    gui::UiMessage,
    gui::{BuildContext, UiNode},
    scene::{EditorScene, Selection},
    send_sync_message, utils, GameEngine, Message,
};
use rg3d::{
    core::pool::Handle,
    gui::{
        border::BorderBuilder,
        decorator::DecoratorBuilder,
        list_view::ListViewBuilder,
        message::UiMessageData,
        message::{ListViewMessage, MessageDirection},
        text::TextBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
    },
    sound::source::SoundSource,
};
use std::{rc::Rc, sync::mpsc::Sender};

#[derive(Debug, Clone)]
pub struct SoundSelection {
    sources: Vec<Handle<SoundSource>>,
}

impl SoundSelection {
    pub fn sources(&self) -> &[Handle<SoundSource>] {
        &self.sources
    }

    pub fn is_single_selection(&self) -> bool {
        self.sources.len() == 1
    }

    pub fn first(&self) -> Option<Handle<SoundSource>> {
        self.sources.first().cloned()
    }
}

impl PartialEq for SoundSelection {
    fn eq(&self, other: &Self) -> bool {
        utils::is_slice_equal_permutation(self.sources(), other.sources())
    }
}

impl Eq for SoundSelection {}

pub struct SoundPanel {
    pub window: Handle<UiNode>,
    sounds: Handle<UiNode>,
}

impl SoundPanel {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let sounds;
        let window = WindowBuilder::new(WidgetBuilder::new())
            .with_title(WindowTitle::text("Sounds"))
            .with_content({
                sounds = ListViewBuilder::new(WidgetBuilder::new()).build(ctx);
                sounds
            })
            .build(ctx);
        Self { window, sounds }
    }

    pub fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        let ui = &mut engine.user_interface;
        let context = &engine.scenes[editor_scene.scene].sound_context;
        let list_view_items = ui.node(self.sounds).as_list_view().items().to_vec();
        let context_state = context.state();
        let sources = context_state.sources();

        if sources.alive_count() < list_view_items.len() {
            // A source was removed.
            for &item in list_view_items.iter() {
                let associated_source = *ui
                    .node(item)
                    .user_data_ref::<Handle<SoundSource>>()
                    .unwrap();

                if sources.pair_iter().all(|(h, _)| h != associated_source) {
                    send_sync_message(
                        ui,
                        ListViewMessage::remove_item(self.sounds, MessageDirection::ToWidget, item),
                    );
                }
            }
        } else if sources.alive_count() > list_view_items.len() {
            // A source was added.
            for (handle, source) in context_state.sources().pair_iter() {
                if list_view_items
                    .iter()
                    .all(|i| *ui.node(*i).user_data_ref::<Handle<SoundSource>>().unwrap() != handle)
                {
                    let item = DecoratorBuilder::new(BorderBuilder::new(
                        WidgetBuilder::new()
                            .with_user_data(Rc::new(handle))
                            .with_child(
                                TextBuilder::new(WidgetBuilder::new())
                                    .with_text(format!(
                                        "{} ({}:{})",
                                        source.name(),
                                        handle.index(),
                                        handle.generation()
                                    ))
                                    .build(&mut ui.build_ctx()),
                            ),
                    ))
                    .build(&mut ui.build_ctx());
                    send_sync_message(
                        ui,
                        ListViewMessage::add_item(self.sounds, MessageDirection::ToWidget, item),
                    );
                }
            }
        }

        // Sync selection.
        send_sync_message(
            ui,
            ListViewMessage::selection(
                self.sounds,
                MessageDirection::ToWidget,
                if let Selection::Sound(selection) = &editor_scene.selection {
                    if let Some(first) = selection.first() {
                        ui.node(self.sounds)
                            .as_list_view()
                            .items()
                            .iter()
                            .position(|i| {
                                *ui.node(*i).user_data_ref::<Handle<SoundSource>>().unwrap()
                                    == first
                            })
                    } else {
                        None
                    }
                } else {
                    None
                },
            ),
        )
    }

    pub fn handle_ui_message(
        &mut self,
        sender: &Sender<Message>,
        editor_scene: &EditorScene,
        message: &UiMessage,
        engine: &GameEngine,
    ) {
        let ui = &engine.user_interface;
        let list_view_items = ui.node(self.sounds).as_list_view().items();

        match message.data() {
            UiMessageData::ListView(ListViewMessage::SelectionChanged(selection)) => {
                if message.destination() == self.sounds
                    && message.direction() == MessageDirection::FromWidget
                {
                    let new_selection = match selection {
                        None => Default::default(),
                        Some(index) => {
                            // TODO: Implement multi-selection when ListView will have multi-selection support.
                            Selection::Sound(SoundSelection {
                                sources: vec![*ui
                                    .node(list_view_items[*index])
                                    .user_data_ref::<Handle<SoundSource>>()
                                    .unwrap()],
                            })
                        }
                    };

                    if new_selection != editor_scene.selection {
                        sender
                            .send(Message::DoSceneCommand(SceneCommand::ChangeSelection(
                                ChangeSelectionCommand::new(
                                    new_selection,
                                    editor_scene.selection.clone(),
                                ),
                            )))
                            .unwrap();
                    }
                }
            }
            _ => (),
        }
    }
}
