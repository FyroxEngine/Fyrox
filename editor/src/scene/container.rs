use crate::command::CommandStack;
use crate::fyrox::{
    core::{algebra::Vector2, math::Rect, pool::Handle, uuid::Uuid, TypeUuidProvider},
    engine::Engine,
    gui::{
        message::{KeyCode, MouseButton},
        UiNode, UserInterface,
    },
    scene::Scene,
};
use crate::{
    highlight::HighlightRenderPass,
    interaction::{
        move_mode::MoveInteractionMode, navmesh::EditNavmeshMode,
        rotate_mode::RotateInteractionMode, scale_mode::ScaleInteractionMode,
        select_mode::SelectInteractionMode, terrain::TerrainInteractionMode,
        InteractionModeContainer,
    },
    message::MessageSender,
    scene::{controller::SceneController, GameScene, Selection},
    scene_viewer::SceneViewer,
    settings::{keys::KeyBindings, Settings},
    ui_scene::{
        interaction::move_mode::MoveWidgetsInteractionMode, interaction::UiSelectInteractionMode,
        UiScene,
    },
};
use std::{cell::RefCell, path::PathBuf, rc::Rc};

pub struct EditorSceneEntry {
    pub has_unsaved_changes: bool,
    pub path: Option<PathBuf>,
    pub selection: Selection,
    pub command_stack: CommandStack,
    pub controller: Box<dyn SceneController>,
    pub interaction_modes: InteractionModeContainer,
    pub current_interaction_mode: Option<Uuid>,

    pub last_mouse_pos: Option<Vector2<f32>>,
    pub click_mouse_pos: Option<Vector2<f32>>,
    pub sender: MessageSender,
    pub id: Uuid,
}

impl EditorSceneEntry {
    pub fn new_game_scene(
        scene: Scene,
        path: Option<PathBuf>,
        engine: &mut Engine,
        settings: &mut Settings,
        message_sender: MessageSender,
        scene_viewer: &SceneViewer,
        highlighter: Option<Rc<RefCell<HighlightRenderPass>>>,
    ) -> Self {
        let game_scene = GameScene::from_native_scene(
            scene,
            engine,
            path.as_deref(),
            settings,
            message_sender.clone(),
            highlighter,
        );

        let mut interaction_modes = InteractionModeContainer::default();
        interaction_modes.add(SelectInteractionMode::new(
            scene_viewer.frame(),
            scene_viewer.selection_frame(),
            message_sender.clone(),
        ));
        interaction_modes.add(MoveInteractionMode::new(
            &game_scene,
            engine,
            message_sender.clone(),
        ));
        interaction_modes.add(ScaleInteractionMode::new(
            &game_scene,
            engine,
            message_sender.clone(),
        ));
        interaction_modes.add(RotateInteractionMode::new(
            &game_scene,
            engine,
            message_sender.clone(),
        ));
        interaction_modes.add(EditNavmeshMode::new(
            &game_scene,
            engine,
            message_sender.clone(),
        ));
        interaction_modes.add(TerrainInteractionMode::new(
            &game_scene,
            engine,
            message_sender.clone(),
            scene_viewer.frame(),
        ));
        interaction_modes.sender = Some(message_sender.clone());

        let mut entry = EditorSceneEntry {
            has_unsaved_changes: false,
            interaction_modes,
            controller: Box::new(game_scene),
            current_interaction_mode: None,
            last_mouse_pos: None,
            click_mouse_pos: None,
            sender: message_sender,
            id: Uuid::new_v4(),
            path,
            selection: Default::default(),
            command_stack: CommandStack::new(false, settings.general.max_history_entries),
        };

        entry.set_interaction_mode(engine, Some(MoveInteractionMode::type_uuid()));

        entry
    }

    pub fn new_ui_scene(
        ui: UserInterface,
        path: Option<PathBuf>,
        message_sender: MessageSender,
        scene_viewer: &SceneViewer,
        engine: &mut Engine,
        settings: &Settings,
    ) -> Self {
        let mut interaction_modes = InteractionModeContainer::default();
        interaction_modes.add(UiSelectInteractionMode::new(
            scene_viewer.frame(),
            scene_viewer.selection_frame(),
            message_sender.clone(),
        ));
        interaction_modes.add(MoveWidgetsInteractionMode::new(message_sender.clone()));
        interaction_modes.sender = Some(message_sender.clone());

        let mut entry = EditorSceneEntry {
            has_unsaved_changes: false,
            interaction_modes,
            controller: Box::new(UiScene::new(ui, message_sender.clone())),
            current_interaction_mode: None,
            last_mouse_pos: None,
            click_mouse_pos: None,
            sender: message_sender,
            id: Uuid::new_v4(),
            path,
            selection: Default::default(),
            command_stack: CommandStack::new(false, settings.general.max_history_entries),
        };

        entry.set_interaction_mode(engine, Some(UiSelectInteractionMode::type_uuid()));

        entry
    }

    pub fn set_interaction_mode(&mut self, engine: &mut Engine, mode: Option<Uuid>) {
        if self.current_interaction_mode != mode {
            // Deactivate current first.
            if let Some(interaction_mode) = self
                .current_interaction_mode
                .and_then(|current_mode| self.interaction_modes.get_mut(&current_mode))
            {
                interaction_mode.deactivate(&*self.controller, engine);
            }

            self.current_interaction_mode = mode;

            // Activate new.
            if let Some(interaction_mode) = self
                .current_interaction_mode
                .and_then(|current_mode| self.interaction_modes.get_mut(&current_mode))
            {
                interaction_mode.activate(&*self.controller, engine);
            }
        }
    }

