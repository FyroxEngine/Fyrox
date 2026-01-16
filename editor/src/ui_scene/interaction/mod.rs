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
    fyrox::graph::SceneGraph,
    fyrox::{
        core::{
            algebra::Vector2,
            pool::Handle,
            uuid::{uuid, Uuid},
            TypeUuidProvider,
        },
        engine::Engine,
        gui::{widget::WidgetMessage, BuildContext, UiNode},
    },
    interaction::{make_interaction_mode_button, InteractionMode},
    message::MessageSender,
    scene::commands::ChangeSelectionCommand,
    scene::{controller::SceneController, Selection},
    settings::Settings,
    ui_scene::{UiScene, UiSelection},
};
use fyrox::gui::border::Border;
use fyrox::gui::button::Button;

pub mod move_mode;

pub struct UiSelectInteractionMode {
    preview: Handle<UiNode>,
    selection_frame: Handle<Border>,
    message_sender: MessageSender,
    stack: Vec<Handle<UiNode>>,
    click_pos: Vector2<f32>,
}

impl UiSelectInteractionMode {
    pub fn new(
        preview: Handle<UiNode>,
        selection_frame: Handle<Border>,
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
        let ui = &mut engine.user_interfaces.first_mut();
        ui.send(self.selection_frame, WidgetMessage::Visibility(true));
        ui.send(
            self.selection_frame,
            WidgetMessage::DesiredPosition(mouse_pos),
        );
        ui.send(self.selection_frame, WidgetMessage::Width(0.0));
        ui.send(self.selection_frame, WidgetMessage::Height(0.0));
    }

    fn on_left_mouse_button_up(
        &mut self,
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        mouse_pos: Vector2<f32>,
        _frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        let Some(ui_scene) = controller.downcast_mut::<UiScene>() else {
            return;
        };

        let preview_screen_bounds = engine
            .user_interfaces
            .first_mut()
            .node(self.preview)
            .screen_bounds();
        let frame_screen_bounds =
            engine.user_interfaces.first_mut()[self.selection_frame].screen_bounds();

        let relative_bounds = frame_screen_bounds.translate(-preview_screen_bounds.position);

        // Small selection box is considered as a click that does single selection.
        if relative_bounds.size.x < 2.0 && relative_bounds.size.y < 2.0 {
            let picked = ui_scene.ui.hit_test(mouse_pos);
            if picked.is_some() {
                let mut new_selection = if let (Some(current), true) = (
                    editor_selection.as_ui(),
                    engine
                        .user_interfaces
                        .first_mut()
                        .keyboard_modifiers()
                        .control,
                ) {
                    current.clone()
                } else {
                    Default::default()
                };
                new_selection.insert_or_exclude(picked);
                self.message_sender
                    .do_command(ChangeSelectionCommand::new(Selection::new(new_selection)));
            }
        } else {
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
                }

                self.stack.extend_from_slice(node.children());
            }

            let new_selection = Selection::new(ui_selection);

            if &new_selection != editor_selection {
                self.message_sender
                    .do_command(ChangeSelectionCommand::new(new_selection));
            }
        }
        engine
            .user_interfaces
            .first()
            .send(self.selection_frame, WidgetMessage::Visibility(false));
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
        let ui = &mut engine.user_interfaces.first_mut();
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
        ui.send(
            self.selection_frame,
            WidgetMessage::DesiredPosition(position),
        );
        ui.send(self.selection_frame, WidgetMessage::Width(width.abs()));
        ui.send(self.selection_frame, WidgetMessage::Height(height.abs()));
    }

    fn update(
        &mut self,
        _editor_selection: &Selection,
        _controller: &mut dyn SceneController,
        _engine: &mut Engine,
        _settings: &Settings,
    ) {
    }

    fn make_button(&mut self, ctx: &mut BuildContext, selected: bool) -> Handle<Button> {
        let select_mode_tooltip = "Select Object(s) - Shortcut: [1]\n\nSelection interaction mode \
        allows you to select an object by a single left mouse button click or multiple objects using either \
        frame selection (click and drag) or by holding Ctrl+Click";

        make_interaction_mode_button(
            ctx,
            include_bytes!("../../../resources/select.png"),
            select_mode_tooltip,
            selected,
        )
    }

    fn uuid(&self) -> Uuid {
        Self::type_uuid()
    }
}
