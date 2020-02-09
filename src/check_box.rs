use crate::{
    widget::{
        Widget,
        WidgetBuilder,
    },
    UserInterface,
    UINode,
    event::{
        UIEvent,
        UIEventKind,
    },
    Thickness,
    Visibility,
    bool_to_visibility,
    border::BorderBuilder,
    Control,
    ControlTemplate,
    UINodeContainer,
    Builder,
    core::{
        pool::Handle,
        color::Color,
    },
    brush::Brush
};
use std::collections::HashMap;

pub struct CheckBox {
    widget: Widget,
    checked: Option<bool>,
    check_mark: Handle<UINode>,
}

impl Control for CheckBox {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget {
        &mut self.widget
    }

    fn raw_copy(&self) -> Box<dyn Control> {
        Box::new(Self {
            widget: *self.widget.raw_copy().downcast::<Widget>().unwrap_or_else(|_| panic!()),
            checked: self.checked,
            check_mark: self.check_mark,
        })
    }

    fn resolve(&mut self, _: &ControlTemplate, node_map: &HashMap<Handle<UINode>, Handle<UINode>>) {
        self.check_mark = *node_map.get(&self.check_mark).unwrap();
    }

    fn handle_event(&mut self, self_handle: Handle<UINode>, ui: &mut UserInterface, evt: &mut UIEvent) {
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
                let check_mark = ui.node_mut(self.check_mark).widget_mut();
                match value {
                    None => {
                        check_mark.set_background(Brush::Solid(Color::opaque(30, 30, 80)));
                    }
                    Some(value) => {
                        check_mark.set_background(Brush::Solid(Color::opaque(200, 200, 200)));
                        check_mark.set_visibility(
                            if value {
                                Visibility::Visible
                            } else {
                                Visibility::Collapsed
                            });
                    }
                }
            }
            _ => {}
        }
    }
}

impl CheckBox {
    pub fn new(widget: Widget, check_mark: Handle<UINode>) -> Self {
        Self {
            widget,
            check_mark,
            checked: None,
        }
    }

    /// Sets new state of check box, three are three possible states:
    /// 1) None - undefined
    /// 2) Some(true) - checked
    /// 3) Some(false) - unchecked
    pub fn set_checked(&mut self, value: Option<bool>) -> &mut Self {
        self.checked = value;
        self.widget.events.borrow_mut().push_back(UIEvent::new(UIEventKind::Checked(value)));
        self
    }
}

pub struct CheckBoxBuilder {
    widget_builder: WidgetBuilder,
    checked: Option<bool>,
    check_mark: Option<Handle<UINode>>,
}

impl CheckBoxBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            checked: Some(false),
            check_mark: None,
        }
    }

    pub fn checked(mut self, value: Option<bool>) -> Self {
        self.checked = value;
        self
    }

    pub fn with_check_mark(mut self, check_mark: Handle<UINode>) -> Self {
        self.check_mark = Some(check_mark);
        self
    }
}

impl Builder for CheckBoxBuilder {
    fn build(self, container: &mut dyn UINodeContainer) -> Handle<UINode> {
        let check_mark = self.check_mark.unwrap_or_else(|| {
            BorderBuilder::new(WidgetBuilder::new()
                .with_background(Brush::Solid(Color::opaque(200, 200, 200)))
                .with_margin(Thickness::uniform(1.0)))
                .with_stroke_thickness(Thickness::uniform(0.0))
                .build(container)
        });

        let visibility = if let Some(value) = self.checked {
            bool_to_visibility(value)
        } else {
            Visibility::Visible
        };
        container.node_mut(check_mark)
            .widget_mut()
            .set_visibility(visibility);

        let check_box = CheckBox {
            widget: self.widget_builder
                .with_child(BorderBuilder::new(WidgetBuilder::new()
                    .with_background(Brush::Solid(Color::opaque(60, 60, 60)))
                    .with_foreground(Brush::Solid(Color::WHITE))
                    .with_child(check_mark))
                    .with_stroke_thickness(Thickness::uniform(1.0))
                    .build(container))
                .build(),
            checked: self.checked,
            check_mark,
        };

        container.add_node(Box::new(check_box))
    }
}