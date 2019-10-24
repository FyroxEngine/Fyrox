use crate::gui::{
    UserInterface,
    maxf,
    node::UINode,
    scroll_content_presenter::ScrollContentPresenterBuilder,
    scroll_bar::{ScrollBarBuilder, Orientation},
    grid::{Row, GridBuilder, Column},
    event::UIEventKind,
    Layout,
    widget::{Widget, WidgetBuilder, AsWidget},
    Draw,
    draw::DrawingContext,
    Visibility,
    event::UIEvent,
    Update
};
use rg3d_core::{
    pool::Handle,
    math::vec2::Vec2,
};

pub struct ScrollViewer {
    widget: Widget,
    content: Handle<UINode>,
    content_presenter: Handle<UINode>,
    v_scroll_bar: Handle<UINode>,
    h_scroll_bar: Handle<UINode>,
}

impl AsWidget for ScrollViewer {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget {
        &mut self.widget
    }
}

impl Layout for ScrollViewer {
    fn measure_override(&self, ui: &UserInterface, available_size: Vec2) -> Vec2 {
        self.widget.measure_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vec2) -> Vec2 {
        let size = self.widget.arrange_override(ui, final_size);

        let content_size = ui.get_node(self.content).widget().desired_size.get();
        let available_size_for_content = ui.get_node(self.content_presenter).widget().desired_size.get();

        let x_max = maxf(0.0, content_size.x - available_size_for_content.x);
        self.widget.events.borrow_mut()
            .push_back(UIEvent::targeted(self.h_scroll_bar, UIEventKind::MaxValueChanged(x_max)));

        let y_max = maxf(0.0, content_size.y - available_size_for_content.y);
        self.widget.events.borrow_mut()
            .push_back(UIEvent::targeted(self.v_scroll_bar, UIEventKind::MaxValueChanged(y_max)));

        size
    }
}

impl Update for ScrollViewer {
    fn update(&mut self, dt: f32) {
        self.widget.update(dt)
    }
}

impl Draw for ScrollViewer {
    fn draw(&mut self, drawing_context: &mut DrawingContext) {
        self.widget.draw(drawing_context)
    }
}

pub struct ScrollViewerBuilder {
    widget_builder: WidgetBuilder,
    content: Handle<UINode>,
}

impl ScrollViewerBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            content: Handle::NONE,
        }
    }

    pub fn with_content(mut self, content: Handle<UINode>) -> Self {
        self.content = content;
        self
    }

    pub fn build(self, ui: &mut UserInterface) -> Handle<UINode> {
        let content_presenter = ScrollContentPresenterBuilder::new(WidgetBuilder::new()
            .with_child(self.content)
            .on_row(0)
            .on_column(0))
            .build(ui);

        let v_scroll_bar = ScrollBarBuilder::new(WidgetBuilder::new()
            .on_row(0)
            .on_column(1)
            .with_width(20.0)
            .with_event_handler(Box::new(|ui, handle, evt| {
                if evt.target == handle {
                    if let UIEventKind::MaxValueChanged(new_value) = evt.kind {
                        let scroll_bar = ui.get_node_mut(handle).as_scroll_bar_mut();

                        scroll_bar.set_max_value(new_value);

                        if scroll_bar.get_max_value() == scroll_bar.get_min_value() {
                            scroll_bar.widget_mut().set_visibility(Visibility::Collapsed)
                        } else {
                            scroll_bar.widget_mut().set_visibility(Visibility::Visible)
                        }
                    }
                }
            })))
            .with_orientation(Orientation::Vertical)
            .build(ui);

        let h_scroll_bar = ScrollBarBuilder::new(WidgetBuilder::new()
            .on_row(1)
            .on_column(0)
            .with_height(20.0)
            .with_event_handler(Box::new(|ui, handle, evt| {
                if evt.target == handle {
                    if let UIEventKind::MaxValueChanged(new_value) = evt.kind {
                        let scroll_bar = ui.get_node_mut(handle).as_scroll_bar_mut();

                        scroll_bar.set_max_value(new_value);

                        if scroll_bar.get_max_value() == scroll_bar.get_min_value() {
                            scroll_bar.widget_mut().set_visibility(Visibility::Collapsed)
                        } else {
                            scroll_bar.widget_mut().set_visibility(Visibility::Visible)
                        }
                    }
                }
            })))
            .with_orientation(Orientation::Horizontal)
            .build(ui);

        let scroll_viewer = UINode::ScrollViewer(ScrollViewer {
            widget: self.widget_builder
                .with_child(GridBuilder::new(WidgetBuilder::new()
                    .with_child(content_presenter)
                    .with_child(h_scroll_bar)
                    .with_child(v_scroll_bar))
                    .add_row(Row::stretch())
                    .add_row(Row::auto())
                    .add_column(Column::stretch())
                    .add_column(Column::auto())
                    .build(ui))
                .with_event_handler(Box::new(move |ui, handle, event| {
                    match event.kind {
                        UIEventKind::NumericValueChanged { new_value, .. } => {
                            let content_presenter = ui.get_node_mut(content_presenter).as_scroll_content_presenter_mut();
                            if event.source == h_scroll_bar {
                                content_presenter.set_horizontal_scroll(new_value);
                            } else if event.source == v_scroll_bar {
                                content_presenter.set_vertical_scroll(new_value);
                            }
                        }
                        UIEventKind::MouseWheel { amount, .. } => {
                            if !event.handled {
                                if event.source == handle || ui.is_node_child_of(event.source, handle) {
                                    let v_scroll_bar = ui.get_node_mut(v_scroll_bar).as_scroll_bar_mut();
                                    v_scroll_bar.scroll(-amount * 10.0);
                                    event.handled = true;
                                }
                            }
                        }
                        _ => {}
                    }
                }))
                .build(),
            content: self.content,
            v_scroll_bar,
            h_scroll_bar,
            content_presenter,
        });
        ui.add_node(scroll_viewer)
    }
}
