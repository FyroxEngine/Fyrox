use crate::{
    scene::commands::particle_system::{
        EmitterNumericParameter, SetEmitterNumericParameterCommand, SetEmitterPositionCommand,
        SetEmitterResurrectParticlesCommand,
    },
    send_sync_message,
    sidebar::{
        make_bool_input_field, make_f32_input_field, make_section, make_text_mark,
        make_vec3_input_field,
        particle::{cuboid::BoxSection, cylinder::CylinderSection, sphere::SphereSection},
        COLUMN_WIDTH, ROW_HEIGHT,
    },
    Message,
};
use rg3d::gui::message::UiMessage;
use rg3d::gui::numeric::NumericUpDownMessage;
use rg3d::gui::vec::vec3::Vec3EditorMessage;
use rg3d::gui::{BuildContext, UiNode, UserInterface};
use rg3d::{
    core::pool::Handle,
    gui::{
        grid::{Column, GridBuilder, Row},
        message::{CheckBoxMessage, MessageDirection, UiMessageData, WidgetMessage},
        numeric::NumericUpDownBuilder,
        stack_panel::StackPanelBuilder,
        widget::WidgetBuilder,
        Thickness,
    },
    scene::{
        node::Node,
        particle_system::{emitter::Emitter, ParticleLimit},
    },
};
use std::sync::mpsc::Sender;

pub struct EmitterSection {
    pub section: Handle<UiNode>,
    position: Handle<UiNode>,
    spawn_rate: Handle<UiNode>,
    max_particles: Handle<UiNode>,
    min_lifetime: Handle<UiNode>,
    max_lifetime: Handle<UiNode>,
    min_size_modifier: Handle<UiNode>,
    max_size_modifier: Handle<UiNode>,
    min_x_velocity: Handle<UiNode>,
    max_x_velocity: Handle<UiNode>,
    min_y_velocity: Handle<UiNode>,
    max_y_velocity: Handle<UiNode>,
    min_z_velocity: Handle<UiNode>,
    max_z_velocity: Handle<UiNode>,
    min_rotation_speed: Handle<UiNode>,
    max_rotation_speed: Handle<UiNode>,
    min_rotation: Handle<UiNode>,
    max_rotation: Handle<UiNode>,
    resurrect_particles: Handle<UiNode>,
    sender: Sender<Message>,
    sphere_section: SphereSection,
    cylinder_section: CylinderSection,
    box_section: BoxSection,
}

fn make_range_field(ctx: &mut BuildContext, column: usize) -> Handle<UiNode> {
    NumericUpDownBuilder::<f32>::new(
        WidgetBuilder::new()
            .on_column(column)
            .with_margin(Thickness::uniform(1.0)),
    )
    .build(ctx)
}

fn make_range(
    ctx: &mut BuildContext,
    row: usize,
) -> (Handle<UiNode>, Handle<UiNode>, Handle<UiNode>) {
    let min;
    let max;
    let grid = GridBuilder::new(
        WidgetBuilder::new()
            .on_row(row)
            .on_column(1)
            .with_child({
                min = make_range_field(ctx, 0);
                min
            })
            .with_child({
                max = make_range_field(ctx, 1);
                max
            }),
    )
    .add_column(Column::stretch())
    .add_column(Column::stretch())
    .add_row(Row::stretch())
    .build(ctx);
    (grid, min, max)
}

