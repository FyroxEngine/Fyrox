use crate::fyrox::core::uuid::{uuid, Uuid};
use crate::fyrox::core::TypeUuidProvider;
use crate::fyrox::graph::BaseSceneGraph;
use crate::fyrox::gui::BuildContext;
use crate::fyrox::{
    core::{algebra::Vector2, pool::Handle},
    gui::{message::MessageDirection, widget::WidgetMessage, UiNode},
    scene::node::Node,
};
use crate::interaction::make_interaction_mode_button;
use crate::scene::controller::SceneController;
use crate::{
    interaction::InteractionMode,
    message::MessageSender,
    scene::{commands::ChangeSelectionCommand, GameScene, Selection},
    settings::Settings,
    world::graph::selection::GraphSelection,
    Engine,
};

pub struct SelectInteractionMode {
    preview: Handle<UiNode>,
    selection_frame: Handle<UiNode>,
    message_sender: MessageSender,
    stack: Vec<Handle<Node>>,
    click_pos: Vector2<f32>,
}

impl SelectInteractionMode {
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

impl TypeUuidProvider for SelectInteractionMode {
    fn type_uuid() -> Uuid {
        uuid!("bab9ce8c-d679-4c49-beb9-f5a8482e0678")
    }
}

impl InteractionMode for SelectInteractionMode {
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
        frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        let Some(game_scene) = controller.downcast_mut::<GameScene>() else {
            return;
        };

        let scene = &engine.scenes[game_scene.scene];
        let camera = scene.graph[game_scene.camera_controller.camera].as_camera();
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
        self.stack.push(scene.graph.get_root());
        let mut graph_selection = GraphSelection::default();
        while let Some(handle) = self.stack.pop() {
            let node = &scene.graph[handle];
            if handle == game_scene.editor_objects_root {
                continue;
            }
            if handle == scene.graph.get_root() {
                self.stack.extend_from_slice(node.children());
                continue;
            }

            for screen_corner in node
                .local_bounding_box()
                .corners()
                .iter()
                .filter_map(|&p| camera.project(p + node.global_position(), frame_size))
            {
                if relative_bounds.contains(screen_corner) {
                    graph_selection.insert_or_exclude(handle);
                    break;
                }
            }

            self.stack.extend_from_slice(node.children());
        }

        let new_selection = Selection::new(graph_selection);

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
            include_bytes!("../../resources/select.png"),
            select_mode_tooltip,
            selected,
        )
    }

    fn uuid(&self) -> Uuid {
        Self::type_uuid()
    }
}
