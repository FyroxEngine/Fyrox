use crate::{
    border::BorderBuilder,
    brush::Brush,
    canvas::CanvasBuilder,
    core::{algebra::Vector2, color::Color, pool::Handle},
    define_constructor,
    message::{MessageDirection, UiMessage},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, NodeHandleMapping, UiNode, UserInterface,
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone, PartialEq)]
pub enum ProgressBarMessage {
    Progress(f32),
}

impl ProgressBarMessage {
    define_constructor!(ProgressBarMessage:Progress => fn progress(f32), layout: false);
}

#[derive(Clone)]
pub struct ProgressBar {
    pub widget: Widget,
    pub progress: f32,
    pub indicator: Handle<UiNode>,
    pub body: Handle<UiNode>,
}

crate::define_widget_deref!(ProgressBar);

impl Control for ProgressBar {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        node_map.resolve(&mut self.indicator);
        node_map.resolve(&mut self.body);
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        let size = self.widget.arrange_override(ui, final_size);

        ui.send_message(WidgetMessage::width(
            self.indicator,
            MessageDirection::ToWidget,
            size.x * self.progress,
        ));

        ui.send_message(WidgetMessage::height(
            self.indicator,
            MessageDirection::ToWidget,
            size.y,
        ));

        size
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle {
            if let Some(&ProgressBarMessage::Progress(progress)) =
                message.data::<ProgressBarMessage>()
            {
                if progress != self.progress {
                    self.set_progress(progress);
                    self.invalidate_layout();
                }
            }
        }
    }
}

impl ProgressBar {
    pub fn set_progress(&mut self, progress: f32) {
        self.progress = progress.clamp(0.0, 1.0);
    }

    pub fn progress(&self) -> f32 {
        self.progress
    }
}

pub struct ProgressBarBuilder {
    widget_builder: WidgetBuilder,
    body: Option<Handle<UiNode>>,
    indicator: Option<Handle<UiNode>>,
    progress: f32,
}

impl ProgressBarBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            body: None,
            indicator: None,
            progress: 0.0,
        }
    }

    pub fn with_body(mut self, body: Handle<UiNode>) -> Self {
        self.body = Some(body);
        self
    }

    pub fn with_indicator(mut self, indicator: Handle<UiNode>) -> Self {
        self.indicator = Some(indicator);
        self
    }

    pub fn with_progress(mut self, progress: f32) -> Self {
        self.progress = progress.clamp(0.0, 1.0);
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
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

        ctx.add_node(UiNode::new(progress_bar))
    }
}
