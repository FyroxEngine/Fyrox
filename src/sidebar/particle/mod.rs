use crate::gui::DeletableItem;
use crate::sidebar::make_section;
use crate::{
    gui::{DeletableItemBuilder, DeletableItemMessage},
    load_image,
    scene::commands::particle_system::{
        AddParticleSystemEmitterCommand, DeleteEmitterCommand, SetParticleSystemAccelerationCommand,
    },
    send_sync_message,
    sidebar::{
        make_text_mark, make_vec3_input_field, particle::emitter::EmitterSection, COLUMN_WIDTH,
        ROW_HEIGHT,
    },
    Message,
};
use rg3d::gui::dropdown_list::DropdownList;
use rg3d::gui::message::UiMessage;
use rg3d::gui::vec::vec3::Vec3EditorMessage;
use rg3d::gui::{BuildContext, UiNode, UserInterface};
use rg3d::{
    core::{pool::Handle, scope_profile},
    gui::{
        button::ButtonBuilder,
        dropdown_list::DropdownListBuilder,
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        message::{
            ButtonMessage, DropdownListMessage, MessageDirection, UiMessageData, WidgetMessage,
        },
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        widget::WidgetBuilder,
        HorizontalAlignment, Orientation, Thickness, VerticalAlignment,
    },
    scene::{
        node::Node,
        particle_system::emitter::{
            base::BaseEmitterBuilder, cuboid::CuboidEmitterBuilder,
            cylinder::CylinderEmitterBuilder, sphere::SphereEmitterBuilder, Emitter,
        },
    },
};
use std::sync::mpsc::Sender;

mod cuboid;
mod cylinder;
mod emitter;
mod sphere;

pub struct ParticleSystemSection {
    pub section: Handle<UiNode>,
    acceleration: Handle<UiNode>,
    add_box_emitter: Handle<UiNode>,
    add_sphere_emitter: Handle<UiNode>,
    add_cylinder_emitter: Handle<UiNode>,
    emitters: Handle<UiNode>,
    sender: Sender<Message>,
    emitter_index: Option<usize>,
    emitter_section: EmitterSection,
    play_pause: Handle<UiNode>,
    stop: Handle<UiNode>,
    restart: Handle<UiNode>,
}

fn make_button_image(ctx: &mut BuildContext, image_data: &[u8]) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .with_width(ROW_HEIGHT - 11.0)
            .with_height(ROW_HEIGHT - 11.0)
            .with_margin(Thickness::uniform(1.0)),
    )
    .with_content(
        ImageBuilder::new(WidgetBuilder::new())
            .with_opt_texture(load_image(image_data))
            .build(ctx),
    )
    .build(ctx)
}

