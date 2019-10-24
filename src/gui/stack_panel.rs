use crate::gui::{
    Draw,
    Layout,
    UserInterface,
    maxf,
    widget::{Widget, AsWidget, WidgetBuilder},
    draw::DrawingContext,
    node::UINode,
    scroll_bar::Orientation,
    Update
};
use rg3d_core::{
    math::{
        vec2::Vec2,
        Rect,
    },
    pool::Handle,
};

pub struct StackPanel {
    widget: Widget,
    orientation: Orientation,
}

impl Draw for StackPanel {
    fn draw(&mut self, drawing_context: &mut DrawingContext) {
        self.widget.draw(drawing_context)
    }
}

impl Update for StackPanel {
    fn update(&mut self, dt: f32) {
        self.widget.update(dt)
    }
}

impl Layout for StackPanel {
    fn measure_override(&self, ui: &UserInterface, available_size: Vec2) -> Vec2 {
        let mut child_constraint = Vec2::new(std::f32::INFINITY, std::f32::INFINITY);

        match self.orientation {
            Orientation::Vertical => {
                child_constraint.x = available_size.x;

                if !self.widget.width.get().is_nan() {
                    child_constraint.x = self.widget.width.get();
                }

                if child_constraint.x < self.widget.min_size.x {
                    child_constraint.x = self.widget.min_size.x;
                }
                if child_constraint.x > self.widget.max_size.x {
                    child_constraint.x = self.widget.max_size.x;
                }
            }
            Orientation::Horizontal => {
                child_constraint.y = available_size.y;

                if !self.widget.height.get().is_nan() {
                    child_constraint.y = self.widget.height.get();
                }

                if child_constraint.y < self.widget.min_size.y {
                    child_constraint.y = self.widget.min_size.y;
                }
                if child_constraint.y > self.widget.max_size.y {
                    child_constraint.y = self.widget.max_size.y;
                }
            }
        }

        let mut measured_size = Vec2::ZERO;

        for child_handle in self.widget.children.iter() {
            ui.measure(*child_handle, child_constraint);

            let child = ui.get_node(*child_handle).widget();
            let desired = child.desired_size.get();
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

    fn arrange_override(&self, ui: &UserInterface, final_size: Vec2) -> Vec2 {
        let mut width = final_size.x;
        let mut height = final_size.y;

        match self.orientation {
            Orientation::Vertical => height = 0.0,
            Orientation::Horizontal => width = 0.0,
        }

        for child_handle in self.widget.children.iter() {
            let child = ui.get_node(*child_handle).widget();
            match self.orientation {
                Orientation::Vertical => {
                    let child_bounds = Rect::new(
                        0.0,
                        height,
                        maxf(width, child.desired_size.get().x),
                        child.desired_size.get().y,
                    );
                    ui.arrange(*child_handle, &child_bounds);
                    width = maxf(width, child.desired_size.get().x);
                    height += child.desired_size.get().y;
                }
                Orientation::Horizontal => {
                    let child_bounds = Rect::new(
                        width,
                        0.0,
                        child.desired_size.get().x,
                        maxf(height, child.desired_size.get().y),
                    );
                    ui.arrange(*child_handle, &child_bounds);
                    width += child.desired_size.get().x;
                    height = maxf(height, child.desired_size.get().y);
                }
            }
        }

        match self.orientation {
            Orientation::Vertical => {
                height = maxf(height, final_size.y);
            }
            Orientation::Horizontal => {
                width = maxf(width, final_size.x);
            }
        }

        Vec2::new(width, height)
    }
}

impl AsWidget for StackPanel {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget {
        &mut self.widget
    }
}

pub struct StackPanelBuilder {
    widget_builder: WidgetBuilder,
    orientation: Option<Orientation>,
}

impl StackPanelBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            orientation: None,
        }
    }

    pub fn with_orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = Some(orientation);
        self
    }

    pub fn build(self, ui: &mut UserInterface) -> Handle<UINode> {
        let stack_panel = UINode::StackPanel(StackPanel {
            widget: self.widget_builder.build(),
            orientation: self.orientation.unwrap_or(Orientation::Vertical),
        });

        ui.add_node(stack_panel)
    }
}