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

pub mod controller;
pub mod entry;
pub mod nullscene;

use crate::message::MessageSender;
use crate::settings::Settings;
use controller::SceneController;
use entry::EditorSceneEntry;
use fyrox::core::uuid::Uuid;
use fyrox::engine::Engine;

pub struct SceneContainer {
    /// The scene entry that will be current when the container is empty.
    null_scene: EditorSceneEntry,
    pub entries: Vec<EditorSceneEntry>,
    current_scene: Option<usize>,
}

impl SceneContainer {
    pub fn new(engine: &mut Engine, settings: &mut Settings, sender: MessageSender) -> Self {
        Self {
            null_scene: EditorSceneEntry::new_null_scene(engine, settings, sender),
            entries: vec![],
            current_scene: None,
        }
    }

    pub fn has_active_scene(&self) -> bool {
        self.current_scene.is_some()
    }

    pub fn current_scene_entry_ref(&self) -> &EditorSceneEntry {
        self.current_scene
            .and_then(|i| self.entries.get(i))
            .unwrap_or(&self.null_scene)
    }

    pub fn current_scene_entry_mut(&mut self) -> &mut EditorSceneEntry {
        self.current_scene
            .and_then(|i| self.entries.get_mut(i))
            .unwrap_or(&mut self.null_scene)
    }

    pub fn current_scene_controller_ref(&self) -> &dyn SceneController {
        &*self.current_scene_entry_ref().controller
    }

    pub fn current_scene_controller_mut(&mut self) -> &mut dyn SceneController {
        &mut *self.current_scene_entry_mut().controller
    }

    pub fn first_unsaved_scene(&self) -> Option<&EditorSceneEntry> {
        self.entries.iter().find(|s| s.need_save())
    }

    pub fn unsaved_scene_count(&self) -> usize {
        self.entries.iter().filter(|s| s.need_save()).count()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &EditorSceneEntry> {
        self.entries.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut EditorSceneEntry> {
        self.entries.iter_mut()
    }

    pub fn try_get(&self, index: usize) -> Option<&EditorSceneEntry> {
        self.entries.get(index)
    }

    pub fn try_get_mut(&mut self, index: usize) -> Option<&mut EditorSceneEntry> {
        self.entries.get_mut(index)
    }

    pub fn current_scene_index(&self) -> Option<usize> {
        self.current_scene
    }

    pub fn set_current_scene(&mut self, id: Uuid) -> bool {
        if let Some(index) = self.entries.iter().position(|e| e.id == id) {
            self.current_scene = Some(index);
            true
        } else {
            false
        }
    }

    pub fn has_scene(&self, id: Uuid) -> bool {
        self.entry_by_scene_id(id).is_some()
    }

    pub fn entry_by_scene_id(&self, id: Uuid) -> Option<&EditorSceneEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    pub fn entry_by_scene_id_mut(&mut self, id: Uuid) -> Option<&mut EditorSceneEntry> {
        self.entries.iter_mut().find(|e| e.id == id)
    }

    pub fn add_and_select(&mut self, entry: EditorSceneEntry) {
        self.current_scene = Some(self.entries.len());
        self.entries.push(entry);
    }

    pub fn take_scene(&mut self, id: Uuid) -> Option<EditorSceneEntry> {
        // Remember the UUID of the current scene, because the index is about to become invalid
        let current_id = self.current_scene_entry_ref().id;
        let scene = self
            .entries
            .iter()
            .position(|e| e.id == id)
            .map(|i| self.entries.remove(i));
        // Update the current scene index based on the UUID of the current scene.
        if !self.set_current_scene(current_id) {
            // If the scene could not be set by UUID, then the current scene was taken.
            self.current_scene = if self.entries.is_empty() {
                None
            } else {
                // TODO: Maybe set it to the previous one?
                Some(0)
            };
        }
        scene
    }
}
