#![allow(unused_variables)] // TODO

use crate::{
    define_command_stack,
    interaction::{make_interaction_mode_button, InteractionMode},
    message::MessageSender,
    scene::{controller::SceneController, Selection},
    settings::{keys::KeyBindings, Settings},
    world::WorldViewerDataProvider,
    Message,
};
use fyrox::core::reflect::Reflect;
use fyrox::scene::SceneContainer;
use fyrox::{
    core::{
        algebra::Vector2,
        color::Color,
        log::Log,
        math::Rect,
        pool::{ErasedHandle, Handle},
        uuid::{uuid, Uuid},
        TypeUuidProvider,
    },
    engine::Engine,
    gui::{
        draw::SharedTexture,
        message::{KeyCode, MessageDirection, MouseButton},
        widget::WidgetMessage,
        BuildContext, UiNode, UserInterface,
    },
    resource::texture::{TextureKind, TextureResource, TextureResourceExtension},
};
use std::{
    any::Any,
    fmt::Debug,
    fs::File,
    io::Write,
    ops::{Deref, DerefMut},
    path::Path,
};

pub struct UiScene {
    pub ui: UserInterface,
    pub render_target: TextureResource,
    pub command_stack: UiCommandStack,
    pub message_sender: MessageSender,
}

impl UiScene {
    pub fn new(ui: UserInterface, message_sender: MessageSender) -> Self {
        Self {
            ui,
            render_target: TextureResource::new_render_target(200, 200),
            command_stack: UiCommandStack::new(false),
            message_sender,
        }
    }

    pub fn do_command(
        &mut self,
        command: Box<dyn UiCommand>,
        selection: &mut Selection,
        engine: &mut Engine,
    ) {
        self.command_stack.do_command(
            command,
            UiSceneContext {
                selection,
                message_sender: &self.message_sender,
            },
        );
    }
}

impl SceneController for UiScene {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn on_key_up(&mut self, key: KeyCode, engine: &mut Engine, key_bindings: &KeyBindings) -> bool {
        false
    }

    fn on_key_down(
        &mut self,
        key: KeyCode,
        engine: &mut Engine,
        key_bindings: &KeyBindings,
    ) -> bool {
        false
    }

    fn on_mouse_move(
        &mut self,
        pos: Vector2<f32>,
        offset: Vector2<f32>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    ) {
    }

    fn on_mouse_up(
        &mut self,
        button: MouseButton,
        pos: Vector2<f32>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    ) {
    }

    fn on_mouse_down(
        &mut self,
        button: MouseButton,
        pos: Vector2<f32>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    ) {
    }

    fn on_mouse_wheel(&mut self, amount: f32, engine: &mut Engine, settings: &Settings) {}

    fn on_mouse_leave(&mut self, engine: &mut Engine, settings: &Settings) {}

    fn on_drag_over(
        &mut self,
        handle: Handle<UiNode>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    ) {
    }

    fn on_drop(
        &mut self,
        handle: Handle<UiNode>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
        editor_selection: &Selection,
    ) {
    }

    fn render_target(&self, engine: &Engine) -> Option<TextureResource> {
        Some(self.render_target.clone())
    }

    fn save(
        &mut self,
        path: &Path,
        settings: &Settings,
        engine: &mut Engine,
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

    fn undo(&mut self, selection: &mut Selection, engine: &mut Engine) {
        self.command_stack.undo(UiSceneContext {
            selection,
            message_sender: &self.message_sender,
        });
    }

    fn redo(&mut self, selection: &mut Selection, engine: &mut Engine) {
        self.command_stack.redo(UiSceneContext {
            selection,
            message_sender: &self.message_sender,
        });
    }

    fn clear_command_stack(&mut self, selection: &mut Selection, engine: &mut Engine) {
        self.command_stack.clear(UiSceneContext {
            selection,
            message_sender: &self.message_sender,
        });
    }

    fn on_before_render(&mut self, engine: &mut Engine) {
        Log::verify(
            engine
                .graphics_context
                .as_initialized_mut()
                .renderer
                .render_ui_to_texture(self.render_target.clone(), &mut self.ui, Color::DARK_GRAY),
        );
    }

    fn on_after_render(&mut self, engine: &mut Engine) {}

    fn update(
        &mut self,
        editor_selection: &Selection,
        engine: &mut Engine,
        dt: f32,
        path: Option<&Path>,
        settings: &mut Settings,
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
            }
        }

