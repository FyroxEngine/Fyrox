use std::collections::HashMap;
use crate::{
    core::pool::Handle,
    gui::{
        UserInterface,
        widget::{
            Widget,
            WidgetBuilder
        },
        Control,
        ControlTemplate,
        UINode,
        Builder,
        UINodeContainer,
        border::BorderBuilder,
        button::ButtonBuilder,
        grid::{
            GridBuilder,
            Column,
            Row,
        },
        event::{
            UIEvent,
            UIEventKind,
        },
        Visibility
    }
};
use rg3d_core::color::Color;

pub struct Tab {
    header_button: Handle<UINode>,
    content: Handle<UINode>,
}

pub struct TabControl {
    widget: Widget,
    tabs: Vec<Tab>,
}

impl Control for TabControl {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget {
        &mut self.widget
    }

    fn raw_copy(&self) -> Box<dyn Control> {
        Box::new(Self {
            widget: *self.widget.raw_copy().downcast::<Widget>().unwrap_or_else(|_| panic!()),
            tabs: Default::default(),
        })
    }

    fn resolve(&mut self, _: &ControlTemplate, node_map: &HashMap<Handle<UINode>, Handle<UINode>>) {
        for tab in self.tabs.iter_mut() {
            tab.header_button = *node_map.get(&tab.header_button).unwrap();
            tab.content = *node_map.get(&tab.content).unwrap();
        }
    }

    fn handle_event(&mut self, _: Handle<UINode>, ui: &mut UserInterface, evt: &mut UIEvent) {
        match evt.kind {
            UIEventKind::Click => {
                for (i, tab) in self.tabs.iter().enumerate() {
                    if evt.source == tab.header_button {
                        for (j, other_tab) in self.tabs.iter().enumerate() {
                            let visibility = if j == i {
                                Visibility::Visible
                            } else {
                                Visibility::Collapsed
                            };
                            ui.node_mut(other_tab.content)
                                .widget_mut()
                                .set_visibility(visibility);
                        }
                        break;
                    }
                }
            }
            _ => ()
        }
    }
}

pub struct TabControlBuilder {
    widget_builder: WidgetBuilder,
    tabs: Vec<TabDefinition>,
}

pub struct TabDefinition {
    pub header: Handle<UINode>,
    pub content: Handle<UINode>,
}

impl TabControlBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            tabs: Default::default(),
        }
    }

    pub fn with_tab(mut self, tab: TabDefinition) -> Self {
        self.tabs.push(tab);
        self
    }
}

impl Builder for TabControlBuilder {
    fn build(self, container: &mut dyn UINodeContainer) -> Handle<UINode> {
        let tab_buttons = self.tabs
            .iter()
            .enumerate()
            .map(|(i, tab)| {
                ButtonBuilder::new(WidgetBuilder::new()
                    .on_column(i))
                    .with_content(tab.header)
                    .build(container)
            }).collect::<Vec<Handle<UINode>>>();

        // Hide everything but first tab content.
        for tab_def in self.tabs.iter().skip(1) {
            container.node_mut(tab_def.content)
                .widget_mut()
                .set_visibility(Visibility::Collapsed);
        }

        let headers_grid = GridBuilder::new(WidgetBuilder::new()
            .with_children(&tab_buttons)
            .on_row(0))
            .add_row(Row::auto())
            .add_columns((0..self.tabs.len())
                .map(|_| Column::auto())
                .collect())
            .build(container);

        let content_grid = GridBuilder::new(WidgetBuilder::new()
            .with_children(&self.tabs
                .iter()
                .map(|tab| tab.content)
                .collect::<Vec<Handle<UINode>>>())
            .on_row(1))
            .build(container);

        let grid = GridBuilder::new(WidgetBuilder::new()
            .with_child(headers_grid)
            .with_child(content_grid))
            .add_column(Column::auto())
            .add_row(Row::strict(30.0))
            .add_row(Row::auto())
            .build(container);

        let tab_control = TabControl {
            widget: self.widget_builder
                .with_child(BorderBuilder::new(WidgetBuilder::new()
                    .with_background(Color::from_rgba(0,0,0,0))
                    .with_child(grid))
                    .build(container))
                .build(),
            tabs: tab_buttons.iter()
                .zip(self.tabs.iter())
                .map(|(tab_button, tab_definition)| {
                    Tab {
                        header_button: *tab_button,
                        content: tab_definition.content,
                    }
                })
                .collect(),
        };

        container.add_node(Box::new(tab_control))
    }
}