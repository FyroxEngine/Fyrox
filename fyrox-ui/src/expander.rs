use crate::{
    check_box::{CheckBoxBuilder, CheckBoxMessage},
    core::pool::Handle,
    define_constructor,
    grid::{Column, GridBuilder, Row},
    message::{MessageDirection, UiMessage},
    utils::{make_arrow, ArrowDirection},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, UiNode, UserInterface, VerticalAlignment,
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpanderMessage {
    Expand(bool),
}

impl ExpanderMessage {
    define_constructor!(Self:Expand => fn expand(bool), layout: false);
}

#[derive(Clone)]
pub struct Expander {
    pub widget: Widget,
    pub content: Handle<UiNode>,
    pub expander: Handle<UiNode>,
    pub is_expanded: bool,
}

crate::define_widget_deref!(Expander);

impl Control for Expander {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        if let Some(&ExpanderMessage::Expand(expand)) = message.data::<ExpanderMessage>() {
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::ToWidget
                && self.is_expanded != expand
            {
                // Switch state of expander.
                ui.send_message(CheckBoxMessage::checked(
                    self.expander,
                    MessageDirection::ToWidget,
                    Some(expand),
                ));
                // Show or hide content.
                ui.send_message(WidgetMessage::visibility(
                    self.content,
                    MessageDirection::ToWidget,
                    expand,
                ));
                self.is_expanded = expand;
            }
        } else if let Some(CheckBoxMessage::Check(value)) = message.data::<CheckBoxMessage>() {
            if message.destination() == self.expander
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

pub struct ExpanderBuilder {
    pub widget_builder: WidgetBuilder,
    header: Handle<UiNode>,
    content: Handle<UiNode>,
    check_box: Handle<UiNode>,
    is_expanded: bool,
    expander_column: Option<Column>,
}

impl ExpanderBuilder {
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

    pub fn with_header(mut self, header: Handle<UiNode>) -> Self {
        self.header = header;
        self
    }

    pub fn with_content(mut self, content: Handle<UiNode>) -> Self {
        self.content = content;
        self
    }

    pub fn with_expanded(mut self, expanded: bool) -> Self {
        self.is_expanded = expanded;
        self
    }

    pub fn with_checkbox(mut self, check_box: Handle<UiNode>) -> Self {
        self.check_box = check_box;
        self
    }

    pub fn with_expander_column(mut self, expander_column: Column) -> Self {
        self.expander_column = Some(expander_column);
        self
    }

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
            content: self.content,
            expander,
            is_expanded: self.is_expanded,
        });
        ctx.add_node(e)
    }
}
