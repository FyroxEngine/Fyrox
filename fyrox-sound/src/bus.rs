//! # Data flow diagram
//!
//! ┌────────────┐                                                        ┌────────────┐
//! │            │                                                        │            │
//! │   Source1  ├────────────┐                                ┌──────────┤   Source2  │
//! │            │            │                                │          │            │
//! └────────────┘            │                                │          └────────────┘
//!                           ▼                                ▼
//! ┌────────────┐      ┌────────────┐                  ┌────────────┐    ┌────────────┐
//! │            │      │            │                  │            │    │            │
//! │   Source3  ├─────►│    Bus1    │                  │    BusN    │◄───┤   Source4  │
//! │            │      ├────────────┤                  ├────────────┤    │            │
//! └────────────┘      │            │                  │            │    └────────────┘
//!                     │  Effect1 │ │   ┌───────────┐  │  Effect1 │ │
//!                     │          │ │   │           │  │          │ │
//!                     │  Effect2 │ │   │  SourceN  │  │  Effect2 │ │
//!                     │          │ │   │           │  │          │ │
//!                     │  EffectN ▼ │   └─────┬─────┘  │  EffectN ▼ │
//!                     │            │         │        │            │
//!                     └─────┬──────┘         │        └─────┬──────┘
//!                           │                │              │
//!                           │                ▼              │
//!                           │          ┌────────────┐       │
//!                           │          │            │       │
//!                           └─────────►│   Master   │◄──────┘
//!                                      ├────────────┤
//!                                      │            │
//!                                      │  Effect1 │ │
//!                                      │          │ │
//!                                      │  Effect2 │ │
//!                                      │          │ │
//!                                      │  EffectN ▼ │
//!                                      │            │
//!                                      └─────┬──────┘
//!                                            │
//!                                            │
//!                                            ▼
//!                               ┌───────────────────────────┐
//!                               │                           │
//!                               │       Output Device       │
//!                               │                           │
//!                               └───────────────────────────┘

#![allow(missing_docs)] // TODO

use crate::effects::{Effect, EffectRenderTrait, EffectWrapper};
use fyrox_core::{
    pool::{Handle, Pool, Ticket},
    reflect::prelude::*,
    visitor::prelude::*,
};
use std::fmt::{Debug, Formatter};

#[derive(Default, Clone)]
struct PingPongBuffer {
    buffer1: Vec<(f32, f32)>,
    buffer2: Vec<(f32, f32)>,
    first_is_input: bool,
}

impl Debug for PingPongBuffer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PingPongBuffer")
            .field("Buffer1", &format_args!("{:?} bytes", self.buffer1.len()))
            .field("Buffer2", &format_args!("{:?} bytes", self.buffer2.len()))
            .field("FirstIsInput", &self.first_is_input)
            .finish()
    }
}

impl PingPongBuffer {
    fn resize(&mut self, size: usize) {
        self.buffer1 = Vec::with_capacity(size);
        self.buffer2 = Vec::with_capacity(size);
        self.clear();
    }

    fn clear(&mut self) {
        self.buffer1.clear();
        self.buffer2.clear();
        for _ in 0..self.buffer1.capacity() {
            self.buffer1.push((0.0, 0.0));
            self.buffer2.push((0.0, 0.0));
        }
    }

    fn capacity(&self) -> usize {
        self.buffer1.capacity()
    }

    fn swap(&mut self) {
        self.first_is_input = !self.first_is_input;
    }

    fn input_output_buffers(&mut self) -> (&[(f32, f32)], &mut [(f32, f32)]) {
        if self.first_is_input {
            (&self.buffer1, &mut self.buffer2)
        } else {
            (&self.buffer2, &mut self.buffer1)
        }
    }

    fn input_ref(&self) -> &[(f32, f32)] {
        if self.first_is_input {
            &self.buffer1
        } else {
            &self.buffer2
        }
    }

    fn input_mut(&mut self) -> &mut [(f32, f32)] {
        if self.first_is_input {
            &mut self.buffer1
        } else {
            &mut self.buffer2
        }
    }
}

#[derive(Debug, Reflect, Visit, Clone)]
pub struct AudioBus {
    pub(crate) name: String,
    effects: Vec<EffectWrapper>,
    gain: f32,

    #[reflect(hidden)]
    child_buses: Vec<Handle<AudioBus>>,

    #[reflect(hidden)]
    parent_bus: Handle<AudioBus>,

    #[reflect(hidden)]
    #[visit(skip)]
    ping_pong_buffer: PingPongBuffer,
}

