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

use crate::command::make_command;
use crate::fyrox::core::reflect::Reflect;
use crate::fyrox::{
    core::pool::Handle,
    gui::inspector::{CollectionChanged, FieldKind, PropertyChanged},
    scene::{node::Node, terrain::Terrain},
};
use crate::scene::commands::{GameSceneContext, RevertSceneNodePropertyCommand};
use crate::{
    scene::commands::terrain::{AddTerrainLayerCommand, DeleteTerrainLayerCommand},
    Command,
};

pub struct SceneNodePropertyChangedHandler;

impl SceneNodePropertyChangedHandler {
    fn try_get_command(
        &self,
        args: &PropertyChanged,
        handle: Handle<Node>,
        node: &mut Node,
    ) -> Option<Command> {
        // Terrain is special and have its own commands for specific properties.
        if args.path() == Terrain::LAYERS && node.is_terrain() {
            match args.value {
                FieldKind::Collection(ref collection_changed) => match **collection_changed {
                    CollectionChanged::Add(_) => {
                        Some(Command::new(AddTerrainLayerCommand::new(handle)))
                    }
                    CollectionChanged::Remove(index) => {
                        Some(Command::new(DeleteTerrainLayerCommand::new(handle, index)))
                    }
                    CollectionChanged::ItemChanged { .. } => None,
                },
                _ => None,
            }
        } else {
            None
        }
    }
}

impl SceneNodePropertyChangedHandler {
    pub fn handle(
        &self,
        args: &PropertyChanged,
        handle: Handle<Node>,
        node: &mut Node,
    ) -> Option<Command> {
        self.try_get_command(args, handle, node).or_else(|| {
            if args.is_inheritable() {
                // Prevent reverting property value if there's no parent resource.
                if node.resource().is_some() {
                    Some(Command::new(RevertSceneNodePropertyCommand::new(
                        args.path(),
                        handle,
                    )))
                } else {
                    None
                }
            } else {
                make_command(args, move |ctx| {
                    &mut ctx.get_mut::<GameSceneContext>().scene.graph[handle] as &mut dyn Reflect
                })
            }
        })
    }
}
