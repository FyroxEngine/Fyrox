use crate::{
    check_box::CheckBoxBuilder,
    core::pool::Handle,
    grid::{Column, GridBuilder, Row},
    message::{
        CheckBoxMessage, ExpanderMessage, MessageData, MessageDirection, UiMessage, UiMessageData,
        WidgetMessage,
    },
    utils::{make_arrow, ArrowDirection},
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UINode, UserInterface, VerticalAlignment,
};
use std::ops::{Deref, DerefMut};

#[derive(Clone)]
pub struct Expander<M: MessageData, C: Control<M, C>> {
    widget: Widget<M, C>,
    header: Handle<UINode<M, C>>,
    content: Handle<UINode<M, C>>,
    expander: Handle<UINode<M, C>>,
    is_expanded: bool,
}

crate::define_widget_deref!(Expander<M, C>);

impl<M: MessageData, C: Control<M, C>> Control<M, C> for Expander<M, C> {
    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        match message.data() {
            UiMessageData::Expander(msg) => {
                if let ExpanderMessage::Expand(expand) = *msg {
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
                }
            }
            UiMessageData::CheckBox(msg) => {
                if let CheckBoxMessage::Check(value) = *msg {
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
            }
            _ => {}
        }

        self.widget.handle_routed_message(ui, message);
    }
}

pub struct ExpanderBuilder<M: MessageData, C: Control<M, C>> {
    pub widget_builder: WidgetBuilder<M, C>,
    header: Handle<UINode<M, C>>,
    content: Handle<UINode<M, C>>,
    is_expanded: bool,
}

impl<M: MessageData, C: Control<M, C>> ExpanderBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            header: Handle::NONE,
            content: Handle::NONE,
            is_expanded: true,
        }
    }

    pub fn with_header(mut self, header: Handle<UINode<M, C>>) -> Self {
        self.header = header;
        self
    }

    pub fn with_content(mut self, content: Handle<UINode<M, C>>) -> Self {
        self.content = content;
        self
    }

    pub fn with_expanded(mut self, expanded: bool) -> Self {
        self.is_expanded = expanded;
        self
    }

    pub fn build(self, ctx: &mut BuildContext<'_, M, C>) -> Handle<UINode<M, C>> {
        let expander = CheckBoxBuilder::new(
            WidgetBuilder::new().with_vertical_alignment(VerticalAlignment::Center),
        )
        .with_check_mark(make_arrow(ctx, ArrowDirection::Bottom, 8.0))
        .with_uncheck_mark(make_arrow(ctx, ArrowDirection::Right, 8.0))
        .with_content(self.header)
        .checked(Some(self.is_expanded))
        .build(ctx);

        if self.content.is_some() {
            ctx[self.content]
                .set_row(1)
                .set_column(0)
                .set_visibility(self.is_expanded);
        }

        let e = UINode::Expander(Expander {
            widget: self
                .widget_builder
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .with_child(expander)
                            .with_child(self.content),
                    )
                    .add_column(Column::auto())
                    .add_row(Row::strict(24.0))
                    .add_row(Row::stretch())
                    .build(ctx),
                )
                .build(),
            header: self.header,
            content: self.content,
            expander,
            is_expanded: self.is_expanded,
        });
        ctx.add_node(e)
    }
}
