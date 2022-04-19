use crate::{
    animation::{
        machine::{Machine, PoseNode},
        Animation, AnimationContainer,
    },
    core::{
        pool::{Handle, Pool},
        visitor::prelude::*,
    },
};

#[derive(Default, Clone, Visit, Debug)]
pub struct AnimationMachineContainer {
    pool: Pool<Machine>,
}

impl AnimationMachineContainer {
    pub fn add(&mut self, machine: Machine) -> Handle<Machine> {
        self.pool.spawn(machine)
    }

    pub fn try_get(&self, handle: Handle<Machine>) -> Option<&Machine> {
        self.pool.try_borrow(handle)
    }

    pub fn try_get_mut(&mut self, handle: Handle<Machine>) -> Option<&mut Machine> {
        self.pool.try_borrow_mut(handle)
    }

    /// Removes animation machine from the container. The method does not remove animations used by the
    /// machine. If you need to remove every animation associated with the machine, use
    /// [`Self::remove_with_animations`] instead.
    pub fn remove(&mut self, handle: Handle<Machine>) -> Machine {
        self.pool.free(handle)
    }

    /// Removes animation machine from the container. It also removes every associated animation
    /// from the animation container.
    pub fn remove_with_animations(
        &mut self,
        handle: Handle<Machine>,
        animations: &mut AnimationContainer,
    ) -> (Machine, Vec<Animation>) {
        let machine = self.remove(handle);

        let mut removed_animations = Vec::new();
        for node in machine.nodes.iter() {
            if let PoseNode::PlayAnimation(play_animation) = node {
                if animations.try_get(play_animation.animation).is_some() {
                    removed_animations.push(animations.remove(play_animation.animation));
                }
            }
        }

        (machine, removed_animations)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Machine> {
        self.pool.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Machine> {
        self.pool.iter_mut()
    }
}