impl EmitterSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let sphere_section = SphereSection::new(ctx, sender.clone());
        let cylinder_section = CylinderSection::new(ctx, sender.clone());
        let box_section = BoxSection::new(ctx, sender.clone());

        let position;
        let spawn_rate;
        let max_particles;
        let min_lifetime;
        let max_lifetime;
        let min_size_modifier;
        let max_size_modifier;
        let min_x_velocity;
        let max_x_velocity;
        let min_y_velocity;
        let max_y_velocity;
        let min_z_velocity;
        let max_z_velocity;
        let min_rotation_speed;
        let max_rotation_speed;
        let min_rotation;
        let max_rotation;
        let resurrect_particles;
        let common_properties = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(make_text_mark(ctx, "Position", 0))
                .with_child({
                    position = make_vec3_input_field(ctx, 0);
                    position
                })
                .with_child(make_text_mark(ctx, "Spawn Rate", 1))
                .with_child({
                    spawn_rate = make_f32_input_field(ctx, 1, 0.0, std::f32::MAX, 1.0);
                    spawn_rate
                })
                .with_child(make_text_mark(ctx, "Max Particles", 2))
                .with_child({
                    max_particles = make_f32_input_field(ctx, 2, 0.0, std::f32::MAX, 1.0);
                    max_particles
                })
                .with_child(make_text_mark(ctx, "Lifetime Range", 3))
                .with_child({
                    let (grid, min, max) = make_range(ctx, 3);
                    min_lifetime = min;
                    max_lifetime = max;
                    grid
                })
                .with_child(make_text_mark(ctx, "Size Modifier Range", 4))
                .with_child({
                    let (grid, min, max) = make_range(ctx, 4);
                    min_size_modifier = min;
                    max_size_modifier = max;
                    grid
                })
                .with_child(make_text_mark(ctx, "X Velocity Range", 5))
                .with_child({
                    let (grid, min, max) = make_range(ctx, 5);
                    min_x_velocity = min;
                    max_x_velocity = max;
                    grid
                })
                .with_child(make_text_mark(ctx, "Y Velocity Range", 6))
                .with_child({
                    let (grid, min, max) = make_range(ctx, 6);
                    min_y_velocity = min;
                    max_y_velocity = max;
                    grid
                })
                .with_child(make_text_mark(ctx, "Z Velocity Range", 7))
                .with_child({
                    let (grid, min, max) = make_range(ctx, 7);
                    min_z_velocity = min;
                    max_z_velocity = max;
                    grid
                })
                .with_child(make_text_mark(ctx, "Rotation Speed Range", 8))
                .with_child({
                    let (grid, min, max) = make_range(ctx, 8);
                    min_rotation_speed = min;
                    max_rotation_speed = max;
                    grid
                })
                .with_child(make_text_mark(ctx, "Rotation Range", 9))
                .with_child({
                    let (grid, min, max) = make_range(ctx, 9);
                    min_rotation = min;
                    max_rotation = max;
                    grid
                })
                .with_child(make_text_mark(ctx, "Resurrect Particles", 10))
                .with_child({
                    resurrect_particles = make_bool_input_field(ctx, 10);
                    resurrect_particles
                }),
        )
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .add_column(Column::strict(COLUMN_WIDTH))
        .add_column(Column::stretch())
        .build(ctx);

        let section = make_section(
            "Emitter Properties",
            StackPanelBuilder::new(
                WidgetBuilder::new()
                    .with_child(common_properties)
                    .with_child(sphere_section.section)
                    .with_child(cylinder_section.section)
                    .with_child(box_section.section),
            )
            .build(ctx),
            ctx,
        );

        Self {
            section,
            sender,
            position,
            spawn_rate,
            max_particles,
            min_lifetime,
            max_lifetime,
            min_size_modifier,
            max_size_modifier,
            min_x_velocity,
            max_x_velocity,
            min_y_velocity,
            max_y_velocity,
            min_z_velocity,
            max_z_velocity,
            min_rotation_speed,
            max_rotation_speed,
            min_rotation,
            max_rotation,
            resurrect_particles,
            sphere_section,
            cylinder_section,
            box_section,
        }
    }

    pub fn sync_to_model(&mut self, emitter: &Emitter, ui: &mut UserInterface) {
        send_sync_message(
            ui,
            Vec3EditorMessage::value(
                self.position,
                MessageDirection::ToWidget,
                emitter.position(),
            ),
        );

        let sync_f32 = |destination: Handle<UiNode>, value: f32| {
            send_sync_message(
                ui,
                NumericUpDownMessage::value(destination, MessageDirection::ToWidget, value),
            );
        };

        sync_f32(
            self.max_particles,
            match emitter.max_particles() {
                ParticleLimit::Unlimited => -1.0,
                ParticleLimit::Strict(value) => value as f32,
            },
        );
        sync_f32(self.spawn_rate, emitter.spawn_rate() as f32);
        sync_f32(self.min_lifetime, emitter.life_time_range().bounds[0]);
        sync_f32(self.max_lifetime, emitter.life_time_range().bounds[1]);
        sync_f32(
            self.min_size_modifier,
            emitter.size_modifier_range().bounds[0],
        );
        sync_f32(
            self.max_size_modifier,
            emitter.size_modifier_range().bounds[1],
        );
        sync_f32(self.min_x_velocity, emitter.x_velocity_range().bounds[0]);
        sync_f32(self.max_x_velocity, emitter.x_velocity_range().bounds[1]);
        sync_f32(self.min_y_velocity, emitter.y_velocity_range().bounds[0]);
        sync_f32(self.max_y_velocity, emitter.y_velocity_range().bounds[1]);
        sync_f32(self.min_z_velocity, emitter.z_velocity_range().bounds[0]);
        sync_f32(self.max_z_velocity, emitter.z_velocity_range().bounds[1]);
        sync_f32(
            self.min_rotation_speed,
            emitter.rotation_speed_range().bounds[0],
        );
        sync_f32(
            self.max_rotation_speed,
            emitter.rotation_speed_range().bounds[1],
        );
        sync_f32(self.min_rotation, emitter.rotation_range().bounds[0]);
        sync_f32(self.max_rotation, emitter.rotation_range().bounds[1]);

        send_sync_message(
            ui,
            CheckBoxMessage::checked(
                self.resurrect_particles,
                MessageDirection::ToWidget,
                Some(emitter.is_particles_resurrects()),
            ),
        );

        fn toggle_visibility(ui: &mut UserInterface, destination: Handle<UiNode>, value: bool) {
            send_sync_message(
                ui,
                WidgetMessage::visibility(destination, MessageDirection::ToWidget, value),
            );
        }

        toggle_visibility(ui, self.sphere_section.section, false);
        toggle_visibility(ui, self.cylinder_section.section, false);
        toggle_visibility(ui, self.box_section.section, false);

        match emitter {
            Emitter::Unknown => unreachable!(),
            Emitter::Cuboid(box_emitter) => {
                toggle_visibility(ui, self.box_section.section, true);
                self.box_section.sync_to_model(box_emitter, ui);
            }
            Emitter::Sphere(sphere) => {
                toggle_visibility(ui, self.sphere_section.section, true);
                self.sphere_section.sync_to_model(sphere, ui);
            }
            Emitter::Cylinder(cylinder) => {
                toggle_visibility(ui, self.cylinder_section.section, true);
                self.cylinder_section.sync_to_model(cylinder, ui);
            }
        }
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        emitter: &Emitter,
        emitter_index: usize,
        handle: Handle<Node>,
    ) {
        match emitter {
            Emitter::Unknown => unreachable!(),
            Emitter::Cuboid(box_emitter) => {
                self.box_section
                    .handle_message(message, box_emitter, handle, emitter_index);
            }
            Emitter::Sphere(sphere) => {
                self.sphere_section
                    .handle_message(message, sphere, handle, emitter_index);
            }
            Emitter::Cylinder(cylinder) => {
                self.cylinder_section
                    .handle_message(message, cylinder, handle, emitter_index);
            }
        }

        match message.data() {
            UiMessageData::User(msg) => {
                if let Some(&NumericUpDownMessage::Value(value)) =
                    msg.cast::<NumericUpDownMessage<f32>>()
                {
                    let mut parameter = None;
                    let mut final_value = value;

                    if message.destination() == self.max_particles {
                        let max_particles = match emitter.max_particles() {
                            ParticleLimit::Unlimited => -1.0,
                            ParticleLimit::Strict(value) => value as f32,
                        };
                        if max_particles.ne(&value) {
                            parameter = Some(EmitterNumericParameter::MaxParticles);
                        }
                    } else if message.destination() == self.spawn_rate
                        && (emitter.spawn_rate() as f32).ne(&value)
                    {
                        parameter = Some(EmitterNumericParameter::SpawnRate);
                    } else if message.destination() == self.min_lifetime
                        && emitter.life_time_range().bounds[0].ne(&value)
                    {
                        parameter = Some(EmitterNumericParameter::MinLifetime);
                        emitter.life_time_range().clamp_value(&mut final_value);
                    } else if message.destination() == self.max_lifetime
                        && emitter.life_time_range().bounds[1].ne(&value)
                    {
                        parameter = Some(EmitterNumericParameter::MaxLifetime);
                        emitter.life_time_range().clamp_value(&mut final_value);
                    } else if message.destination() == self.min_size_modifier
                        && emitter.size_modifier_range().bounds[0].ne(&value)
                    {
                        parameter = Some(EmitterNumericParameter::MinSizeModifier);
                        emitter.size_modifier_range().clamp_value(&mut final_value);
                    } else if message.destination() == self.max_size_modifier
                        && emitter.size_modifier_range().bounds[1].ne(&value)
                    {
                        parameter = Some(EmitterNumericParameter::MaxSizeModifier);
                        emitter.size_modifier_range().clamp_value(&mut final_value);
                    } else if message.destination() == self.min_x_velocity
                        && emitter.x_velocity_range().bounds[0].ne(&value)
                    {
                        parameter = Some(EmitterNumericParameter::MinXVelocity);
                        emitter.x_velocity_range().clamp_value(&mut final_value);
                    } else if message.destination() == self.max_x_velocity
                        && emitter.x_velocity_range().bounds[1].ne(&value)
                    {
                        parameter = Some(EmitterNumericParameter::MaxXVelocity);
                        emitter.x_velocity_range().clamp_value(&mut final_value);
                    } else if message.destination() == self.min_y_velocity
                        && emitter.y_velocity_range().bounds[0].ne(&value)
                    {
                        parameter = Some(EmitterNumericParameter::MinYVelocity);
                        emitter.y_velocity_range().clamp_value(&mut final_value);
                    } else if message.destination() == self.max_y_velocity
                        && emitter.y_velocity_range().bounds[1].ne(&value)
                    {
                        parameter = Some(EmitterNumericParameter::MaxYVelocity);
                        emitter.y_velocity_range().clamp_value(&mut final_value);
                    } else if message.destination() == self.min_z_velocity
                        && emitter.z_velocity_range().bounds[0].ne(&value)
                    {
                        emitter.z_velocity_range().clamp_value(&mut final_value);
                        parameter = Some(EmitterNumericParameter::MinZVelocity);
                    } else if message.destination() == self.max_z_velocity
                        && emitter.z_velocity_range().bounds[1].ne(&value)
                    {
                        parameter = Some(EmitterNumericParameter::MaxZVelocity);
                        emitter.z_velocity_range().clamp_value(&mut final_value);
                    } else if message.destination() == self.min_rotation_speed
                        && emitter.rotation_speed_range().bounds[0].ne(&value)
                    {
                        parameter = Some(EmitterNumericParameter::MinRotationSpeed);
                        emitter.rotation_speed_range().clamp_value(&mut final_value);
                    } else if message.destination() == self.max_rotation_speed
                        && emitter.rotation_speed_range().bounds[1].ne(&value)
                    {
                        parameter = Some(EmitterNumericParameter::MaxRotationSpeed);
                        emitter.rotation_speed_range().clamp_value(&mut final_value);
                    } else if message.destination() == self.min_rotation
                        && emitter.rotation_range().bounds[0].ne(&value)
                    {
                        parameter = Some(EmitterNumericParameter::MinRotation);
                        emitter.rotation_range().clamp_value(&mut final_value);
                    } else if message.destination() == self.max_rotation
                        && emitter.rotation_range().bounds[1].ne(&value)
                    {
                        parameter = Some(EmitterNumericParameter::MaxRotation);
                        emitter.rotation_range().clamp_value(&mut final_value);
                    }
                    if let Some(parameter) = parameter {
                        self.sender
                            .send(Message::do_scene_command(
                                SetEmitterNumericParameterCommand::new(
                                    handle,
                                    emitter_index,
                                    parameter,
                                    final_value,
                                ),
                            ))
                            .unwrap();
                    }
                } else if let Some(&Vec3EditorMessage::Value(value)) =
                    msg.cast::<Vec3EditorMessage<f32>>()
                {
                    if message.destination() == self.position && emitter.position().ne(&value) {
                        self.sender
                            .send(Message::do_scene_command(SetEmitterPositionCommand::new(
                                handle,
                                emitter_index,
                                value,
                            )))
                            .unwrap();
                    }
                }
            }
            UiMessageData::CheckBox(CheckBoxMessage::Check(Some(value)))
                if message.destination() == self.resurrect_particles =>
            {
                if emitter.is_particles_resurrects() != *value {
                    self.sender
                        .send(Message::do_scene_command(
                            SetEmitterResurrectParticlesCommand::new(handle, emitter_index, *value),
                        ))
                        .unwrap();
                }
            }

            _ => {}
        }
    }
}
