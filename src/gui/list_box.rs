use crate::{
    gui::{
        scroll_viewer::ScrollViewerBuilder,
        Thickness,
        border::BorderBuilder,
        widget::{
            Widget,
            WidgetBuilder
        },
        UINode,
        UserInterface,
        stack_panel::StackPanelBuilder,
        event::{
            UIEventKind,
            UIEvent,
        },
        Control,
        border::Border,
        ControlTemplate,
        UINodeContainer,
        Builder
    },
    core::{
        pool::Handle,
        color::Color,
    },
};
use std::collections::HashMap;

pub struct ListBox {
    widget: Widget,
    selected_index: Option<usize>,
    items: Vec<Handle<UINode>>,
}

impl ListBox {
    pub fn new(widget: Widget, items: Vec<Handle<UINode>>) -> Self {
        Self {
            widget,
            selected_index: None,
            items
        }
    }

    pub fn set_selected(&mut self, new_index: Option<usize>) {
        let old_value = self.selected_index;

        self.selected_index = new_index;

        if old_value.is_none() && new_index.is_some() ||
            old_value.is_some() && new_index.is_none() ||
            old_value.unwrap() != new_index.unwrap() {
            self.widget
                .events
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

pub struct ListBoxItem {
    widget: Widget,
    body: Handle<UINode>,
    index: usize,
}

impl Control for ListBoxItem {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget {
        &mut self.widget
    }

    fn raw_copy(&self) -> Box<dyn Control> {
        Box::new(Self {
            widget: *self.widget.raw_copy().downcast::<Widget>().unwrap_or_else(|_| panic!()),
            body: self.body,
            index: self.index
        })
    }

    fn resolve(&mut self, _: &ControlTemplate, node_map: &HashMap<Handle<UINode>, Handle<UINode>>) {
        self.body = *node_map.get(&self.body).unwrap();
    }

    fn handle_event(&mut self, self_handle: Handle<UINode>, ui: &mut UserInterface, evt: &mut UIEvent) {
        let list_box = self.widget().find_by_criteria_up(ui, |node| node.is::<ListBox>());
        if evt.source == self_handle || self.widget().has_descendant(evt.source, ui) {
            let body = ui.node_mut(self.body).widget_mut();
            match evt.kind {
                UIEventKind::MouseLeave => {
                    body.widget_mut().set_background(Color::opaque(100, 100, 100));
                }
                UIEventKind::MouseEnter => {
                    body.widget_mut().set_background(Color::opaque(130, 130, 130));
                }
                UIEventKind::MouseDown { .. } => {
                    // Explicitly set selection on parent list box. This will send
                    // SelectionChanged event and all items will react.
                    ui.node_mut(list_box)
                        .downcast_mut::<ListBox>()
                        .unwrap()
                        .set_selected(Some(self.index))
                }
                _ => ()
            }
        } else if evt.source == list_box {
            if let Some(border) = ui.node_mut(self.body).downcast_mut::<Border>() {
                if let UIEventKind::SelectionChanged(new_value) = evt.kind {
                    // We know now that selection has changed in parent list box,
                    // check at which index and keep visual state according to it.
                    if let Some(new_value) = new_value {
                        if new_value == self.index {
                            border.widget_mut().set_foreground(Color::opaque(0, 0, 0));
                            border.set_stroke_thickness(Thickness::uniform(2.0));
                            return;
                        }
                    }
                    border.widget_mut().set_foreground(Color::opaque(80, 80, 80));
                    border.set_stroke_thickness(Thickness::uniform(1.0));
                }
            }
        }
    }
}

impl Control for ListBox {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget {
        &mut self.widget
    }

    fn raw_copy(&self) -> Box<dyn Control> {
        Box::new(Self {
            widget: *self.widget.raw_copy().downcast::<Widget>().unwrap_or_else(|_| panic!()),
            selected_index: self.selected_index.clone(),
            items: self.items.clone(),
        })
    }

    fn resolve(&mut self, _: &ControlTemplate, node_map: &HashMap<Handle<UINode>, Handle<UINode>>) {
        for item in self.items.iter_mut() {
            *item = *node_map.get(item).unwrap();
        }
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
}

impl Builder for ListBoxBuilder {
    fn build(self, ui: &mut dyn UINodeContainer) -> Handle<UINode> {
        // Wrap each item into container which will have selection behaviour
        let items: Vec<Handle<UINode>> = self.items.iter().enumerate().map(|(index, item)| {
            let body = BorderBuilder::new(WidgetBuilder::new()
                .with_foreground(Color::opaque(60, 60, 60))
                .with_background(Color::opaque(80, 80, 80))
                .with_child(*item))
                .with_stroke_thickness(Thickness::uniform(1.0))
                .build(ui);

            let item = ListBoxItem {
                widget: WidgetBuilder::new()
                    .with_child(body)
                    .build(),
                body,
                index
            };

            ui.add_node(Box::new(item))
        }).collect();

        let panel = StackPanelBuilder::new(WidgetBuilder::new()
            .with_children(&items))
            .build(ui);

        let scroll_viewer = ScrollViewerBuilder::new(WidgetBuilder::new()
            .with_margin(Thickness::uniform(3.0)))
            .with_content(panel)
            .build(ui);

        let list_box = ListBox {
            widget: self.widget_builder
                .with_child(BorderBuilder::new(WidgetBuilder::new()
                    .with_background(Color::opaque(100, 100, 100))
                    .with_child(scroll_viewer))
                    .build(ui))
                .build(),
            selected_index: None,
            items,
        };

        ui.add_node(Box::new(list_box))
    }
}