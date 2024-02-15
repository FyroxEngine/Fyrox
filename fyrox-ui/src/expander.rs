//! Expander is a simple container that has a header and collapsible/expandable content zone. It is used to
//! create collapsible regions with headers. See [`Expander`] docs for more info and usage examples.

#![warn(missing_docs)]

use crate::{
    check_box::{CheckBoxBuilder, CheckBoxMessage},
    core::pool::Handle,
    core::{reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*},
    define_constructor,
    grid::{Column, GridBuilder, Row},
    message::{MessageDirection, UiMessage},
    utils::{make_arrow, ArrowDirection},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, UiNode, UserInterface, VerticalAlignment,
};
use fyrox_core::uuid_provider;
use fyrox_core::variable::InheritableVariable;
use std::ops::{Deref, DerefMut};

/// A set messages that can be used to either alternate the state of an [`Expander`] widget, or to listen for
/// state changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpanderMessage {
    /// A message, that could be used to either switch expander state (with [`MessageDirection::ToWidget`]) or
    /// to get its new state [`MessageDirection::FromWidget`].
    Expand(bool),
}

impl ExpanderMessage {
    define_constructor!(
        /// Creates [`ExpanderMessage::Expand`] message.
        ExpanderMessage:Expand => fn expand(bool), layout: false
    );
}

/// Expander is a simple container that has a header and collapsible/expandable content zone. It is used to
/// create collapsible regions with headers.
///
/// ## Examples
///
/// The following example creates a simple expander with a textual header and a stack panel widget with few
/// buttons a content:
///
/// ```rust
/// # use fyrox_ui::{
/// #     button::ButtonBuilder, core::pool::Handle, expander::ExpanderBuilder,
/// #     stack_panel::StackPanelBuilder, text::TextBuilder, widget::WidgetBuilder, BuildContext,
/// #     UiNode,
/// # };
/// #
/// fn create_expander(ctx: &mut BuildContext) -> Handle<UiNode> {
///     ExpanderBuilder::new(WidgetBuilder::new())
///         // Header is visible all the time.
///         .with_header(
///             TextBuilder::new(WidgetBuilder::new())
///                 .with_text("Foobar")
///                 .build(ctx),
///         )
///         // Define a content of collapsible area.
///         .with_content(
///             StackPanelBuilder::new(
///                 WidgetBuilder::new()
///                     .with_child(
///                         ButtonBuilder::new(WidgetBuilder::new())
///                             .with_text("Button 1")
///                             .build(ctx),
///                     )
///                     .with_child(
///                         ButtonBuilder::new(WidgetBuilder::new())
///                             .with_text("Button 2")
///                             .build(ctx),
///                     ),
///             )
///             .build(ctx),
///         )
///         .build(ctx)
/// }
/// ```
///
/// ## Customization
///
/// It is possible to completely change the arrow of the header of the expander. By default, the arrow consists
/// of [`crate::check_box::CheckBox`] widget. By changing the arrow, you can customize the look of the header.
/// For example, you can set the new check box with image check marks, which will use custom graphics:
///
/// ```rust
/// # use fyrox_ui::{
/// #     check_box::CheckBoxBuilder, core::pool::Handle, expander::ExpanderBuilder,
/// #     image::ImageBuilder, widget::WidgetBuilder, BuildContext, UiNode,
/// # };
/// #
/// fn create_expander(ctx: &mut BuildContext) -> Handle<UiNode> {
///     ExpanderBuilder::new(WidgetBuilder::new())
///         .with_checkbox(
///             CheckBoxBuilder::new(WidgetBuilder::new())
///                 .with_check_mark(
///                     ImageBuilder::new(WidgetBuilder::new().with_height(16.0).with_height(16.0))
///                         .with_opt_texture(None) // Set this to required image.
///                         .build(ctx),
///                 )
///                 .with_uncheck_mark(
///                     ImageBuilder::new(WidgetBuilder::new().with_height(16.0).with_height(16.0))
///                         .with_opt_texture(None) // Set this to required image.
///                         .build(ctx),
///                 )
///                 .build(ctx),
///         )
///         // The rest is omitted.
///         .build(ctx)
/// }
/// ```
///
/// ## Messages
///
/// Use [`ExpanderMessage::Expand`] message to catch the moment when its state changes:
///
/// ```rust
/// # use fyrox_ui::{core::pool::Handle, expander::ExpanderMessage, message::{MessageDirection, UiMessage}};
/// fn on_ui_message(message: &UiMessage) {
///     let your_expander_handle = Handle::NONE;
///     if let Some(ExpanderMessage::Expand(expanded)) = message.data() {
///         if message.destination() == your_expander_handle && message.direction() == MessageDirection::FromWidget {
///             println!(
///                 "{} expander has changed its state to {}!",
///                 message.destination(),
///                 expanded
///             );
///         }
///     }
/// }
/// ```
///
/// To switch expander state at runtime, send [`ExpanderMessage::Expand`] to your Expander widget instance with
/// [`MessageDirection::ToWidget`].
#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider)]
pub struct Expander {
    /// Base widget of the expander.
    pub widget: Widget,
    /// Current content of the expander.
    pub content: InheritableVariable<Handle<UiNode>>,
    /// Current expander check box of the expander.
    pub expander: InheritableVariable<Handle<UiNode>>,
    /// A flag, that indicates whether the expander is expanded or collapsed.
    pub is_expanded: InheritableVariable<bool>,
}

