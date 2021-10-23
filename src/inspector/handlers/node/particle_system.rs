use crate::{
    do_command, inspector::handlers::node::base::handle_base_property_changed,
    inspector::SenderHelper, scene::commands::particle_system::*,
};
use rg3d::scene::particle_system::emitter::cuboid::CuboidEmitter;
use rg3d::scene::particle_system::emitter::cylinder::CylinderEmitter;
use rg3d::{
    core::pool::Handle,
    gui::{
        button::ButtonBuilder,
        grid::{Column, GridBuilder, Row},
        message::{
            ButtonMessage, CollectionChanged, FieldKind, MessageDirection, PropertyChanged,
            UiMessage, UiMessageData, WindowMessage,
        },
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        BuildContext, UiNode, UserInterface,
    },
    scene::{
        node::Node,
        particle_system::{
            emitter::{base::BaseEmitter, sphere::SphereEmitter, Emitter},
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
        node_handle: Handle<Node>,
        helper: &SenderHelper,
        ui: &UserInterface,
    ) {
        if let UiMessageData::Button(ButtonMessage::Click) = message.data() {
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
                helper.do_scene_command(AddParticleSystemEmitterCommand::new(node_handle, emitter));
                ui.send_message(WindowMessage::close(
                    self.selector_window,
                    MessageDirection::ToWidget,
                ));
            }
        }
    }

    pub fn handle(
        &self,
        args: &PropertyChanged,
        handle: Handle<Node>,
        node: &Node,
        helper: &SenderHelper,
        ui: &UserInterface,
    ) -> Option<()> {
        if let Node::ParticleSystem(particle_system) = node {
            match args.value {
                FieldKind::Object(ref value) => match args.name.as_ref() {
                    ParticleSystem::TEXTURE => {
                        do_command!(helper, SetParticleSystemTextureCommand, handle, value)
                    }
                    ParticleSystem::ACCELERATION => {
                        do_command!(helper, SetAccelerationCommand, handle, value)
                    }
                    ParticleSystem::ENABLED => {
                        do_command!(helper, SetParticleSystemEnabledCommand, handle, value)
                    }
                    ParticleSystem::SOFT_BOUNDARY_SHARPNESS_FACTOR => {
                        do_command!(helper, SetSoftBoundarySharpnessFactorCommand, handle, value)
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
                            Some(())
                        }
                        CollectionChanged::Remove(index) => {
                            helper.do_scene_command(DeleteEmitterCommand::new(handle, *index))
                        }
                        CollectionChanged::ItemChanged { index, property } => {
                            let emitter = particle_system.emitters.get(*index)?;
                            if property.owner_type_id == TypeId::of::<SphereEmitter>() {
                                handle_sphere_emitter_property_changed(
                                    handle, emitter, property, helper, *index,
                                )
                            } else if property.owner_type_id == TypeId::of::<CylinderEmitter>() {
                                handle_cylinder_emitter_property_changed(
                                    handle, emitter, property, helper, *index,
                                )
                            } else if property.owner_type_id == TypeId::of::<CuboidEmitter>() {
                                handle_cuboid_emitter_property_changed(
                                    handle, emitter, property, helper, *index,
                                )
                            } else {
                                None
                            }
                        }
                    },
                    _ => None,
                },
                FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
                    ParticleSystem::BASE => {
                        handle_base_property_changed(&inner, handle, node, helper)
                    }
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
    helper: &SenderHelper,
    index: usize,
) -> Option<()> {
    match property_changed.value {
        FieldKind::Object(ref value) => match property_changed.name.as_ref() {
            BaseEmitter::POSITION => helper.do_scene_command(SetEmitterPositionCommand::new(
                handle,
                index,
                value.cast_value_cloned()?,
            )),
            BaseEmitter::PARTICLE_SPAWN_RATE => helper.do_scene_command(
                SetEmitterSpawnRateCommand::new(handle, index, value.cast_value_cloned()?),
            ),
            BaseEmitter::MAX_PARTICLES => helper.do_scene_command(
                SetEmitterParticleLimitCommand::new(handle, index, value.cast_value_cloned()?),
            ),
            BaseEmitter::LIFETIME => helper.do_scene_command(SetEmitterLifetimeRangeCommand::new(
                handle,
                index,
                value.cast_value_cloned()?,
            )),
            BaseEmitter::SIZE => helper.do_scene_command(SetEmitterSizeRangeCommand::new(
                handle,
                index,
                value.cast_value_cloned()?,
            )),
            BaseEmitter::SIZE_MODIFIER => helper.do_scene_command(
                SetEmitterSizeModifierRangeCommand::new(handle, index, value.cast_value_cloned()?),
            ),
            BaseEmitter::X_VELOCITY => helper.do_scene_command(
                SetEmitterXVelocityRangeCommand::new(handle, index, value.cast_value_cloned()?),
            ),
            BaseEmitter::Y_VELOCITY => helper.do_scene_command(
                SetEmitterYVelocityRangeCommand::new(handle, index, value.cast_value_cloned()?),
            ),
            BaseEmitter::Z_VELOCITY => helper.do_scene_command(
                SetEmitterZVelocityRangeCommand::new(handle, index, value.cast_value_cloned()?),
            ),
            BaseEmitter::ROTATION_SPEED => helper.do_scene_command(
                SetEmitterRotationSpeedRangeCommand::new(handle, index, value.cast_value_cloned()?),
            ),
            BaseEmitter::ROTATION => helper.do_scene_command(SetEmitterRotationRangeCommand::new(
                handle,
                index,
                value.cast_value_cloned()?,
            )),
            BaseEmitter::RESURRECT_PARTICLES => helper.do_scene_command(
                SetEmitterResurrectParticlesCommand::new(handle, index, value.cast_value_cloned()?),
            ),
            _ => None,
        },
        _ => None,
    }
}

fn handle_sphere_emitter_property_changed(
    handle: Handle<Node>,
    emitter: &Emitter,
    property_changed: &PropertyChanged,
    helper: &SenderHelper,
    index: usize,
) -> Option<()> {
    if let Emitter::Sphere(_) = emitter {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                SphereEmitter::RADIUS => helper.do_scene_command(
                    SetSphereEmitterRadiusCommand::new(handle, index, value.cast_value().cloned()?),
                ),
                _ => None,
            },
            FieldKind::Inspectable(ref inner) => match property_changed.name.as_ref() {
                SphereEmitter::EMITTER => {
                    handle_base_emitter_property_changed(handle, inner, helper, index)
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
    helper: &SenderHelper,
    index: usize,
) -> Option<()> {
    if let Emitter::Cylinder(_) = emitter {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                CylinderEmitter::RADIUS => helper.do_scene_command(
                    SetCylinderEmitterRadiusCommand::new(handle, index, value.cast_value_cloned()?),
                ),
                CylinderEmitter::HEIGHT => helper.do_scene_command(
                    SetCylinderEmitterHeightCommand::new(handle, index, value.cast_value_cloned()?),
                ),
                _ => None,
            },
            FieldKind::Inspectable(ref inner) => match property_changed.name.as_ref() {
                CylinderEmitter::EMITTER => {
                    handle_base_emitter_property_changed(handle, inner, helper, index)
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
    helper: &SenderHelper,
    index: usize,
) -> Option<()> {
    if let Emitter::Cuboid(_) = emitter {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                CuboidEmitter::HALF_HEIGHT => helper.do_scene_command(
                    SetBoxEmitterHalfHeightCommand::new(handle, index, value.cast_value_cloned()?),
                ),
                CuboidEmitter::HALF_WIDTH => helper.do_scene_command(
                    SetBoxEmitterHalfWidthCommand::new(handle, index, value.cast_value_cloned()?),
                ),
                CuboidEmitter::HALF_DEPTH => helper.do_scene_command(
                    SetBoxEmitterHalfDepthCommand::new(handle, index, value.cast_value_cloned()?),
                ),
                _ => None,
            },
            FieldKind::Inspectable(ref inner) => match property_changed.name.as_ref() {
                CylinderEmitter::EMITTER => {
                    handle_base_emitter_property_changed(handle, inner, helper, index)
                }
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}
