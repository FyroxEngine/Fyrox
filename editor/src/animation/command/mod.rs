use crate::fyrox::{
    core::{
        log::Log,
        math::curve::Curve,
        pool::{ErasedHandle, Handle, Ticket},
        uuid::Uuid,
        variable::InheritableVariable,
    },
    generic_animation::{
        signal::AnimationSignal, track::Track, value::ValueBinding, Animation, AnimationContainer,
        RootMotionSettings,
    },
    graph::{BaseSceneGraph, SceneGraphNode},
};
use crate::{
    animation::selection::AnimationSelection,
    command::{CommandContext, CommandTrait},
    scene::{commands::GameSceneContext, Selection},
    ui_scene::commands::UiSceneContext,
};
use std::{
    fmt::Debug,
    ops::{IndexMut, Range},
};

pub fn fetch_animations_container<N: Debug + 'static>(
    handle: Handle<N>,
    context: &mut dyn CommandContext,
) -> &mut InheritableVariable<AnimationContainer<Handle<N>>> {
    // SAFETY: Borrow checker cannot resolve lifetime properly in the following `if` chain.
    // This is safe to do, because there's only one mutable reference anyway. Should be fixed
    // with Polonius.
    let context2 = unsafe { &mut *(context as *mut dyn CommandContext) };

    if let Some(game_scene) = context.component_mut::<GameSceneContext>() {
        game_scene
            .scene
            .graph
            .node_mut(ErasedHandle::from(handle).into())
            .component_mut::<InheritableVariable<AnimationContainer<Handle<N>>>>()
            .unwrap()
    } else if let Some(ui) = context2.component_mut::<UiSceneContext>() {
        ui.ui
            .node_mut(ErasedHandle::from(handle).into())
            .component_mut::<InheritableVariable<AnimationContainer<Handle<N>>>>()
            .unwrap()
    } else {
        panic!("Unsupported container!")
    }
}

#[derive(Debug)]
pub struct AddTrackCommand<N: Debug + 'static> {
    animation_player: Handle<N>,
    animation: Handle<Animation<Handle<N>>>,
    track: Option<Track<Handle<N>>>,
}

impl<N: Debug + 'static> AddTrackCommand<N> {
    pub fn new(
        animation_player: Handle<N>,
        animation: Handle<Animation<Handle<N>>>,
        track: Track<Handle<N>>,
    ) -> Self {
        Self {
            animation_player,
            animation,
            track: Some(track),
        }
    }
}

impl<N: Debug + 'static> CommandTrait for AddTrackCommand<N> {
    fn name(&mut self, _: &dyn CommandContext) -> String {
        "Add Track".to_string()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        fetch_animations_container(self.animation_player, context)[self.animation]
            .add_track(self.track.take().unwrap());
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        self.track =
            fetch_animations_container(self.animation_player, context)[self.animation].pop_track();
    }
}

#[derive(Debug)]
pub struct RemoveTrackCommand<N: Debug + 'static> {
    animation_player: Handle<N>,
    animation: Handle<Animation<Handle<N>>>,
    index: usize,
    track: Option<Track<Handle<N>>>,
}

impl<N: Debug + 'static> RemoveTrackCommand<N> {
    pub fn new(
        animation_player: Handle<N>,
        animation: Handle<Animation<Handle<N>>>,
        index: usize,
    ) -> Self {
        Self {
            animation_player,
            animation,
            index,
            track: None,
        }
    }
}

impl<N: Debug + 'static> CommandTrait for RemoveTrackCommand<N> {
    fn name(&mut self, _: &dyn CommandContext) -> String {
        "Remove Track".to_string()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        self.track = Some(
            fetch_animations_container(self.animation_player, context)[self.animation]
                .remove_track(self.index),
        );
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        fetch_animations_container(self.animation_player, context)[self.animation]
            .insert_track(self.index, self.track.take().unwrap());
    }
}

#[derive(Debug)]
pub struct ReplaceTrackCurveCommand<N: Debug + 'static> {
    pub animation_player: Handle<N>,
    pub animation: Handle<Animation<Handle<N>>>,
    pub curve: Curve,
}

impl<N: Debug + 'static> ReplaceTrackCurveCommand<N> {
    fn swap(&mut self, context: &mut dyn CommandContext) {
        for track in
            fetch_animations_container(self.animation_player, context)[self.animation].tracks_mut()
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

impl<N: Debug + 'static> CommandTrait for ReplaceTrackCurveCommand<N> {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Replace Track Curve".to_string()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        self.swap(context)
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        self.swap(context)
    }
}

#[derive(Debug)]
pub enum AddAnimationCommand<N: Debug + 'static> {
    Unknown,
    NonExecuted {
        animation_player: Handle<N>,
        animation: Animation<Handle<N>>,
    },
    Executed {
        animation_player: Handle<N>,
        animation: Handle<Animation<Handle<N>>>,
        selection: Selection,
    },
    Reverted {
        animation_player: Handle<N>,
        animation: Animation<Handle<N>>,
        ticket: Ticket<Animation<Handle<N>>>,
        selection: Selection,
    },
}

