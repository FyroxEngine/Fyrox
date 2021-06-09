use crate::scene::EditorScene;
use crate::{
    gui::{BuildContext, Ui, UiNode},
    GameEngine, Message,
};
use rg3d::{
    core::pool::Handle,
    gui::{
        border::BorderBuilder,
        decorator::DecoratorBuilder,
        list_view::ListViewBuilder,
        message::{ListViewMessage, MessageDirection},
        text::TextBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
    },
    sound::{context::SoundContext, source::SoundSource},
};
use std::rc::Rc;
use std::sync::mpsc::Sender;

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
                    ui.send_message(ListViewMessage::remove_item(
                        self.sounds,
                        MessageDirection::ToWidget,
                        item,
                    ));
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
                    ui.send_message(ListViewMessage::add_item(
                        self.sounds,
                        MessageDirection::ToWidget,
                        item,
                    ));
                }
            }
        }
    }

    pub fn handle_ui_message(&mut self, sender: &Sender<Message>) {}
}