    pub fn default_file_name(&self) -> PathBuf {
        format!("unnamed.{}", self.controller.extension()).into()
    }

    pub fn need_save(&self) -> bool {
        self.has_unsaved_changes || self.path.is_none()
    }

    pub fn before_drop(&mut self, engine: &mut Engine) {
        for mut interaction_mode in self.interaction_modes.drain() {
            interaction_mode.on_drop(engine);
        }
    }

    pub fn name(&self) -> String {
        self.path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| String::from("Unnamed Scene"))
    }

    pub fn save(
        &mut self,
        path: PathBuf,
        settings: &Settings,
        engine: &mut Engine,
    ) -> Result<String, String> {
        let result = self.controller.save(&path, settings, engine);
        self.path = Some(path);
        result
    }

    #[must_use]
    pub fn on_key_up(
        &mut self,
        key: KeyCode,
        engine: &mut Engine,
        key_bindings: &KeyBindings,
    ) -> bool {
        if self.controller.on_key_up(key, engine, key_bindings) {
            return true;
        }

        if let Some(interaction_mode) = self
            .current_interaction_mode
            .and_then(|id| self.interaction_modes.get_mut(&id))
        {
            if interaction_mode.on_key_up(key, &mut *self.controller, engine) {
                return true;
            }
        }

        false
    }

    #[must_use]
    pub fn on_key_down(
        &mut self,
        key: KeyCode,
        engine: &mut Engine,
        key_bindings: &KeyBindings,
    ) -> bool {
        if self.controller.on_key_down(key, engine, key_bindings) {
            return true;
        }

        if let Some(interaction_mode) = self
            .current_interaction_mode
            .and_then(|id| self.interaction_modes.get_mut(&id))
        {
            if interaction_mode.on_key_down(key, &self.selection, &mut *self.controller, engine) {
                return true;
            }
        }

        false
    }

    pub fn on_mouse_move(
        &mut self,
        pos: Vector2<f32>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    ) {
        let last_pos = *self.last_mouse_pos.get_or_insert(pos);
        let mouse_offset = pos - last_pos;
        let rel_pos = pos - screen_bounds.position;

        if let Some(interaction_mode) = self
            .current_interaction_mode
            .and_then(|id| self.interaction_modes.get_mut(&id))
        {
            interaction_mode.on_mouse_move(
                mouse_offset,
                rel_pos,
                &self.selection,
                &mut *self.controller,
                engine,
                screen_bounds.size,
                settings,
            );
        }

        self.last_mouse_pos = Some(pos);

        self.controller
            .on_mouse_move(pos, mouse_offset, screen_bounds, engine, settings)
    }

    pub fn on_mouse_up(
        &mut self,
        button: MouseButton,
        pos: Vector2<f32>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    ) {
        if button == MouseButton::Left {
            if let Some(interaction_mode) = self
                .current_interaction_mode
                .and_then(|id| self.interaction_modes.get_mut(&id))
            {
                let rel_pos = pos - screen_bounds.position;
                interaction_mode.on_left_mouse_button_up(
                    &self.selection,
                    &mut *self.controller,
                    engine,
                    rel_pos,
                    screen_bounds.size,
                    settings,
                );
            }
        }

        self.controller
            .on_mouse_up(button, pos, screen_bounds, engine, settings)
    }

    pub fn on_mouse_down(
        &mut self,
        button: MouseButton,
        pos: Vector2<f32>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    ) {
        if button == MouseButton::Left {
            if let Some(interaction_mode) = self
                .current_interaction_mode
                .and_then(|id| self.interaction_modes.get_mut(&id))
            {
                let rel_pos = pos - screen_bounds.position;

                interaction_mode.on_left_mouse_button_down(
                    &self.selection,
                    &mut *self.controller,
                    engine,
                    rel_pos,
                    screen_bounds.size,
                    settings,
                );
            }
        }

        self.controller
            .on_mouse_down(button, pos, screen_bounds, engine, settings)
    }

    pub fn on_mouse_wheel(&mut self, amount: f32, engine: &mut Engine, settings: &Settings) {
        self.controller.on_mouse_wheel(amount, engine, settings)
    }

    pub fn on_mouse_leave(&mut self, engine: &mut Engine, settings: &Settings) {
        self.controller.on_mouse_leave(engine, settings)
    }

    pub fn on_drag_over(
        &mut self,
        handle: Handle<UiNode>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    ) {
        self.controller
            .on_drag_over(handle, screen_bounds, engine, settings)
    }

    pub fn on_drop(
        &mut self,
        handle: Handle<UiNode>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    ) {
        self.controller
            .on_drop(handle, screen_bounds, engine, settings)
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

    pub fn current_scene_controller_ref(&self) -> Option<&dyn SceneController> {
        self.current_scene_entry_ref().map(|e| &*e.controller)
    }

    pub fn current_scene_controller_mut(&mut self) -> Option<&mut dyn SceneController> {
        self.current_scene_entry_mut()
            .map(move |e| &mut *e.controller)
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
