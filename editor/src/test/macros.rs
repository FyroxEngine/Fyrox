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

use crate::game_scene::GameScene;
use crate::plugins::inspector::InspectorPlugin;
use crate::{
    Editor, fyrox::engine::ApplicationLoopController, test::utils::EditorTestingExtension,
};
use fyrox::core::info;
use fyrox::core::pool::Handle;
use fyrox::core::uuid::Uuid;
use fyrox::graph::SceneGraph;
use fyrox::gui::UiNode;
use fyrox::gui::message::MouseButton;
use fyrox::scene::node::NodeTrait;
use std::collections::VecDeque;

type Action = Box<dyn FnOnce(&mut Editor)>;

pub struct Macro {
    pub actions: VecDeque<Action>,
}

impl Macro {
    pub fn begin() -> Self {
        Self {
            actions: Default::default(),
        }
    }

    pub fn then(mut self, func: impl FnOnce(&mut Editor) + 'static) -> Self {
        self.actions.push_back(Box::new(func));
        self
    }

    pub fn lmb_click_at(self, uuid: Uuid) -> Self {
        self.then(move |editor| {
            editor.click_at(uuid, MouseButton::Left);
        })
    }

    pub fn lmb_click_at_text(self, uuid: Uuid, text: impl AsRef<str> + 'static) -> Self {
        self.then(move |editor| editor.click_at_text(uuid, MouseButton::Left, text.as_ref()))
    }

    pub fn rmb_click_at(self, uuid: Uuid) -> Self {
        self.then(move |editor| {
            editor.click_at(uuid, MouseButton::Right);
        })
    }

    pub fn rmb_click_at_text(self, uuid: Uuid, text: impl AsRef<str> + 'static) -> Self {
        self.then(move |editor| editor.click_at_text(uuid, MouseButton::Right, text.as_ref()))
    }

    pub fn click_at(self, uuid: Uuid, button: MouseButton) -> Self {
        self.then(move |editor| {
            editor.click_at(uuid, button);
        })
    }

    pub fn click_at_text(
        self,
        uuid: Uuid,
        button: MouseButton,
        text: impl AsRef<str> + 'static,
    ) -> Self {
        self.then(move |editor| editor.click_at_text(uuid, button, text.as_ref()))
    }

    pub fn type_text(self, text: impl AsRef<str>) -> Self {
        let text = text.as_ref().to_owned();
        self.then(move |editor| editor.type_text(&text))
    }

    pub fn skip_frame(self) -> Self {
        self.then(|_| info!("Frame skipped."))
    }

    pub fn assert_visible(self, uuid: Uuid) -> Self {
        self.then(move |editor| {
            if !editor.is_visible(uuid) {
                panic!("The widget {uuid} must be visible!")
            }
        })
    }

    pub fn assert_window_open(self, uuid: Uuid) -> Self {
        self.then(move |editor| {
            if !editor.is_window_open(uuid) {
                panic!("The window {uuid} must be open!")
            }
        })
    }

    pub fn scroll_inspector_to_property(self, name: impl AsRef<str>) -> Self {
        let name = name.as_ref().to_owned();
        self.then(move |editor| {
            let ui = editor.engine.user_interfaces.first();
            editor
                .plugins
                .get::<InspectorPlugin>()
                .scroll_to_property(&name, ui)
        })
    }

    pub fn then_with_property<F>(self, name: impl AsRef<str>, func: F) -> Self
    where
        F: FnOnce(&mut Editor, Handle<UiNode>) + 'static,
    {
        let name = name.as_ref().to_owned();
        self.then(move |editor| {
            let ui = editor.engine.user_interfaces.first();
            let property_editor = editor
                .plugins
                .get::<InspectorPlugin>()
                .find_property_editor(&name, ui);
            if property_editor.is_none() {
                panic!("Property editor for {name} property does not exist!");
            }
            func(editor, property_editor)
        })
    }

    pub fn assert_has_scene_node<T: NodeTrait>(self, name: impl AsRef<str>) -> Self {
        let name = name.as_ref().to_owned();
        self.then(move |editor| {
            for entry in editor.scenes.iter() {
                let mut found = false;
                if let Some(game_scene) = entry.controller.downcast_ref::<GameScene>() {
                    let scene = &editor.engine.scenes[game_scene.scene];
                    for (_, n) in scene.graph.traverse_iter(scene.graph.root()) {
                        if let Some(target_node) = n.cast::<T>()
                            && target_node.name() == name
                        {
                            found = true;
                        }
                    }
                }
                if !found {
                    panic!("There's no game scene with {name} node!");
                }
            }
        })
    }

    pub fn assert_scene_node_has_child(
        self,
        parent_name: impl AsRef<str>,
        child_name: impl AsRef<str>,
    ) -> Self {
        let parent_name = parent_name.as_ref().to_owned();
        let child_name = child_name.as_ref().to_owned();
        self.then(move |editor| {
            for entry in editor.scenes.iter() {
                if let Some(game_scene) = entry.controller.downcast_ref::<GameScene>() {
                    let mut found = false;
                    let scene = &editor.engine.scenes[game_scene.scene];
                    if let Some((_, parent)) = scene.graph.find_by_name_from_root(&parent_name) {
                        for child in parent.children() {
                            if scene.graph[*child].name() == child_name {
                                found = true;
                            }
                        }
                    }
                    if !found {
                        panic!("There's no game with a node {parent_name} that has direct child {child_name} node!");
                    }
                } else {
                    panic!("There's no game scene with {parent_name} node!");
                }
            }
        })
    }

    pub fn assert_game_scene_has_n_nodes<T: NodeTrait>(
        self,
        name: impl AsRef<str>,
        required_count: usize,
    ) -> Self {
        let name = name.as_ref().to_owned();
        self.then(move |editor| {
            for entry in editor.scenes.iter() {
                let mut count = 0;
                if let Some(game_scene) = entry.controller.downcast_ref::<GameScene>() {
                    let scene = &editor.engine.scenes[game_scene.scene];
                    for (_, n) in scene.graph.traverse_iter(scene.graph.root()) {
                        if let Some(target_node) = n.cast::<T>()
                            && target_node.name() == name
                        {
                            count += 1;
                        }
                    }
                }
                if count != required_count {
                    panic!("A game scene needs to have {required_count}, but only {count} nodes are present!");
                }
            }
        })
    }

    pub fn assert_in_preview_mode(self) -> Self {
        self.then(|editor| assert!(editor.is_in_preview_mode()))
    }

    pub fn execute_next(
        &mut self,
        editor: &mut Editor,
        loop_controller: ApplicationLoopController,
    ) {
        let Some(action) = self.actions.pop_front() else {
            loop_controller.exit();
            return;
        };

        action(editor);
    }
}