impl ParticleSystemSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let emitter_section = EmitterSection::new(ctx, sender.clone());

        let box_emitter_img = include_bytes!("../../../resources/embed/add_box_emitter.png");
        let sphere_emitter_img = include_bytes!("../../../resources/embed/add_sphere_emitter.png");
        let cylinder_emitter_img =
            include_bytes!("../../../resources/embed/add_cylinder_emitter.png");

        let play_pause;
        let stop;
        let restart;
        let acceleration;
        let emitters;
        let add_box_emitter;
        let add_sphere_emitter;
        let add_cylinder_emitter;
        let section = make_section(
            "Particle System Properties",
            StackPanelBuilder::new(
                WidgetBuilder::new()
                    .with_child(
                        GridBuilder::new(
                            WidgetBuilder::new()
                                .with_child(make_text_mark(ctx, "Acceleration", 0))
                                .with_child({
                                    acceleration = make_vec3_input_field(ctx, 0);
                                    acceleration
                                })
                                .with_child(make_text_mark(ctx, "Emitters", 1))
                                .with_child(
                                    GridBuilder::new(
                                        WidgetBuilder::new()
                                            .on_row(1)
                                            .on_column(1)
                                            .with_child(
                                                StackPanelBuilder::new(
                                                    WidgetBuilder::new()
                                                        .on_row(0)
                                                        .with_child(
                                                            TextBuilder::new(WidgetBuilder::new())
                                                                .with_text("Add Emitter: ")
                                                                .with_vertical_text_alignment(
                                                                    VerticalAlignment::Center,
                                                                )
                                                                .build(ctx),
                                                        )
                                                        .with_child({
                                                            add_box_emitter = make_button_image(
                                                                ctx,
                                                                box_emitter_img,
                                                            );
                                                            add_box_emitter
                                                        })
                                                        .with_child({
                                                            add_sphere_emitter = make_button_image(
                                                                ctx,
                                                                sphere_emitter_img,
                                                            );
                                                            add_sphere_emitter
                                                        })
                                                        .with_child({
                                                            add_cylinder_emitter =
                                                                make_button_image(
                                                                    ctx,
                                                                    cylinder_emitter_img,
                                                                );
                                                            add_cylinder_emitter
                                                        }),
                                                )
                                                .with_orientation(Orientation::Horizontal)
                                                .build(ctx),
                                            )
                                            .with_child({
                                                emitters = DropdownListBuilder::new(
                                                    WidgetBuilder::new().on_row(1),
                                                )
                                                .with_close_on_selection(true)
                                                .build(ctx);
                                                emitters
                                            }),
                                    )
                                    .add_row(Row::strict(ROW_HEIGHT))
                                    .add_row(Row::strict(ROW_HEIGHT))
                                    .add_column(Column::stretch())
                                    .build(ctx),
                                ),
                        )
                        .add_column(Column::strict(COLUMN_WIDTH))
                        .add_column(Column::stretch())
                        .add_row(Row::strict(ROW_HEIGHT))
                        .add_row(Row::strict(ROW_HEIGHT * 2.0))
                        .build(ctx),
                    )
                    .with_child(
                        GridBuilder::new(
                            WidgetBuilder::new()
                                .with_child({
                                    restart = ButtonBuilder::new(
                                        WidgetBuilder::new()
                                            .on_column(0)
                                            .with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_text("Restart")
                                    .build(ctx);
                                    restart
                                })
                                .with_child({
                                    play_pause = ButtonBuilder::new(
                                        WidgetBuilder::new()
                                            .on_column(1)
                                            .with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_text("Play/Pause")
                                    .build(ctx);
                                    play_pause
                                })
                                .with_child({
                                    stop = ButtonBuilder::new(
                                        WidgetBuilder::new()
                                            .on_column(2)
                                            .with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_text("Stop")
                                    .build(ctx);
                                    stop
                                }),
                        )
                        .add_column(Column::stretch())
                        .add_column(Column::stretch())
                        .add_column(Column::stretch())
                        .add_row(Row::strict(ROW_HEIGHT))
                        .build(ctx),
                    )
                    .with_child(emitter_section.section),
            )
            .build(ctx),
            ctx,
        );

        Self {
            section,
            acceleration,
            add_box_emitter,
            add_sphere_emitter,
            add_cylinder_emitter,
            emitters,
            sender,
            emitter_index: None,
            emitter_section,
            stop,
            play_pause,
            restart,
        }
    }

    pub fn sync_to_model(&mut self, node: &Node, ui: &mut UserInterface) {
        send_sync_message(
            ui,
            WidgetMessage::visibility(
                self.section,
                MessageDirection::ToWidget,
                node.is_particle_system(),
            ),
        );

        if let Node::ParticleSystem(particle_system) = node {
            send_sync_message(
                ui,
                Vec3EditorMessage::value(
                    self.acceleration,
                    MessageDirection::ToWidget,
                    particle_system.acceleration(),
                ),
            );

            let ctx = &mut ui.build_ctx();
            let emitters = particle_system
                .emitters
                .iter()
                .enumerate()
                .map(|(i, e)| {
                    let item = DeletableItemBuilder::new(WidgetBuilder::new())
                        .with_content(
                            TextBuilder::new(WidgetBuilder::new())
                                .with_text(match e {
                                    Emitter::Unknown => unreachable!(),
                                    Emitter::Cuboid(_) => "Box",
                                    Emitter::Sphere(_) => "Sphere",
                                    Emitter::Cylinder(_) => "Cylinder",
                                })
                                .with_horizontal_text_alignment(HorizontalAlignment::Center)
                                .with_vertical_text_alignment(VerticalAlignment::Center)
                                .build(ctx),
                        )
                        .with_data(i)
                        .build(ctx);
                    ctx.add_node(UiNode::new(item))
                })
                .collect::<Vec<_>>();

            // Try to keep selection.
            if let Some(emitter_index) = self.emitter_index {
                if emitter_index >= particle_system.emitters.len() {
                    self.emitter_index = None;
                }
            } else if !particle_system.emitters.is_empty() {
                self.emitter_index = Some(0);
            }

            send_sync_message(
                ui,
                DropdownListMessage::items(self.emitters, MessageDirection::ToWidget, emitters),
            );

            send_sync_message(
                ui,
                DropdownListMessage::selection(
                    self.emitters,
                    MessageDirection::ToWidget,
                    self.emitter_index,
                ),
            );

            if let Some(emitter_index) = self.emitter_index {
                self.emitter_section
                    .sync_to_model(&particle_system.emitters[emitter_index], ui);
            }
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        node: &mut Node,
        handle: Handle<Node>,
        ui: &UserInterface,
    ) {
        scope_profile!();

        if let Node::ParticleSystem(particle_system) = node {
            if let Some(emitter_index) = self.emitter_index {
                self.emitter_section.handle_message(
                    message,
                    &particle_system.emitters[emitter_index],
                    emitter_index,
                    handle,
                );
            }

            match message.data() {
                UiMessageData::Button(ButtonMessage::Click) => {
                    if message.destination() == self.add_box_emitter {
                        self.sender
                            .send(Message::do_scene_command(
                                AddParticleSystemEmitterCommand::new(
                                    handle,
                                    CuboidEmitterBuilder::new(BaseEmitterBuilder::new()).build(),
                                ),
                            ))
                            .unwrap();
                    } else if message.destination() == self.add_sphere_emitter {
                        self.sender
                            .send(Message::do_scene_command(
                                AddParticleSystemEmitterCommand::new(
                                    handle,
                                    SphereEmitterBuilder::new(BaseEmitterBuilder::new()).build(),
                                ),
                            ))
                            .unwrap();
                    } else if message.destination() == self.add_cylinder_emitter {
                        self.sender
                            .send(Message::do_scene_command(
                                AddParticleSystemEmitterCommand::new(
                                    handle,
                                    CylinderEmitterBuilder::new(BaseEmitterBuilder::new()).build(),
                                ),
                            ))
                            .unwrap();
                    } else if message.destination() == self.restart {
                        particle_system.clear_particles();
                    } else if message.destination() == self.stop {
                        particle_system.set_enabled(false); // TODO: Do this via command.
                        particle_system.clear_particles();
                    } else if message.destination() == self.play_pause {
                        let new_state = !particle_system.is_enabled();
                        particle_system.set_enabled(new_state);
                    }
                }
                UiMessageData::User(msg) => {
                    if let Some(DeletableItemMessage::Delete) = msg.cast::<DeletableItemMessage>() {
                        if ui
                            .node(self.emitters)
                            .cast::<DropdownList>()
                            .unwrap()
                            .items()
                            .contains(&message.destination())
                        {
                            if let Some(ei) = ui
                                .node(message.destination())
                                .cast::<DeletableItem<usize>>()
                            {
                                self.sender
                                    .send(Message::do_scene_command(DeleteEmitterCommand::new(
                                        handle,
                                        ei.data.unwrap(),
                                    )))
                                    .unwrap();
                            } else {
                                unreachable!()
                            }
                        }
                    } else if let Some(Vec3EditorMessage::Value(value)) =
                        msg.cast::<Vec3EditorMessage<f32>>()
                    {
                        if particle_system.acceleration() != *value
                            && message.destination() == self.acceleration
                        {
                            self.sender
                                .send(Message::do_scene_command(
                                    SetParticleSystemAccelerationCommand::new(handle, *value),
                                ))
                                .unwrap();
                        }
                    }
                }
                UiMessageData::DropdownList(DropdownListMessage::SelectionChanged(selection)) => {
                    if message.destination() == self.emitters {
                        self.emitter_index = *selection;
                        self.sender.send(Message::SyncToModel).unwrap();
                    }
                }
                _ => {}
            }
        }
    }
}
