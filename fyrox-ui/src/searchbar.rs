//! Search bar widget is a text box with a "clear text" button. It is used as an input field for search functionality.
//! Keep in mind, that it does **not** provide any built-in searching functionality by itself! See [`SearchBar`] docs
//! for more info and usage examples.

#![warn(missing_docs)]

use crate::{
    border::BorderBuilder,
    button::{ButtonBuilder, ButtonMessage},
    core::{algebra::Vector2, pool::Handle},
    core::{reflect::prelude::*, visitor::prelude::*},
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
use fyrox_core::uuid_provider;
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

/// A set of messages that can be used to get the state of a search bar.
#[derive(Debug, Clone, PartialEq)]
pub enum SearchBarMessage {
    /// Emitted when a user types something in the search bar.
    Text(String),
}

impl SearchBarMessage {
    define_constructor!(
        /// Creates [`SearchBarMessage::Text`] message.
        SearchBarMessage:Text => fn text(String), layout: false
    );
}

/// Search bar widget is a text box with a "clear text" button. It is used as an input field for search functionality.
/// Keep in mind, that it does **not** provide any built-in searching functionality by itself, you need to implement
/// it manually. This widget provides a "standard" looking search bar with very little functionality.
///
/// ## Examples
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle,
/// #     message::UiMessage,
/// #     searchbar::{SearchBarBuilder, SearchBarMessage},
/// #     widget::WidgetBuilder,
/// #     BuildContext, UiNode,
/// # };
/// #
/// fn create_search_bar(ctx: &mut BuildContext) -> Handle<UiNode> {
///     SearchBarBuilder::new(WidgetBuilder::new()).build(ctx)
/// }
///
/// // Somewhere in a UI message loop:
/// fn handle_ui_message(my_search_bar: Handle<UiNode>, message: &UiMessage) {
///     // Catch the moment when the search text has changed and do the actual searching.
///     if let Some(SearchBarMessage::Text(search_text)) = message.data() {
///         if message.destination() == my_search_bar {
///             let items = ["foo", "bar", "baz"];
///
///             println!(
///                 "{} found at {:?} position",
///                 search_text,
///                 items.iter().position(|i| *i == search_text)
///             );
///         }
///     }
/// }
/// ```
#[derive(Default, Clone, Visit, Reflect, Debug)]
pub struct SearchBar {
    /// Base widget of the search bar.
    pub widget: Widget,
    /// A handle of a text box widget used for text input.
    pub text_box: Handle<UiNode>,
    /// A handle of a button, that is used to clear the text.
    pub clear: Handle<UiNode>,
}

define_widget_deref!(SearchBar);

uuid_provider!(SearchBar = "23db1179-0e07-493d-98fd-2b3c0c795215");

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

/// Search bar builder creates [`SearchBar`] widget instances and adds them to the user interface.
pub struct SearchBarBuilder {
    widget_builder: WidgetBuilder,
}

impl SearchBarBuilder {
    /// Creates a new builder instance.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self { widget_builder }
    }

    /// Finishes search bar building and adds the new instance to the user interface.
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
