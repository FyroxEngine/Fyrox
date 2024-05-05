//! Everything related to audio buses and audio bus graphs. See docs of [`AudioBus`] and [`AudioBusGraph`]
//! for more info and examples

use crate::effects::{Effect, EffectRenderTrait};
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

    #[allow(clippy::type_complexity)]
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

/// Audio bus is a top-level audio processing unit. It takes data from multiple audio sources and passes their
/// samples through a chain of effects. Output signal is then can be either sent to an audio playback device or
/// to some other audio bus and be processed again, but with different sound effects (this can be done via
/// [`AudioBusGraph`].
#[derive(Debug, Reflect, Visit, Clone)]
pub struct AudioBus {
    pub(crate) name: String,
    effects: Vec<Effect>,
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
    /// Creates a new audio bus with the given name with no audio effects and unit gain. Produced audio bus must
    /// be added to an [`AudioBusGraph`] to take effect.
    pub fn new(name: String) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }

    /// Sets a new name of the audio bus. Be careful when changing the name at runtime, each sound source is linked
    /// to a bus by its name (implicit binding), so when changing the name you should also change the output bus name
    /// of a sound source, that uses the bus.
    pub fn set_name<S: AsRef<str>>(&mut self, name: S) {
        name.as_ref().clone_into(&mut self.name);
    }

    /// Returns current name of the audio bus. Could be useful if you need to find all sound sources that uses the bus.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns a handle of the parent audio bus. Primary audio bus has no parent and will return ([`Handle::NONE`]).
    pub fn parent(&self) -> Handle<AudioBus> {
        self.parent_bus
    }

    /// Returns a list of handle to children audio buses.
    pub fn children(&self) -> &[Handle<AudioBus>] {
        &self.child_buses
    }

    /// Sets new gain of the audio bus.
    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain;
    }

    /// Returns current gain of the audio bus.
    pub fn gain(&self) -> f32 {
        self.gain
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

    /// Adds new effect to the effects chain.
    pub fn add_effect(&mut self, effect: Effect) {
        self.effects.push(effect)
    }

    /// Removes an effect by the given handle.
    pub fn remove_effect(&mut self, index: usize) {
        self.effects.remove(index);
    }

    /// Returns a shared reference to an effect at the given handle.
    pub fn effect(&self, index: usize) -> Option<&Effect> {
        self.effects.get(index)
    }

    /// Returns mutable reference to effect at given handle.
    pub fn effect_mut(&mut self, index: usize) -> Option<&mut Effect> {
        self.effects.get_mut(index)
    }

    /// Returns an iterator over effects used by this audio bus.
    pub fn effects(&self) -> impl Iterator<Item = &Effect> {
        self.effects.iter()
    }

    /// Returns an iterator over effects used by this audio bus.
    pub fn effects_mut(&mut self) -> impl Iterator<Item = &mut Effect> {
        self.effects.iter_mut()
    }
}

