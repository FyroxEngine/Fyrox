use crate::gui::{
    widget::{Widget, WidgetBuilder},
    UserInterface,
    UINode,
    event::{UIEvent, UIEventKind},
    Thickness,
    Visibility,
    bool_to_visibility,
    border::BorderBuilder,
    Control
};
use crate::core::{
    pool::Handle,
    color::Color
};

pub struct CheckBox {
    widget: Widget,
    checked: Option<bool>,
    check_mark: Handle<UINode>
}

impl Control for CheckBox {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget {
        &mut self.widget
    }

    fn handle_event(&mut self, self_handle: Handle<UINode>, ui: &mut UserInterface, evt: &mut UIEvent) {
        let check_mark_color = Color::opaque(200, 200, 200);

        match evt.kind {
            UIEventKind::MouseDown { .. } => {
                if evt.source == self_handle || self.widget.has_descendant(evt.source, ui) {
                    ui.capture_mouse(self_handle);
                }
            }
            UIEventKind::MouseUp { .. } => {
                if evt.source == self_handle || self.widget.has_descendant(evt.source, ui) {
                    ui.release_mouse_capture();

                    if let Some(value) = self.checked {
                        // Invert state if it is defined.
                        self.set_checked(Some(!value));
                    } else {
                        // Switch from undefined state to checked.
                        self.set_checked(Some(true));
                    }
                }
            }
            UIEventKind::Checked(value) if evt.source == self_handle => {
                let check_mark = ui.get_node_mut(self.check_mark).widget_mut();
                match value {
                    None => {
                        check_mark.set_background(Color::opaque(30, 30, 80))
                    }
                    Some(value) => {
                        check_mark.set_background(check_mark_color);
                        let visibility = if value { Visibility::Visible } else { Visibility::Collapsed };
                        check_mark.set_visibility(visibility);
                    }
                }
            }
            _ => {}
        }
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

        let check_mark = {
            let visibility = if let Some(value) = self.checked {
                bool_to_visibility(value)
            } else {
                Visibility::Visible
            };
            let check_mark = BorderBuilder::new(WidgetBuilder::new()
                .with_visibility(visibility)
                .with_background(check_mark_color)
                .with_margin(Thickness::uniform(1.0)))
                .with_stroke_thickness(Thickness::uniform(0.0))
                .build(ui);
            check_mark
        };

        let check_box = CheckBox {
            widget: self.widget_builder
                .with_child(BorderBuilder::new(WidgetBuilder::new()
                    .with_background(Color::opaque(60, 60, 60))
                    .with_foreground(Color::WHITE)
                    .with_child(check_mark))
                    .with_stroke_thickness(Thickness::uniform(1.0))
                    .build(ui))
                .build(),
            checked: self.checked,
            check_mark
        };

        ui.add_node(check_box)
    }
}
