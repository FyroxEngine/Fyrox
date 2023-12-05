use crate::{
    command::CommandStack,
    interaction::{
        move_mode::MoveInteractionMode, navmesh::EditNavmeshMode,
        rotate_mode::RotateInteractionMode, scale_mode::ScaleInteractionMode,
        select_mode::SelectInteractionMode, terrain::TerrainInteractionMode,
        InteractionModeContainer,
    },
    message::MessageSender,
    scene::EditorScene,
    scene_viewer::SceneViewer,
    settings::Settings,
};
use fyrox::{
    core::{algebra::Vector2, pool::Handle, uuid::Uuid, TypeUuidProvider},
    engine::Engine,
    fxhash::FxHashSet,
    scene::{node::Node, Scene},
};
use std::path::PathBuf;

pub struct PreviewInstance {
    pub instance: Handle<Node>,
    pub nodes: FxHashSet<Handle<Node>>,
}

pub struct EditorSceneEntry {
    pub editor_scene: EditorScene,
    pub command_stack: CommandStack,
    pub interaction_modes: InteractionModeContainer,
    pub current_interaction_mode: Option<Uuid>,
    pub preview_instance: Option<PreviewInstance>,
    pub last_mouse_pos: Option<Vector2<f32>>,
    pub click_mouse_pos: Option<Vector2<f32>>,
    pub sender: MessageSender,
    pub id: Uuid,
}

impl EditorSceneEntry {
    pub fn set_interaction_mode(&mut self, engine: &mut Engine, mode: Option<Uuid>) {
        if self.current_interaction_mode != mode {
            // Deactivate current first.
            if let Some(current_mode) = self.current_interaction_mode {
                self.interaction_modes
                    .map
                    .get_mut(&current_mode)
                    .unwrap()
                    .deactivate(&self.editor_scene, engine);
            }

            self.current_interaction_mode = mode;

            // Activate new.
            if let Some(current_mode) = self.current_interaction_mode {
                self.interaction_modes
                    .map
                    .get_mut(&current_mode)
                    .unwrap()
                    .activate(&self.editor_scene, engine);
            }
        }
    }

    pub fn before_drop(&mut self, engine: &mut Engine) {
        for (_, mut interaction_mode) in self.interaction_modes.map.drain() {
            interaction_mode.on_drop(engine);
        }
    }
}

#[derive(Default)]
pub struct SceneContainer {
    pub entries: Vec<EditorSceneEntry>,
    current_scene: Option<usize>,
}

impl SceneContainer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn current_scene_entry_ref(&self) -> Option<&EditorSceneEntry> {
        self.current_scene.and_then(|i| self.entries.get(i))
    }

    pub fn current_scene_entry_mut(&mut self) -> Option<&mut EditorSceneEntry> {
        self.current_scene.and_then(|i| self.entries.get_mut(i))
    }

    pub fn current_editor_scene_ref(&self) -> Option<&EditorScene> {
        self.current_scene_entry_ref().map(|e| &e.editor_scene)
    }

    pub fn current_editor_scene_mut(&mut self) -> Option<&mut EditorScene> {
        self.current_scene_entry_mut().map(|e| &mut e.editor_scene)
    }

    pub fn first_unsaved_scene(&self) -> Option<&EditorSceneEntry> {
        self.entries.iter().find(|s| s.editor_scene.need_save())
    }

    pub fn unsaved_scene_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|s| s.editor_scene.need_save())
            .count()
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

    pub fn entry_by_scene_id(&self, id: Uuid) -> Option<&EditorSceneEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    pub fn entry_by_scene_id_mut(&mut self, id: Uuid) -> Option<&mut EditorSceneEntry> {
        self.entries.iter_mut().find(|e| e.id == id)
    }

    pub fn add_scene_and_select(
        &mut self,
        scene: Scene,
        path: Option<PathBuf>,
        engine: &mut Engine,
        settings: &Settings,
        message_sender: MessageSender,
        scene_viewer: &SceneViewer,
    ) {
        self.current_scene = Some(self.entries.len());

        let editor_scene = EditorScene::from_native_scene(scene, engine, path, settings);
        let mut interaction_modes = InteractionModeContainer::default();
        interaction_modes.add(SelectInteractionMode::new(
            scene_viewer.frame(),
            scene_viewer.selection_frame(),
            message_sender.clone(),
        ));
        interaction_modes.add(MoveInteractionMode::new(
            &editor_scene,
            engine,
            message_sender.clone(),
        ));
        interaction_modes.add(ScaleInteractionMode::new(
            &editor_scene,
            engine,
            message_sender.clone(),
        ));
        interaction_modes.add(RotateInteractionMode::new(
            &editor_scene,
            engine,
            message_sender.clone(),
        ));
        interaction_modes.add(EditNavmeshMode::new(
            &editor_scene,
            engine,
            message_sender.clone(),
        ));
        interaction_modes.add(TerrainInteractionMode::new(
            &editor_scene,
            engine,
            message_sender.clone(),
            scene_viewer.frame(),
        ));

        let mut entry = EditorSceneEntry {
            interaction_modes,
            editor_scene,
            command_stack: CommandStack::new(false),
            current_interaction_mode: None,
            preview_instance: None,
            last_mouse_pos: None,
            click_mouse_pos: None,
            sender: message_sender,
            id: Uuid::new_v4(),
        };

        entry.set_interaction_mode(engine, Some(MoveInteractionMode::type_uuid()));

        self.entries.push(entry);
    }

    pub fn take_scene(&mut self, id: Uuid) -> Option<EditorSceneEntry> {
        let scene = self
            .entries
            .iter()
            .position(|e| e.id == id)
            .map(|i| self.entries.remove(i));
        self.current_scene = if self.entries.is_empty() {
            None
        } else {
            // TODO: Maybe set it to the previous one?
            Some(0)
        };
        scene
    }
}
