//! Checkbox is a UI widget that have three states - `Checked`, `Unchecked` and `Undefined`. In most cases it is used
//! only with two values which fits in `bool` type. Third, undefined, state is used for specific situations when your
//! data have such state. See [`CheckBox`] docs for more info and usage examples.

#![warn(missing_docs)]

use crate::{
    border::BorderBuilder,
    brush::Brush,
    core::{
        algebra::Vector2, color::Color, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        variable::InheritableVariable, visitor::prelude::*,
    },
    define_constructor,
    grid::{Column, GridBuilder, Row},
    message::{KeyCode, MessageDirection, UiMessage},
    vector_image::{Primitive, VectorImageBuilder},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, HorizontalAlignment, MouseButton, Thickness, UiNode, UserInterface,
    VerticalAlignment, BRUSH_BRIGHT, BRUSH_BRIGHT_BLUE, BRUSH_DARKEST, BRUSH_LIGHT, BRUSH_TEXT,
};
use std::ops::{Deref, DerefMut};

/// A set of possible check box messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckBoxMessage {
    /// Emitted when the check box changed its state. Could also be used to modify check box state.
    Check(Option<bool>),
}

impl CheckBoxMessage {
    define_constructor!(
        /// Creates [`CheckBoxMessage::checked`] message.
        CheckBoxMessage:Check => fn checked(Option<bool>), layout: false
    );
}

/// Checkbox is a UI widget that have three states - `Checked`, `Unchecked` and `Undefined`. In most cases it is used
/// only with two values which fits in `bool` type. Third, undefined, state is used for specific situations when your
/// data have such state.
///
/// ## How to create
///
/// To create a checkbox you should do something like this:
///
/// ```rust,no_run
/// # use fyrox_ui::{
/// #     core::pool::Handle,
/// #     check_box::CheckBoxBuilder, widget::WidgetBuilder, UiNode, UserInterface
/// # };
/// fn create_checkbox(ui: &mut UserInterface) -> Handle<UiNode> {
///     CheckBoxBuilder::new(WidgetBuilder::new())
///         // A custom value can be set during initialization.
///         .checked(Some(true))
///         .build(&mut ui.build_ctx())
/// }
/// ```
///
/// The above code will create a checkbox without any textual info, but usually checkboxes have some useful info
/// near them. To create such checkbox, you could use [`CheckBoxBuilder::with_content`] method which accepts any widget handle.
/// For checkbox with text, you could use [`crate::text::TextBuilder`] to create textual content, for checkbox with image - use
/// [`crate::image::ImageBuilder`]. As already said, you're free to use any widget handle there.
///
/// Here's an example of checkbox with textual content.
///
/// ```rust,no_run
/// # use fyrox_ui::{
/// #     core::pool::Handle,
/// #     check_box::CheckBoxBuilder, text::TextBuilder, widget::WidgetBuilder, UiNode,
/// #     UserInterface,
/// # };
/// fn create_checkbox(ui: &mut UserInterface) -> Handle<UiNode> {
///     let ctx = &mut ui.build_ctx();
///
///     CheckBoxBuilder::new(WidgetBuilder::new())
///         // A custom value can be set during initialization.
///         .checked(Some(true))
///         .with_content(
///             TextBuilder::new(WidgetBuilder::new())
///                 .with_text("This is a checkbox")
///                 .build(ctx),
///         )
///         .build(ctx)
/// }
/// ```
///
/// ## Message handling
///
/// Checkboxes are not static widget and have multiple states. To handle a message from a checkbox, you need to handle
/// the [`CheckBoxMessage::Check`] message. To do so, you can do something like this:
///
/// ```rust,no_run
/// # use fyrox_ui::{
/// #     core::pool::Handle,
/// #     check_box::CheckBoxMessage, message::UiMessage, UiNode
/// # };
/// #
/// # struct Foo {
/// #     checkbox: Handle<UiNode>,
/// # }
/// #
/// # impl Foo {
/// fn on_ui_message(
///     &mut self,
///     message: &UiMessage,
/// ) {
///     if let Some(CheckBoxMessage::Check(value)) = message.data() {
///         if message.destination() == self.checkbox {
///             //
///             // Insert your clicking handling code here.
///             //
///         }
///     }
/// }
/// # }
/// ```
///
/// Keep in mind that checkbox (as any other widget) generates [`WidgetMessage`] instances. You can catch them too and
/// do a custom handling if you need.
///
/// ## Theme
///
/// Checkbox can be fully customized to have any look you want, there are few methods that will help you with
/// customization:
///
/// 1) [`CheckBoxBuilder::with_content`] - sets the content that will be shown near the checkbox.
/// 2) [`CheckBoxBuilder::with_check_mark`] - sets the widget that will be used as checked icon.
/// 3) [`CheckBoxBuilder::with_uncheck_mark`] - sets the widget that will be used as unchecked icon.
/// 4) [`CheckBoxBuilder::with_undefined_mark`] - sets the widget that will be used as undefined icon.
#[derive(Default, Clone, Debug, Visit, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "3a866ba8-7682-4ce7-954a-46360f5837dc")]
pub struct CheckBox {
    /// Base widget of the check box.
    pub widget: Widget,
    /// Current state of the check box.
    pub checked: InheritableVariable<Option<bool>>,
    /// Check mark that is used when the state is `Some(true)`.
    pub check_mark: InheritableVariable<Handle<UiNode>>,
    /// Check mark that is used when the state is `Some(false)`.
    pub uncheck_mark: InheritableVariable<Handle<UiNode>>,
    /// Check mark that is used when the state is `None`.
    pub undefined_mark: InheritableVariable<Handle<UiNode>>,
}