impl<N: Debug + 'static> AddAnimationCommand<N> {
    pub fn new(animation_player: Handle<N>, animation: Animation<Handle<N>>) -> Self {
        Self::NonExecuted {
            animation_player,
            animation,
        }
    }
}

impl<N: Debug + 'static> CommandTrait for AddAnimationCommand<N> {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Add Animation".to_string()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        match std::mem::replace(self, Self::Unknown) {
            AddAnimationCommand::NonExecuted {
                animation_player,
                animation,
            } => {
                let handle = fetch_animations_container(animation_player, context).add(animation);
                let current_selection = context.get_mut::<&mut Selection>();

                let old_selection = std::mem::replace(
                    *current_selection,
                    Selection::new(AnimationSelection {
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
                let handle = fetch_animations_container(animation_player, context)
                    .put_back(ticket, animation);

                let current_selection = context.get_mut::<&mut Selection>();
                let old_selection = std::mem::replace(*current_selection, selection);

                *self = Self::Executed {
                    animation_player,
                    animation: handle,
                    selection: old_selection,
                };
            }
            _ => unreachable!(),
        }
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        match std::mem::replace(self, Self::Unknown) {
            AddAnimationCommand::Executed {
                animation_player,
                animation,
                selection,
            } => {
                let (ticket, animation) =
                    fetch_animations_container(animation_player, context).take_reserve(animation);

                let current_selection = context.get_mut::<&mut Selection>();
                let old_selection = std::mem::replace(*current_selection, selection);

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

    fn finalize(&mut self, context: &mut dyn CommandContext) {
        if let AddAnimationCommand::Reverted {
            animation_player,
            ticket,
            ..
        } = std::mem::replace(self, Self::Unknown)
        {
            fetch_animations_container(animation_player, context).forget_ticket(ticket);
        }
    }
}

#[derive(Debug)]
pub enum RemoveAnimationCommand<N: Debug + 'static> {
    Unknown,
    NonExecuted {
        animation_player: Handle<N>,
        animation: Handle<Animation<Handle<N>>>,
    },
    Executed {
        animation_player: Handle<N>,
        animation: Animation<Handle<N>>,
        ticket: Ticket<Animation<Handle<N>>>,
    },
    Reverted {
        animation_player: Handle<N>,
        animation: Handle<Animation<Handle<N>>>,
    },
}

impl<N: Debug + 'static> RemoveAnimationCommand<N> {
    pub fn new(animation_player: Handle<N>, animation: Handle<Animation<Handle<N>>>) -> Self {
        Self::NonExecuted {
            animation_player,
            animation,
        }
    }
}

impl<N: Debug + 'static> CommandTrait for RemoveAnimationCommand<N> {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Remove Animation".to_string()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        match std::mem::replace(self, Self::Unknown) {
            RemoveAnimationCommand::NonExecuted {
                animation_player,
                animation,
            }
            | RemoveAnimationCommand::Reverted {
                animation_player,
                animation,
            } => {
                let (ticket, animation) =
                    fetch_animations_container(animation_player, context).take_reserve(animation);

                *self = Self::Executed {
                    animation_player,
                    animation,
                    ticket,
                }
            }
            _ => unreachable!(),
        }
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        match std::mem::replace(self, Self::Unknown) {
            RemoveAnimationCommand::Executed {
                animation_player,
                animation,
                ticket,
            } => {
                let handle = fetch_animations_container(animation_player, context)
                    .put_back(ticket, animation);

                *self = Self::Reverted {
                    animation_player,
                    animation: handle,
                };
            }
            _ => unreachable!(),
        }
    }

    fn finalize(&mut self, context: &mut dyn CommandContext) {
        if let RemoveAnimationCommand::Executed {
            animation_player,
            ticket,
            ..
        } = std::mem::replace(self, Self::Unknown)
        {
            fetch_animations_container(animation_player, context).forget_ticket(ticket);
        }
    }
}

#[derive(Debug)]
pub struct ReplaceAnimationCommand<N: Debug + 'static> {
    pub animation_player: Handle<N>,
    pub animation_handle: Handle<Animation<Handle<N>>>,
    pub animation: Animation<Handle<N>>,
}

impl<N: Debug + 'static> ReplaceAnimationCommand<N> {
    fn swap(&mut self, context: &mut dyn CommandContext) {
        std::mem::swap(
            fetch_animations_container(self.animation_player, context)
                .get_mut(self.animation_handle),
            &mut self.animation,
        );
    }
}

impl<N: Debug + 'static> CommandTrait for ReplaceAnimationCommand<N> {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Replace Animation".to_string()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        self.swap(context);
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        self.swap(context);
    }
}

