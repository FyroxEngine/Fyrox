use rg3d_core::{
    pool::Handle,
    color::Color,
    math::vec2::Vec2,
};
use crate::gui::{
    scroll_viewer::ScrollViewerBuilder,
    Thickness,
    border::BorderBuilder,
    widget::{Widget, AsWidget, WidgetBuilder},
    node::UINode,
    UserInterface,
    Draw,
    Layout,
    draw::DrawingContext,
    stack_panel::StackPanelBuilder,
    event::{
        UIEventKind,
        UIEvent,
    },
    Update
};

pub struct ListBox {
    widget: Widget,
    selected_index: Option<usize>,
    items: Vec<Handle<UINode>>,
}

impl ListBox {
    pub fn set_selected(&mut self, new_index: Option<usize>) {
        let old_value = self.selected_index;

        self.selected_index = new_index;

        if old_value.is_none() && new_index.is_some() ||
            old_value.is_some() && new_index.is_none() ||
            old_value.unwrap() != new_index.unwrap() {
            self.widget.events
                .borrow_mut()
                .push_back(UIEvent::new(UIEventKind::SelectionChanged(self.selected_index)))
        }
    }

    pub fn get_selected(&self) -> Option<usize> {
        self.selected_index
    }

    pub fn get_items(&self) -> &[Handle<UINode>] {
        &self.items
    }
}

impl AsWidget for ListBox {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget {
        &mut self.widget
    }
}

impl Draw for ListBox {
    fn draw(&mut self, drawing_context: &mut DrawingContext) {
        self.widget.draw(drawing_context)
    }
}

impl Update for ListBox {
    fn update(&mut self, dt: f32) {
        self.widget.update(dt)
    }
}

impl Layout for ListBox {
    fn measure_override(&self, ui: &UserInterface, available_size: Vec2) -> Vec2 {
        self.widget.measure_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vec2) -> Vec2 {
        self.widget.arrange_override(ui, final_size)
    }
}

pub struct ListBoxBuilder {
    widget_builder: WidgetBuilder,
    items: Vec<Handle<UINode>>,
}

impl ListBoxBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            items: Vec::new(),
        }
    }

    pub fn with_items(mut self, items: Vec<Handle<UINode>>) -> Self {
        self.items = items;
        self
    }

    pub fn build(self, ui: &mut UserInterface) -> Handle<UINode> {
        // Wrap each item into container which will have selection behaviour
        let items: Vec<Handle<UINode>> = self.items.iter().enumerate().map(|(index, item)| {
            BorderBuilder::new(WidgetBuilder::new()
                .with_child(*item)
                .with_event_handler(Box::new(move |ui, handle, evt| {
                    let list_box = ui.find_by_criteria_up(handle, |node| node.is_list_box());
                    if evt.source == handle || ui.is_node_child_of(evt.source, handle) {
                        let border = ui.get_node_mut(handle).as_border_mut();
                        match evt.kind {
                            UIEventKind::MouseLeave => {
                                border.widget_mut().set_color(Color::opaque(100, 100, 100));
                            }
                            UIEventKind::MouseEnter => {
                                border.widget_mut().set_color(Color::opaque(130, 130, 130));
                            }
                            UIEventKind::MouseDown { .. } => {
                                // Explicitly set selection on parent list box. This will send
                                // SelectionChanged event and all items will react.
                                let list_box = ui.get_node_mut(list_box).as_list_box_mut();
                                list_box.set_selected(Some(index))
                            }
                            _ => ()
                        }
                    } else if evt.source == list_box {
                        let border = ui.get_node_mut(handle).as_border_mut();
                        if let UIEventKind::SelectionChanged(new_value) = evt.kind {
                            // We know now that selection has changed in parent list box,
                            // check at which index and keep visual state according to it.
                            if let Some(new_value) = new_value {
                                if new_value == index {
                                    border.set_stroke_color(Color::opaque(0, 0, 0));
                                    border.set_stroke_thickness(Thickness::uniform(2.0));
                                    return;
                                }
                            }
                            border.set_stroke_color(Color::opaque(80, 80, 80));
                            border.set_stroke_thickness(Thickness::uniform(1.0));
                        }
                    }
                }))
                .with_color(Color::opaque(80, 80, 80)))
                .with_stroke_color(Color::opaque(60, 60, 60))
                .with_stroke_thickness(Thickness::uniform(1.0))
                .build(ui)
        }).collect();

        let panel = StackPanelBuilder::new(WidgetBuilder::new()
            .with_children(&items))
            .build(ui);

        let scroll_viewer = ScrollViewerBuilder::new(WidgetBuilder::new()
            .with_margin(Thickness::uniform(3.0)))
            .with_content(panel)
            .build(ui);

        let list_box = UINode::ListBox(ListBox {
            widget: self.widget_builder
                .with_child(BorderBuilder::new(WidgetBuilder::new()
                    .with_color(Color::opaque(100, 100, 100))
                    .with_child(scroll_viewer))
                    .build(ui))
                .build(),
            selected_index: None,
            items,
        });

        ui.add_node(list_box)
    }
}