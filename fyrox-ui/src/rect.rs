//! Rect editor widget is used to show and edit [`Rect`] values. It shows four numeric fields: two for top left corner
//! of a rect, two for its size. See [`RectEditor`] docs for more info and usage examples.

#![warn(missing_docs)]

use crate::{
    core::{
        algebra::Vector2, math::Rect, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        visitor::prelude::*,
    },
    define_constructor,
    grid::{Column, GridBuilder, Row},
    message::{MessageDirection, UiMessage},
    numeric::NumericType,
    text::TextBuilder,
    vec::{VecEditorBuilder, VecEditorMessage},
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, Thickness, UiNode, UserInterface, VerticalAlignment,
};
use fyrox_core::variable::InheritableVariable;
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

/// A set of possible messages, that can be used to either modify or fetch the state of a [`RectEditor`] widget.
#[derive(Debug, Clone, PartialEq)]
pub enum RectEditorMessage<T>
where
    T: NumericType,
{
    /// A message, that can be used to either modify or fetch the current value of a [`RectEditor`] widget.
    Value(Rect<T>),
}

impl<T: NumericType> RectEditorMessage<T> {
    define_constructor!(
        /// Creates [`RectEditorMessage::Value`] message.
        RectEditorMessage:Value => fn value(Rect<T>), layout: false
    );
}

/// Rect editor widget is used to show and edit [`Rect`] values. It shows four numeric fields: two for top left corner
/// of a rect, two for its size.
///
/// ## Example
///
/// Rect editor can be created using [`RectEditorBuilder`], like so:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::{math::Rect, pool::Handle},
/// #     rect::RectEditorBuilder,
/// #     widget::WidgetBuilder,
/// #     BuildContext, UiNode,
/// # };
/// #
/// fn create_rect_editor(ctx: &mut BuildContext) -> Handle<UiNode> {
///     RectEditorBuilder::new(WidgetBuilder::new())
///         .with_value(Rect::new(0, 0, 10, 20))
///         .build(ctx)
/// }
/// ```
///
/// ## Value
///
/// To change the value of a rect editor, use [`RectEditorMessage::Value`] message:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::{math::Rect, pool::Handle},
/// #     message::MessageDirection,
/// #     rect::RectEditorMessage,
/// #     UiNode, UserInterface,
/// # };
/// #
/// fn change_value(rect_editor: Handle<UiNode>, ui: &UserInterface) {
///     ui.send_message(RectEditorMessage::value(
///         rect_editor,
///         MessageDirection::ToWidget,
///         Rect::new(20, 20, 60, 80),
///     ));
/// }
/// ```
///
/// To "catch" the moment when the value of a rect editor has changed, listen to the same message, but check its direction:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle,
/// #     message::{MessageDirection, UiMessage},
/// #     rect::RectEditorMessage,
/// #     UiNode,
/// # };
/// #
/// fn fetch_value(rect_editor: Handle<UiNode>, message: &UiMessage) {
///     if let Some(RectEditorMessage::Value(value)) = message.data::<RectEditorMessage<u32>>() {
///         if message.destination() == rect_editor
///             && message.direction() == MessageDirection::FromWidget
///         {
///             println!("The new value is: {:?}", value)
///         }
///     }
/// }
/// ```
#[derive(Default, Debug, Clone, Visit, Reflect, ComponentProvider)]
pub struct RectEditor<T>
where
    T: NumericType,
{
    /// Base widget of the rect editor.
    pub widget: Widget,
    /// A handle to a widget, that is used to show/edit position part of the rect.
    pub position: InheritableVariable<Handle<UiNode>>,
    /// A handle to a widget, that is used to show/edit size part of the rect.
    pub size: InheritableVariable<Handle<UiNode>>,
    /// Current value of the rect editor.
    pub value: InheritableVariable<Rect<T>>,
}

impl<T> Deref for RectEditor<T>
where
    T: NumericType,
{
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<T> DerefMut for RectEditor<T>
where
    T: NumericType,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<T> TypeUuidProvider for RectEditor<T>
where
    T: NumericType,
{
    fn type_uuid() -> Uuid {
        combine_uuids(
            uuid!("5a3daf9d-f33b-494b-b111-eb55721dc7ac"),
            T::type_uuid(),
        )
    }
}

impl<T> Control for RectEditor<T>
where
    T: NumericType,
{
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(RectEditorMessage::Value(value)) = message.data::<RectEditorMessage<T>>() {
            if message.destination() == self.handle
                && message.direction() == MessageDirection::ToWidget
                && *value != *self.value
            {
                self.value.set_value_and_mark_modified(*value);

                ui.send_message(message.reverse());
            }
        } else if let Some(VecEditorMessage::Value(value)) =
            message.data::<VecEditorMessage<T, 2>>()
        {
            if message.direction() == MessageDirection::FromWidget {
                if message.destination() == *self.position {
                    if self.value.position != *value {
                        ui.send_message(RectEditorMessage::value(
                            self.handle,
                            MessageDirection::ToWidget,
                            Rect::new(value.x, value.y, self.value.size.x, self.value.size.y),
                        ));
                    }
                } else if message.destination() == *self.size && self.value.size != *value {
                    ui.send_message(RectEditorMessage::value(
                        self.handle,
                        MessageDirection::ToWidget,
                        Rect::new(
                            self.value.position.x,
                            self.value.position.y,
                            value.x,
                            value.y,
                        ),
                    ));
                }
            }
        }
    }
}

/// Rect editor builder creates [`RectEditor`] widget instances and adds them to the user interface.
pub struct RectEditorBuilder<T>
where
    T: NumericType,
{
    widget_builder: WidgetBuilder,
    value: Rect<T>,
}

fn create_field<T: NumericType>(
    ctx: &mut BuildContext,
    name: &str,
    value: Vector2<T>,
    row: usize,
) -> (Handle<UiNode>, Handle<UiNode>) {
    let editor;
    let grid = GridBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::left(10.0))
            .on_row(row)
            .with_child(
                TextBuilder::new(WidgetBuilder::new())
                    .with_text(name)
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .build(ctx),
            )
            .with_child({
                editor = VecEditorBuilder::new(WidgetBuilder::new().on_column(1))
                    .with_value(value)
                    .build(ctx);
                editor
            }),
    )
    .add_column(Column::strict(70.0))
    .add_column(Column::stretch())
    .add_row(Row::stretch())
    .build(ctx);
    (grid, editor)
}

impl<T> RectEditorBuilder<T>
where
    T: NumericType,
{
    /// Creates new rect editor builder instance.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            value: Default::default(),
        }
    }

    /// Sets the desired value.
    pub fn with_value(mut self, value: Rect<T>) -> Self {
        self.value = value;
        self
    }

    /// Finished rect editor widget building and adds it to the user interface.
    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let (position_grid, position) = create_field(ctx, "Position", self.value.position, 0);
        let (size_grid, size) = create_field(ctx, "Size", self.value.size, 1);
        let node = RectEditor {
            widget: self
                .widget_builder
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .with_child(position_grid)
                            .with_child(size_grid),
                    )
                    .add_row(Row::stretch())
                    .add_row(Row::stretch())
                    .add_column(Column::stretch())
                    .build(ctx),
                )
                .build(),
            value: self.value.into(),
            position: position.into(),
            size: size.into(),
        };

        ctx.add_node(UiNode::new(node))
    }
}
