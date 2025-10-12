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

use crate::{
    command::{make_command, Command, SetPropertyCommand},
    fyrox::{
        core::{
            pool::{ErasedHandle, Handle},
            reflect::Reflect,
            uuid::Uuid,
            variable::InheritableVariable,
        },
        engine::Engine,
        generic_animation::{Animation, AnimationContainer},
        graph::{BaseSceneGraph, SceneGraphNode},
        gui::inspector::PropertyChanged,
        scene::SceneContainer,
    },
    message::MessageSender,
    plugins::{animation, animation::command::fetch_animations_container},
    scene::{controller::SceneController, GameScene, SelectionContainer},
    ui_scene::UiScene,
};
use std::fmt::{Debug, Formatter};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SelectedEntity {
    Track(Uuid),
    Curve(Uuid),
    Signal(Uuid),
}

#[derive(Eq)]
pub struct AnimationSelection<N>
where
    N: Reflect,
{
    pub animation_player: Handle<N>,
    pub animation: Handle<Animation<Handle<N>>>,
    pub entities: Vec<SelectedEntity>,
}

impl<N> Debug for AnimationSelection<N>
where
    N: Reflect,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {:?}",
            self.animation_player, self.animation, self.entities
        )
    }
}

impl<N> Clone for AnimationSelection<N>
where
    N: Reflect,
{
    fn clone(&self) -> Self {
        Self {
            animation_player: self.animation_player,
            animation: self.animation,
            entities: self.entities.clone(),
        }
    }
}

impl<N> PartialEq for AnimationSelection<N>
where
    N: Reflect,
{
    fn eq(&self, other: &Self) -> bool {
        self.entities == other.entities
            && self.animation == other.animation
            && self.animation_player == other.animation_player
    }
}

pub fn get_animations_container<'a, N: Reflect>(
    handle: Handle<N>,
    controller: &'a dyn SceneController,
    scenes: &'a SceneContainer,
) -> Option<&'a InheritableVariable<AnimationContainer<Handle<N>>>> {
    if let Some(game_scene) = controller.downcast_ref::<GameScene>() {
        scenes[game_scene.scene]
            .graph
            .node(ErasedHandle::from(handle).into())
            .component_ref::<InheritableVariable<AnimationContainer<Handle<N>>>>()
    } else if let Some(ui) = controller.downcast_ref::<UiScene>() {
        ui.ui
            .node(ErasedHandle::from(handle).into())
            .component_ref::<InheritableVariable<AnimationContainer<Handle<N>>>>()
    } else {
        None
    }
}

impl<N> SelectionContainer for AnimationSelection<N>
where
    N: Reflect,
{
    fn len(&self) -> usize {
        self.entities.len()
    }

    fn first_selected_entity(
        &self,
        controller: &dyn SceneController,
        scenes: &SceneContainer,
        callback: &mut dyn FnMut(&dyn Reflect, bool),
    ) {
        if let Some(container) = get_animations_container(self.animation_player, controller, scenes)
        {
            if let Some(animation) = container.try_get(self.animation) {
                if let Some(animation::selection::SelectedEntity::Signal(id)) =
                    self.entities.first()
                {
                    if let Some(signal) = animation.signals().iter().find(|s| s.id == *id) {
                        (callback)(signal as &dyn Reflect, false);
                    }
                }
            }
        }
    }

    fn on_property_changed(
        &mut self,
        _controller: &mut dyn SceneController,
        args: &PropertyChanged,
        _engine: &mut Engine,
        sender: &MessageSender,
    ) {
        let animation_player = self.animation_player;
        let animation = self.animation;
        let group: Vec<Command> = self
            .entities
            .iter()
            .filter_map(|e| {
                if let &animation::selection::SelectedEntity::Signal(id) = e {
                    make_command(args, move |ctx| {
                        fetch_animations_container(animation_player, ctx)[animation]
                            .signals_mut()
                            .iter_mut()
                            .find(|s| s.id == id)
                            .map(|s| s as &mut dyn Reflect)
                    })
                } else {
                    None
                }
            })
            .collect();

        sender.do_command_group_with_inheritance(group, args);
    }

    fn paste_property(&mut self, path: &str, value: &dyn Reflect, sender: &MessageSender) {
        let animation_player = self.animation_player;
        let animation = self.animation;
        let group = self
            .entities
            .iter()
            .filter_map(|e| {
                if let &animation::selection::SelectedEntity::Signal(id) = e {
                    value.try_clone_box().map(|value| {
                        Command::new(SetPropertyCommand::new(
                            path.to_string(),
                            value,
                            move |ctx| {
                                fetch_animations_container(animation_player, ctx)[animation]
                                    .signals_mut()
                                    .iter_mut()
                                    .find(|s| s.id == id)
                                    .map(|s| s as &mut dyn Reflect)
                            },
                        ))
                    })
                } else {
                    None
                }
            })
            .collect();

        sender.do_command_group(group);
    }

    fn provide_docs(&self, controller: &dyn SceneController, engine: &Engine) -> Option<String> {
        if let Some(game_scene) = controller.downcast_ref::<GameScene>() {
            Some(
                engine.scenes[game_scene.scene]
                    .graph
                    .node(ErasedHandle::from(self.animation_player).into())
                    .doc()
                    .to_string(),
            )
        } else {
            controller.downcast_ref::<UiScene>().map(|ui| {
                ui.ui
                    .node(ErasedHandle::from(self.animation_player).into())
                    .doc()
                    .to_string()
            })
        }
    }
}

impl<N> AnimationSelection<N>
where
    N: Reflect,
{
    pub fn first_selected_track(&self) -> Option<Uuid> {
        self.entities.iter().find_map(|e| {
            if let SelectedEntity::Track(id) = e {
                Some(*id)
            } else {
                None
            }
        })
    }
}