/// Audio bus graph is a complex audio data processing entity; it allows you to route samples from
/// audio sources through a chain of audio buses or directly to an audio playback device. To get a
/// better understanding of how the audio graph works take a look the data flow diagram below:
///
/// ┌────────────┐                                                        ┌────────────┐
/// │            │                                                        │            │
/// │   Source1  ├────────────┐                                ┌──────────┤   Source2  │
/// │            │            │                                │          │            │
/// └────────────┘            │                                │          └────────────┘
///                           ▼                                ▼
/// ┌────────────┐      ┌────────────┐                  ┌────────────┐    ┌────────────┐
/// │            │      │            │                  │            │    │            │
/// │   Source3  ├─────►│    Bus1    │                  │    BusN    │◄───┤   Source4  │
/// │            │      ├────────────┤                  ├────────────┤    │            │
/// └────────────┘      │            │                  │            │    └────────────┘
///                     │  Effect1 │ │   ┌───────────┐  │  Effect1 │ │
///                     │          │ │   │           │  │          │ │
///                     │  Effect2 │ │   │  SourceN  │  │  Effect2 │ │
///                     │          │ │   │           │  │          │ │
///                     │  EffectN ▼ │   └─────┬─────┘  │  EffectN ▼ │
///                     │            │         │        │            │
///                     └─────┬──────┘         │        └─────┬──────┘
///                           │                │              │
///                           │                ▼              │
///                           │          ┌────────────┐       │
///                           │          │            │       │
///                           └─────────►│   Primary  │◄──────┘
///                                      ├────────────┤
///                                      │            │
///                                      │  Effect1 │ │
///                                      │          │ │
///                                      │  Effect2 │ │
///                                      │          │ │
///                                      │  EffectN ▼ │
///                                      │            │
///                                      └─────┬──────┘
///                                            │
///                                            │
///                                            ▼
///                               ┌───────────────────────────┐
///                               │                           │
///                               │       Output Device       │
///                               │                           │
///                               └───────────────────────────┘
///
/// Each audio bus is backed with data (samples) by a set of sound sources (`Source1`, `Source2`, ..) of
/// current audio context. This data is then passed through a set of effects, which could include various
/// filters (lowpass, highpass, bandpass, shelf filters, etc.) and complex effects such as reverberation.
///
/// By default, each audio bus graph has a single audio bus called Primary. It is mandatory to at least one
/// audio bus. Primary bus is responsible for outputting the data to an audio playback device.
///
/// # Sound source binding
///
/// Each sound source binds to an audio bus using its name; this is so called implicit binding. While it may
/// look weird, it is actually very useful. Explicit binding requires you to know the exact handle of an
/// audio bus to which a sound is "attached". This makes it less convenient to re-route data from one bus to
/// another. Implicit binding is as much more effective: all you need to do is to set a new name of a bus
/// to which output the samples from a sound source and the engine will do the rest for you. A simple example
/// of a such binding is something like this:
///
/// ```rust
/// # use fyrox_sound::bus::AudioBus;
/// # use fyrox_sound::context::SoundContext;
/// # use fyrox_sound::source::SoundSourceBuilder;
/// let context = SoundContext::new();
/// let mut state = context.state();
///
/// let sfx_bus = AudioBus::new("SFX".to_string());
/// let bus_graph = state.bus_graph_mut();
/// let primary_bus = bus_graph.primary_bus_handle();
/// bus_graph.add_bus(sfx_bus, primary_bus);
///
/// // Create a source and implicitly bind to the SFX audio bus. By default each source
/// // is bound to the primary audio bus.
/// state.add_source(SoundSourceBuilder::new().with_bus("SFX").build().unwrap());
/// ```
///
/// If you delete an audio bus to which a bunch of sound sources is bound, then they will simply stop playing.
#[derive(Default, Debug, Clone, Visit, Reflect)]
pub struct AudioBusGraph {
    buses: Pool<AudioBus>,
    root: Handle<AudioBus>,
}

impl AudioBusGraph {
    /// The name of the audio bus that output samples directly to an audio playback device.
    pub const PRIMARY_BUS: &'static str = "Primary";

    /// Creates a new audio bus graph. Sound context already has an audio graph instance, so calling
    /// this method is needed only for very specific cases (mostly tests).
    pub fn new() -> Self {
        let root = AudioBus::new(Self::PRIMARY_BUS.to_string());
        let mut buses = Pool::new();
        let root = buses.spawn(root);
        Self { buses, root }
    }

    /// Adds a new audio bus to the graph and attaches it to the given parent. `parent` handle must be
    /// valid, otherwise the method will panic. In most cases you can use primary bus as a parent.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fyrox_sound::bus::{AudioBus, AudioBusGraph};
    ///
    /// // By default it has one primary audio bus.
    /// let mut graph = AudioBusGraph::new();
    ///
    /// // Add another bus to the graph and attach it to the primary bus.
    /// let primary_bus_handle = graph.primary_bus_handle();
    /// graph.add_bus(AudioBus::new("SFX".to_owned()), primary_bus_handle);
    ///
    /// ```
    pub fn add_bus(&mut self, mut bus: AudioBus, parent: Handle<AudioBus>) -> Handle<AudioBus> {
        bus.parent_bus = parent;
        let bus = self.buses.spawn(bus);
        self.buses[parent].child_buses.push(bus);
        bus
    }

    fn unlink_internal(&mut self, node_handle: Handle<AudioBus>) {
        // Replace parent handle of child
        let parent_handle =
            std::mem::replace(&mut self.buses[node_handle].parent_bus, Handle::NONE);

        // Remove child from parent's children list
        if let Some(parent) = self.buses.try_borrow_mut(parent_handle) {
            if let Some(i) = parent.children().iter().position(|h| *h == node_handle) {
                parent.child_buses.remove(i);
            }
        }
    }

