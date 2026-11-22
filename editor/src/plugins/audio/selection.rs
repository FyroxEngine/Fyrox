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

use crate::command::{Command, SetPropertyCommand, make_command};
use crate::game_scene::commands::GameSceneContext;
use crate::game_scene::controller::SceneController;
use crate::game_scene::{EntityInfo, GameScene, SelectionContainer};
use crate::message::MessageSender;
use fyrox::core::pool::Handle;
use fyrox::core::reflect::prelude::*;
use fyrox::core::some_or_return;
use fyrox::engine::Engine;
use fyrox::gui::inspector::PropertyChanged;
use fyrox::scene::SceneContainer;
use fyrox::scene::sound::AudioBus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioBusSelection {
    pub buses: Vec<Handle<AudioBus>>,
}

impl SelectionContainer for AudioBusSelection {
    fn len(&self) -> usize {
        self.buses.len()
    }

    fn first_selected_entity(
        &self,
        controller: &dyn SceneController,
        scenes: &SceneContainer,
        callback: &mut dyn FnMut(EntityInfo),
    ) {
        let game_scene = some_or_return!(controller.downcast_ref::<GameScene>());
        let scene = &scenes[game_scene.scene];
        let state = scene.graph.sound_context.state();
        if let Some(selected_bus) = self.buses.first()
            && let Ok(effect) = state.bus_graph_ref().try_get_bus_ref(*selected_bus)
        {
            (callback)(EntityInfo::with_no_parent(effect, selected_bus));
        }
    }

    fn on_property_changed(
        &mut self,
        _controller: &mut dyn SceneController,
        args: &PropertyChanged,
        _engine: &mut Engine,
        sender: &MessageSender,
    ) {
        let group = self
            .buses
            .iter()
            .filter_map(|&handle| {
                make_command(args, move |ctx| {
                    let mut state = ctx
                        .get_mut::<GameSceneContext>()
                        .scene
                        .graph
                        .sound_context
                        .state();
                    let bus = state.bus_graph_mut().try_get_bus_mut(handle).ok()?;
                    // FIXME: HACK!
                    unsafe {
                        Some(std::mem::transmute::<&'_ mut AudioBus, &'static mut AudioBus>(bus))
                    }
                })
            })
            .collect::<Vec<_>>();

        sender.do_command_group_with_inheritance(group, args);
    }

    fn paste_property(&mut self, path: &str, value: &dyn Reflect, sender: &MessageSender) {
        let group =
            self.buses
                .iter()
                .filter_map(|&handle| {
                    value.try_clone_box().map(|value| {
                        Command::new(SetPropertyCommand::new(
                            path.to_string(),
                            value,
                            move |ctx| {
                                let mut state = ctx
                                    .get_mut::<GameSceneContext>()
                                    .scene
                                    .graph
                                    .sound_context
                                    .state();
                                let bus = state.bus_graph_mut().try_get_bus_mut(handle).ok()?;
                                // FIXME: HACK!
                                unsafe {
                                    Some(std::mem::transmute::<
                                        &'_ mut AudioBus,
                                        &'static mut AudioBus,
                                    >(bus))
                                }
                            },
                        ))
                    })
                })
                .collect::<Vec<_>>();

        sender.do_command_group(group);
    }
}
