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

use crate::scene::controller::SceneController;
use crate::scene::nullscene::NullSceneController;
use crate::{
    command::CommandStack,
    fyrox::{
        core::{algebra::Vector2, math::Rect, pool::Handle, reflect::Reflect, uuid::Uuid},
        engine::Engine,
        gui::{
            UiNode, UserInterface,
            file_browser::FileType,
            message::{KeyCode, MouseButton},
        },
        scene::Scene,
    },
    game_scene::{
        GameScene, Selection,
        interaction::{
            move_mode::MoveInteractionMode, rotate_mode::RotateInteractionMode,
            scale_mode::ScaleInteractionMode, select_mode::SelectInteractionMode,
        },
    },
    highlight::HighlightRenderPass,
    interaction::container::InteractionModeContainer,
    message::MessageSender,
    scene_viewer::SceneViewer,
    settings::{Settings, keys::KeyBindings},
    ui_scene::{
        UiScene, interaction::UiSelectInteractionMode,
        interaction::move_mode::MoveWidgetsInteractionMode,
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
    // Create an entry for when there is no scene using a [`NullSceneController`].
    pub fn new_null_scene(
        engine: &mut Engine,
        settings: &mut Settings,
        message_sender: MessageSender,
    ) -> Self {
        Self {
            has_unsaved_changes: false,
            path: None,
            selection: Selection::default(),
            command_stack: CommandStack::new(false, settings.general.max_history_entries),
            controller: Box::new(NullSceneController {
                sender: message_sender.clone(),
                resource_manager: engine.resource_manager.clone(),
            }),
            interaction_modes: InteractionModeContainer::default(),
            current_interaction_mode: None,
            last_mouse_pos: None,
            click_mouse_pos: None,
            sender: message_sender,
            id: Default::default(),
        }
    }
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
        interaction_modes.add(RotateInteractionMode::new(
            &game_scene,
            engine,
            message_sender.clone(),
        ));
        interaction_modes.add(ScaleInteractionMode::new(
            &game_scene,
            engine,
            message_sender.clone(),
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

        entry.set_interaction_mode(engine, Some(MoveInteractionMode::type_info().type_uuid));

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

        entry.set_interaction_mode(engine, Some(UiSelectInteractionMode::type_info().type_uuid));

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

    pub fn default_file_info(&self) -> (PathBuf, FileType) {
        let file_type = self.controller.file_type();
        (file_type.make_file_name("unnamed"), file_type)
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
            && interaction_mode.on_key_up(key, &mut *self.controller, engine)
        {
            return true;
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
            && interaction_mode.on_key_down(key, &self.selection, &mut *self.controller, engine)
        {
            return true;
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
        if button == MouseButton::Left
            && let Some(interaction_mode) = self
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
        if button == MouseButton::Left
            && let Some(interaction_mode) = self
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

        self.controller
            .on_mouse_down(button, pos, screen_bounds, engine, settings)
    }

    pub fn on_mouse_wheel(&mut self, amount: f32, engine: &mut Engine, settings: &Settings) {
        self.controller.on_mouse_wheel(amount, engine, settings)
    }

    /// Called when the mouse enters the scene viewer.
    pub fn on_mouse_enter(
        &mut self,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    ) {
        if let Some(interaction_mode) = self
            .current_interaction_mode
            .and_then(|id| self.interaction_modes.get_mut(&id))
        {
            interaction_mode.on_mouse_enter(
                &self.selection,
                &mut *self.controller,
                engine,
                screen_bounds.size,
                settings,
            );
        }
    }

    /// Called when the moue leaves the scene viewer.
    pub fn on_mouse_leave(
        &mut self,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    ) {
        let last_pos = self
            .last_mouse_pos
            .unwrap_or_else(|| engine.user_interfaces.first().cursor_position());
        let rel_pos = last_pos - screen_bounds.position;

        if let Some(interaction_mode) = self
            .current_interaction_mode
            .and_then(|id| self.interaction_modes.get_mut(&id))
        {
            interaction_mode.on_mouse_leave(
                rel_pos,
                &self.selection,
                &mut *self.controller,
                engine,
                screen_bounds.size,
                settings,
            );
        }
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
