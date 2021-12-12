use crate::{
    inspector::handlers::node::base::handle_base_property_changed, make_command,
    scene::commands::particle_system::*, GraphSelection, SceneCommand,
};
use rg3d::{
    core::pool::Handle,
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        grid::{Column, GridBuilder, Row},
        inspector::{CollectionChanged, FieldKind, PropertyChanged},
        message::{MessageDirection, UiMessage},
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, UiNode, UserInterface,
    },
    scene::{
        node::Node,
        particle_system::{
            emitter::{
                base::BaseEmitter, cuboid::CuboidEmitter, cylinder::CylinderEmitter,
                sphere::SphereEmitter, Emitter,
            },
            ParticleSystem,
        },
    },
};
use std::any::TypeId;

pub struct ParticleSystemHandler {
    selector_window: Handle<UiNode>,
    sphere: Handle<UiNode>,
    cuboid: Handle<UiNode>,
    cylinder: Handle<UiNode>,
}

impl ParticleSystemHandler {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let sphere;
        let cuboid;
        let cylinder;
        let selector_window = WindowBuilder::new(WidgetBuilder::new())
            .open(false)
            .with_title(WindowTitle::text("Select Emitter to Add"))
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            sphere = ButtonBuilder::new(WidgetBuilder::new().on_column(0))
                                .with_text("Sphere")
                                .build(ctx);
                            sphere
                        })
                        .with_child({
                            cuboid = ButtonBuilder::new(WidgetBuilder::new().on_column(1))
                                .with_text("Cuboid")
                                .build(ctx);
                            cuboid
                        })
                        .with_child({
                            cylinder = ButtonBuilder::new(WidgetBuilder::new().on_column(2))
                                .with_text("Cylinder")
                                .build(ctx);
                            cylinder
                        }),
                )
                .add_column(Column::stretch())
                .add_column(Column::stretch())
                .add_column(Column::stretch())
                .add_row(Row::strict(25.0))
                .build(ctx),
            )
            .build(ctx);

        Self {
            selector_window,
            sphere,
            cuboid,
            cylinder,
        }
    }

    pub fn handle_ui_message(
        &self,
        message: &UiMessage,
        selection: &GraphSelection,
        ui: &UserInterface,
    ) -> Option<Vec<SceneCommand>> {
        if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            let emitter = if message.destination() == self.cuboid {
                Some(Emitter::Cuboid(Default::default()))
            } else if message.destination() == self.sphere {
                Some(Emitter::Sphere(Default::default()))
            } else if message.destination() == self.cylinder {
                Some(Emitter::Cylinder(Default::default()))
            } else {
                None
            };

            if let Some(emitter) = emitter {
                ui.send_message(WindowMessage::close(
                    self.selector_window,
                    MessageDirection::ToWidget,
                ));

                return Some(
                    selection
                        .nodes
                        .iter()
                        .map(|&node_handle| {
                            SceneCommand::new(AddParticleSystemEmitterCommand::new(
                                node_handle,
                                emitter.clone(),
                            ))
                        })
                        .collect::<Vec<_>>(),
                );
            }
        }
        None
    }

    pub fn handle(
        &self,
        args: &PropertyChanged,
        handle: Handle<Node>,
        node: &Node,
        ui: &UserInterface,
    ) -> Option<SceneCommand> {
        if let Node::ParticleSystem(particle_system) = node {
            match args.value {
                FieldKind::Object(ref value) => match args.name.as_ref() {
                    ParticleSystem::TEXTURE => {
                        make_command!(SetParticleSystemTextureCommand, handle, value)
                    }
                    ParticleSystem::ACCELERATION => {
                        make_command!(SetAccelerationCommand, handle, value)
                    }
                    ParticleSystem::ENABLED => {
                        make_command!(SetParticleSystemEnabledCommand, handle, value)
                    }
                    ParticleSystem::SOFT_BOUNDARY_SHARPNESS_FACTOR => {
                        make_command!(SetSoftBoundarySharpnessFactorCommand, handle, value)
                    }
                    _ => None,
                },
                FieldKind::Collection(ref collection_changed) => match args.name.as_ref() {
                    ParticleSystem::EMITTERS => match &**collection_changed {
                        CollectionChanged::Add => {
                            ui.send_message(WindowMessage::open_modal(
                                self.selector_window,
                                MessageDirection::ToWidget,
                                true,
                            ));
                            None
                        }
                        CollectionChanged::Remove(index) => {
                            Some(SceneCommand::new(DeleteEmitterCommand::new(handle, *index)))
                        }
                        CollectionChanged::ItemChanged { index, property } => {
                            let emitter = particle_system.emitters.get(*index)?;
                            if property.owner_type_id == TypeId::of::<SphereEmitter>() {
                                handle_sphere_emitter_property_changed(
                                    handle, emitter, property, *index,
                                )
                            } else if property.owner_type_id == TypeId::of::<CylinderEmitter>() {
                                handle_cylinder_emitter_property_changed(
                                    handle, emitter, property, *index,
                                )
                            } else if property.owner_type_id == TypeId::of::<CuboidEmitter>() {
                                handle_cuboid_emitter_property_changed(
                                    handle, emitter, property, *index,
                                )
                            } else {
                                None
                            }
                        }
                    },
                    _ => None,
                },
                FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
                    ParticleSystem::BASE => handle_base_property_changed(inner, handle, node),
                    _ => None,
                },
            }
        } else {
            None
        }
    }
}

