use crate::{
    animation::machine::Machine,
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

    pub fn free(&mut self, handle: Handle<Machine>) -> Machine {
        self.pool.free(handle)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Machine> {
        self.pool.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Machine> {
        self.pool.iter_mut()
    }
}
