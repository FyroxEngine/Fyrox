use crate::{command::Command, scene::commands::SceneContext};
use fyrox::{
    animation::{Animation, NodeTrack},
    core::{curve::Curve, pool::Handle, pool::Ticket},
    scene::{animation::AnimationPlayer, node::Node},
    utils::log::Log,
};
use std::{
    fmt::Debug,
    ops::{IndexMut, Range},
};

fn fetch_animation_player<'a>(
    handle: Handle<Node>,
    context: &'a mut SceneContext,
) -> &'a mut AnimationPlayer {
    context.scene.graph[handle]
        .query_component_mut::<AnimationPlayer>()
        .unwrap()
}

#[derive(Debug)]
pub struct AddTrackCommand {
    animation_player: Handle<Node>,
    animation: Handle<Animation>,
    track: Option<NodeTrack>,
}

impl AddTrackCommand {
    pub fn new(
        animation_player: Handle<Node>,
        animation: Handle<Animation>,
        track: NodeTrack,
    ) -> Self {
        Self {
            animation_player,
            animation,
            track: Some(track),
        }
    }
}

impl Command for AddTrackCommand {
    fn name(&mut self, _: &SceneContext) -> String {
        "Add Track".to_string()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        fetch_animation_player(self.animation_player, context).animations_mut()[self.animation]
            .add_track(self.track.take().unwrap());
    }

    fn revert(&mut self, context: &mut SceneContext) {
        self.track = fetch_animation_player(self.animation_player, context).animations_mut()
            [self.animation]
            .pop_track();
    }
}

#[derive(Debug)]
pub struct ReplaceTrackCurveCommand {
    pub animation_player: Handle<Node>,
    pub animation: Handle<Animation>,
    pub curve: Curve,
}

impl ReplaceTrackCurveCommand {
    fn swap(&mut self, context: &mut SceneContext) {
        for track in fetch_animation_player(self.animation_player, context).animations_mut()
            [self.animation]
            .tracks_mut()
        {
            for curve in track.frames_container_mut().curves_mut() {
                if curve.id() == self.curve.id() {
                    std::mem::swap(&mut self.curve, curve);
                    return;
                }
            }
        }

        Log::err(format!("There's no such curve with id {}", self.curve.id()))
    }
}

impl Command for ReplaceTrackCurveCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Replace Track Curve".to_string()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        self.swap(context)
    }

    fn revert(&mut self, context: &mut SceneContext) {
        self.swap(context)
    }
}

#[derive(Debug)]
pub enum AddAnimationCommand {
    Unknown,
    NonExecuted {
        animation_player: Handle<Node>,
        animation: Animation,
    },
    Executed {
        animation_player: Handle<Node>,
        animation: Handle<Animation>,
    },
    Reverted {
        animation_player: Handle<Node>,
        animation: Animation,
        ticket: Ticket<Animation>,
    },
}

impl AddAnimationCommand {
    pub fn new(animation_player: Handle<Node>, animation: Animation) -> Self {
        Self::NonExecuted {
            animation_player,
            animation,
        }
    }
}

impl Command for AddAnimationCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Add Animation".to_string()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        match std::mem::replace(self, Self::Unknown) {
            AddAnimationCommand::NonExecuted {
                animation_player,
                animation,
            } => {
                let handle = fetch_animation_player(animation_player, context)
                    .animations_mut()
                    .add(animation);

                *self = Self::Executed {
                    animation_player,
                    animation: handle,
                };
            }
            AddAnimationCommand::Reverted {
                animation_player,
                animation,
                ticket,
            } => {
                let handle = fetch_animation_player(animation_player, context)
                    .animations_mut()
                    .put_back(ticket, animation);

                *self = Self::Executed {
                    animation_player,
                    animation: handle,
                };
            }
            _ => unreachable!(),
        }
    }

    fn revert(&mut self, context: &mut SceneContext) {
        match std::mem::replace(self, Self::Unknown) {
            AddAnimationCommand::Executed {
                animation_player,
                animation,
            } => {
                let (ticket, animation) = fetch_animation_player(animation_player, context)
                    .animations_mut()
                    .take_reserve(animation);

                *self = Self::Reverted {
                    animation_player,
                    animation,
                    ticket,
                }
            }
            _ => unreachable!(),
        }
    }

    fn finalize(&mut self, context: &mut SceneContext) {
        if let AddAnimationCommand::Reverted {
            animation_player,
            ticket,
            ..
        } = std::mem::replace(self, Self::Unknown)
        {
            fetch_animation_player(animation_player, context)
                .animations_mut()
                .forget_ticket(ticket);
        }
    }
}

