use crate::{
    inspector::editors::make_property_editors_container, message::MessageSender, MSG_SYNC_FLAG,
};
use fyrox::gui::button::ButtonMessage;
use fyrox::{
    core::{log::Log, pool::Handle, reflect::prelude::*},
    gui::{
        button::ButtonBuilder,
        grid::{Column, GridBuilder, Row},
        inspector::{InspectorBuilder, InspectorContext, InspectorMessage, PropertyAction},
        message::{MessageDirection, UiMessage},
        stack_panel::StackPanelBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, Thickness, UiNode, UserInterface,
    },
    gui::{HorizontalAlignment, Orientation},
    scene::node::Node,
};
use std::rc::Rc;

#[derive(Reflect, Debug)]
pub struct RagdollPreset {
    hips: Handle<Node>,
    left_up_leg: Handle<Node>,
    left_leg: Handle<Node>,
    left_foot: Handle<Node>,
    right_up_leg: Handle<Node>,
    right_leg: Handle<Node>,
    right_foot: Handle<Node>,
    spine: Handle<Node>,
    spine1: Handle<Node>,
    spine2: Handle<Node>,
    left_shoulder: Handle<Node>,
    left_arm: Handle<Node>,
    left_fore_arm: Handle<Node>,
    left_hand: Handle<Node>,
    right_shoulder: Handle<Node>,
    right_arm: Handle<Node>,
    right_fore_arm: Handle<Node>,
    right_hand: Handle<Node>,
    total_mass: f32,
}

impl Default for RagdollPreset {
    fn default() -> Self {
        Self {
            hips: Default::default(),
            left_up_leg: Default::default(),
            left_leg: Default::default(),
            left_foot: Default::default(),
            right_up_leg: Default::default(),
            right_leg: Default::default(),
            right_foot: Default::default(),
            spine: Default::default(),
            spine1: Default::default(),
            spine2: Default::default(),
            left_shoulder: Default::default(),
            left_arm: Default::default(),
            left_fore_arm: Default::default(),
            left_hand: Default::default(),
            right_shoulder: Default::default(),
            right_arm: Default::default(),
            right_fore_arm: Default::default(),
            right_hand: Default::default(),
            total_mass: 20.0,
        }
    }
}

pub struct RagdollWizard {
    pub window: Handle<UiNode>,
    pub preset: RagdollPreset,
    inspector: Handle<UiNode>,
    ok: Handle<UiNode>,
    cancel: Handle<UiNode>,
}

impl RagdollWizard {
    pub fn new(ctx: &mut BuildContext, sender: MessageSender) -> Self {
        let preset = RagdollPreset::default();
        let container = Rc::new(make_property_editors_container(sender));

        let inspector;
        let ok;
        let cancel;
        let window = WindowBuilder::new(
            WidgetBuilder::new()
                .with_width(350.0)
                .with_height(500.0)
                .with_name("RagdollWizard"),
        )
        .open(false)
        .with_title(WindowTitle::text("Ragdoll Wizard"))
        .with_content(
            GridBuilder::new(
                WidgetBuilder::new()
                    .with_child({
                        inspector = InspectorBuilder::new(
                            WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                        )
                        .with_context(InspectorContext::from_object(
                            &preset,
                            ctx,
                            container,
                            None,
                            MSG_SYNC_FLAG,
                            0,
                            true,
                            Default::default(),
                        ))
                        .build(ctx);
                        inspector
                    })
                    .with_child(
                        StackPanelBuilder::new(
                            WidgetBuilder::new()
                                .with_horizontal_alignment(HorizontalAlignment::Right)
                                .on_row(1)
                                .with_margin(Thickness::uniform(1.0))
                                .with_child({
                                    ok = ButtonBuilder::new(
                                        WidgetBuilder::new()
                                            .with_width(100.0)
                                            .with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_text("OK")
                                    .build(ctx);
                                    ok
                                })
                                .with_child({
                                    cancel = ButtonBuilder::new(
                                        WidgetBuilder::new()
                                            .with_width(100.0)
                                            .with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_text("Cancel")
                                    .build(ctx);
                                    cancel
                                }),
                        )
                        .with_orientation(Orientation::Horizontal)
                        .build(ctx),
                    ),
            )
            .add_row(Row::stretch())
            .add_row(Row::strict(24.0))
            .add_column(Column::stretch())
            .build(ctx),
        )
        .build(ctx);

        Self {
            window,
            preset,
            inspector,
            ok,
            cancel,
        }
    }

    pub fn open(&self, ui: &UserInterface) {
        ui.send_message(WindowMessage::open(
            self.window,
            MessageDirection::ToWidget,
            true,
        ));
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, ui: &UserInterface) {
        if let Some(InspectorMessage::PropertyChanged(args)) = message.data() {
            if message.destination() == self.inspector
                && message.direction() == MessageDirection::FromWidget
            {
                PropertyAction::from_field_kind(&args.value).apply(
                    &args.path(),
                    &mut self.preset,
                    &mut |result| {
                        Log::verify(result);
                    },
                );
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.ok {
                // TODO: Add ragdoll generation.

                ui.send_message(WindowMessage::close(
                    self.window,
                    MessageDirection::ToWidget,
                ));
            } else if message.destination() == self.cancel {
                ui.send_message(WindowMessage::close(
                    self.window,
                    MessageDirection::ToWidget,
                ));
            }
        }
    }
}
