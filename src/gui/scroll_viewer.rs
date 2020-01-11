use crate::gui::{
    UserInterface,
    maxf,
    scroll_content_presenter::ScrollContentPresenterBuilder,
    scroll_bar::{ScrollBarBuilder, Orientation},
    grid::{Row, GridBuilder, Column},
    event::UIEventKind,
    widget::{Widget, WidgetBuilder},
    Visibility,
    event::UIEvent,
    Control,
    UINode
};
use crate::core::{
    pool::Handle,
    math::vec2::Vec2,
};
use crate::gui::scroll_bar::ScrollBar;
use crate::gui::scroll_content_presenter::ScrollContentPresenter;

pub struct ScrollViewer {
    widget: Widget,
    content: Handle<UINode>,
    content_presenter: Handle<UINode>,
    v_scroll_bar: Handle<UINode>,
    h_scroll_bar: Handle<UINode>,
}

impl Control for ScrollViewer {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget {
        &mut self.widget
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

    fn handle_event(&mut self, self_handle: Handle<UINode>, ui: &mut UserInterface, evt: &mut UIEvent) {
        if evt.target == self.v_scroll_bar {
            if let UIEventKind::MaxValueChanged(new_value) = evt.kind {
                let scroll_bar =  ui.get_node_mut(self.v_scroll_bar).downcast_mut::<ScrollBar>().unwrap();

                scroll_bar.set_max_value(new_value);

                if (scroll_bar.get_max_value() - scroll_bar.get_min_value()).abs() <= std::f32::EPSILON {
                    scroll_bar.widget_mut().set_visibility(Visibility::Collapsed)
                } else {
                    scroll_bar.widget_mut().set_visibility(Visibility::Visible)
                }
            }
        }

        if evt.target == self.h_scroll_bar {
            if let UIEventKind::MaxValueChanged(new_value) = evt.kind {
                let scroll_bar = ui.get_node_mut(self.h_scroll_bar).downcast_mut::<ScrollBar>().unwrap();

                scroll_bar.set_max_value(new_value);

                if (scroll_bar.get_max_value() - scroll_bar.get_min_value()).abs() <= std::f32::EPSILON {
                    scroll_bar.widget_mut().set_visibility(Visibility::Collapsed)
                } else {
                    scroll_bar.widget_mut().set_visibility(Visibility::Visible)
                }
            }
        }

        match evt.kind {
            UIEventKind::NumericValueChanged { new_value, .. } => {
                let content_presenter = ui.get_node_mut(self.content_presenter).downcast_mut::<ScrollContentPresenter>().unwrap();
                if evt.source == self.h_scroll_bar {
                    content_presenter.set_horizontal_scroll(new_value);
                } else if evt.source == self.v_scroll_bar {
                    content_presenter.set_vertical_scroll(new_value);
                }
            }
            UIEventKind::MouseWheel { amount, .. } => {
                if !evt.handled && (evt.source == self_handle || self.widget().has_descendant(evt.source, ui)) {
                    let v_scroll_bar = ui.get_node_mut(self.v_scroll_bar).downcast_mut::<ScrollBar>().unwrap();
                    v_scroll_bar.scroll(-amount * 10.0);
                    evt.handled = true;
                }
            }
            _ => {}
        }
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
            .with_width(20.0))
            .with_orientation(Orientation::Vertical)
            .build(ui);

        let h_scroll_bar = ScrollBarBuilder::new(WidgetBuilder::new()
            .on_row(1)
            .on_column(0)
            .with_height(20.0))
            .with_orientation(Orientation::Horizontal)
            .build(ui);

        let scroll_viewer = ScrollViewer {
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
                .build(),
            content: self.content,
            v_scroll_bar,
            h_scroll_bar,
            content_presenter,
        };
        ui.add_node(scroll_viewer)
    }
}
