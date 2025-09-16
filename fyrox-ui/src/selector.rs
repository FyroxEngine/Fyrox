// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
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
use fyrox_core::pool::NodeVariant;

use fyrox_graph::constructor::{ConstructorProvider, GraphNodeConstructor};
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
#[reflect(derived_type = "UiNode")]
#[type_uuid(id = "25118853-5c3c-4197-9e4b-2e3b9d92f4d2")]
pub struct Selector {
    widget: Widget,
    items: InheritableVariable<Vec<Handle<UiNode>>>,
    items_panel: InheritableVariable<Handle<UiNode>>,
    current: InheritableVariable<Option<usize>>,
    prev: InheritableVariable<Handle<UiNode>>,
    next: InheritableVariable<Handle<UiNode>>,
}

impl NodeVariant<UiNode> for Selector {}

impl ConstructorProvider<UiNode, UserInterface> for Selector {
    fn constructor() -> GraphNodeConstructor<UiNode, UserInterface> {
        GraphNodeConstructor::new::<Self>()
            .with_variant("Selector", |ui| {
                SelectorBuilder::new(WidgetBuilder::new().with_name("Selector"))
                    .build(&mut ui.build_ctx())
                    .into()
            })
            .with_group("Input")
    }
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
                            *self.current == Some(i),
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
            ctx[*item].set_visibility(self.current == Some(i));
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
                    .with_stroke_thickness(Thickness::uniform(0.0).into())
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
            widget: self.widget_builder.with_child(grid).build(ctx),
            items: self.items.into(),
            items_panel: items_panel.into(),
            prev: prev.into(),
            next: next.into(),
            current: self.current.into(),
        };

        ctx.add_node(UiNode::new(selector))
    }
}

#[cfg(test)]
mod test {
    use crate::selector::SelectorBuilder;
    use crate::{test::test_widget_deletion, widget::WidgetBuilder};

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| SelectorBuilder::new(WidgetBuilder::new()).build(ctx));
    }
}
