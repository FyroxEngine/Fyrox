pub mod clipboard;
pub mod commands;
pub mod interaction;
pub mod menu;
pub mod selection;
pub mod utils;

use crate::{
    inspector::editors::handle::HandlePropertyEditorMessage,
    message::MessageSender,
    scene::{controller::SceneController, selector::HierarchyNode, Selection},
    settings::{keys::KeyBindings, Settings},
    ui_scene::{
        clipboard::Clipboard,
        commands::{
            make_set_widget_property_command, ChangeUiSelectionCommand, UiCommand, UiCommandGroup,
            UiCommandStack, UiSceneContext,
        },
        selection::UiSelection,
    },
    Message,
};
use fyrox::{
    core::{
        algebra::Vector2,
        color::Color,
        log::Log,
        math::Rect,
        pool::{ErasedHandle, Handle},
        reflect::Reflect,
    },
    engine::Engine,
    gui::{
        brush::Brush,
        draw::{CommandTexture, Draw},
        inspector::PropertyChanged,
        message::{KeyCode, MessageDirection, MouseButton},
        UiNode, UserInterface,
    },
    renderer::framework::gpu_texture::PixelKind,
    resource::texture::{TextureKind, TextureResource, TextureResourceExtension},
    scene::SceneContainer,
};
use std::{any::Any, fs::File, io::Write, path::Path};

pub struct UiScene {
    pub ui: UserInterface,
    pub render_target: TextureResource,
    pub command_stack: UiCommandStack,
    pub message_sender: MessageSender,
    pub clipboard: Clipboard,
}

impl UiScene {
    pub fn new(ui: UserInterface, message_sender: MessageSender) -> Self {
        Self {
            ui,
            render_target: TextureResource::new_render_target(200, 200),
            command_stack: UiCommandStack::new(false),
            message_sender,
            clipboard: Default::default(),
        }
    }

    pub fn do_command(
        &mut self,
        command: Box<dyn UiCommand>,
        selection: &mut Selection,
        _engine: &mut Engine,
    ) {
        self.command_stack.do_command(
            command,
            UiSceneContext {
                ui: &mut self.ui,
                selection,
                message_sender: &self.message_sender,
                clipboard: &mut self.clipboard,
            },
        );

        self.ui.invalidate_layout();
    }

    fn select_object(&mut self, handle: ErasedHandle, selection: &Selection) {
        if self.ui.try_get_node(handle.into()).is_some() {
            self.message_sender
                .do_ui_scene_command(ChangeUiSelectionCommand::new(
                    Selection::Ui(UiSelection::single_or_empty(handle.into())),
                    selection.clone(),
                ))
        }
    }
}

