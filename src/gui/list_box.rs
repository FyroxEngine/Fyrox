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
};

pub struct ListBox {
    widget: Widget,
    panel: Handle<UINode>,
    selected_index: Option<usize>,
}

impl ListBox {}

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
    items: Vec<Handle<UINode>>
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
        let panel = StackPanelBuilder::new(WidgetBuilder::new()
            .with_children(self.items))
            .build(ui);

        let scroll_viewer = ScrollViewerBuilder::new(WidgetBuilder::new()
            .with_margin(Thickness::uniform(3.0)))
            .with_content(panel)
            .build(ui);

        let list_box = UINode::ListBox(ListBox {
            widget: self.widget_builder
                .with_event_handler(Box::new(|ui, handle, evt| {}))
                .with_child(BorderBuilder::new(WidgetBuilder::new()
                    .with_color(Color::opaque(100, 100, 100))
                    .with_child(scroll_viewer))
                    .build(ui))
                .build(),
            selected_index: None,
            panel,
        });

        ui.add_node(list_box)
    }
}