use crate::{
    border::BorderBuilder,
    brush::Brush,
    button::{ButtonBuilder, ButtonMessage},
    core::{color::Color, pool::Handle},
    define_constructor,
    grid::{Column, GridBuilder, Row},
    message::{MessageDirection, UiMessage},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, NodeHandleMapping, UiNode, UserInterface,
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

/// A set of messages for [`TabControl`] widget.
#[derive(Debug, Clone, PartialEq)]
pub enum TabControlMessage {
    /// Used to change the active tab of a [`TabControl`] widget (with [`MessageDirection::ToWidget`]) or to fetch if the active
    /// tab has changed (with [`MessageDirection::FromWidget`]).
    ActiveTab(Option<usize>),
}

impl TabControlMessage {
    define_constructor!(
        /// Creates [`TabControlMessage::ActiveTab`] message.
        TabControlMessage:ActiveTab => fn active_tab(Option<usize>), layout: false
    );
}

#[derive(Clone, PartialEq, Eq)]
pub struct Tab {
    pub header_button: Handle<UiNode>,
    pub content: Handle<UiNode>,
}

#[derive(Clone)]
pub struct TabControl {
    pub widget: Widget,
    pub tabs: Vec<Tab>,
    pub active_tab: Option<usize>,
}

crate::define_widget_deref!(TabControl);

impl Control for TabControl {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        for tab in self.tabs.iter_mut() {
            node_map.resolve(&mut tab.header_button);
            node_map.resolve(&mut tab.content);
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(ButtonMessage::Click) = message.data() {
            for (i, tab) in self.tabs.iter().enumerate() {
                if message.destination() == tab.header_button
                    && tab.header_button.is_some()
                    && tab.content.is_some()
                {
                    ui.send_message(TabControlMessage::active_tab(
                        self.handle,
                        MessageDirection::ToWidget,
                        Some(i),
                    ));
                    break;
                }
            }
        } else if let Some(TabControlMessage::ActiveTab(active_tab)) = message.data() {
            if self.active_tab != *active_tab {
                for (existing_tab_index, tab) in self.tabs.iter().enumerate() {
                    ui.send_message(WidgetMessage::visibility(
                        tab.content,
                        MessageDirection::ToWidget,
                        active_tab.map_or(false, |active_tab_index| {
                            existing_tab_index == active_tab_index
                        }),
                    ));
                }
                self.active_tab = *active_tab;
                // Notify potential listeners, that the active tab has changed.
                ui.send_message(message.reverse());
            }
        }
    }
}

pub struct TabControlBuilder {
    widget_builder: WidgetBuilder,
    tabs: Vec<TabDefinition>,
}

pub struct TabDefinition {
    pub header: Handle<UiNode>,
    pub content: Handle<UiNode>,
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

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let mut headers = Vec::new();
        let mut content = Vec::new();
        let tab_count = self.tabs.len();
        for (i, tab) in self.tabs.into_iter().enumerate() {
            headers.push(tab.header);
            // Hide everything but first tab content.
            if i > 0 {
                ctx[tab.content].set_visibility(false);
            }
            content.push(tab.content);
        }

        let tab_buttons = headers
            .into_iter()
            .enumerate()
            .map(|(i, header)| {
                ButtonBuilder::new(WidgetBuilder::new().on_column(i))
                    .with_content(header)
                    .build(ctx)
            })
            .collect::<Vec<Handle<UiNode>>>();

        let headers_grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_children(tab_buttons.iter().cloned())
                .on_row(0),
        )
        .add_row(Row::auto())
        .add_columns((0..tab_count).map(|_| Column::auto()).collect())
        .build(ctx);

        let content_grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_children(content.iter().cloned())
                .on_row(1),
        )
        .build(ctx);

        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(headers_grid)
                .with_child(content_grid),
        )
        .add_column(Column::auto())
        .add_row(Row::strict(30.0))
        .add_row(Row::auto())
        .build(ctx);

        let tc = TabControl {
            widget: self
                .widget_builder
                .with_child(
                    BorderBuilder::new(
                        WidgetBuilder::new()
                            .with_background(Brush::Solid(Color::from_rgba(0, 0, 0, 0)))
                            .with_child(grid),
                    )
                    .build(ctx),
                )
                .build(),
            active_tab: if tab_count == 0 { None } else { Some(0) },
            tabs: tab_buttons
                .iter()
                .zip(content)
                .map(|(tab_button, content)| Tab {
                    header_button: *tab_button,
                    content,
                })
                .collect(),
        };

        ctx.add_node(UiNode::new(tc))
    }
}