        while let Some(message) = self.ui.poll_message() {}

        new_render_target
    }

    fn is_interacting(&self) -> bool {
        false
    }

    fn on_destroy(&mut self, engine: &mut Engine) {}

    fn on_message(
        &mut self,
        message: &Message,
        selection: &Selection,
        engine: &mut Engine,
    ) -> bool {
        false
    }

    fn top_command_index(&self) -> Option<usize> {
        self.command_stack.top
    }

    fn command_names(&mut self, selection: &mut Selection, engine: &mut Engine) -> Vec<String> {
        self.command_stack
            .commands
            .iter_mut()
            .map(|c| {
                c.name(&UiSceneContext {
                    selection,
                    message_sender: &self.message_sender,
                })
            })
            .collect::<Vec<_>>()
    }

    fn first_selected_entity(
        &self,
        selection: &Selection,
        scenes: &SceneContainer,
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
}

pub struct UiSceneWrapper<'a> {
    pub ui: &'a UserInterface,
    pub path: Option<&'a Path>,
    pub selection: &'a Selection,
    pub sender: &'a MessageSender,
}

impl<'a> WorldViewerDataProvider for UiSceneWrapper<'a> {
    fn root_node(&self) -> ErasedHandle {
        self.ui.root().into()
    }

    fn path(&self) -> Option<&Path> {
        self.path
    }

    fn children_of(&self, node: ErasedHandle) -> Vec<ErasedHandle> {
        self.ui
            .try_get_node(node.into())
            .map(|n| n.children.iter().map(|c| (*c).into()).collect::<Vec<_>>())
            .unwrap_or_default()
    }

    fn child_count_of(&self, node: ErasedHandle) -> usize {
        self.ui
            .try_get_node(node.into())
            .map(|n| n.children.len())
            .unwrap_or_default()
    }

    fn is_node_has_child(&self, node: ErasedHandle, child: ErasedHandle) -> bool {
        self.ui
            .try_get_node(node.into())
            .map_or(false, |n| n.children().iter().any(|c| *c == child.into()))
    }

    fn parent_of(&self, node: ErasedHandle) -> ErasedHandle {
        self.ui
            .try_get_node(node.into())
            .map(|n| n.parent().into())
            .unwrap_or_default()
    }

    fn name_of(&self, node: ErasedHandle) -> Option<&str> {
        self.ui.try_get_node(node.into()).map(|n| n.name())
    }

    fn is_valid_handle(&self, node: ErasedHandle) -> bool {
        self.ui.try_get_node(node.into()).is_some()
    }

    fn icon_of(&self, node: ErasedHandle) -> Option<SharedTexture> {
        // TODO
        None
    }

    fn is_instance(&self, node: ErasedHandle) -> bool {
        false
    }

    fn selection(&self) -> Vec<ErasedHandle> {
        if let Selection::Ui(ref selection) = self.selection {
            selection
                .widgets
                .iter()
                .map(|h| ErasedHandle::from(*h))
                .collect::<Vec<_>>()
        } else {
            Default::default()
        }
    }

    fn on_drop(&self, child: ErasedHandle, parent: ErasedHandle) {}

    fn validate(&self) -> Vec<(ErasedHandle, Result<(), String>)> {
        Default::default()
    }

