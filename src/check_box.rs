use crate::{
    widget::{
        Widget,
        WidgetBuilder,
    },
    UserInterface,
    UINode,
    message::{
        UiMessage,
        UiMessageData,
        CheckBoxMessage,
        WidgetMessage
    },
    Thickness,
    border::BorderBuilder,
    Control,
    core::{
        pool::Handle,
        color::Color,
    },
    brush::Brush,
    NodeHandleMapping
};

pub struct CheckBox<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    checked: Option<bool>,
    check_mark: Handle<UINode<M, C>>,
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for CheckBox<M, C> {
    fn widget(&self) -> &Widget<M, C> {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget<M, C> {
        &mut self.widget
    }

    fn raw_copy(&self) -> UINode<M, C> {
        UINode::CheckBox(Self {
            widget: self.widget.raw_copy(),
            checked: self.checked,
            check_mark: self.check_mark,
        })
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        self.check_mark = *node_map.get(&self.check_mark).unwrap();
    }

    fn handle_message(&mut self, self_handle: Handle<UINode<M, C>>, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_message(self_handle, ui, message);

        match message.data {
            UiMessageData::Widget(ref msg) => {
                match msg {
                    WidgetMessage::MouseDown { .. } => {
                        if message.source == self_handle || self.widget.has_descendant(message.source, ui) {
                            ui.capture_mouse(self_handle);
                        }
                    }
                    WidgetMessage::MouseUp { .. } => {
                        if message.source == self_handle || self.widget.has_descendant(message.source, ui) {
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
                    _ => ()
                }
            }
            UiMessageData::CheckBox(ref msg) => {
                if let CheckBoxMessage::Checked(value) = msg {
                    if message.source == self_handle && self.check_mark.is_some() {
                        let check_mark = ui.node_mut(self.check_mark).widget_mut();
                        match value {
                            None => {
                                check_mark.set_background(Brush::Solid(Color::opaque(30, 30, 80)));
                            }
                            Some(value) => {
                                check_mark.set_background(Brush::Solid(Color::opaque(200, 200, 200)));
                                check_mark.set_visibility(*value);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn remove_ref(&mut self, handle: Handle<UINode<M, C>>) {
        if self.check_mark == handle {
            self.check_mark = Handle::NONE;
        }
    }
}

impl<M, C: 'static + Control<M, C>> CheckBox<M, C> {
    pub fn new(widget: Widget<M, C>, check_mark: Handle<UINode<M, C>>) -> Self {
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
        if self.checked != value {
            self.checked = value;
            self.widget.post_message(UiMessage::new(UiMessageData::CheckBox(CheckBoxMessage::Checked(value))));
        }
        self
    }
}

pub struct CheckBoxBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    checked: Option<bool>,
    check_mark: Option<Handle<UINode<M, C>>>,
}

impl<M, C: 'static + Control<M, C>> CheckBoxBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
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

    pub fn with_check_mark(mut self, check_mark: Handle<UINode<M, C>>) -> Self {
        self.check_mark = Some(check_mark);
        self
    }

    pub fn build(self, ui: &mut UserInterface<M, C>) -> Handle<UINode<M, C>> {
        let check_mark = self.check_mark.unwrap_or_else(|| {
            BorderBuilder::new(WidgetBuilder::new()
                .with_background(Brush::Solid(Color::opaque(200, 200, 200)))
                .with_margin(Thickness::uniform(1.0)))
                .with_stroke_thickness(Thickness::uniform(0.0))
                .build(ui)
        });

        let visibility = if let Some(value) = self.checked {
            value
        } else {
            true
        };
        ui.node_mut(check_mark)
            .widget_mut()
            .set_visibility(visibility);

        let check_box = CheckBox {
            widget: self.widget_builder
                .with_child(BorderBuilder::new(WidgetBuilder::new()
                    .with_background(Brush::Solid(Color::opaque(60, 60, 60)))
                    .with_child(check_mark))
                    .with_stroke_thickness(Thickness::uniform(1.0))
                    .build(ui))
                .build(),
            checked: self.checked,
            check_mark,
        };

        let handle = ui.add_node(UINode::CheckBox(check_box));

        ui.flush_messages();

        handle
    }
}
