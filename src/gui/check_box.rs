use crate::gui::{
    widget::{Widget, AsWidget, WidgetBuilder},
    Draw,
    Layout,
    UserInterface,
    Update,
    draw::DrawingContext,
    node::UINode,
    event::{UIEvent, UIEventKind},
    Thickness,
    Visibility,
    bool_to_visibility,
    Styleable,
    border::BorderBuilder,
};
use crate::core::{
    math::vec2::Vec2,
    pool::Handle,
    color::Color
};
use std::any::Any;

pub struct CheckBox {
    widget: Widget,
    checked: Option<bool>,
}

impl AsWidget for CheckBox {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget {
        &mut self.widget
    }
}

impl Draw for CheckBox {
    fn draw(&mut self, drawing_context: &mut DrawingContext) {
        self.widget.draw(drawing_context)
    }
}

impl Layout for CheckBox {
    fn measure_override(&self, ui: &UserInterface, available_size: Vec2) -> Vec2 {
        self.widget.measure_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vec2) -> Vec2 {
        self.widget.arrange_override(ui, final_size)
    }
}

impl Update for CheckBox {
    fn update(&mut self, dt: f32) {
        self.widget.update(dt)
    }
}

impl CheckBox {
    /// Sets new state of check box, three are three possible states:
    /// 1) None - undefined
    /// 2) Some(true) - checked
    /// 3) Some(false) - unchecked
    pub fn set_checked(&mut self, value: Option<bool>) {
        self.checked = value;
        self.widget.events.borrow_mut().push_back(UIEvent::new(UIEventKind::Checked(value)));
    }
}

pub struct CheckBoxBuilder {
    widget_builder: WidgetBuilder,
    checked: Option<bool>,
}

impl CheckBoxBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            checked: Some(false),
        }
    }

    pub fn checked(mut self, value: Option<bool>) -> Self {
        self.checked = value;
        self
    }

    pub fn build(self, ui: &mut UserInterface) -> Handle<UINode> {
        let check_mark_color = Color::opaque(200, 200, 200);
        let check_mark;
        let check_box = UINode::CheckBox(CheckBox {
            widget: self.widget_builder
                .with_child(BorderBuilder::new(WidgetBuilder::new()
                    .with_color(Color::opaque(60, 60, 60))
                    .with_child({
                        let visibility = if let Some(value) = self.checked {
                            bool_to_visibility(value)
                        } else {
                            Visibility::Visible
                        };
                        check_mark = BorderBuilder::new(WidgetBuilder::new()
                            .with_visibility(visibility)
                            .with_color(check_mark_color)
                            .with_margin(Thickness::uniform(1.0)))
                            .with_stroke_thickness(Thickness::uniform(0.0))
                            .build(ui);
                        check_mark
                    }))
                    .with_stroke_thickness(Thickness::uniform(1.0))
                    .with_stroke_color(Color::WHITE)
                    .build(ui))
                .with_event_handler(Box::new(move |ui, handle, evt| {
                    match evt.kind {
                        UIEventKind::MouseDown { .. } => {
                            if evt.source == handle || ui.is_node_child_of(evt.source, handle) {
                                ui.capture_mouse(handle);
                            }
                        }
                        UIEventKind::MouseUp { .. } => {
                            if evt.source == handle || ui.is_node_child_of(evt.source, handle) {
                                ui.release_mouse_capture();

                                let check_box = ui.get_node_mut(handle).as_check_box_mut();
                                if let Some(value) = check_box.checked {
                                    // Invert state if it is defined.
                                    check_box.set_checked(Some(!value));
                                } else {
                                    // Switch from undefined state to checked.
                                    check_box.set_checked(Some(true));
                                }
                            }
                        }
                        UIEventKind::Checked(value) if evt.source == handle => {
                            let check_mark = ui.get_node_mut(check_mark).widget_mut();
                            match value {
                                None => {
                                    check_mark.set_color(Color::opaque(30, 30, 80))
                                }
                                Some(value) => {
                                    check_mark.set_color(check_mark_color);
                                    let visibility = if value { Visibility::Visible } else { Visibility::Collapsed };
                                    check_mark.set_visibility(visibility);
                                }
                            }
                        }
                        _ => {}
                    }
                }))
                .build(),
            checked: self.checked,
        });

        ui.add_node(check_box)
    }
}

impl Styleable for CheckBox {
    fn set_property(&mut self, _name: &str, _value: &dyn Any) {
        // TODO
    }

    fn get_property(&self, _name: &str) -> Option<&dyn Any> {
        None
    }
}