crate::define_widget_deref!(Expander);

uuid_provider!(Expander = "24976179-b338-4c55-84c3-72d21663efd2");

impl Control for Expander {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        if let Some(&ExpanderMessage::Expand(expand)) = message.data::<ExpanderMessage>() {
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::ToWidget
                && *self.is_expanded != expand
            {
                // Switch state of expander.
                ui.send_message(CheckBoxMessage::checked(
                    *self.expander,
                    MessageDirection::ToWidget,
                    Some(expand),
                ));
                // Show or hide content.
                ui.send_message(WidgetMessage::visibility(
                    *self.content,
                    MessageDirection::ToWidget,
                    expand,
                ));
                self.is_expanded.set_value_and_mark_modified(expand);
            }
        } else if let Some(CheckBoxMessage::Check(value)) = message.data::<CheckBoxMessage>() {
            if message.destination() == *self.expander
                && message.direction() == MessageDirection::FromWidget
            {
                ui.send_message(ExpanderMessage::expand(
                    self.handle,
                    MessageDirection::ToWidget,
                    value.unwrap_or(false),
                ));
            }
        }
        self.widget.handle_routed_message(ui, message);
    }
}

/// Expander builder allows you to create [`Expander`] widgets and add them to user interface.
pub struct ExpanderBuilder {
    /// Base builder.
    pub widget_builder: WidgetBuilder,
    header: Handle<UiNode>,
    content: Handle<UiNode>,
    check_box: Handle<UiNode>,
    is_expanded: bool,
    expander_column: Option<Column>,
}

impl ExpanderBuilder {
    /// Creates a new expander builder.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            header: Handle::NONE,
            content: Handle::NONE,
            check_box: Default::default(),
            is_expanded: true,
            expander_column: None,
        }
    }

    /// Sets the desired header of the expander.
    pub fn with_header(mut self, header: Handle<UiNode>) -> Self {
        self.header = header;
        self
    }

    /// Sets the desired content of the expander.
    pub fn with_content(mut self, content: Handle<UiNode>) -> Self {
        self.content = content;
        self
    }

    /// Sets the desired state of the expander.
    pub fn with_expanded(mut self, expanded: bool) -> Self {
        self.is_expanded = expanded;
        self
    }

    /// Sets the desired check box (arrow part) of the expander.
    pub fn with_checkbox(mut self, check_box: Handle<UiNode>) -> Self {
        self.check_box = check_box;
        self
    }

    /// Sets the desired expander column properties of the expander.
    pub fn with_expander_column(mut self, expander_column: Column) -> Self {
        self.expander_column = Some(expander_column);
        self
    }

    /// Finishes widget building and adds it to the user interface, returning a handle to the new instance.
    pub fn build(self, ctx: &mut BuildContext<'_>) -> Handle<UiNode> {
        let expander = if self.check_box.is_some() {
            self.check_box
        } else {
            CheckBoxBuilder::new(
                WidgetBuilder::new().with_vertical_alignment(VerticalAlignment::Center),
            )
            .with_check_mark(make_arrow(ctx, ArrowDirection::Bottom, 8.0))
            .with_uncheck_mark(make_arrow(ctx, ArrowDirection::Right, 8.0))
            .checked(Some(self.is_expanded))
            .build(ctx)
        };

        ctx[expander].set_row(0).set_column(0);

        if self.header.is_some() {
            ctx[self.header].set_row(0).set_column(1);
        }

        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(expander)
                .with_child(self.header),
        )
        .add_row(Row::auto())
        .add_column(self.expander_column.unwrap_or_else(Column::auto))
        .add_column(Column::stretch())
        .build(ctx);

        if self.content.is_some() {
            ctx[self.content]
                .set_row(1)
                .set_column(0)
                .set_visibility(self.is_expanded);
        }

        let e = UiNode::new(Expander {
            widget: self
                .widget_builder
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .with_child(grid)
                            .with_child(self.content),
                    )
                    .add_column(Column::auto())
                    .add_row(Row::auto())
                    .add_row(Row::stretch())
                    .build(ctx),
                )
                .build(),
            content: self.content.into(),
            expander: expander.into(),
            is_expanded: self.is_expanded.into(),
        });
        ctx.add_node(e)
    }
}
