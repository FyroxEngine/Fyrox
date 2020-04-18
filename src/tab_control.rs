use crate::{
    core::{
        pool::Handle,
        color::Color,
    },
    UserInterface,
    widget::{
        Widget,
        WidgetBuilder,
    },
    Control,
    UINode,
    border::BorderBuilder,
    button::ButtonBuilder,
    grid::{
        GridBuilder,
        Column,
        Row,
    },
    message::{
        UiMessage,
        UiMessageData,
    },
    brush::Brush,
    message::ButtonMessage,
    NodeHandleMapping,
};
use std::ops::{Deref, DerefMut};

pub struct Tab<M: 'static, C: 'static + Control<M, C>> {
    header_button: Handle<UINode<M, C>>,
    content: Handle<UINode<M, C>>,
}

pub struct TabControl<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    tabs: Vec<Tab<M, C>>,
}

impl<M: 'static, C: 'static + Control<M, C>> Deref for TabControl<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> DerefMut for TabControl<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for TabControl<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        UINode::TabControl(Self {
            widget: self.widget.raw_copy(),
            tabs: Default::default(),
        })
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        for tab in self.tabs.iter_mut() {
            tab.header_button = *node_map.get(&tab.header_button).unwrap();
            tab.content = *node_map.get(&tab.content).unwrap();
        }
    }

    fn handle_message(&mut self, self_handle: Handle<UINode<M, C>>, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_message(self_handle, ui, message);

        if let UiMessageData::Button(msg) = &message.data {
            if let ButtonMessage::Click = msg {
                for (i, tab) in self.tabs.iter().enumerate() {
                    if message.source == tab.header_button && tab.header_button.is_some() && tab.content.is_some() {
                        for (j, other_tab) in self.tabs.iter().enumerate() {
                            ui.node_mut(other_tab.content).set_visibility(j == i);
                        }
                        break;
                    }
                }
            }
        }
    }

    fn remove_ref(&mut self, handle: Handle<UINode<M, C>>) {
        for tab in self.tabs.iter_mut() {
            if tab.content == handle {
                tab.content = Handle::NONE;
            }
            if tab.header_button == handle {
                tab.header_button = Handle::NONE;
            }
        }
    }
}

pub struct TabControlBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    tabs: Vec<TabDefinition<M, C>>,
}

pub struct TabDefinition<M: 'static, C: 'static + Control<M, C>> {
    pub header: Handle<UINode<M, C>>,
    pub content: Handle<UINode<M, C>>,
}

impl<M, C: 'static + Control<M, C>> TabControlBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            tabs: Default::default(),
        }
    }

    pub fn with_tab(mut self, tab: TabDefinition<M, C>) -> Self {
        self.tabs.push(tab);
        self
    }

    pub fn build(self, ui: &mut UserInterface<M, C>) -> Handle<UINode<M, C>> {
        let tab_buttons = self.tabs
            .iter()
            .enumerate()
            .map(|(i, tab)| {
                ButtonBuilder::new(WidgetBuilder::new()
                    .on_column(i))
                    .with_content(tab.header)
                    .build(ui)
            }).collect::<Vec<Handle<UINode<M, C>>>>();

        // Hide everything but first tab content.
        for tab_def in self.tabs.iter().skip(1) {
            ui.node_mut(tab_def.content).set_visibility(false);
        }

        let headers_grid = GridBuilder::new(WidgetBuilder::new()
            .with_children(&tab_buttons)
            .on_row(0))
            .add_row(Row::auto())
            .add_columns((0..self.tabs.len())
                .map(|_| Column::auto())
                .collect())
            .build(ui);

        let content_grid = GridBuilder::new(WidgetBuilder::new()
            .with_children(&self.tabs
                .iter()
                .map(|tab| tab.content)
                .collect::<Vec<Handle<UINode<M, C>>>>())
            .on_row(1))
            .build(ui);

        let grid = GridBuilder::new(WidgetBuilder::new()
            .with_child(headers_grid)
            .with_child(content_grid))
            .add_column(Column::auto())
            .add_row(Row::strict(30.0))
            .add_row(Row::auto())
            .build(ui);

        let tab_control = TabControl {
            widget: self.widget_builder
                .with_child(BorderBuilder::new(WidgetBuilder::new()
                    .with_background(Brush::Solid(Color::from_rgba(0, 0, 0, 0)))
                    .with_child(grid))
                    .build(ui))
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

        let handle = ui.add_node(UINode::TabControl(tab_control));

        ui.flush_messages();

        handle
    }
}
