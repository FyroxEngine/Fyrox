use crate::{
    border::BorderBuilder,
    button::{ButtonBuilder, ButtonMessage},
    core::{algebra::Vector2, pool::Handle},
    define_constructor, define_widget_deref,
    grid::{Column, GridBuilder, Row},
    message::{MessageDirection, UiMessage},
    text::TextMessage,
    text_box::{TextBoxBuilder, TextCommitMode},
    utils::make_cross,
    vector_image::{Primitive, VectorImageBuilder},
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, Thickness, UiNode, UserInterface, VerticalAlignment, BRUSH_DARKER,
    BRUSH_LIGHT, BRUSH_LIGHTEST,
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone, PartialEq)]
pub enum SearchBarMessage {
    Text(String),
}

impl SearchBarMessage {
    define_constructor!(SearchBarMessage:Text => fn text(String), layout: false);
}

#[derive(Clone)]
pub struct SearchBar {
    widget: Widget,
    text_box: Handle<UiNode>,
    clear: Handle<UiNode>,
}

define_widget_deref!(SearchBar);

impl Control for SearchBar {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle && message.direction() == MessageDirection::ToWidget
        {
            if let Some(SearchBarMessage::Text(text)) = message.data() {
                ui.send_message(TextMessage::text(
                    self.text_box,
                    MessageDirection::ToWidget,
                    text.clone(),
                ));
            }
        }

        if message.destination() == self.clear {
            if let Some(ButtonMessage::Click) = message.data() {
                ui.send_message(SearchBarMessage::text(
                    self.handle,
                    MessageDirection::ToWidget,
                    String::new(),
                ));
            }
        }

        if message.destination() == self.text_box
            && message.direction() == MessageDirection::FromWidget
        {
            if let Some(TextMessage::Text(text)) = message.data() {
                ui.send_message(SearchBarMessage::text(
                    self.handle,
                    MessageDirection::FromWidget,
                    text.clone(),
                ));
            }
        }
    }
}

pub struct SearchBarBuilder {
    widget_builder: WidgetBuilder,
}

impl SearchBarBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self { widget_builder }
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let text_box;
        let clear;
        let content = BorderBuilder::new(
            WidgetBuilder::new()
                .with_foreground(BRUSH_LIGHT)
                .with_background(BRUSH_DARKER)
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .with_child(
                                VectorImageBuilder::new(
                                    WidgetBuilder::new()
                                        .with_width(12.0)
                                        .with_height(12.0)
                                        .with_vertical_alignment(VerticalAlignment::Center)
                                        .with_foreground(BRUSH_LIGHTEST)
                                        .with_margin(Thickness::left(1.0)),
                                )
                                .with_primitives(vec![
                                    Primitive::Circle {
                                        center: Vector2::new(4.0, 4.0),
                                        radius: 4.0,
                                        segments: 16,
                                    },
                                    Primitive::Line {
                                        begin: Vector2::new(6.0, 6.0),
                                        end: Vector2::new(11.0, 11.0),
                                        thickness: 1.5,
                                    },
                                ])
                                .build(ctx),
                            )
                            .with_child({
                                text_box = TextBoxBuilder::new(
                                    WidgetBuilder::new()
                                        .on_column(1)
                                        .with_margin(Thickness::uniform(1.0)),
                                )
                                .with_text_commit_mode(TextCommitMode::Immediate)
                                .with_vertical_text_alignment(VerticalAlignment::Center)
                                .build(ctx);
                                text_box
                            })
                            .with_child({
                                clear = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_width(18.0)
                                        .with_height(18.0)
                                        .on_column(2),
                                )
                                .with_content(make_cross(ctx, 12.0, 2.0))
                                .build(ctx);
                                clear
                            }),
                    )
                    .add_row(Row::stretch())
                    .add_column(Column::auto())
                    .add_column(Column::stretch())
                    .add_column(Column::auto())
                    .build(ctx),
                ),
        )
        .with_stroke_thickness(Thickness::uniform(1.0))
        .build(ctx);

        let search_bar = SearchBar {
            widget: self.widget_builder.with_child(content).build(),
            text_box,
            clear,
        };

        ctx.add_node(UiNode::new(search_bar))
    }
}
