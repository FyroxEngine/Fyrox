use crate::{
    UserInterface,
    widget::{
        Widget,
        WidgetBuilder,
    },
    UINode,
    scroll_bar::Orientation,
    Control,
    core::{
        math::{
            vec2::Vec2,
            Rect,
        },
        pool::Handle,
    },
    message::UiMessage
};

pub struct StackPanel<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    orientation: Orientation,
}

impl<M, C: 'static + Control<M, C>> StackPanel<M, C> {
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

impl<M, C: 'static + Control<M, C>> Control<M, C> for StackPanel<M, C> {
    fn widget(&self) -> &Widget<M, C> {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget<M, C> {
        &mut self.widget
    }

    fn raw_copy(&self) -> UINode<M, C> {
        UINode::StackPanel(Self {
            widget: self.widget.raw_copy(),
            orientation: self.orientation,
        })
    }

    fn measure_override(&self, ui: &UserInterface<M, C>, available_size: Vec2) -> Vec2 {
        let mut child_constraint = Vec2::new(std::f32::INFINITY, std::f32::INFINITY);

        match self.orientation {
            Orientation::Vertical => {
                child_constraint.x = available_size.x;

                if !self.widget.width().is_nan() {
                    child_constraint.x = self.widget.width();
                }

                if child_constraint.x < self.widget.min_size().x {
                    child_constraint.x = self.widget.min_size().x;
                }
                if child_constraint.x > self.widget.max_size().x {
                    child_constraint.x = self.widget.max_size().x;
                }
            }
            Orientation::Horizontal => {
                child_constraint.y = available_size.y;

                if !self.widget.height().is_nan() {
                    child_constraint.y = self.widget.height();
                }

                if child_constraint.y < self.widget.min_size().y {
                    child_constraint.y = self.widget.min_size().y;
                }
                if child_constraint.y > self.widget.max_size().y {
                    child_constraint.y = self.widget.max_size().y;
                }
            }
        }

        let mut measured_size = Vec2::ZERO;

        for child_handle in self.widget.children() {
            ui.node(*child_handle).measure(ui, child_constraint);

            let child = ui.node(*child_handle).widget();
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

    fn arrange_override(&self, ui: &UserInterface<M, C>, final_size: Vec2) -> Vec2 {
        let mut width = final_size.x;
        let mut height = final_size.y;

        match self.orientation {
            Orientation::Vertical => height = 0.0,
            Orientation::Horizontal => width = 0.0,
        }

        for child_handle in self.widget.children() {
            let child = ui.node(*child_handle).widget();
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

        Vec2::new(width, height)
    }

    fn handle_message(&mut self, self_handle: Handle<UINode<M, C>>, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_message(self_handle, ui, message);
    }
}

pub struct StackPanelBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    orientation: Option<Orientation>,
}

impl<M, C: 'static + Control<M, C>> StackPanelBuilder<M, C> {
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

    pub fn build(self, ui: &mut UserInterface<M, C>) -> Handle<UINode<M, C>> {
        let stack_panel = StackPanel {
            widget: self.widget_builder.build(),
            orientation: self.orientation.unwrap_or(Orientation::Vertical),
        };

        let handle = ui.add_node(UINode::StackPanel(stack_panel));

        ui.flush_messages();

        handle
    }
}