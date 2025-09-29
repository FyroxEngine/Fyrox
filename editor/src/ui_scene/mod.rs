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

pub mod clipboard;
pub mod commands;
pub mod interaction;
pub mod menu;
pub mod selection;
pub mod utils;

use crate::{
    asset::item::AssetItem,
    command::{Command, CommandGroup, CommandStack},
    fyrox::{
        core::{
            algebra::{Vector2, Vector3},
            color::Color,
            futures::executor::block_on,
            log::Log,
            make_relative_path,
            math::Rect,
            pool::{ErasedHandle, Handle},
        },
        engine::Engine,
        fxhash::FxHashSet,
        graph::{BaseSceneGraph, SceneGraph},
        graphics::gpu_texture::PixelKind,
        gui::{
            brush::Brush,
            draw::{CommandTexture, Draw},
            message::{KeyCode, MessageDirection, MouseButton},
            UiNode, UiUpdateSwitches, UserInterface, UserInterfaceResourceExtension,
        },
        resource::texture::{TextureKind, TextureResource, TextureResourceExtension},
        scene::SceneContainer,
    },
    message::MessageSender,
    plugins::inspector::editors::handle::{
        HandlePropertyEditorHierarchyMessage, HandlePropertyEditorNameMessage,
    },
    scene::{
        commands::ChangeSelectionCommand, controller::SceneController, selector::HierarchyNode,
        Selection,
    },
    settings::{keys::KeyBindings, Settings},
    ui_scene::{
        clipboard::Clipboard,
        commands::{graph::AddUiPrefabCommand, UiSceneContext},
        selection::UiSelection,
    },
    Message,
};
use fyrox::gui::message::UiMessage;
use fyrox::renderer::ui_renderer::UiRenderInfo;
use std::{fs::File, io::Write, path::Path};

pub struct PreviewInstance {
    pub instance: Handle<UiNode>,
    pub nodes: FxHashSet<Handle<UiNode>>,
}

pub struct UiScene {
    pub ui: UserInterface,
    pub render_target: TextureResource,
    pub message_sender: MessageSender,
    pub clipboard: Clipboard,
    pub preview_instance: Option<PreviewInstance>,
    pub ui_update_switches: UiUpdateSwitches,
}

impl UiScene {
    pub fn new(ui: UserInterface, message_sender: MessageSender) -> Self {
        Self {
            ui,
            render_target: TextureResource::new_render_target(200, 200),
            message_sender,
            clipboard: Default::default(),
            preview_instance: None,
            ui_update_switches: UiUpdateSwitches {
                // Disable update for everything.
                node_overrides: Some(Default::default()),
            },
        }
    }

    fn select_object(&mut self, handle: ErasedHandle) {
        if self.ui.try_get(handle.into()).is_some() {
            self.message_sender
                .do_command(ChangeSelectionCommand::new(Selection::new(
                    UiSelection::single_or_empty(handle.into()),
                )))
        }
    }
}

impl SceneController for UiScene {
    fn on_key_up(
        &mut self,
        _key: KeyCode,
        _engine: &mut Engine,
        _key_bindings: &KeyBindings,
    ) -> bool {
        false
    }

    fn on_key_down(
        &mut self,
        _key: KeyCode,
        _engine: &mut Engine,
        _key_bindings: &KeyBindings,
    ) -> bool {
        false
    }

    fn on_mouse_move(
        &mut self,
        _pos: Vector2<f32>,
        _offset: Vector2<f32>,
        _screen_bounds: Rect<f32>,
        _engine: &mut Engine,
        _settings: &Settings,
    ) {
    }

    fn on_mouse_up(
        &mut self,
        _button: MouseButton,
        _pos: Vector2<f32>,
        _screen_bounds: Rect<f32>,
        _engine: &mut Engine,
        _settings: &Settings,
    ) {
    }

    fn on_mouse_down(
        &mut self,
        _button: MouseButton,
        _pos: Vector2<f32>,
        _screen_bounds: Rect<f32>,
        _engine: &mut Engine,
        _settings: &Settings,
    ) {
    }

    fn on_mouse_wheel(&mut self, _amount: f32, _engine: &mut Engine, _settings: &Settings) {}

    fn on_mouse_leave(&mut self, _engine: &mut Engine, _settings: &Settings) {}

