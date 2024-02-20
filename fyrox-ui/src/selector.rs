use crate::{
    border::BorderBuilder,
    button::{ButtonBuilder, ButtonMessage},
    core::{
        pool::Handle, reflect::prelude::*, type_traits::prelude::*, variable::InheritableVariable,
        visitor::prelude::*,
    },
    define_constructor, define_widget_deref,
    grid::{Column, GridBuilder, Row},
    message::{MessageDirection, UiMessage},
    utils::{make_arrow, ArrowDirection},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, Thickness, UiNode, UserInterface,
};
use std::ops::{Deref, DerefMut};

#[derive(Debug, PartialEq, Clone)]
pub enum SelectorMessage {
    AddItem(Handle<UiNode>),
    RemoveItem(Handle<UiNode>),
    SetItems {
        items: Vec<Handle<UiNode>>,
        remove_previous: bool,
    },
    Current(Option<usize>),
}

impl SelectorMessage {
    define_constructor!(
        /// Creates [`SelectorMessage::AddItem`] message.
        SelectorMessage:AddItem => fn add_item(Handle<UiNode>), layout: false
    );
    define_constructor!(
        /// Creates [`SelectorMessage::RemoveItem`] message.
        SelectorMessage:RemoveItem => fn remove_item(Handle<UiNode>), layout: false
    );
    define_constructor!(
        /// Creates [`SelectorMessage::SetItems`] message.
        SelectorMessage:SetItems => fn set_items(items: Vec<Handle<UiNode>>, remove_previous: bool), layout: false
    );
    define_constructor!(
        /// Creates [`SelectorMessage::Current`] message.
        SelectorMessage:Current => fn current(Option<usize>), layout: false
    );
}

#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider, TypeUuidProvider)]
#[type_uuid(id = "25118853-5c3c-4197-9e4b-2e3b9d92f4d2")]
pub struct Selector {
    widget: Widget,
    items: InheritableVariable<Vec<Handle<UiNode>>>,
    items_panel: InheritableVariable<Handle<UiNode>>,
    current: InheritableVariable<Option<usize>>,
    prev: InheritableVariable<Handle<UiNode>>,
    next: InheritableVariable<Handle<UiNode>>,
}

define_widget_deref!(Selector);

impl Control for Selector {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<SelectorMessage>() {
            match msg {
                SelectorMessage::AddItem(item) => {
                    ui.send_message(WidgetMessage::link(
                        *item,
                        MessageDirection::ToWidget,
                        *self.items_panel,
                    ));
                    self.items.push(*item);
                }
                SelectorMessage::RemoveItem(item) => {
                    if let Some(position) = self.items.iter().position(|i| i == item) {
                        ui.send_message(WidgetMessage::remove(*item, MessageDirection::ToWidget));

                        self.items.remove(position);
                    }
                }
                SelectorMessage::SetItems {
                    items,
                    remove_previous,
                } => {
                    if *remove_previous {
                        for &item in &*self.items {
                            ui.send_message(WidgetMessage::remove(
                                item,
                                MessageDirection::ToWidget,
                            ));
                        }
                    }

                    for &item in items {
                        ui.send_message(WidgetMessage::link(
                            item,
                            MessageDirection::ToWidget,
                            *self.items_panel,
                        ));
                    }

                    self.items.set_value_and_mark_modified(items.clone());

                    for (i, item) in self.items.iter().enumerate() {
                        ui.send_message(WidgetMessage::visibility(
                            *item,
                            MessageDirection::ToWidget,
                            self.current.map_or(false, |current| current == i),
                        ));
                    }
                }
                SelectorMessage::Current(current) => {
                    if &*self.current != current
                        && message.direction() == MessageDirection::ToWidget
                    {
                        if let Some(current) = *self.current {
                            if let Some(current_item) = self.items.get(current) {
                                ui.send_message(WidgetMessage::visibility(
                                    *current_item,
                                    MessageDirection::ToWidget,
                                    false,
                                ));
                            }
                        }

                        self.current.set_value_and_mark_modified(*current);

                        if let Some(new_current) = *self.current {
                            if let Some(new_current_item) = self.items.get(new_current) {
                                ui.send_message(WidgetMessage::visibility(
                                    *new_current_item,
                                    MessageDirection::ToWidget,
                                    true,
                                ));
                            }
                        }

                        ui.send_message(message.reverse());
                    }
                }
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == *self.prev {
                if let Some(current) = *self.current {
                    let new_current = current.saturating_sub(1);
                    ui.send_message(SelectorMessage::current(
                        self.handle,
                        MessageDirection::ToWidget,
                        Some(new_current),
                    ));
                }
            } else if message.destination() == *self.next {
                if let Some(current) = *self.current {
                    let new_current = current
                        .saturating_add(1)
                        .min(self.items.len().saturating_sub(1));
                    ui.send_message(SelectorMessage::current(
                        self.handle,
                        MessageDirection::ToWidget,
                        Some(new_current),
                    ));
                }
            }
        }
    }
}

pub struct SelectorBuilder {
    widget_builder: WidgetBuilder,
    items: Vec<Handle<UiNode>>,
    current: Option<usize>,
}

impl SelectorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            items: Default::default(),
            current: None,
        }
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        for (i, item) in self.items.iter().enumerate() {
            ctx[*item].set_visibility(self.current.map_or(false, |current| current == i));
        }

        let prev;
        let next;
        let items_panel;
        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child({
                    prev = ButtonBuilder::new(WidgetBuilder::new().on_column(0))
                        .with_content(make_arrow(ctx, ArrowDirection::Left, 24.0))
                        .build(ctx);
                    prev
                })
                .with_child({
                    items_panel = BorderBuilder::new(
                        WidgetBuilder::new()
                            .with_children(self.items.clone())
                            .on_column(1),
                    )
                    .with_stroke_thickness(Thickness::uniform(0.0))
                    .build(ctx);
                    items_panel
                })
                .with_child({
                    next = ButtonBuilder::new(WidgetBuilder::new().on_column(2))
                        .with_content(make_arrow(ctx, ArrowDirection::Right, 24.0))
                        .build(ctx);
                    next
                }),
        )
        .add_row(Row::auto())
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .build(ctx);

        let selector = Selector {
            widget: self.widget_builder.with_child(grid).build(),
            items: self.items.into(),
            items_panel: items_panel.into(),
            prev: prev.into(),
            next: next.into(),
            current: self.current.into(),
        };

        ctx.add_node(UiNode::new(selector))
    }
}
