use crate::{
    scene::commands::effect::AddEffectCommand, ChangeSelectionCommand, EditorScene, GridBuilder,
    Message, MessageDirection, SceneCommand, Selection, UserInterface,
};
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
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        Orientation, Thickness, UiNode,
    },
    scene::sound::effect::{BaseEffectBuilder, Effect, ReverbEffectBuilder},
};
use std::cmp::Ordering;
use std::{rc::Rc, sync::mpsc::Sender};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectSelection {
    pub effects: Vec<Handle<Effect>>,
}

impl EffectSelection {
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    pub fn len(&self) -> usize {
        self.effects.len()
    }
}

pub struct AudioPanel {
    pub window: Handle<UiNode>,
    edit_context: Handle<UiNode>,
    add_effect: Handle<UiNode>,
    effects: Handle<UiNode>,
}

fn item_effect(item: Handle<UiNode>, ui: &UserInterface) -> Handle<Effect> {
    *ui.node(item)
        .user_data_ref::<Handle<Effect>>()
        .expect("Must be Handle<Effect>")
}

impl AudioPanel {
    pub fn new(engine: &mut Engine) -> Self {
        let ctx = &mut engine.user_interface.build_ctx();

        let edit_context;
        let add_effect;
        let effects;
        let window = WindowBuilder::new(WidgetBuilder::new())
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            effects =
                                ListViewBuilder::new(WidgetBuilder::new().on_row(0)).build(ctx);
                            effects
                        })
                        .with_child(
                            StackPanelBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(1)
                                    .with_child({
                                        add_effect = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        // TODO: Add selector when there's more effects.
                                        .with_text("Add Reverb")
                                        .build(ctx);
                                        add_effect
                                    })
                                    .with_child({
                                        edit_context = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_text("Edit Context")
                                        .build(ctx);
                                        edit_context
                                    }),
                            )
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
            effects,
            add_effect,
            edit_context,
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
            if message.destination() == self.edit_context {
                sender
                    .send(Message::DoSceneCommand(SceneCommand::new(
                        ChangeSelectionCommand::new(
                            Selection::SoundContext,
                            editor_scene.selection.clone(),
                        ),
                    )))
                    .unwrap();
            } else if message.destination() == self.add_effect {
                sender
                    .send(Message::DoSceneCommand(SceneCommand::new(
                        AddEffectCommand::new(
                            ReverbEffectBuilder::new(
                                BaseEffectBuilder::new().with_name("Reverb".to_owned()),
                            )
                            .build_effect(),
                        ),
                    )))
                    .unwrap()
            }
        } else if let Some(ListViewMessage::SelectionChanged(Some(effect_index))) = message.data() {
            if message.destination() == self.effects
                && message.direction() == MessageDirection::FromWidget
            {
                let ui = &engine.user_interface;

                let effect = item_effect(
                    ui.node(self.effects)
                        .cast::<ListView>()
                        .expect("Must be ListView")
                        .items()[*effect_index],
                    ui,
                );

                sender
                    .send(Message::DoSceneCommand(SceneCommand::new(
                        ChangeSelectionCommand::new(
                            Selection::Effect(EffectSelection {
                                effects: vec![effect],
                            }),
                            editor_scene.selection.clone(),
                        ),
                    )))
                    .unwrap()
            }
        }
    }

    pub fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut Engine) {
        let context = &engine.scenes[editor_scene.scene].graph.sound_context;
        let ui = &mut engine.user_interface;

        let items = ui
            .node(self.effects)
            .cast::<ListView>()
            .expect("Must be ListView!")
            .items()
            .to_vec();

        match (context.effects_count() as usize).cmp(&items.len()) {
            Ordering::Less => {
                for item in items {
                    let effect_handle = item_effect(item, ui);
                    if context.effects().all(|(e, _)| e != effect_handle) {
                        ui.send_message(ListViewMessage::remove_item(
                            self.effects,
                            MessageDirection::ToWidget,
                            item,
                        ));
                    }
                }
            }
            Ordering::Greater => {
                for (effect_handle, effect) in context.effects() {
                    if items.iter().all(|i| item_effect(*i, ui) != effect_handle) {
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
}