    /// Attaches the `child` audio bus to the `parent` audio bus. **Important:** this method does not
    /// checks for any loops, adding any loops to the graph will cause infinite loop in the mixer thread.
    #[inline]
    pub fn link_buses(&mut self, child: Handle<AudioBus>, parent: Handle<AudioBus>) {
        self.unlink_internal(child);
        self.buses[child].parent_bus = parent;
        self.buses[parent].child_buses.push(child);
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

    /// Removes an audio bus at the given handle.
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

    /// Returns a handle of the primary audio bus. Primary bus outputs its samples directly to an audio playback
    /// device.
    pub fn primary_bus_handle(&self) -> Handle<AudioBus> {
        self.root
    }

    /// Returns a reference to the primary audio bus.
    pub fn primary_bus_ref(&self) -> &AudioBus {
        &self.buses[self.root]
    }

    /// Returns a reference to the primary audio bus.
    pub fn primary_bus_mut(&mut self) -> &mut AudioBus {
        &mut self.buses[self.root]
    }

    /// Tries to borrow an audio bus by its handle.
    pub fn try_get_bus_ref(&self, handle: Handle<AudioBus>) -> Option<&AudioBus> {
        self.buses.try_borrow(handle)
    }

    /// Tries to borrow an audio bus by its handle.
    pub fn try_get_bus_mut(&mut self, handle: Handle<AudioBus>) -> Option<&mut AudioBus> {
        self.buses.try_borrow_mut(handle)
    }

    /// Returns total amount of audio buses in the graph.
    pub fn len(&self) -> usize {
        self.buses.alive_count() as usize
    }

    /// Checks if the graph is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Tries to move out an audio bus with a promise that it will be returned back. See [`Pool::try_take_reserve`] method
    /// docs for more info.
    pub fn try_take_reserve_bus(
        &mut self,
        handle: Handle<AudioBus>,
    ) -> Option<(Ticket<AudioBus>, AudioBus)> {
        self.buses.try_take_reserve(handle)
    }

    /// Puts the audio bus back to graph on its previous place by the given ticket. See [`Pool::put_back`] method docs
    /// for more info.
    pub fn put_bus_back(&mut self, ticket: Ticket<AudioBus>, bus: AudioBus) -> Handle<AudioBus> {
        self.buses.put_back(ticket, bus)
    }

    /// Forget an audio bus ticket making the respective handle free again. See [`Pool::forget_ticket`] method docs for
    /// more info.
    pub fn forget_bus_ticket(&mut self, ticket: Ticket<AudioBus>) {
        self.buses.forget_ticket(ticket)
    }

    /// Returns an iterator over each audio bus in the graph.
    pub fn buses_iter(&self) -> impl Iterator<Item = &AudioBus> {
        self.buses.iter()
    }

    /// Returns an iterator over each audio bus in the graph.
    pub fn buses_iter_mut(&mut self) -> impl Iterator<Item = &mut AudioBus> {
        self.buses.iter_mut()
    }

    /// Returns an iterator yielding a pair of handle and a reference to each audio bus in the graph.
    pub fn buses_pair_iter(&self) -> impl Iterator<Item = (Handle<AudioBus>, &AudioBus)> {
        self.buses.pair_iter()
    }

    /// Returns an iterator yielding a pair of handle and a reference to each audio bus in the graph.
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
                let ctx = self.buses.begin_multi_borrow();

                let leaf_ref = ctx.try_get_mut(leaf).expect("Malformed bus graph!");

                let input_buffer = leaf_ref.ping_pong_buffer.input_ref();
                let leaf_gain = leaf_ref.gain;
                let mut parent_buffer = ctx.try_get_mut(leaf_ref.parent_bus);
                let output_buffer = parent_buffer
                    .as_mut()
                    .map(|parent| parent.ping_pong_buffer.input_mut())
                    // Special case for the root bus - it writes directly to the output device buffer.
                    .unwrap_or(&mut *output_device_buffer);
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
        bus1.add_effect(Effect::Attenuate(Attenuate::new(0.5)));
        bus1.add_effect(Effect::Attenuate(Attenuate::new(0.5)));

        let bus1 = graph.add_bus(bus1, graph.root);

        let mut bus2 = AudioBus::new("Bus2".to_string());
        bus2.add_effect(Effect::Attenuate(Attenuate::new(0.5)));
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