impl SceneController for UiScene {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

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
        _handle: Handle<UiNode>,
        _screen_bounds: Rect<f32>,
        _engine: &mut Engine,
        _settings: &Settings,
    ) {
    }

    fn on_drop(
        &mut self,
        _handle: Handle<UiNode>,
        _screen_bounds: Rect<f32>,
        _engine: &mut Engine,
        _settings: &Settings,
        _editor_selection: &Selection,
    ) {
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
                    let text = visitor.save_text();
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

    fn undo(&mut self, selection: &mut Selection, _engine: &mut Engine) {
        self.command_stack.undo(UiSceneContext {
            ui: &mut self.ui,
            selection,
            message_sender: &self.message_sender,
            clipboard: &mut self.clipboard,
        });

        self.ui.invalidate_layout();
    }

    fn redo(&mut self, selection: &mut Selection, _engine: &mut Engine) {
        self.command_stack.redo(UiSceneContext {
            ui: &mut self.ui,
            selection,
            message_sender: &self.message_sender,
            clipboard: &mut self.clipboard,
        });

        self.ui.invalidate_layout();
    }

    fn clear_command_stack(&mut self, selection: &mut Selection, _engine: &mut Engine) {
        self.command_stack.clear(UiSceneContext {
            ui: &mut self.ui,
            selection,
            message_sender: &self.message_sender,
            clipboard: &mut self.clipboard,
        });

        self.ui.invalidate_layout();
    }

    fn on_before_render(&mut self, editor_selection: &Selection, engine: &mut Engine) {
        self.ui.draw();

        // Draw selection on top.
        if let Selection::Ui(selection) = editor_selection {
            for node in selection.widgets.iter() {
                if let Some(node) = self.ui.try_get_node(*node) {
                    let bounds = node.screen_bounds();
                    let clip_bounds = node.clip_bounds();
                    let drawing_context = self.ui.get_drawing_context_mut();
                    drawing_context.push_rect(&bounds, 1.0);
                    drawing_context.commit(
                        clip_bounds,
                        Brush::Solid(Color::GREEN),
                        CommandTexture::None,
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
                .render_ui_to_texture(
                    self.render_target.clone(),
                    self.ui.screen_size(),
                    self.ui.get_drawing_context(),
                    Color::DIM_GRAY,
                    PixelKind::RGBA8,
                ),
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
        self.ui.update(screen_bounds.size, dt);

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

    fn on_destroy(&mut self, _engine: &mut Engine) {}

    fn on_message(
        &mut self,
        message: &Message,
        selection: &Selection,
        engine: &mut Engine,
    ) -> bool {
        match message {
            Message::SelectObject { handle } => {
                self.select_object(*handle, selection);
            }
            Message::SyncNodeHandleName { view, handle } => {
                engine
                    .user_interface
                    .send_message(HandlePropertyEditorMessage::name(
                        *view,
                        MessageDirection::ToWidget,
                        self.ui
                            .try_get_node((*handle).into())
                            .map(|n| n.name().to_owned()),
                    ));
            }
            Message::ProvideSceneHierarchy { view } => {
                engine
                    .user_interface
                    .send_message(HandlePropertyEditorMessage::hierarchy(
                        *view,
                        MessageDirection::ToWidget,
                        HierarchyNode::from_ui_node(self.ui.root(), Handle::NONE, &self.ui),
                    ));
            }
            _ => {}
        }

        false
    }

    fn top_command_index(&self) -> Option<usize> {
        self.command_stack.top
    }

    fn command_names(&mut self, selection: &mut Selection, _engine: &mut Engine) -> Vec<String> {
        self.command_stack
            .commands
            .iter_mut()
            .map(|c| {
                c.name(&UiSceneContext {
                    ui: &mut self.ui,
                    selection,
                    message_sender: &self.message_sender,
                    clipboard: &mut self.clipboard,
                })
            })
            .collect::<Vec<_>>()
    }

    fn first_selected_entity(
        &self,
        selection: &Selection,
        _scenes: &SceneContainer,
        callback: &mut dyn FnMut(&dyn Reflect),
    ) {
        if let Selection::Ui(selection) = selection {
            if let Some(first) = selection.widgets.first() {
                if let Some(node) = self.ui.try_get_node(*first).map(|n| n as &dyn Reflect) {
                    (callback)(node)
                }
            }
        }
    }

    fn on_property_changed(
        &mut self,
        args: &PropertyChanged,
        selection: &Selection,
        _engine: &mut Engine,
    ) {
        let group = match selection {
            Selection::Ui(selection) => selection
                .widgets
                .iter()
                .filter_map(|&node_handle| {
                    if self.ui.try_get_node(node_handle).is_some() {
                        make_set_widget_property_command(node_handle, args)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>(),
            _ => vec![],
        };

        if group.is_empty() {
            if !args.is_inheritable() {
                Log::err(format!("Failed to handle a property {}", args.path()))
            }
        } else if group.len() == 1 {
            self.message_sender
                .send(Message::DoUiSceneCommand(group.into_iter().next().unwrap()))
        } else {
            self.message_sender
                .do_ui_scene_command(UiCommandGroup::from(group));
        }
    }

    fn provide_docs(&self, selection: &Selection, _engine: &Engine) -> Option<String> {
        match selection {
            Selection::Ui(selection) => selection
                .widgets
                .first()
                .and_then(|h| self.ui.try_get_node(*h).map(|n| n.doc().to_string())),
            _ => None,
        }
    }
}