    fn on_drag_over(
        &mut self,
        handle: Handle<UiNode>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    ) {
        match self.preview_instance.as_ref() {
            None => {
                if let Some(item) = engine
                    .user_interfaces
                    .first_mut()
                    .node(handle)
                    .cast::<AssetItem>()
                {
                    // Make sure all resources loaded with relative paths only.
                    // This will make scenes portable.
                    if let Ok(relative_path) = make_relative_path(&item.path) {
                        // No model was loaded yet, do it.
                        if let Some(prefab) = engine
                            .resource_manager
                            .try_request::<UserInterface>(relative_path)
                            .and_then(|m| block_on(m).ok())
                        {
                            // Instantiate the model.
                            let (instance, _) = prefab.instantiate(&mut self.ui);

                            let nodes = self
                                .ui
                                .traverse_handle_iter(instance)
                                .collect::<FxHashSet<Handle<UiNode>>>();

                            self.preview_instance = Some(PreviewInstance { instance, nodes });
                        }
                    }
                }
            }
            Some(preview) => {
                let cursor_pos = engine.user_interfaces.first_mut().cursor_position();
                let rel_pos = cursor_pos - screen_bounds.position;

                let root = self.ui.node_mut(preview.instance);
                root.set_desired_local_position(
                    settings
                        .move_mode_settings
                        .try_snap_vector_to_grid(Vector3::new(rel_pos.x, rel_pos.y, 0.0))
                        .xy(),
                );
                root.invalidate_layout();
            }
        }
    }

    fn on_drop(
        &mut self,
        handle: Handle<UiNode>,
        _screen_bounds: Rect<f32>,
        _engine: &mut Engine,
        _settings: &Settings,
    ) {
        if handle.is_none() {
            return;
        }

        if let Some(preview) = self.preview_instance.take() {
            // Immediately after extract if from the scene to subgraph. This is required to not violate
            // the rule of one place of execution, only commands allowed to modify the scene.
            let sub_graph = self.ui.take_reserve_sub_graph(preview.instance);

            let group = vec![
                Command::new(AddUiPrefabCommand::new(sub_graph)),
                // We also want to select newly instantiated model.
                Command::new(ChangeSelectionCommand::new(Selection::new(
                    UiSelection::single_or_empty(preview.instance),
                ))),
            ];

            self.message_sender.do_command(CommandGroup::from(group));
        }
    }

    fn render_target(&self, _engine: &Engine) -> Option<TextureResource> {
        Some(self.render_target.clone())
    }

    fn extension(&self) -> &str {
        "ui"
    }

    fn save(
        &mut self,
        path: &Path,
        settings: &Settings,
        _engine: &mut Engine,
    ) -> Result<String, String> {
        match self.ui.save(path) {
            Ok(visitor) => {
                if settings.debugging.save_scene_in_text_form {
                    let text = visitor.save_ascii_to_string();
                    let mut path = path.to_path_buf();
                    path.set_extension("txt");
                    if let Ok(mut file) = File::create(path) {
                        Log::verify(file.write_all(text.as_bytes()));
                    }
                }

                Ok(format!(
                    "Ui scene was successfully saved to {}",
                    path.display()
                ))
            }
            Err(e) => Err(format!(
                "Unable to save the ui scene to {} file. Reason: {:?}",
                path.display(),
                e
            )),
        }
    }

    fn do_command(
        &mut self,
        command_stack: &mut CommandStack,
        command: Command,
        selection: &mut Selection,
        _engine: &mut Engine,
    ) {
        UiSceneContext::exec(
            &mut self.ui,
            selection,
            self.message_sender.clone(),
            &mut self.clipboard,
            |ctx| {
                command_stack.do_command(command, ctx);
            },
        );

        self.ui.invalidate_layout();
    }

    fn undo(
        &mut self,
        command_stack: &mut CommandStack,
        selection: &mut Selection,
        _engine: &mut Engine,
    ) {
        UiSceneContext::exec(
            &mut self.ui,
            selection,
            self.message_sender.clone(),
            &mut self.clipboard,
            |ctx| command_stack.undo(ctx),
        );

        self.ui.invalidate_layout();
    }

    fn redo(
        &mut self,
        command_stack: &mut CommandStack,
        selection: &mut Selection,
        _engine: &mut Engine,
    ) {
        UiSceneContext::exec(
            &mut self.ui,
            selection,
            self.message_sender.clone(),
            &mut self.clipboard,
            |ctx| command_stack.redo(ctx),
        );

        self.ui.invalidate_layout();
    }

