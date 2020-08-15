use crate::{
    border::BorderBuilder,
    brush::Brush,
    canvas::CanvasBuilder,
    core::{color::Color, math::vec2::Vec2, pool::Handle},
    message::{ProgressBarMessage, UiMessage, UiMessageData, WidgetMessage},
    node::UINode,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UserInterface,
};
use std::ops::{Deref, DerefMut};

pub struct ProgressBar<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    progress: f32,
    indicator: Handle<UINode<M, C>>,
    body: Handle<UINode<M, C>>,
}

impl<M: 'static, C: 'static + Control<M, C>> Deref for ProgressBar<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> DerefMut for ProgressBar<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> Control<M, C> for ProgressBar<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        UINode::ProgressBar(Self {
            widget: self.widget.raw_copy(),
            progress: self.progress,
            indicator: self.indicator,
            body: self.body,
        })
    }

    fn arrange_override(&self, ui: &UserInterface<M, C>, final_size: Vec2) -> Vec2 {
        let size = self.widget.arrange_override(ui, final_size);

        ui.send_message(UiMessage {
            destination: self.indicator,
            data: UiMessageData::Widget(WidgetMessage::Width(size.x * self.progress)),
            handled: false,
        });

        ui.send_message(UiMessage {
            destination: self.indicator,
            data: UiMessageData::Widget(WidgetMessage::Height(size.y)),
            handled: false,
        });

        size
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);

        if message.destination == self.handle {
            if let UiMessageData::ProgressBar(msg) = &message.data {
                match *msg {
                    ProgressBarMessage::Progress(progress) => {
                        if progress != self.progress {
                            self.set_progress(progress);
                            self.invalidate_layout();
                        }
                    }
                }
            }
        }
    }
}

impl<M: 'static, C: 'static + Control<M, C>> ProgressBar<M, C> {
    pub fn set_progress(&mut self, progress: f32) {
        self.progress = progress.min(1.0).max(0.0);
    }

    pub fn progress(&self) -> f32 {
        self.progress
    }
}

pub struct ProgressBarBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    body: Option<Handle<UINode<M, C>>>,
    indicator: Option<Handle<UINode<M, C>>>,
    progress: f32,
}

impl<M: 'static, C: 'static + Control<M, C>> ProgressBarBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            body: None,
            indicator: None,
            progress: 0.0,
        }
    }

    pub fn with_body(mut self, body: Handle<UINode<M, C>>) -> Self {
        self.body = Some(body);
        self
    }

    pub fn with_indicator(mut self, indicator: Handle<UINode<M, C>>) -> Self {
        self.indicator = Some(indicator);
        self
    }

    pub fn with_progress(mut self, progress: f32) -> Self {
        self.progress = progress.min(1.0).max(0.0);
        self
    }

    pub fn build(self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        let body = self
            .body
            .unwrap_or_else(|| BorderBuilder::new(WidgetBuilder::new()).build(ctx));

        let indicator = self.indicator.unwrap_or_else(|| {
            BorderBuilder::new(
                WidgetBuilder::new().with_background(Brush::Solid(Color::opaque(180, 180, 180))),
            )
            .build(ctx)
        });

        let canvas = CanvasBuilder::new(WidgetBuilder::new().with_child(indicator)).build(ctx);

        ctx.link(canvas, body);

        let progress_bar = ProgressBar {
            widget: self.widget_builder.with_child(body).build(),
            progress: self.progress,
            indicator,
            body,
        };

        ctx.add_node(UINode::ProgressBar(progress_bar))
    }
}