impl Default for AudioBus {
    fn default() -> Self {
        Self {
            name: "Bus".to_string(),
            child_buses: Default::default(),
            effects: Default::default(),
            gain: 1.0,
            ping_pong_buffer: Default::default(),
            parent_bus: Default::default(),
        }
    }
}

impl AudioBus {
    pub fn new(name: String) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }

    pub fn set_name<S: AsRef<str>>(&mut self, name: S) {
        self.name = name.as_ref().to_owned();
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn input_buffer(&mut self) -> &mut [(f32, f32)] {
        self.ping_pong_buffer.input_mut()
    }

    pub(crate) fn begin_render(&mut self, buffer_size: usize) {
        if self.ping_pong_buffer.capacity() < buffer_size {
            self.ping_pong_buffer.resize(buffer_size);
        } else {
            self.ping_pong_buffer.clear();
        }
    }

    fn apply_effects(&mut self) {
        // Pass through the chain of effects.
        for effect in self.effects.iter_mut() {
            let (input, output) = self.ping_pong_buffer.input_output_buffers();
            effect.render(input, output);
            self.ping_pong_buffer.swap();
        }
    }

    /// Adds new effect to effects chain.
    pub fn add_effect(&mut self, effect: Effect) {
        self.effects.push(EffectWrapper(effect))
    }

    /// Removes effect by given handle.
    pub fn remove_effect(&mut self, index: usize) {
        self.effects.remove(index);
    }

    /// Returns shared reference to effect at given handle.
    pub fn effect(&self, index: usize) -> Option<&Effect> {
        self.effects.get(index).map(|w| &w.0)
    }

    /// Returns mutable reference to effect at given handle.
    pub fn effect_mut(&mut self, index: usize) -> Option<&mut Effect> {
        self.effects.get_mut(index).map(|w| &mut w.0)
    }
}

#[derive(Default, Debug, Clone, Visit, Reflect)]
pub struct AudioBusGraph {
    buses: Pool<AudioBus>,
    root: Handle<AudioBus>,
}

impl AudioBusGraph {
    pub const PRIMARY_BUS: &'static str = "Primary";

    pub fn new() -> Self {
        let root = AudioBus::new(Self::PRIMARY_BUS.to_string());
        let mut buses = Pool::new();
        let root = buses.spawn(root);
        Self { buses, root }
    }

    pub fn add_bus(&mut self, mut bus: AudioBus, parent: Handle<AudioBus>) -> Handle<AudioBus> {
        bus.parent_bus = parent;
        let bus = self.buses.spawn(bus);
        self.buses[parent].child_buses.push(bus);
        bus
    }

    pub(crate) fn try_get_bus_input_buffer(&mut self, name: &str) -> Option<&mut [(f32, f32)]> {
        self.buses.iter_mut().find_map(|bus| {
            if bus.name == name {
                Some(bus.input_buffer())
            } else {
                None
            }
        })
    }

    pub fn remove_bus(&mut self, handle: Handle<AudioBus>) -> AudioBus {
        assert_ne!(handle, self.root);

        let bus = self.buses.free(handle);
        let parent_bus = &mut self.buses[bus.parent_bus];

        let position = parent_bus
            .child_buses
            .iter()
            .position(|h| *h == handle)
            .expect("Malformed bus graph!");
        parent_bus.child_buses.remove(position);

        bus
    }

    pub fn primary_bus_handle(&self) -> Handle<AudioBus> {
        self.root
    }

    pub fn primary_bus_ref(&self) -> &AudioBus {
        &self.buses[self.root]
    }

    pub fn primary_bus_mut(&mut self) -> &mut AudioBus {
        &mut self.buses[self.root]
    }

    pub fn try_get_bus_ref(&self, handle: Handle<AudioBus>) -> Option<&AudioBus> {
        self.buses.try_borrow(handle)
    }

    pub fn try_get_bus_mut(&mut self, handle: Handle<AudioBus>) -> Option<&mut AudioBus> {
        self.buses.try_borrow_mut(handle)
    }

    pub fn len(&self) -> usize {
        self.buses.alive_count() as usize
    }

    pub fn try_take_reserve_bus(
        &mut self,
        handle: Handle<AudioBus>,
    ) -> Option<(Ticket<AudioBus>, AudioBus)> {
        self.buses.try_take_reserve(handle)
    }

    pub fn put_bus_back(&mut self, ticket: Ticket<AudioBus>, bus: AudioBus) -> Handle<AudioBus> {
        self.buses.put_back(ticket, bus)
    }

    pub fn forget_bus_ticket(&mut self, ticket: Ticket<AudioBus>) {
        self.buses.forget_ticket(ticket)
    }

    pub fn buses_iter(&self) -> impl Iterator<Item = &AudioBus> {
        self.buses.iter()
    }