    fn clear_command_stack(
        &mut self,
        command_stack: &mut CommandStack,
        selection: &mut Selection,
        _scenes: &mut SceneContainer,
    ) {
        UiSceneContext::exec(
            &mut self.ui,
            selection,
            self.message_sender.clone(),
            &mut self.clipboard,
            |ctx| command_stack.clear(ctx),
        );

        self.ui.invalidate_layout();
    }

    fn on_before_render(&mut self, editor_selection: &Selection, engine: &mut Engine) {
        self.ui.draw();

        // Draw selection on top.
        if let Some(selection) = editor_selection.as_ui() {
            for node in selection.widgets.iter() {
                if let Some(node) = self.ui.try_get(*node) {
                    let bounds = node.screen_bounds();
                    let clip_bounds = node.clip_bounds();
                    let drawing_context = &mut self.ui.drawing_context;
                    drawing_context.push_rect(&bounds, 1.0);
                    drawing_context.commit(
                        clip_bounds,
                        Brush::Solid(Color::GREEN),
                        CommandTexture::None,
                        &self.ui.standard_material,
                        None,
                    );
                }
            }
        }

        // Render to texture.
        Log::verify(
            engine
                .graphics_context
                .as_initialized_mut()
                .renderer
                .render_ui(UiRenderInfo {
                    render_target: Some(self.render_target.clone()),
                    screen_size: self.ui.screen_size(),
                    drawing_context: &self.ui.drawing_context,
                    clear_color: Color::DIM_GRAY,
                    resource_manager: &engine.resource_manager,
                }),
        );
    }

    fn on_after_render(&mut self, _engine: &mut Engine) {}

    fn update(
        &mut self,
        _editor_selection: &Selection,
        _engine: &mut Engine,
        dt: f32,
        _path: Option<&Path>,
        _settings: &mut Settings,
        screen_bounds: Rect<f32>,
    ) -> Option<TextureResource> {
        self.ui
            .update(screen_bounds.size, dt, &self.ui_update_switches);

        // Create new render target if preview frame has changed its size.
        let mut new_render_target = None;
        if let TextureKind::Rectangle { width, height } =
            self.render_target.clone().data_ref().kind()
        {
            let frame_size = screen_bounds.size;
            if width != frame_size.x as u32 || height != frame_size.y as u32 {
                self.render_target =
                    TextureResource::new_render_target(frame_size.x as u32, frame_size.y as u32);
                new_render_target = Some(self.render_target.clone());

                self.ui.invalidate_layout();
            }
        }

        while self.ui.poll_message().is_some() {}

        new_render_target
    }

    fn is_interacting(&self) -> bool {
        false
    }

    fn on_destroy(
        &mut self,
        command_stack: &mut CommandStack,
        _engine: &mut Engine,
        selection: &mut Selection,
    ) {
        UiSceneContext::exec(
            &mut self.ui,
            selection,
            self.message_sender.clone(),
            &mut self.clipboard,
            |ctx| command_stack.clear(ctx),
        );
    }

    fn on_message(
        &mut self,
        message: &Message,
        _selection: &Selection,
        engine: &mut Engine,
    ) -> bool {
        match message {
            Message::SelectObject { handle } => {
                self.select_object(*handle);
            }
            Message::SyncNodeHandleName { view, handle } => {
                engine.user_interfaces.first_mut().send_message(
                    UiMessage::with_data(HandlePropertyEditorNameMessage(
                        self.ui
                            .try_get((*handle).into())
                            .map(|n| n.name().to_owned()),
                    ))
                    .with_destination(*view)
                    .with_direction(MessageDirection::ToWidget),
                );
            }
            Message::ProvideSceneHierarchy { view } => {
                engine.user_interfaces.first_mut().send_message(
                    UiMessage::with_data(HandlePropertyEditorHierarchyMessage(
                        HierarchyNode::from_scene_node(self.ui.root(), Handle::NONE, &self.ui),
                    ))
                    .with_destination(*view)
                    .with_direction(MessageDirection::ToWidget),
                );
            }
            _ => {}
        }

        false
    }

    fn command_names(
        &mut self,
        command_stack: &mut CommandStack,
        selection: &mut Selection,
        _engine: &mut Engine,
    ) -> Vec<String> {
        command_stack
            .commands
            .iter_mut()
            .map(|c| {
                let mut name = String::new();
                UiSceneContext::exec(
                    &mut self.ui,
                    selection,
                    self.message_sender.clone(),
                    &mut self.clipboard,
                    |ctx| {
                        name = c.name(ctx);
                    },
                );
                name
            })
            .collect::<Vec<_>>()
    }
}