#[macro_export]
macro_rules! define_animation_swap_command {
    ($name:ident<$value_type:ty>($self:ident, $context:ident) $swap:block) => {
        #[derive(Debug)]
        pub struct $name<N: Debug + 'static> {
            pub node_handle: Handle<N>,
            pub animation_handle: Handle<Animation<Handle<N>>>,
            pub value: $value_type,
        }

        impl<N: Debug + 'static> $name<N> {
            fn swap(&mut $self, $context: &mut dyn CommandContext) {
                $swap
            }
        }

        impl<N: Debug + 'static> CommandTrait for $name<N> {
            fn name(&mut self, _context: &dyn CommandContext) -> String {
                stringify!($name).to_string()
            }

            fn execute(&mut self, context: &mut dyn CommandContext) {
                self.swap(context)
            }

            fn revert(&mut self, context: &mut dyn CommandContext) {
                self.swap(context)
            }
        }
    };
}

fn fetch_animation<N: Debug + 'static>(
    animation_player: Handle<N>,
    animation: Handle<Animation<Handle<N>>>,
    ctx: &mut dyn CommandContext,
) -> &mut Animation<Handle<N>> {
    fetch_animations_container(animation_player, ctx).index_mut(animation)
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

define_animation_swap_command!(SetAnimationRootMotionSettingsCommand<Option<RootMotionSettings<Handle<N>>>>(self, context) {
    let animation = fetch_animation(self.node_handle, self.animation_handle, context);
    let old = animation.root_motion_settings_ref().cloned();
    animation.set_root_motion_settings(self.value.clone());
    self.value = old;
});

#[derive(Debug)]
pub struct AddAnimationSignal<N: Debug + 'static> {
    pub animation_player_handle: Handle<N>,
    pub animation_handle: Handle<Animation<Handle<N>>>,
    pub signal: Option<AnimationSignal>,
}

impl<N: Debug + 'static> CommandTrait for AddAnimationSignal<N> {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Add Animation Signal".to_string()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        fetch_animation(self.animation_player_handle, self.animation_handle, context)
            .add_signal(self.signal.take().unwrap());
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        self.signal = fetch_animation(self.animation_player_handle, self.animation_handle, context)
            .pop_signal();
    }
}

#[derive(Debug)]
pub struct MoveAnimationSignal<N: Debug + 'static> {
    pub animation_player_handle: Handle<N>,
    pub animation_handle: Handle<Animation<Handle<N>>>,
    pub signal: Uuid,
    pub time: f32,
}

impl<N: Debug + 'static> MoveAnimationSignal<N> {
    fn swap(&mut self, context: &mut dyn CommandContext) {
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

impl<N: Debug + 'static> CommandTrait for MoveAnimationSignal<N> {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Move Animation Signal".to_string()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        self.swap(context)
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        self.swap(context)
    }
}

#[derive(Debug)]
pub struct RemoveAnimationSignal<N: Debug + 'static> {
    pub animation_player_handle: Handle<N>,
    pub animation_handle: Handle<Animation<Handle<N>>>,
    pub signal_index: usize,
    pub signal: Option<AnimationSignal>,
}

impl<N: Debug + 'static> CommandTrait for RemoveAnimationSignal<N> {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Remove Animation".to_string()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        let animation =
            fetch_animation(self.animation_player_handle, self.animation_handle, context);
        self.signal = Some(animation.remove_signal(self.signal_index));
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        let animation =
            fetch_animation(self.animation_player_handle, self.animation_handle, context);
        animation.insert_signal(self.signal_index, self.signal.take().unwrap());
    }
}

#[derive(Debug)]
pub struct SetTrackEnabledCommand<N: Debug + 'static> {
    pub animation_player_handle: Handle<N>,
    pub animation_handle: Handle<Animation<Handle<N>>>,
    pub track: Uuid,
    pub enabled: bool,
}

impl<N: Debug + 'static> SetTrackEnabledCommand<N> {
    fn swap(&mut self, context: &mut dyn CommandContext) {
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

impl<N: Debug + 'static> CommandTrait for SetTrackEnabledCommand<N> {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Set Track Enabled".to_string()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        self.swap(context)
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        self.swap(context)
    }
}

#[derive(Debug)]
pub struct SetTrackTargetCommand<N: Debug + 'static> {
    pub animation_player_handle: Handle<N>,
    pub animation_handle: Handle<Animation<Handle<N>>>,
    pub track: Uuid,
    pub target: Handle<N>,
}

impl<N: Debug + 'static> SetTrackTargetCommand<N> {
    fn swap(&mut self, context: &mut dyn CommandContext) {
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

impl<N: Debug + 'static> CommandTrait for SetTrackTargetCommand<N> {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Set Track Target".to_string()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        self.swap(context)
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        self.swap(context)
    }
}

#[derive(Debug)]
pub struct SetTrackBindingCommand<N: Debug + 'static> {
    pub animation_player_handle: Handle<N>,
    pub animation_handle: Handle<Animation<Handle<N>>>,
    pub track: Uuid,
    pub binding: ValueBinding,
}

impl<N: Debug + 'static> SetTrackBindingCommand<N> {
    fn swap(&mut self, context: &mut dyn CommandContext) {
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

impl<N: Debug + 'static> CommandTrait for SetTrackBindingCommand<N> {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Set Track Binding".to_string()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        self.swap(context)
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        self.swap(context)
    }
}
