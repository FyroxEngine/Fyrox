use crate::fyrox::{
    core::{
        algebra::Point2, algebra::Vector2, pool::Handle, uuid::Uuid, uuid_provider,
        TypeUuidProvider,
    },
    engine::Engine,
    graph::BaseSceneGraph,
    gui::{BuildContext, UiNode},
};
use crate::{
    command::{Command, CommandGroup},
    interaction::{make_interaction_mode_button, InteractionMode},
    message::MessageSender,
    scene::{commands::ChangeSelectionCommand, controller::SceneController, Selection},
    settings::Settings,
    ui_scene::{commands::widget::MoveWidgetCommand, UiScene},
};

struct Entry {
    widget: Handle<UiNode>,
    initial_local_position: Vector2<f32>,
    new_local_position: Vector2<f32>,
    delta: Vector2<f32>,
}

struct MoveContext {
    entries: Vec<Entry>,
}

pub struct MoveWidgetsInteractionMode {
    move_context: Option<MoveContext>,
    sender: MessageSender,
}

impl MoveWidgetsInteractionMode {
    pub fn new(sender: MessageSender) -> Self {
        Self {
            move_context: None,
            sender,
        }
    }
}

uuid_provider!(MoveWidgetsInteractionMode = "e5c09b04-5c31-4044-ac48-5227ab4a4b83");

impl InteractionMode for MoveWidgetsInteractionMode {
    fn on_left_mouse_button_down(
        &mut self,
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        _engine: &mut Engine,
        mouse_position: Vector2<f32>,
        _frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        let Some(ui_scene) = controller.downcast_ref::<UiScene>() else {
            return;
        };

        if let Some(selection) = editor_selection.as_ui() {
            let mut in_bounds = false;
            let entries = selection
                .widgets
                .iter()
                .filter_map(|w| {
                    if let Some(widget_ref) = ui_scene.ui.try_get(*w) {
                        if !in_bounds && widget_ref.screen_bounds().contains(mouse_position) {
                            in_bounds = true;
                        }

                        Some(Entry {
                            widget: *w,
                            initial_local_position: widget_ref.desired_local_position(),
                            new_local_position: widget_ref.desired_local_position(),
                            delta: mouse_position - widget_ref.screen_position(),
                        })
                    } else {
                        None
                    }
                })
                .collect();

            if in_bounds {
                self.move_context = Some(MoveContext { entries });
            }
        }
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
        let Some(ui_scene) = controller.downcast_ref::<UiScene>() else {
            return;
        };

        if let Some(context) = self.move_context.take() {
            if context
                .entries
                .iter()
                .any(|e| e.new_local_position != e.initial_local_position)
            {
                let commands = context
                    .entries
                    .into_iter()
                    .map(|e| {
                        Command::new(MoveWidgetCommand::new(
                            e.widget,
                            e.initial_local_position,
                            e.new_local_position,
                        ))
                    })
                    .collect::<Vec<_>>();
                self.sender.do_command(CommandGroup::from(commands));
            }
        } else {
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
                self.sender
                    .do_command(ChangeSelectionCommand::new(Selection::new(new_selection)));
            }
        }
    }

    fn on_mouse_move(
        &mut self,
        _mouse_offset: Vector2<f32>,
        mouse_position: Vector2<f32>,
        _editor_selection: &Selection,
        controller: &mut dyn SceneController,
        _engine: &mut Engine,
        _frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        let Some(ui_scene) = controller.downcast_mut::<UiScene>() else {
            return;
        };

        if let Some(move_context) = self.move_context.as_mut() {
            for entry in move_context.entries.iter_mut() {
                let new_screen_space_position = mouse_position - entry.delta;
                let parent_inv_transform = ui_scene
                    .ui
                    .try_get(ui_scene.ui.node(entry.widget).parent)
                    .and_then(|w| w.visual_transform().try_inverse())
                    .unwrap_or_default();
                let new_local_position = parent_inv_transform.transform_point(&Point2::new(
                    new_screen_space_position.x,
                    new_screen_space_position.y,
                ));
                ui_scene
                    .ui
                    .node_mut(entry.widget)
                    .set_desired_local_position(new_local_position.coords);
                ui_scene.ui.invalidate_layout();
                entry.new_local_position = new_local_position.coords;
            }
        }
    }

    fn deactivate(&mut self, _controller: &dyn SceneController, _engine: &mut Engine) {}

    fn make_button(&mut self, ctx: &mut BuildContext, selected: bool) -> Handle<UiNode> {
        let move_mode_tooltip =
            "Move Object(s) - Shortcut: [2]\n\nMovement interaction mode allows you to move selected \
        objects. Keep in mind that movement always works in local coordinates!\n\n\
        This also allows you to select an object or add an object to current selection using Ctrl+Click";

        make_interaction_mode_button(
            ctx,
            include_bytes!("../../../resources/move_arrow.png"),
            move_mode_tooltip,
            selected,
        )
    }

    fn uuid(&self) -> Uuid {
        Self::type_uuid()
    }
}
