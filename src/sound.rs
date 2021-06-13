use crate::{
    gui::{
        BuildContext, CustomWidget, EditorUiMessage, EditorUiNode, Ui, UiMessage, UiNode,
        UiWidgetBuilder,
    },
    scene::{
        commands::{ChangeSelectionCommand, SceneCommand},
        EditorScene, Selection,
    },
    send_sync_message, utils, GameEngine, Message,
};
use rg3d::{
    core::pool::Handle,
    gui::{
        border::BorderBuilder,
        decorator::DecoratorBuilder,
        list_view::ListViewBuilder,
        message::{ListViewMessage, MessageDirection, TextMessage, UiMessageData},
        node::UINode,
        text::TextBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        Control, NodeHandleMapping, UserInterface,
    },
    sound::source::SoundSource,
};
use std::{
    ops::{Deref, DerefMut},
    sync::mpsc::Sender,
};

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

#[derive(Clone, Debug)]
pub struct SoundItem {
    widget: CustomWidget,
    text: Handle<UiNode>,
    sound_source: Handle<SoundSource>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SoundItemMessage {
    Name(String),
}

impl Deref for SoundItem {
    type Target = CustomWidget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl DerefMut for SoundItem {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl Control<EditorUiMessage, EditorUiNode> for SoundItem {
    fn resolve(&mut self, node_map: &NodeHandleMapping<EditorUiMessage, EditorUiNode>) {
        node_map.resolve(&mut self.text)
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<EditorUiMessage, EditorUiNode>,
        message: &mut UiMessage,
    ) {
        if let UiMessageData::User(EditorUiMessage::SoundItem(SoundItemMessage::Name(name))) =
            message.data()
        {
            ui.send_message(TextMessage::text(
                self.text,
                MessageDirection::ToWidget,
                make_item_name(name, self.sound_source),
            ));
        }
    }
}

pub struct SoundItemBuilder {
    widget_builder: UiWidgetBuilder,
    name: String,
    sound_source: Handle<SoundSource>,
}

fn make_item_name(name: &str, handle: Handle<SoundSource>) -> String {
    format!("{} ({}:{})", name, handle.index(), handle.generation())
}

impl SoundItemBuilder {
    pub fn new(widget_builder: UiWidgetBuilder) -> Self {
        Self {
            widget_builder,
            name: Default::default(),
            sound_source: Default::default(),
        }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    pub fn with_sound_source(mut self, source: Handle<SoundSource>) -> Self {
        self.sound_source = source;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let text;
        let decorator =
            DecoratorBuilder::new(BorderBuilder::new(WidgetBuilder::new().with_child({
                text = TextBuilder::new(WidgetBuilder::new())
                    .with_text(make_item_name(&self.name, self.sound_source))
                    .build(ctx);
                text
            })))
            .build(ctx);

        let node = UiNode::User(EditorUiNode::SoundItem(SoundItem {
            widget: self.widget_builder.with_child(decorator).build(),
            text,
            sound_source: self.sound_source,
        }));

        ctx.add_node(node)
    }
}

pub struct SoundPanel {
    pub window: Handle<UiNode>,
    sounds: Handle<UiNode>,
}

fn fetch_source(handle: Handle<UiNode>, ui: &Ui) -> Handle<SoundSource> {
    if let UINode::User(EditorUiNode::SoundItem(item)) = ui.node(handle) {
        item.sound_source
    } else {
        unreachable!()
    }
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
                let associated_source = fetch_source(item, ui);

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
                    .all(|i| fetch_source(*i, ui) != handle)
                {
                    let item = SoundItemBuilder::new(WidgetBuilder::new())
                        .with_name(source.name_owned())
                        .with_sound_source(handle)
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
                            .position(|i| fetch_source(*i, ui) == first)
                    } else {
                        None
                    }
                } else {
                    None
                },
            ),
        );

        // Sync sound names.
        for item in ui.node(self.sounds).as_list_view().items() {
            let associated_source = fetch_source(*item, ui);
            ui.send_message(UiMessage::user(
                *item,
                MessageDirection::ToWidget,
                EditorUiMessage::SoundItem(SoundItemMessage::Name(
                    context_state.source(associated_source).name_owned(),
                )),
            ));
        }
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
                                sources: vec![fetch_source(list_view_items[*index], ui)],
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
