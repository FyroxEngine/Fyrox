use crate::{
    gui::{
        UserInterface,
        maxf,
        scroll_content_presenter::{
            ScrollContentPresenterBuilder,
            ScrollContentPresenter
        },
        scroll_bar::{
            ScrollBarBuilder,
            Orientation,
            ScrollBar
        },
        grid::{
            Row,
            GridBuilder,
            Column
        },
        event::{
            UIEventKind,
            UIEvent
        },
        widget::{
            Widget,
            WidgetBuilder
        },
        Visibility,
        Control,
        UINode,
        ControlTemplate,
        UINodeContainer,
        Builder
    },
    core::{
        pool::Handle,
        math::vec2::Vec2,
    },
};
use std::collections::HashMap;

pub struct ScrollViewer {
    widget: Widget,
    content: Handle<UINode>,
    content_presenter: Handle<UINode>,
    v_scroll_bar: Handle<UINode>,
    h_scroll_bar: Handle<UINode>,
}

impl ScrollViewer {
    pub fn new(
        widget: Widget,
        content: Handle<UINode>,
        content_presenter: Handle<UINode>,
        v_scroll_bar: Handle<UINode>,
        h_scroll_bar: Handle<UINode>,
    ) -> Self {
        Self {
            widget,
            content,
            content_presenter,
            v_scroll_bar,
            h_scroll_bar
        }
    }
}

impl Control for ScrollViewer {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget {
        &mut self.widget
    }

    fn raw_copy(&self) -> Box<dyn Control> {
        Box::new(Self {
            widget: *self.widget.raw_copy().downcast::<Widget>().unwrap_or_else(|_| panic!()),
            content: self.content,
            content_presenter: self.content_presenter,
            v_scroll_bar: self.v_scroll_bar,
            h_scroll_bar: self.h_scroll_bar
        })
    }

    fn resolve(&mut self, _: &ControlTemplate, node_map: &HashMap<Handle<UINode>, Handle<UINode>>) {
        self.content = *node_map.get(&self.content).unwrap();
        self.content_presenter = *node_map.get(&self.content_presenter).unwrap();
        self.v_scroll_bar = *node_map.get(&self.v_scroll_bar).unwrap();
        self.h_scroll_bar = *node_map.get(&self.h_scroll_bar).unwrap();
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vec2) -> Vec2 {
        let size = self.widget.arrange_override(ui, final_size);

        let content_size = ui.node(self.content).widget().desired_size.get();
        let available_size_for_content = ui.node(self.content_presenter).widget().desired_size.get();

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
                if let Some(scroll_bar) = ui.node_mut(self.v_scroll_bar).downcast_mut::<ScrollBar>() {
                    scroll_bar.set_max_value(new_value);

                    if (scroll_bar.max_value() - scroll_bar.min_value()).abs() <= std::f32::EPSILON {
                        scroll_bar.widget_mut()
                            .set_visibility(Visibility::Collapsed);
                    } else {
                        scroll_bar.widget_mut()
                            .set_visibility(Visibility::Visible);
                    }
                }
            }
        }

        if evt.target == self.h_scroll_bar {
            if let UIEventKind::MaxValueChanged(new_value) = evt.kind {
                if let Some(scroll_bar) = ui.node_mut(self.h_scroll_bar).downcast_mut::<ScrollBar>() {
                    scroll_bar.set_max_value(new_value);

                    if (scroll_bar.max_value() - scroll_bar.min_value()).abs() <= std::f32::EPSILON {
                        scroll_bar.widget_mut()
                            .set_visibility(Visibility::Collapsed);
                    } else {
                        scroll_bar.widget_mut()
                            .set_visibility(Visibility::Visible);
                    }
                }
            }
        }

        match evt.kind {
            UIEventKind::NumericValueChanged { new_value, .. } => {
                if let Some(content_presenter) = ui.node_mut(self.content_presenter).downcast_mut::<ScrollContentPresenter>() {
                    if evt.source == self.h_scroll_bar {
                        content_presenter.set_horizontal_scroll(new_value);
                    } else if evt.source == self.v_scroll_bar {
                        content_presenter.set_vertical_scroll(new_value);
                    }
                }
            }
            UIEventKind::MouseWheel { amount, .. } => {
                if !evt.handled && (evt.source == self_handle || self.widget().has_descendant(evt.source, ui)) {
                    if let Some(v_scroll_bar) = ui.node_mut(self.v_scroll_bar).downcast_mut::<ScrollBar>() {
                        v_scroll_bar.scroll(-amount * 10.0);
                        evt.handled = true;
                    }
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
}

impl Builder for ScrollViewerBuilder {
    fn build(self, ui: &mut dyn UINodeContainer) -> Handle<UINode> {
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
        ui.add_node(Box::new(scroll_viewer))
    }
}