    fn on_selection_changed(&self, selection: &[ErasedHandle]) {
        let mut new_selection = Selection::None;
        for &selected_item in selection {
            match new_selection {
                Selection::None => {
                    new_selection =
                        Selection::Ui(UiSelection::single_or_empty(selected_item.into()));
                }
                Selection::Ui(ref mut selection) => {
                    selection.insert_or_exclude(selected_item.into())
                }
                _ => (),
            }
        }

        if &new_selection != self.selection {
            self.sender
                .do_ui_scene_command(ChangeUiSelectionCommand::new(
                    new_selection,
                    self.selection.clone(),
                ));
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UiSelection {
    pub widgets: Vec<Handle<UiNode>>,
}

impl UiSelection {
    /// Creates new selection as single if node handle is not none, and empty if it is.
    pub fn single_or_empty(node: Handle<UiNode>) -> Self {
        if node.is_none() {
            Self {
                widgets: Default::default(),
            }
        } else {
            Self {
                widgets: vec![node],
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.widgets.is_empty()
    }

    pub fn len(&self) -> usize {
        self.widgets.len()
    }

    pub fn insert_or_exclude(&mut self, handle: Handle<UiNode>) {
        if let Some(position) = self.widgets.iter().position(|&h| h == handle) {
            self.widgets.remove(position);
        } else {
            self.widgets.push(handle);
        }
    }
}

pub struct UiSelectInteractionMode {
    preview: Handle<UiNode>,
    selection_frame: Handle<UiNode>,
    message_sender: MessageSender,
    stack: Vec<Handle<UiNode>>,
    click_pos: Vector2<f32>,
}

impl UiSelectInteractionMode {
    pub fn new(
        preview: Handle<UiNode>,
        selection_frame: Handle<UiNode>,
        message_sender: MessageSender,
    ) -> Self {
        Self {
            preview,
            selection_frame,
            message_sender,
            stack: Vec::new(),
            click_pos: Vector2::default(),
        }
    }
}

impl TypeUuidProvider for UiSelectInteractionMode {
    fn type_uuid() -> Uuid {
        uuid!("12e550dc-0fb2-4a45-8060-fa363db3e197")
    }
}

impl InteractionMode for UiSelectInteractionMode {
    fn on_left_mouse_button_down(
        &mut self,
        _editor_selection: &Selection,
        _controller: &mut dyn SceneController,
        engine: &mut Engine,
        mouse_pos: Vector2<f32>,
        _frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        self.click_pos = mouse_pos;
        let ui = &mut engine.user_interface;
        ui.send_message(WidgetMessage::visibility(
            self.selection_frame,
            MessageDirection::ToWidget,
            true,
        ));
        ui.send_message(WidgetMessage::desired_position(
            self.selection_frame,
            MessageDirection::ToWidget,
            mouse_pos,
        ));
        ui.send_message(WidgetMessage::width(
            self.selection_frame,
            MessageDirection::ToWidget,
            0.0,
        ));
        ui.send_message(WidgetMessage::height(
            self.selection_frame,
            MessageDirection::ToWidget,
            0.0,
        ));
    }

    fn on_left_mouse_button_up(
        &mut self,
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        _mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        let Some(ui_scene) = controller.downcast_mut::<UiScene>() else {
            return;
        };

        let preview_screen_bounds = engine.user_interface.node(self.preview).screen_bounds();
        let frame_screen_bounds = engine
            .user_interface
            .node(self.selection_frame)
            .screen_bounds();
        let relative_bounds = frame_screen_bounds.translate(-preview_screen_bounds.position);
        self.stack.clear();
        self.stack.push(ui_scene.ui.root());
        let mut ui_selection = UiSelection::default();
        while let Some(handle) = self.stack.pop() {
            let node = ui_scene.ui.node(handle);
            if handle == ui_scene.ui.root() {
                self.stack.extend_from_slice(node.children());
                continue;
            }

            if relative_bounds.intersects(node.screen_bounds()) {
                ui_selection.insert_or_exclude(handle);
                break;
            }

            self.stack.extend_from_slice(node.children());
        }

        let new_selection = Selection::Ui(ui_selection);

        if &new_selection != editor_selection {
            self.message_sender
                .do_ui_scene_command(ChangeUiSelectionCommand::new(
                    new_selection,
                    editor_selection.clone(),
                ));
        }
        engine
            .user_interface
            .send_message(WidgetMessage::visibility(
                self.selection_frame,
                MessageDirection::ToWidget,
                false,
            ));
    }

    fn on_mouse_move(
        &mut self,
        _mouse_offset: Vector2<f32>,
        mouse_position: Vector2<f32>,
        _editor_selection: &Selection,
        _controller: &mut dyn SceneController,
        engine: &mut Engine,
        _frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        let ui = &mut engine.user_interface;
        let width = mouse_position.x - self.click_pos.x;
        let height = mouse_position.y - self.click_pos.y;

        let position = Vector2::new(
            if width < 0.0 {
                mouse_position.x
            } else {
                self.click_pos.x
            },
            if height < 0.0 {
                mouse_position.y
            } else {
                self.click_pos.y
            },
        );
        ui.send_message(WidgetMessage::desired_position(
            self.selection_frame,
            MessageDirection::ToWidget,
            position,
        ));
        ui.send_message(WidgetMessage::width(
            self.selection_frame,
            MessageDirection::ToWidget,
            width.abs(),
        ));
        ui.send_message(WidgetMessage::height(
            self.selection_frame,
            MessageDirection::ToWidget,
            height.abs(),
        ));
    }

    fn update(
        &mut self,
        _editor_selection: &Selection,
        _controller: &mut dyn SceneController,
        _engine: &mut Engine,
        _settings: &Settings,
    ) {
    }

    fn deactivate(&mut self, _controller: &dyn SceneController, _engine: &mut Engine) {}

    fn make_button(&mut self, ctx: &mut BuildContext, selected: bool) -> Handle<UiNode> {
        let select_mode_tooltip = "Select Object(s) - Shortcut: [1]\n\nSelection interaction mode \
        allows you to select an object by a single left mouse button click or multiple objects using either \
        frame selection (click and drag) or by holding Ctrl+Click";

        make_interaction_mode_button(
            ctx,
            include_bytes!("../../resources/select.png"),
            select_mode_tooltip,
            selected,
        )
    }

    fn uuid(&self) -> Uuid {
        Self::type_uuid()
    }
}

pub struct UiSceneContext<'a> {
    selection: &'a mut Selection,
    message_sender: &'a MessageSender,
}

#[derive(Debug)]
pub struct UiSceneCommand(pub Box<dyn UiCommand>);

impl Deref for UiSceneCommand {
    type Target = dyn UiCommand;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl DerefMut for UiSceneCommand {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

impl UiSceneCommand {
    pub fn new<C: UiCommand>(cmd: C) -> Self {
        Self(Box::new(cmd))
    }

    pub fn into_inner(self) -> Box<dyn UiCommand> {
        self.0
    }
}

define_command_stack!(UiCommand, UiCommandStack, UiSceneContext);

#[derive(Debug)]
pub struct ChangeUiSelectionCommand {
    new_selection: Selection,
    old_selection: Selection,
}

impl ChangeUiSelectionCommand {
    pub fn new(new_selection: Selection, old_selection: Selection) -> Self {
        Self {
            new_selection,
            old_selection,
        }
    }

    fn swap(&mut self) -> Selection {
        let selection = self.new_selection.clone();
        std::mem::swap(&mut self.new_selection, &mut self.old_selection);
        selection
    }

    fn exec(&mut self, context: &mut UiSceneContext) {
        let old_selection = self.old_selection.clone();
        let new_selection = self.swap();
        if &new_selection != context.selection {
            *context.selection = new_selection;
            context
                .message_sender
                .send(Message::SelectionChanged { old_selection });
        }
    }
}

impl UiCommand for ChangeUiSelectionCommand {
    fn name(&mut self, _context: &UiSceneContext) -> String {
        "Change Selection".to_string()
    }

    fn execute(&mut self, context: &mut UiSceneContext) {
        self.exec(context);
    }

    fn revert(&mut self, context: &mut UiSceneContext) {
        self.exec(context);
    }
}