fn handle_base_emitter_property_changed(
    handle: Handle<Node>,
    property_changed: &PropertyChanged,
    index: usize,
) -> Option<SceneCommand> {
    match property_changed.value {
        FieldKind::Object(ref value) => match property_changed.name.as_ref() {
            BaseEmitter::POSITION => Some(SceneCommand::new(SetEmitterPositionCommand::new(
                handle,
                index,
                value.cast_value_cloned()?,
            ))),
            BaseEmitter::PARTICLE_SPAWN_RATE => Some(SceneCommand::new(
                SetEmitterSpawnRateCommand::new(handle, index, value.cast_value_cloned()?),
            )),
            BaseEmitter::MAX_PARTICLES => Some(SceneCommand::new(
                SetEmitterParticleLimitCommand::new(handle, index, value.cast_value_cloned()?),
            )),
            BaseEmitter::LIFETIME => Some(SceneCommand::new(SetEmitterLifetimeRangeCommand::new(
                handle,
                index,
                value.cast_value_cloned()?,
            ))),
            BaseEmitter::SIZE => Some(SceneCommand::new(SetEmitterSizeRangeCommand::new(
                handle,
                index,
                value.cast_value_cloned()?,
            ))),
            BaseEmitter::SIZE_MODIFIER => Some(SceneCommand::new(
                SetEmitterSizeModifierRangeCommand::new(handle, index, value.cast_value_cloned()?),
            )),
            BaseEmitter::X_VELOCITY => Some(SceneCommand::new(
                SetEmitterXVelocityRangeCommand::new(handle, index, value.cast_value_cloned()?),
            )),
            BaseEmitter::Y_VELOCITY => Some(SceneCommand::new(
                SetEmitterYVelocityRangeCommand::new(handle, index, value.cast_value_cloned()?),
            )),
            BaseEmitter::Z_VELOCITY => Some(SceneCommand::new(
                SetEmitterZVelocityRangeCommand::new(handle, index, value.cast_value_cloned()?),
            )),
            BaseEmitter::ROTATION_SPEED => Some(SceneCommand::new(
                SetEmitterRotationSpeedRangeCommand::new(handle, index, value.cast_value_cloned()?),
            )),
            BaseEmitter::ROTATION => Some(SceneCommand::new(SetEmitterRotationRangeCommand::new(
                handle,
                index,
                value.cast_value_cloned()?,
            ))),
            BaseEmitter::RESURRECT_PARTICLES => Some(SceneCommand::new(
                SetEmitterResurrectParticlesCommand::new(handle, index, value.cast_value_cloned()?),
            )),
            _ => None,
        },
        _ => None,
    }
}

fn handle_sphere_emitter_property_changed(
    handle: Handle<Node>,
    emitter: &Emitter,
    property_changed: &PropertyChanged,
    index: usize,
) -> Option<SceneCommand> {
    if let Emitter::Sphere(_) = emitter {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                SphereEmitter::RADIUS => Some(SceneCommand::new(
                    SetSphereEmitterRadiusCommand::new(handle, index, value.cast_value().cloned()?),
                )),
                _ => None,
            },
            FieldKind::Inspectable(ref inner) => match property_changed.name.as_ref() {
                SphereEmitter::EMITTER => {
                    handle_base_emitter_property_changed(handle, inner, index)
                }
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}

fn handle_cylinder_emitter_property_changed(
    handle: Handle<Node>,
    emitter: &Emitter,
    property_changed: &PropertyChanged,
    index: usize,
) -> Option<SceneCommand> {
    if let Emitter::Cylinder(_) = emitter {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                CylinderEmitter::RADIUS => Some(SceneCommand::new(
                    SetCylinderEmitterRadiusCommand::new(handle, index, value.cast_value_cloned()?),
                )),
                CylinderEmitter::HEIGHT => Some(SceneCommand::new(
                    SetCylinderEmitterHeightCommand::new(handle, index, value.cast_value_cloned()?),
                )),
                _ => None,
            },
            FieldKind::Inspectable(ref inner) => match property_changed.name.as_ref() {
                CylinderEmitter::EMITTER => {
                    handle_base_emitter_property_changed(handle, inner, index)
                }
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}

fn handle_cuboid_emitter_property_changed(
    handle: Handle<Node>,
    emitter: &Emitter,
    property_changed: &PropertyChanged,
    index: usize,
) -> Option<SceneCommand> {
    if let Emitter::Cuboid(_) = emitter {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                CuboidEmitter::HALF_HEIGHT => Some(SceneCommand::new(
                    SetBoxEmitterHalfHeightCommand::new(handle, index, value.cast_value_cloned()?),
                )),
                CuboidEmitter::HALF_WIDTH => Some(SceneCommand::new(
                    SetBoxEmitterHalfWidthCommand::new(handle, index, value.cast_value_cloned()?),
                )),
                CuboidEmitter::HALF_DEPTH => Some(SceneCommand::new(
                    SetBoxEmitterHalfDepthCommand::new(handle, index, value.cast_value_cloned()?),
                )),
                _ => None,
            },
            FieldKind::Inspectable(ref inner) => match property_changed.name.as_ref() {
                CylinderEmitter::EMITTER => {
                    handle_base_emitter_property_changed(handle, inner, index)
                }
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}
