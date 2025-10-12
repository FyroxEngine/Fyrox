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

        ctx.send_message(WindowMessage::open_and_align(
            window,
            MessageDirection::ToWidget,
            anchor,
            HorizontalAlignment::Right,
            VerticalAlignment::Top,
            Thickness::uniform(2.0),
            false,
            false,
        ));

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
                    "FPS: {}\nFrame Time:{}\n{}\nMemory Usage:\n{}",
                    global_stats.frames_per_second,
                    global_stats.pure_frame_time,
                    stats.scene_data.statistics,
                    graphics_context.renderer.server.memory_usage()
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
