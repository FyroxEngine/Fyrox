use crate::fyrox::graph::BaseSceneGraph;
use crate::fyrox::{
    core::{
        algebra::Vector2,
        pool::Handle,
        uuid::{uuid, Uuid},
        TypeUuidProvider,
    },
    engine::Engine,
    gui::{message::MessageDirection, widget::WidgetMessage, BuildContext, UiNode},
};
use crate::scene::commands::ChangeSelectionCommand;
use crate::{
    interaction::{make_interaction_mode_button, InteractionMode},
    message::MessageSender,
    scene::{controller::SceneController, Selection},
    settings::Settings,
    ui_scene::{UiScene, UiSelection},
};

pub mod move_mode;

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
        let ui = &mut engine.user_interfaces.first_mut();
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
        let frame_screen_bounds = engine
            .user_interfaces
            .first_mut()
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

        let new_selection = Selection::new(ui_selection);

        if &new_selection != editor_selection {
            self.message_sender
                .do_command(ChangeSelectionCommand::new(new_selection));
        }
        engine
            .user_interfaces
            .first_mut()
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
            include_bytes!("../../../resources/select.png"),
            select_mode_tooltip,
            selected,
        )
    }

    fn uuid(&self) -> Uuid {
        Self::type_uuid()
    }
}
