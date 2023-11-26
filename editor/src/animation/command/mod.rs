use crate::{
    animation::selection::AnimationSelection,
    command::Command,
    scene::{commands::SceneContext, Selection},
};
use fyrox::{
    animation::{
        track::Track, value::ValueBinding, Animation, AnimationSignal, RootMotionSettings,
    },
    core::{
        curve::Curve,
        log::Log,
        pool::{Handle, Ticket},
        uuid::Uuid,
    },
    scene::{animation::AnimationPlayer, node::Node},
};
use std::{
    fmt::Debug,
    ops::{IndexMut, Range},
};

pub mod signal;

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
    track: Option<Track>,
}

impl AddTrackCommand {
    pub fn new(animation_player: Handle<Node>, animation: Handle<Animation>, track: Track) -> Self {
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
pub struct RemoveTrackCommand {
    animation_player: Handle<Node>,
    animation: Handle<Animation>,
    index: usize,
    track: Option<Track>,
}

impl RemoveTrackCommand {
    pub fn new(animation_player: Handle<Node>, animation: Handle<Animation>, index: usize) -> Self {
        Self {
            animation_player,
            animation,
            index,
            track: None,
        }
    }
}

impl Command for RemoveTrackCommand {
    fn name(&mut self, _: &SceneContext) -> String {
        "Remove Track".to_string()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        self.track = Some(
            fetch_animation_player(self.animation_player, context).animations_mut()[self.animation]
                .remove_track(self.index),
        );
    }

    fn revert(&mut self, context: &mut SceneContext) {
        fetch_animation_player(self.animation_player, context).animations_mut()[self.animation]
            .insert_track(self.index, self.track.take().unwrap());
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
            for curve in track.data_container_mut().curves_mut() {
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
        selection: Selection,
    },
    Reverted {
        animation_player: Handle<Node>,
        animation: Animation,
        ticket: Ticket<Animation>,
        selection: Selection,
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

                let old_selection = std::mem::replace(
                    &mut context.editor_scene.selection,
                    Selection::Animation(AnimationSelection {
                        animation_player,
                        animation: handle,
                        entities: vec![],
                    }),
                );

                *self = Self::Executed {
                    animation_player,
                    animation: handle,
                    selection: old_selection,
                };
            }
            AddAnimationCommand::Reverted {
                animation_player,
                animation,
                ticket,
                selection,
            } => {
                let handle = fetch_animation_player(animation_player, context)
                    .animations_mut()
                    .put_back(ticket, animation);

                let old_selection =
                    std::mem::replace(&mut context.editor_scene.selection, selection);

                *self = Self::Executed {
                    animation_player,
                    animation: handle,
                    selection: old_selection,
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
                selection,
            } => {
                let (ticket, animation) = fetch_animation_player(animation_player, context)
                    .animations_mut()
                    .take_reserve(animation);

                let old_selection =
                    std::mem::replace(&mut context.editor_scene.selection, selection);

                *self = Self::Reverted {
                    animation_player,
                    animation,
                    ticket,
                    selection: old_selection,
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

#[derive(Debug)]
pub struct ReplaceAnimationCommand {
    pub animation_player: Handle<Node>,
    pub animation_handle: Handle<Animation>,
    pub animation: Animation,
}

impl ReplaceAnimationCommand {
    fn swap(&mut self, context: &mut SceneContext) {
        std::mem::swap(
            fetch_animation_player(self.animation_player, context)
                .animations_mut()
                .get_mut(self.animation_handle),
            &mut self.animation,
        );
    }
}

impl Command for ReplaceAnimationCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Replace Animation".to_string()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        self.swap(context);
    }

    fn revert(&mut self, context: &mut SceneContext) {
        self.swap(context);
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
    fetch_animation_player(animation_player, ctx)
        .animations_mut()
        .index_mut(animation)
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

define_animation_swap_command!(SetAnimationLoopingCommand<bool>(self, context) {
    let animation = fetch_animation(self.node_handle, self.animation_handle, context);
    let old = animation.is_loop();
    animation.set_loop(self.value);
    self.value = old;
});

define_animation_swap_command!(SetAnimationEnabledCommand<bool>(self, context) {
    let animation = fetch_animation(self.node_handle, self.animation_handle, context);
    let old = animation.is_enabled();
    animation.set_enabled(self.value);
    self.value = old;
});

define_animation_swap_command!(SetAnimationRootMotionSettingsCommand<Option<RootMotionSettings>>(self, context) {
    let animation = fetch_animation(self.node_handle, self.animation_handle, context);
    let old = animation.root_motion_settings_ref().cloned();
    animation.set_root_motion_settings(self.value.clone());
    self.value = old;
});

#[derive(Debug)]
pub struct AddAnimationSignal {
    pub animation_player_handle: Handle<Node>,
    pub animation_handle: Handle<Animation>,
    pub signal: Option<AnimationSignal>,
}

impl Command for AddAnimationSignal {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Add Animation Signal".to_string()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        fetch_animation(self.animation_player_handle, self.animation_handle, context)
            .add_signal(self.signal.take().unwrap());
    }

    fn revert(&mut self, context: &mut SceneContext) {
        self.signal = fetch_animation(self.animation_player_handle, self.animation_handle, context)
            .pop_signal();
    }
}

#[derive(Debug)]
pub struct MoveAnimationSignal {
    pub animation_player_handle: Handle<Node>,
    pub animation_handle: Handle<Animation>,
    pub signal: Uuid,
    pub time: f32,
}

impl MoveAnimationSignal {
    fn swap(&mut self, context: &mut SceneContext) {
        std::mem::swap(
            &mut fetch_animation(self.animation_player_handle, self.animation_handle, context)
                .signals_mut()
                .iter_mut()
                .find(|s| s.id == self.signal)
                .unwrap()
                .time,
            &mut self.time,
        );
    }
}

impl Command for MoveAnimationSignal {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Move Animation Signal".to_string()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        self.swap(context)
    }

    fn revert(&mut self, context: &mut SceneContext) {
        self.swap(context)
    }
}

#[derive(Debug)]
pub struct RemoveAnimationSignal {
    pub animation_player_handle: Handle<Node>,
    pub animation_handle: Handle<Animation>,
    pub signal_index: usize,
    pub signal: Option<AnimationSignal>,
}

impl Command for RemoveAnimationSignal {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Remove Animation".to_string()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        let animation =
            fetch_animation(self.animation_player_handle, self.animation_handle, context);
        self.signal = Some(animation.remove_signal(self.signal_index));
    }

    fn revert(&mut self, context: &mut SceneContext) {
        let animation =
            fetch_animation(self.animation_player_handle, self.animation_handle, context);
        animation.insert_signal(self.signal_index, self.signal.take().unwrap());
    }
}

#[derive(Debug)]
pub struct SetTrackEnabledCommand {
    pub animation_player_handle: Handle<Node>,
    pub animation_handle: Handle<Animation>,
    pub track: Uuid,
    pub enabled: bool,
}

impl SetTrackEnabledCommand {
    fn swap(&mut self, context: &mut SceneContext) {
        let track = fetch_animation(self.animation_player_handle, self.animation_handle, context)
            .tracks_mut()
            .iter_mut()
            .find(|t| t.id() == self.track)
            .unwrap();

        let old = track.is_enabled();
        track.set_enabled(self.enabled);
        self.enabled = old;
    }
}

impl Command for SetTrackEnabledCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Set Track Enabled".to_string()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        self.swap(context)
    }

    fn revert(&mut self, context: &mut SceneContext) {
        self.swap(context)
    }
}

#[derive(Debug)]
pub struct SetTrackTargetCommand {
    pub animation_player_handle: Handle<Node>,
    pub animation_handle: Handle<Animation>,
    pub track: Uuid,
    pub target: Handle<Node>,
}

impl SetTrackTargetCommand {
    fn swap(&mut self, context: &mut SceneContext) {
        let track = fetch_animation(self.animation_player_handle, self.animation_handle, context)
            .tracks_mut()
            .iter_mut()
            .find(|t| t.id() == self.track)
            .unwrap();

        let old = track.target();
        track.set_target(self.target);
        self.target = old;
    }
}

impl Command for SetTrackTargetCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Set Track Target".to_string()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        self.swap(context)
    }

    fn revert(&mut self, context: &mut SceneContext) {
        self.swap(context)
    }
}

#[derive(Debug)]
pub struct SetTrackBindingCommand {
    pub animation_player_handle: Handle<Node>,
    pub animation_handle: Handle<Animation>,
    pub track: Uuid,
    pub binding: ValueBinding,
}

impl SetTrackBindingCommand {
    fn swap(&mut self, context: &mut SceneContext) {
        let track = fetch_animation(self.animation_player_handle, self.animation_handle, context)
            .tracks_mut()
            .iter_mut()
            .find(|t| t.id() == self.track)
            .unwrap();

        let old = track.binding().clone();
        track.set_binding(self.binding.clone());
        self.binding = old;
    }
}

impl Command for SetTrackBindingCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Set Track Binding".to_string()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        self.swap(context)
    }

    fn revert(&mut self, context: &mut SceneContext) {
        self.swap(context)
    }
}
