use crate::fyrox::{
    core::pool::Handle,
    engine::{Engine, GraphicsContext},
    gui::{
        message::{MessageDirection, UiMessage},
        scroll_viewer::ScrollViewerBuilder,
        text::{TextBuilder, TextMessage},
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, HorizontalAlignment, Thickness, UiNode, UserInterface, VerticalAlignment,
    },
};
use fyrox::scene::Scene;

pub struct StatisticsWindow {
    pub window: Handle<UiNode>,
    text: Handle<UiNode>,
}

pub enum StatisticsWindowAction {
    None,
    Remove,
}

impl StatisticsWindow {
    pub fn new(ctx: &mut BuildContext, anchor: Handle<UiNode>) -> Self {
        let text;
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(215.0).with_height(300.0))
            .open(false)
            .with_content(
                ScrollViewerBuilder::new(WidgetBuilder::new())
                    .with_content({
                        text = TextBuilder::new(
                            WidgetBuilder::new().with_margin(Thickness::uniform(2.0)),
                        )
                        .build(ctx);
                        text
                    })
                    .build(ctx),
            )
            .with_title(WindowTitle::text("Rendering Statistics"))
            .build(ctx);

        ctx.sender()
            .send(WindowMessage::open_and_align(
                window,
                MessageDirection::ToWidget,
                anchor,
                HorizontalAlignment::Right,
                VerticalAlignment::Top,
                Thickness::uniform(2.0),
                false,
                false,
            ))
            .unwrap();

        Self { window, text }
    }

    pub fn handle_ui_message(
        &self,
        message: &UiMessage,
        ui: &UserInterface,
    ) -> StatisticsWindowAction {
        if let Some(WindowMessage::Close) = message.data() {
            if message.destination() == self.window {
                ui.send_message(WidgetMessage::remove(
                    self.window,
                    MessageDirection::ToWidget,
                ));

                return StatisticsWindowAction::Remove;
            }
        }
        StatisticsWindowAction::None
    }

    pub fn update(&self, current_scene: Handle<Scene>, engine: &Engine) {
        if let GraphicsContext::Initialized(ref graphics_context) = engine.graphics_context {
            if let Some(stats) = graphics_context.renderer.scene_data_map.get(&current_scene) {
                let global_stats = graphics_context.renderer.get_statistics();
                let statistics = format!(
                    "FPS: {}\nFrame Time:{}\n{}",
                    global_stats.frames_per_second, global_stats.pure_frame_time, stats.statistics
                );
                engine
                    .user_interfaces
                    .first()
                    .send_message(TextMessage::text(
                        self.text,
                        MessageDirection::ToWidget,
                        statistics,
                    ));
            }
        }
    }
}