crate::define_widget_deref!(CheckBox);

impl Control for CheckBox {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<WidgetMessage>() {
            match msg {
                WidgetMessage::MouseDown { button, .. } => {
                    if *button == MouseButton::Left
                        && (message.destination() == self.handle()
                            || self.widget.has_descendant(message.destination(), ui))
                    {
                        ui.capture_mouse(self.handle());
                    }
                }
                WidgetMessage::MouseUp { button, .. } => {
                    if *button == MouseButton::Left
                        && (message.destination() == self.handle()
                            || self.widget.has_descendant(message.destination(), ui))
                    {
                        ui.release_mouse_capture();

                        if let Some(value) = *self.checked {
                            // Invert state if it is defined.
                            ui.send_message(CheckBoxMessage::checked(
                                self.handle(),
                                MessageDirection::ToWidget,
                                Some(!value),
                            ));
                        } else {
                            // Switch from undefined state to checked.
                            ui.send_message(CheckBoxMessage::checked(
                                self.handle(),
                                MessageDirection::ToWidget,
                                Some(true),
                            ));
                        }
                    }
                }
                WidgetMessage::KeyDown(key_code) => {
                    if !message.handled() && *key_code == KeyCode::Space {
                        ui.send_message(CheckBoxMessage::checked(
                            self.handle,
                            MessageDirection::ToWidget,
                            self.checked.map(|checked| !checked),
                        ));
                        message.set_handled(true);
                    }
                }
                _ => (),
            }
        } else if let Some(&CheckBoxMessage::Check(value)) = message.data::<CheckBoxMessage>() {
            if message.direction() == MessageDirection::ToWidget
                && message.destination() == self.handle()
                && *self.checked != value
            {
                self.checked.set_value_and_mark_modified(value);

                ui.send_message(message.reverse());

                if self.check_mark.is_some() {
                    match value {
                        None => {
                            ui.send_message(WidgetMessage::visibility(
                                *self.check_mark,
                                MessageDirection::ToWidget,
                                false,
                            ));
                            ui.send_message(WidgetMessage::visibility(
                                *self.uncheck_mark,
                                MessageDirection::ToWidget,
                                false,
                            ));
                            ui.send_message(WidgetMessage::visibility(
                                *self.undefined_mark,
                                MessageDirection::ToWidget,
                                true,
                            ));
                        }
                        Some(value) => {
                            ui.send_message(WidgetMessage::visibility(
                                *self.check_mark,
                                MessageDirection::ToWidget,
                                value,
                            ));
                            ui.send_message(WidgetMessage::visibility(
                                *self.uncheck_mark,
                                MessageDirection::ToWidget,
                                !value,
                            ));
                            ui.send_message(WidgetMessage::visibility(
                                *self.undefined_mark,
                                MessageDirection::ToWidget,
                                false,
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Check box builder creates [`CheckBox`] instances and adds them to the user interface.
pub struct CheckBoxBuilder {
    widget_builder: WidgetBuilder,
    checked: Option<bool>,
    check_mark: Option<Handle<UiNode>>,
    uncheck_mark: Option<Handle<UiNode>>,
    undefined_mark: Option<Handle<UiNode>>,
    background: Option<Handle<UiNode>>,
    content: Handle<UiNode>,
}

impl CheckBoxBuilder {
    /// Creates new check box builder instance.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            checked: Some(false),
            check_mark: None,
            uncheck_mark: None,
            undefined_mark: None,
            content: Handle::NONE,
            background: None,
        }
    }

    /// Sets the desired state of the check box.
    pub fn checked(mut self, value: Option<bool>) -> Self {
        self.checked = value;
        self
    }

    /// Sets the desired check mark when the state is `Some(true)`.
    pub fn with_check_mark(mut self, check_mark: Handle<UiNode>) -> Self {
        self.check_mark = Some(check_mark);
        self
    }

    /// Sets the desired check mark when the state is `Some(false)`.
    pub fn with_uncheck_mark(mut self, uncheck_mark: Handle<UiNode>) -> Self {
        self.uncheck_mark = Some(uncheck_mark);
        self
    }

    /// Sets the desired check mark when the state is `None`.
    pub fn with_undefined_mark(mut self, undefined_mark: Handle<UiNode>) -> Self {
        self.undefined_mark = Some(undefined_mark);
        self
    }

    /// Sets the new content of the check box.
    pub fn with_content(mut self, content: Handle<UiNode>) -> Self {
        self.content = content;
        self
    }

    /// Sets the desired background widget that will be used a container for check box contents. By
    /// default, it is a simple border.
    pub fn with_background(mut self, background: Handle<UiNode>) -> Self {
        self.background = Some(background);
        self
    }

    /// Finishes check box building and adds it to the user interface.
    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let check_mark = self.check_mark.unwrap_or_else(|| {
            BorderBuilder::new(
                WidgetBuilder::new()
                    .with_background(BRUSH_BRIGHT_BLUE)
                    .with_child(
                        VectorImageBuilder::new(
                            WidgetBuilder::new()
                                .with_margin(Thickness::uniform(3.0))
                                .with_vertical_alignment(VerticalAlignment::Center)
                                .with_horizontal_alignment(HorizontalAlignment::Center)
                                .with_foreground(BRUSH_TEXT),
                        )
                        .with_primitives({
                            let size = 8.0;
                            let half_size = size * 0.5;
                            vec![
                                Primitive::Line {
                                    begin: Vector2::new(0.0, half_size),
                                    end: Vector2::new(half_size, size),
                                    thickness: 2.0,
                                },
                                Primitive::Line {
                                    begin: Vector2::new(half_size, size),
                                    end: Vector2::new(size, 0.0),
                                    thickness: 2.0,
                                },
                            ]
                        })
                        .build(ctx),
                    ),
            )
            .with_pad_by_corner_radius(false)
            .with_corner_radius(3.0)
            .with_stroke_thickness(Thickness::uniform(0.0))
            .build(ctx)
        });
        ctx[check_mark].set_visibility(self.checked.unwrap_or(false));

        let uncheck_mark = self.uncheck_mark.unwrap_or_else(|| {
            BorderBuilder::new(
                WidgetBuilder::new()
                    .with_margin(Thickness::uniform(3.0))
                    .with_width(10.0)
                    .with_height(9.0)
                    .with_background(Brush::Solid(Color::TRANSPARENT))
                    .with_foreground(Brush::Solid(Color::TRANSPARENT)),
            )
            .with_pad_by_corner_radius(false)
            .with_corner_radius(3.0)
            .with_stroke_thickness(Thickness::uniform(0.0))
            .build(ctx)
        });
        ctx[uncheck_mark].set_visibility(!self.checked.unwrap_or(true));

        let undefined_mark = self.undefined_mark.unwrap_or_else(|| {
            BorderBuilder::new(
                WidgetBuilder::new()
                    .with_margin(Thickness::uniform(4.0))
                    .with_background(BRUSH_BRIGHT)
                    .with_foreground(Brush::Solid(Color::TRANSPARENT)),
            )
            .with_pad_by_corner_radius(false)
            .with_corner_radius(3.0)
            .build(ctx)
        });
        ctx[undefined_mark].set_visibility(self.checked.is_none());

        if self.content.is_some() {
            ctx[self.content].set_row(0).set_column(1);
        }

        let background = self.background.unwrap_or_else(|| {
            BorderBuilder::new(
                WidgetBuilder::new()
                    .with_vertical_alignment(VerticalAlignment::Center)
                    .with_background(BRUSH_DARKEST)
                    .with_foreground(BRUSH_LIGHT),
            )
            .with_pad_by_corner_radius(false)
            .with_corner_radius(3.0)
            .with_stroke_thickness(Thickness::uniform(1.0))
            .build(ctx)
        });

        let background_ref = &mut ctx[background];
        background_ref.set_row(0).set_column(0);
        if background_ref.min_width() < 0.01 {
            background_ref.set_min_width(16.0);
        }
        if background_ref.min_height() < 0.01 {
            background_ref.set_min_height(16.0);
        }

        ctx.link(check_mark, background);
        ctx.link(uncheck_mark, background);
        ctx.link(undefined_mark, background);

        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(background)
                .with_child(self.content),
        )
        .add_row(Row::stretch())
        .add_column(Column::auto())
        .add_column(Column::auto())
        .build(ctx);

        let cb = CheckBox {
            widget: self
                .widget_builder
                .with_accepts_input(true)
                .with_child(grid)
                .build(),
            checked: self.checked.into(),
            check_mark: check_mark.into(),
            uncheck_mark: uncheck_mark.into(),
            undefined_mark: undefined_mark.into(),
        };
        ctx.add_node(UiNode::new(cb))
    }
}

#[cfg(test)]
mod test {
    use crate::{
        check_box::{CheckBoxBuilder, CheckBoxMessage},
        message::MessageDirection,
        widget::WidgetBuilder,
        UserInterface,
    };
    use fyrox_core::algebra::Vector2;

    #[test]
    fn check_box() {
        let mut ui = UserInterface::new(Vector2::new(100.0, 100.0));

        assert_eq!(ui.poll_message(), None);

        let check_box = CheckBoxBuilder::new(WidgetBuilder::new()).build(&mut ui.build_ctx());

        assert_eq!(ui.poll_message(), None);

        // Check messages
        let input_message =
            CheckBoxMessage::checked(check_box, MessageDirection::ToWidget, Some(true));

        ui.send_message(input_message.clone());

        // This message that we just send.
        assert_eq!(ui.poll_message(), Some(input_message.clone()));
        // We must get response from check box.
        assert_eq!(ui.poll_message(), Some(input_message.reverse()));
    }
}