    pub fn buses_iter_mut(&mut self) -> impl Iterator<Item = &mut AudioBus> {
        self.buses.iter_mut()
    }

    pub fn buses_pair_iter(&self) -> impl Iterator<Item = (Handle<AudioBus>, &AudioBus)> {
        self.buses.pair_iter()
    }

    pub fn buses_pair_iter_mut(
        &mut self,
    ) -> impl Iterator<Item = (Handle<AudioBus>, &mut AudioBus)> {
        self.buses.pair_iter_mut()
    }

    pub(crate) fn begin_render(&mut self, output_device_buffer_size: usize) {
        for bus in self.buses.iter_mut() {
            bus.begin_render(output_device_buffer_size);
        }
    }

    pub(crate) fn end_render(&mut self, output_device_buffer: &mut [(f32, f32)]) {
        let mut leafs = Vec::new();
        for (handle, bus) in self.buses.pair_iter_mut() {
            bus.apply_effects();

            if bus.child_buses.is_empty() {
                leafs.push(handle);
            }
        }

        for mut leaf in leafs {
            while leaf.is_some() {
                let mut ctx = self.buses.begin_multi_borrow::<2>();

                let leaf_ref = ctx.try_get(leaf).expect("Malformed bus graph!");

                let input_buffer = leaf_ref.ping_pong_buffer.input_ref();
                let leaf_gain = leaf_ref.gain;
                let output_buffer = if leaf_ref.parent_bus.is_none() {
                    // Special case for the root bus - it writes directly to the output device buffer.
                    &mut *output_device_buffer
                } else {
                    ctx.try_get(leaf_ref.parent_bus)
                        .expect("Malformed bus graph!")
                        .ping_pong_buffer
                        .input_mut()
                };

                for ((input_left, input_right), (output_left, output_right)) in
                    input_buffer.iter().zip(output_buffer)
                {
                    *output_left += *input_left * leaf_gain;
                    *output_right += *input_right * leaf_gain;
                }

                leaf = leaf_ref.parent_bus;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        bus::{AudioBus, AudioBusGraph},
        effects::{Attenuate, Effect},
    };

    #[test]
    fn test_multi_bus_data_flow() {
        let mut output_buffer = [(0.0f32, 0.0f32)];

        let mut graph = AudioBusGraph::new();

        let bus1 = graph.add_bus(AudioBus::new("Bus1".to_string()), graph.root);
        let bus2 = graph.add_bus(AudioBus::new("Bus2".to_string()), bus1);

        graph.begin_render(output_buffer.len());

        // Simulate output of sound sources to each bus.
        for (left, right) in graph.buses[bus1].input_buffer() {
            *left = 1.0;
            *right = 1.0;
        }

        for (left, right) in graph.buses[bus2].input_buffer() {
            *left = 1.0;
            *right = 1.0;
        }

        graph.end_render(&mut output_buffer);

        assert_eq!(output_buffer[0], (2.0, 2.0));
    }

    #[test]
    fn test_primary_bus_data_flow() {
        let mut output_buffer = [(0.0f32, 0.0f32)];

        let mut graph = AudioBusGraph::new();

        graph.begin_render(output_buffer.len());

        // Simulate output of sound sources to each bus.
        for (left, right) in graph.buses[graph.root].input_buffer() {
            *left = 1.0;
            *right = 1.0;
        }

        graph.end_render(&mut output_buffer);

        assert_eq!(output_buffer[0], (1.0, 1.0));
    }

    #[test]
    fn test_multi_bus_data_flow_with_effects() {
        let mut output_buffer = [(0.0f32, 0.0f32)];

        let mut graph = AudioBusGraph::new();

        let mut bus1 = AudioBus::new("Bus1".to_string());
        bus1.effects.push(Effect::Attenuate(Attenuate::new(0.5)));
        bus1.effects.push(Effect::Attenuate(Attenuate::new(0.5)));

        let bus1 = graph.add_bus(bus1, graph.root);

        let mut bus2 = AudioBus::new("Bus2".to_string());
        bus2.effects.push(Effect::Attenuate(Attenuate::new(0.5)));
        let bus2 = graph.add_bus(bus2, bus1);

        graph.begin_render(output_buffer.len());

        // Simulate output of sound sources to each bus.
        for (left, right) in graph.buses[bus1].input_buffer() {
            *left = 1.0;
            *right = 1.0;
        }

        for (left, right) in graph.buses[bus2].input_buffer() {
            *left = 1.0;
            *right = 1.0;
        }

        graph.end_render(&mut output_buffer);

        assert_eq!(output_buffer[0], (0.75, 0.75));
    }
}
