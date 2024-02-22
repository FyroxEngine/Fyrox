//! Range editor is used to display and edit closed ranges like `0..1`. See [`Range`] docs for more info and usage
//! examples.

#![warn(missing_docs)]

use crate::{
    core::{pool::Handle, reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*},
    define_constructor,
    grid::{Column, GridBuilder, Row},
    message::{MessageDirection, UiMessage},
    numeric::{NumericType, NumericUpDownBuilder, NumericUpDownMessage},
    text::TextBuilder,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, Thickness, UiNode, UserInterface, VerticalAlignment,
};
use fyrox_core::variable::InheritableVariable;
use std::ops::{Deref, DerefMut, Range};

/// A set of messages, that can be used to modify/fetch the state of a [`RangeEditor`] widget instance.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RangeEditorMessage<T>
where
    T: NumericType,
{
    /// A message, that is used to either modifying or fetching the value of a [`RangeEditor`] widget instance.
    Value(Range<T>),
}

impl<T: NumericType> RangeEditorMessage<T> {
    define_constructor!(
        /// Creates [`RangeEditorMessage::Value`] message.
        RangeEditorMessage:Value => fn value(Range<T>), layout: false
    );
}

/// Range editor is used to display and edit closed ranges like `0..1`. The widget is generic over numeric type,
/// so you can display and editor ranges of any type, such as `u32`, `f32`, `f64`, etc.
///
/// ## Example
///
/// You can create range editors using [`RangeEditorBuilder`], like so:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle, range::RangeEditorBuilder, widget::WidgetBuilder, BuildContext, UiNode,
/// # };
/// fn create_range_editor(ctx: &mut BuildContext) -> Handle<UiNode> {
///     RangeEditorBuilder::new(WidgetBuilder::new())
///         .with_value(0u32..100u32)
///         .build(ctx)
/// }
/// ```
///
/// This example creates an editor for `Range<u32>` type with `0..100` value.
///
/// ## Value
///
/// To change current value of a range editor, use [`RangeEditorMessage::Value`] message:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle, message::MessageDirection, range::RangeEditorMessage, UiNode,
/// #     UserInterface,
/// # };
/// fn change_value(range_editor: Handle<UiNode>, ui: &UserInterface) {
///     ui.send_message(RangeEditorMessage::value(
///         range_editor,
///         MessageDirection::ToWidget,
///         5u32..20u32,
///     ))
/// }
/// ```
///
/// To "catch" the moment when the value has changed, use the same message, but check for [`MessageDirection::FromWidget`] direction
/// on the message:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle,
/// #     message::{MessageDirection, UiMessage},
/// #     range::RangeEditorMessage,
/// #     UiNode,
/// # };
/// #
/// fn fetch_value(range_editor: Handle<UiNode>, message: &UiMessage) {
///     if let Some(RangeEditorMessage::Value(range)) = message.data::<RangeEditorMessage<u32>>() {
///         if message.destination() == range_editor
///             && message.direction() == MessageDirection::FromWidget
///         {
///             println!("The new value is: {:?}", range)
///         }
///     }
/// }
/// ```
///
/// Be very careful about the type of the range when sending a message, you need to send a range of exact type, that match the type
/// of your editor, otherwise the message have no effect. The same applied to fetching.
#[derive(Default, Debug, Clone, Reflect, Visit, ComponentProvider)]
pub struct RangeEditor<T>
where
    T: NumericType,
{
    /// Base widget of the range editor.
    pub widget: Widget,
    /// Current value of the range editor.
    pub value: InheritableVariable<Range<T>>,
    /// A handle to numeric field that is used to show/modify start value of current range.
    pub start: InheritableVariable<Handle<UiNode>>,
    /// A handle to numeric field that is used to show/modify end value of current range.
    pub end: InheritableVariable<Handle<UiNode>>,
}

