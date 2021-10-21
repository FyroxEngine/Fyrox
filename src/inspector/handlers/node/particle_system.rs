use crate::inspector::handlers::node::base::handle_base_property_changed;
use crate::{do_command, inspector::SenderHelper, scene::commands::particle_system::*};
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
        particle_system::{emitter::Emitter, ParticleSystem},
    },
};

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
                _ => println!("Unhandled property of ParticleSystem: {:?}", args),
            },
            FieldKind::Collection(ref collection_changed) => match args.name.as_ref() {
                ParticleSystem::EMITTERS => match &**collection_changed {
                    CollectionChanged::Add => ui.send_message(WindowMessage::open_modal(
                        self.selector_window,
                        MessageDirection::ToWidget,
                        true,
                    )),
                    CollectionChanged::Remove(index) => {
                        helper.do_scene_command(DeleteEmitterCommand::new(handle, *index));
                    }
                    CollectionChanged::ItemChanged { .. } => {}
                },
                _ => (),
            },
            FieldKind::Inspectable(ref inner) => {
                if let ParticleSystem::BASE = args.name.as_ref() {
                    handle_base_property_changed(&inner, handle, node, helper)?
                }
            }
        }

        Some(())
    }
}