#[derive(Debug)]
pub enum RemoveAnimationCommand {
    Unknown,
    NonExecuted {
        animation_player: Handle<Node>,
        animation: Handle<Animation>,
    },
    Executed {
        animation_player: Handle<Node>,
        animation: Animation,
        ticket: Ticket<Animation>,
    },
    Reverted {
        animation_player: Handle<Node>,
        animation: Handle<Animation>,
    },
}

impl RemoveAnimationCommand {
    pub fn new(animation_player: Handle<Node>, animation: Handle<Animation>) -> Self {
        Self::NonExecuted {
            animation_player,
            animation,
        }
    }
}

impl Command for RemoveAnimationCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Remove Animation".to_string()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        match std::mem::replace(self, Self::Unknown) {
            RemoveAnimationCommand::NonExecuted {
                animation_player,
                animation,
            }
            | RemoveAnimationCommand::Reverted {
                animation_player,
                animation,
            } => {
                let (ticket, animation) = fetch_animation_player(animation_player, context)
                    .animations_mut()
                    .take_reserve(animation);

                *self = Self::Executed {
                    animation_player,
                    animation,
                    ticket,
                }
            }
            _ => unreachable!(),
        }
    }

    fn revert(&mut self, context: &mut SceneContext) {
        match std::mem::replace(self, Self::Unknown) {
            RemoveAnimationCommand::Executed {
                animation_player,
                animation,
                ticket,
            } => {
                let handle = fetch_animation_player(animation_player, context)
                    .animations_mut()
                    .put_back(ticket, animation);

                *self = Self::Reverted {
                    animation_player,
                    animation: handle,
                };
            }
            _ => unreachable!(),
        }
    }

    fn finalize(&mut self, context: &mut SceneContext) {
        if let RemoveAnimationCommand::Executed {
            animation_player,
            ticket,
            ..
        } = std::mem::replace(self, Self::Unknown)
        {
            fetch_animation_player(animation_player, context)
                .animations_mut()
                .forget_ticket(ticket);
        }
    }
}

#[macro_export]
macro_rules! define_animation_swap_command {
    ($name:ident<$value_type:ty>($self:ident, $context:ident) $swap:block) => {
        #[derive(Debug)]
        pub struct $name {
            pub node_handle: Handle<Node>,
            pub animation_handle: Handle<Animation>,
            pub value: $value_type,
        }

        impl $name {
            fn swap(&mut $self, $context: &mut SceneContext) {
                $swap
            }
        }

        impl Command for $name {
            fn name(&mut self, _context: &SceneContext) -> String {
                stringify!($name).to_string()
            }

            fn execute(&mut self, context: &mut SceneContext) {
                self.swap(context)
            }

            fn revert(&mut self, context: &mut SceneContext) {
                self.swap(context)
            }
        }
    };
}

fn fetch_animation<'a>(
    animation_player: Handle<Node>,
    animation: Handle<Animation>,
    ctx: &'a mut SceneContext,
) -> &'a mut Animation {
    let animation = fetch_animation_player(animation_player, ctx)
        .animations_mut()
        .index_mut(animation);

    // TODO: Hack. Make sure that the animation will serialize all keyframes on all tracks.
    // This is needed, because by default animation does not serialize its keyframes,
    // instead it takes keyframes from parent prefab. This should be removed when "diff-like"
    // prefabs is implemented.
    for track in animation.tracks_mut() {
        track.set_serialize_frames(true);
    }

    animation
}

define_animation_swap_command!(SetAnimationSpeedCommand<f32>(self, context) {
    let animation = fetch_animation(self.node_handle, self.animation_handle, context);
    let old_speed = animation.speed();
    animation.set_speed(self.value);
    self.value = old_speed;
});

define_animation_swap_command!(SetAnimationTimeSliceCommand<Range<f32>>(self, context) {
    let animation = fetch_animation(self.node_handle, self.animation_handle, context);
    let old_time_slice = animation.time_slice();
    animation.set_time_slice(self.value.clone());
    self.value = old_time_slice;
});

define_animation_swap_command!(SetAnimationNameCommand<String>(self, context) {
    let animation = fetch_animation(self.node_handle, self.animation_handle, context);
    let old_name = animation.name().to_string();
    animation.set_name(self.value.clone());
    self.value = old_name;
});