impl<T> Deref for RangeEditor<T>
where
    T: NumericType,
{
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<T> DerefMut for RangeEditor<T>
where
    T: NumericType,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

const SYNC_FLAG: u64 = 1;

impl<T> TypeUuidProvider for RangeEditor<T>
where
    T: NumericType,
{
    fn type_uuid() -> Uuid {
        combine_uuids(
            uuid!("0eb2948e-8485-490e-8719-18a0bb6fe275"),
            T::type_uuid(),
        )
    }
}

impl<T> Control for RangeEditor<T>
where
    T: NumericType,
{
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if message.direction() == MessageDirection::ToWidget && message.flags != SYNC_FLAG {
            if let Some(RangeEditorMessage::Value(range)) = message.data::<RangeEditorMessage<T>>()
            {
                if message.destination() == self.handle && *self.value != *range {
                    self.value.set_value_and_mark_modified(range.clone());

                    ui.send_message(NumericUpDownMessage::value(
                        *self.start,
                        MessageDirection::ToWidget,
                        range.start,
                    ));
                    ui.send_message(NumericUpDownMessage::value(
                        *self.end,
                        MessageDirection::ToWidget,
                        range.end,
                    ));

                    ui.send_message(message.reverse());
                }
            } else if let Some(NumericUpDownMessage::Value(value)) =
                message.data::<NumericUpDownMessage<T>>()
            {
                if message.destination() == *self.start {
                    if *value < self.value.end {
                        ui.send_message(RangeEditorMessage::value(
                            self.handle,
                            MessageDirection::ToWidget,
                            Range {
                                start: *value,
                                end: self.value.end,
                            },
                        ));
                    } else {
                        let mut msg = NumericUpDownMessage::value(
                            *self.start,
                            MessageDirection::ToWidget,
                            self.value.end,
                        );
                        msg.flags = SYNC_FLAG;
                        ui.send_message(msg);
                    }
                } else if message.destination() == *self.end {
                    if *value > self.value.start {
                        ui.send_message(RangeEditorMessage::value(
                            self.handle,
                            MessageDirection::ToWidget,
                            Range {
                                start: self.value.start,
                                end: *value,
                            },
                        ));
                    } else {
                        let mut msg = NumericUpDownMessage::value(
                            *self.end,
                            MessageDirection::ToWidget,
                            self.value.start,
                        );
                        msg.flags = SYNC_FLAG;
                        ui.send_message(msg);
                    }
                }
            }
        }
    }
}

/// Range editor builder creates [`RangeEditor`] instances and adds them to the user interface.
pub struct RangeEditorBuilder<T>
where
    T: NumericType,
{
    widget_builder: WidgetBuilder,
    value: Range<T>,
}

impl<T> RangeEditorBuilder<T>
where
    T: NumericType,
{
    /// Creates new builder instance.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            value: Range::default(),
        }
    }

    /// Sets a desired value of the editor.
    pub fn with_value(mut self, value: Range<T>) -> Self {
        self.value = value;
        self
    }

    /// Finished widget building and adds the new instance to the user interface.
    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let start = NumericUpDownBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .on_column(0),
        )
        .with_value(self.value.start)
        .build(ctx);
        let end = NumericUpDownBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .on_column(2),
        )
        .with_value(self.value.end)
        .build(ctx);
        let editor = RangeEditor {
            widget: self
                .widget_builder
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .with_child(start)
                            .with_child(
                                TextBuilder::new(
                                    WidgetBuilder::new()
                                        .on_column(1)
                                        .with_margin(Thickness::uniform(1.0)),
                                )
                                .with_vertical_text_alignment(VerticalAlignment::Center)
                                .with_text("..")
                                .build(ctx),
                            )
                            .with_child(end),
                    )
                    .add_column(Column::stretch())
                    .add_column(Column::strict(10.0))
                    .add_column(Column::stretch())
                    .add_row(Row::stretch())
                    .build(ctx),
                )
                .build(),
            value: self.value.into(),
            start: start.into(),
            end: end.into(),
        };

        ctx.add_node(UiNode::new(editor))
    }
}
