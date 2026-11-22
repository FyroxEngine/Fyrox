// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::fyrox::{
    core::pool::{Handle, Ticket},
    scene::sound::AudioBus,
};
use crate::{
    CommandTrait, GameSceneContext,
    command::CommandContext,
    fyrox::scene::sound::{DistanceModel, HrirSphereResource, Renderer, context::SoundContext},
};

macro_rules! define_sound_context_command {
    ($($name:ident($human_readable_name:expr, $value_type:ty, $get:ident, $set:ident); )*) => {
        $(
            #[derive(Debug)]
            pub struct $name {
                value: $value_type,
            }

            impl $name {
                pub fn new(value: $value_type) -> Self {
                    Self { value }
                }

                fn swap(&mut self, sound_context: &mut SoundContext) {
                    let old = sound_context.state().$get();
                    sound_context.state().$set(self.value.clone());
                    self.value = old;
                }
            }

            impl CommandTrait for $name {
                fn name(&mut self, _context: &dyn CommandContext) -> String {
                    $human_readable_name.to_owned()
                }

                fn execute(&mut self, context: &mut dyn CommandContext) {
                    let context = context.get_mut::<GameSceneContext>();
                    self.swap(&mut context.scene.graph.sound_context);
                }

                fn revert(&mut self, context: &mut dyn CommandContext) {
                    let context = context.get_mut::<GameSceneContext>();
                    self.swap(&mut context.scene.graph.sound_context);
                }
            }
        )*
    };
}

define_sound_context_command! {
    SetDistanceModelCommand("Set Distance Model", DistanceModel, distance_model, set_distance_model);
    SetRendererCommand("Set Renderer", Renderer, renderer, set_renderer);
}

#[derive(Debug)]
pub struct SetHrtfRendererHrirSphereResource {
    value: Option<HrirSphereResource>,
}

impl SetHrtfRendererHrirSphereResource {
    pub fn new(value: Option<HrirSphereResource>) -> Self {
        Self { value }
    }

    fn swap(&mut self, sound_context: &mut SoundContext) {
        if let Renderer::HrtfRenderer(hrtf) = sound_context.state().renderer_ref_mut() {
            let old = hrtf.hrir_sphere_resource();
            hrtf.set_hrir_sphere_resource(self.value.clone());
            self.value = old;
        }
    }
}

impl CommandTrait for SetHrtfRendererHrirSphereResource {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Set Hrtf Renderer Hrir Sphere Resource".to_owned()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        self.swap(&mut context.scene.graph.sound_context);
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        self.swap(&mut context.scene.graph.sound_context);
    }
}

#[derive(Debug)]
pub struct AddOrRemoveAudioBusCommand {
    bus: Option<AudioBus>,
    handle: Handle<AudioBus>,
    ticket: Option<Ticket<AudioBus>>,
    add: bool,
}

impl AddOrRemoveAudioBusCommand {
    pub fn new_add(bus: AudioBus) -> Self {
        Self {
            bus: Some(bus),
            handle: Default::default(),
            ticket: None,
            add: true,
        }
    }

    pub fn new_remove(handle: Handle<AudioBus>) -> Self {
        Self {
            bus: None,
            handle,
            ticket: None,
            add: false,
        }
    }

    fn add(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let mut state = context.scene.graph.sound_context.state();
        let parent = state.bus_graph_ref().primary_bus_handle();
        if let Some(bus) = self.bus.take() {
            let bus_graph = state.bus_graph_mut();
            if let Some(ticket) = self.ticket.take() {
                self.handle = bus_graph.put_bus_back(ticket, bus);
            } else {
                self.handle = bus_graph.add_bus(bus, parent);
            }
        }
    }

    fn remove(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        if let Ok((ticket, effect)) = context
            .scene
            .graph
            .sound_context
            .state()
            .bus_graph_mut()
            .try_take_reserve_bus(self.handle)
        {
            self.bus = Some(effect);
            self.ticket = Some(ticket);
        }
    }
}

impl CommandTrait for AddOrRemoveAudioBusCommand {
    fn name(&mut self, _: &dyn CommandContext) -> String {
        if self.add {
            "Add Audio Bus".to_owned()
        } else {
            "Remove Audio Bus".to_owned()
        }
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        if self.add {
            self.add(context)
        } else {
            self.remove(context)
        }
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        if self.add {
            self.remove(context);
        } else {
            self.add(context);
        }
    }

    fn finalize(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        if let Some(ticket) = self.ticket.take() {
            context
                .scene
                .graph
                .sound_context
                .state()
                .bus_graph_mut()
                .forget_bus_ticket(ticket);
        }
    }
}

#[derive(Debug)]
pub struct LinkAudioBuses {
    pub child: Handle<AudioBus>,
    pub parent: Handle<AudioBus>,
}

impl LinkAudioBuses {
    fn swap(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let mut state = context.scene.graph.sound_context.state();
        let graph = state.bus_graph_mut();
        let old_parent = graph.try_get_bus_ref(self.child).unwrap().parent();
        graph.link_buses(self.child, self.parent);
        self.parent = old_parent;
    }
}

impl CommandTrait for LinkAudioBuses {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Link Audio Buses".to_string()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        self.swap(context)
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        self.swap(context)
    }
}
