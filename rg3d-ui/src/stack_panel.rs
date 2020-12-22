use crate::core::algebra::Vector2;
use crate::message::MessageData;
use crate::{
    core::{math::Rect, pool::Handle, scope_profile},
    message::UiMessage,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, Orientation, UINode, UserInterface,
};
use std::ops::{Deref, DerefMut};

#[derive(Clone)]
pub struct StackPanel<M: MessageData, C: Control<M, C>> {
    widget: Widget<M, C>,
    orientation: Orientation,
}

crate::define_widget_deref!(StackPanel<M, C>);

impl<M: MessageData, C: Control<M, C>> StackPanel<M, C> {
    pub fn new(widget: Widget<M, C>) -> Self {
        Self {
            widget,
            orientation: Orientation::Vertical,
        }
    }

    pub fn set_orientation(&mut self, orientation: Orientation) {
        if self.orientation != orientation {
            self.orientation = orientation;
            self.widget.invalidate_layout();
        }
    }

    pub fn orientation(&self) -> Orientation {
        self.orientation
    }
}

impl<M: MessageData, C: Control<M, C>> Control<M, C> for StackPanel<M, C> {
    fn measure_override(
        &self,
        ui: &UserInterface<M, C>,
        available_size: Vector2<f32>,
    ) -> Vector2<f32> {
        scope_profile!();

        let mut child_constraint = Vector2::new(std::f32::INFINITY, std::f32::INFINITY);

        match self.orientation {
            Orientation::Vertical => {
                child_constraint.x = available_size.x;

                if !self.widget.width().is_nan() {
                    child_constraint.x = self.widget.width();
                }

                child_constraint.x = child_constraint
                    .x
                    .min(self.max_width())
                    .max(self.min_width());
            }
            Orientation::Horizontal => {
                child_constraint.y = available_size.y;

                if !self.widget.height().is_nan() {
                    child_constraint.y = self.widget.height();
                }

                child_constraint.y = child_constraint
                    .y
                    .min(self.max_height())
                    .max(self.min_height());
            }
        }

        let mut measured_size = Vector2::default();

        for child_handle in self.widget.children() {
            ui.node(*child_handle).measure(ui, child_constraint);

            let child = ui.node(*child_handle);
            let desired = child.desired_size();
            match self.orientation {
                Orientation::Vertical => {
                    if desired.x > measured_size.x {
                        measured_size.x = desired.x;
                    }
                    measured_size.y += desired.y;
                }
                Orientation::Horizontal => {
                    measured_size.x += desired.x;
                    if desired.y > measured_size.y {
                        measured_size.y = desired.y;
                    }
                }
            }
        }

        measured_size
    }

    fn arrange_override(&self, ui: &UserInterface<M, C>, final_size: Vector2<f32>) -> Vector2<f32> {
        scope_profile!();

        let mut width = final_size.x;
        let mut height = final_size.y;

        match self.orientation {
            Orientation::Vertical => height = 0.0,
            Orientation::Horizontal => width = 0.0,
        }

        for child_handle in self.widget.children() {
            let child = ui.node(*child_handle);
            match self.orientation {
                Orientation::Vertical => {
                    let child_bounds = Rect::new(
                        0.0,
                        height,
                        width.max(child.desired_size().x),
                        child.desired_size().y,
                    );
                    ui.node(*child_handle).arrange(ui, &child_bounds);
                    width = width.max(child.desired_size().x);
                    height += child.desired_size().y;
                }
                Orientation::Horizontal => {
                    let child_bounds = Rect::new(
                        width,
                        0.0,
                        child.desired_size().x,
                        height.max(child.desired_size().y),
                    );
                    ui.node(*child_handle).arrange(ui, &child_bounds);
                    width += child.desired_size().x;
                    height = height.max(child.desired_size().y);
                }
            }
        }

        match self.orientation {
            Orientation::Vertical => {
                height = height.max(final_size.y);
            }
            Orientation::Horizontal => {
                width = width.max(final_size.x);
            }
        }

        Vector2::new(width, height)
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);
    }
}

pub struct StackPanelBuilder<M: MessageData, C: Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    orientation: Option<Orientation>,
}

impl<M: MessageData, C: Control<M, C>> StackPanelBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            orientation: None,
        }
    }

    pub fn with_orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = Some(orientation);
        self
    }

    pub fn build(self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        let stack_panel = StackPanel {
            widget: self.widget_builder.build(),
            orientation: self.orientation.unwrap_or(Orientation::Vertical),
        };

        ctx.add_node(UINode::StackPanel(stack_panel))
    }
}
