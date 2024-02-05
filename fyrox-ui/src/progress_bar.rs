//! Progress bar is used to show a bar that fills in from left to right according to the progress value. It is used to
//! show progress for long actions. See [`ProgressBar`] widget docs for more info and usage examples.

#![warn(missing_docs)]

use crate::{
    border::BorderBuilder,
    brush::Brush,
    canvas::CanvasBuilder,
    core::{
        algebra::Vector2, color::Color, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        visitor::prelude::*,
    },
    define_constructor,
    message::{MessageDirection, UiMessage},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, UiNode, UserInterface,
};
use fyrox_core::uuid_provider;
use fyrox_core::variable::InheritableVariable;
use std::ops::{Deref, DerefMut};

/// A set of messages that can be used to modify the state of a progress bar.
#[derive(Debug, Clone, PartialEq)]
pub enum ProgressBarMessage {
    /// A message, that is used to set progress of the progress bar.
    Progress(f32),
}

impl ProgressBarMessage {
    define_constructor!(
        /// Creates [`ProgressBarMessage::Progress`].
        ProgressBarMessage:Progress => fn progress(f32), layout: false
    );
}

/// Progress bar is used to show a bar that fills in from left to right according to the progress value. It is used to
/// show progress for long actions.
///
/// ## Example
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle, progress_bar::ProgressBarBuilder, widget::WidgetBuilder, BuildContext,
/// #     UiNode,
/// # };
/// fn create_progress_bar(ctx: &mut BuildContext) -> Handle<UiNode> {
///     ProgressBarBuilder::new(WidgetBuilder::new())
///         // Keep mind, that the progress is "normalized", which means that it is defined on
///         // [0..1] range, where 0 - no progress at all, 1 - maximum progress.
///         .with_progress(0.25)
///         .build(ctx)
/// }
/// ```
///
/// ## Changing progress
///
/// To change progress of a progress bar all you need is to send [`ProgressBarMessage::Progress`] to it:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle, message::MessageDirection, progress_bar::ProgressBarMessage, UiNode,
/// #     UserInterface,
/// # };
/// fn change_progress(progress_bar: Handle<UiNode>, ui: &UserInterface) {
///     ui.send_message(ProgressBarMessage::progress(
///         progress_bar,
///         MessageDirection::ToWidget,
///         0.33,
///     ));
/// }
/// ```
#[derive(Default, Clone, Debug, Visit, Reflect, ComponentProvider)]
pub struct ProgressBar {
    /// Base widget of the progress bar.
    pub widget: Widget,
    /// Current progress of the progress bar.
    pub progress: InheritableVariable<f32>,
    /// Handle of a widget that is used to show the progress.
    pub indicator: InheritableVariable<Handle<UiNode>>,
    /// Container widget of the bar of the progress bar.
    pub body: InheritableVariable<Handle<UiNode>>,
}

crate::define_widget_deref!(ProgressBar);

uuid_provider!(ProgressBar = "d6ebb853-d945-46bc-86db-4c8b5d5faf8e");

impl Control for ProgressBar {
    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        let size = self.widget.arrange_override(ui, final_size);

        ui.send_message(WidgetMessage::width(
            *self.indicator,
            MessageDirection::ToWidget,
            size.x * *self.progress,
        ));

        ui.send_message(WidgetMessage::height(
            *self.indicator,
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
                if progress != *self.progress {
                    self.set_progress(progress);
                    self.invalidate_layout();
                }
            }
        }
    }
}

impl ProgressBar {
    fn set_progress(&mut self, progress: f32) {
        self.progress
            .set_value_and_mark_modified(progress.clamp(0.0, 1.0));
    }
}

/// Progress bar builder creates progress bar instances and adds them to the UI.
pub struct ProgressBarBuilder {
    widget_builder: WidgetBuilder,
    body: Option<Handle<UiNode>>,
    indicator: Option<Handle<UiNode>>,
    progress: f32,
}

impl ProgressBarBuilder {
    /// Creates new builder instance.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            body: None,
            indicator: None,
            progress: 0.0,
        }
    }

    /// Sets the desired body of the progress bar, which is used to wrap the indicator (bar).
    pub fn with_body(mut self, body: Handle<UiNode>) -> Self {
        self.body = Some(body);
        self
    }

    /// Sets the desired indicator widget, that will be used to show the progress.
    pub fn with_indicator(mut self, indicator: Handle<UiNode>) -> Self {
        self.indicator = Some(indicator);
        self
    }

    /// Sets the desired progress value. Input value will be clamped to `[0..1]` range.
    pub fn with_progress(mut self, progress: f32) -> Self {
        self.progress = progress.clamp(0.0, 1.0);
        self
    }

    /// Finishes progress bar creation and adds the new instance to the user interface.
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
            progress: self.progress.into(),
            indicator: indicator.into(),
            body: body.into(),
        };

        ctx.add_node(UiNode::new(progress_bar))
    }
}
