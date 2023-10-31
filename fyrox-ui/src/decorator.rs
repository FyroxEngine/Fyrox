//! A visual element that is used to highlight standard states of interactive widgets. It has "pressed", "hover",
//! "selected", "normal" appearances. See [`Decorator`] docs for more info and usage examples.

#![warn(missing_docs)]

use crate::{
    border::{Border, BorderBuilder},
    brush::Brush,
    core::{algebra::Vector2, pool::Handle},
    core::{reflect::prelude::*, visitor::prelude::*},
    define_constructor,
    draw::DrawingContext,
    message::{MessageDirection, UiMessage},
    widget::{Widget, WidgetMessage},
    BuildContext, Control, NodeHandleMapping, UiNode, UserInterface, BRUSH_BRIGHT, BRUSH_DARKER,
    BRUSH_LIGHTEST,
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
    sync::mpsc::Sender,
};

/// A set of messages that is used to modify [`Decorator`] widgets state.
#[derive(Debug, Clone, PartialEq)]
pub enum DecoratorMessage {
    /// This message is used to switch a decorator in a `Selected` state or not.
    Select(bool),
    /// Sets a new brush for `Hovered` state.
    HoverBrush(Brush),
    /// Sets a new brush for `Normal` state.
    NormalBrush(Brush),
    /// Sets a new brush for `Pressed` state.
    PressedBrush(Brush),
    /// Sets a new brush for `Selected` state.
    SelectedBrush(Brush),
}

impl DecoratorMessage {
    define_constructor!(
        /// Creates a [`DecoratorMessage::Select`] message.
        DecoratorMessage:Select => fn select(bool), layout: false
    );
    define_constructor!(
        /// Creates a [`DecoratorMessage::HoverBrush`] message.
        DecoratorMessage:HoverBrush => fn hover_brush(Brush), layout: false
    );
    define_constructor!(
        /// Creates a [`DecoratorMessage::NormalBrush`] message.
        DecoratorMessage:NormalBrush => fn normal_brush(Brush), layout: false
    );
    define_constructor!(
        /// Creates a [`DecoratorMessage::PressedBrush`] message.
        DecoratorMessage:PressedBrush => fn pressed_brush(Brush), layout: false
    );
    define_constructor!(
        /// Creates a [`DecoratorMessage::SelectedBrush`] message.
        DecoratorMessage:SelectedBrush => fn selected_brush(Brush), layout: false
    );
}

/// A visual element that is used to highlight standard states of interactive widgets. It has "pressed", "hover",
/// "selected", "normal" appearances (only one can be active at a time):
///
/// - `Pressed` - enables on mouse down message.
/// - `Selected` - whether decorator selected or not.
/// - `Hovered` - mouse is over decorator.
/// - `Normal` - not selected, pressed, hovered.
///
/// This element is widely used to provide some generic visual behaviour for various widgets. For example it used
/// to decorate buttons - it has use of three of these states. When it is clicked - the decorator will be in `Pressed`
/// state, when hovered by a cursor - `Hovered`, otherwise it stays in `Normal` state.
///
/// ## Example
///
/// ```rust
/// # use fyrox_ui::{
/// #     border::BorderBuilder,
/// #     brush::Brush,
/// #     core::{color::Color, pool::Handle},
/// #     decorator::DecoratorBuilder,
/// #     widget::WidgetBuilder,
/// #     BuildContext, UiNode,
/// # };
/// fn create_decorator(ctx: &mut BuildContext) -> Handle<UiNode> {
///     DecoratorBuilder::new(BorderBuilder::new(WidgetBuilder::new()))
///         .with_hover_brush(Brush::Solid(Color::opaque(0, 255, 0)))
///         .build(ctx)
/// }
/// ```
#[derive(Clone, Visit, Reflect, Debug)]
pub struct Decorator {
    /// Base widget of the decorator.
    pub border: Border,

    /// Current brush used for `Pressed` state.
    pub pressed_brush: Brush,
    /// Current brush used for `Selected` state.
    pub selected_brush: Brush,
    /// Whether the decorator is in `Selected` state or not.
    pub is_selected: bool,
    /// Whether the decorator should react to mouse clicks and switch its state to `Pressed` or not.
    pub is_pressable: bool,
}

impl Deref for Decorator {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.border
    }
}

impl DerefMut for Decorator {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.border
    }
}

impl Control for Decorator {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        self.border.query_component(type_id).or_else(|| {
            if type_id == TypeId::of::<Self>() {
                Some(self)
            } else {
                None
            }
        })
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        self.border.resolve(node_map)
    }

    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        self.border.measure_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        self.border.arrange_override(ui, final_size)
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        self.border.draw(drawing_context)
    }

    fn update(&mut self, dt: f32, sender: &Sender<UiMessage>) {
        self.border.update(dt, sender)
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.border.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<DecoratorMessage>() {
            match msg {
                &DecoratorMessage::Select(value) => {
                    if self.is_selected != value {
                        self.is_selected = value;
                        if self.is_selected {
                            ui.send_message(WidgetMessage::background(
                                self.handle(),
                                MessageDirection::ToWidget,
                                self.selected_brush.clone(),
                            ));
                        } else {
                            self.palette.set_normal(self.handle(), ui);
                        }
                    }
                }

                DecoratorMessage::PressedBrush(brush) => {
                    self.pressed_brush = brush.clone();
                }
                DecoratorMessage::SelectedBrush(brush) => {
                    self.selected_brush = brush.clone();
                    if self.is_selected {
                        ui.send_message(WidgetMessage::background(
                            self.handle(),
                            MessageDirection::ToWidget,
                            self.selected_brush.clone(),
                        ));
                    }
                }

                // Backward compatibility (Updates colors on underlying palette)
                DecoratorMessage::HoverBrush(hover_brush) => {
                    self.palette.background_hover = hover_brush.clone();
                }
                DecoratorMessage::NormalBrush(normal_brush) => {
                    self.palette.background_normal = normal_brush.clone();
                }
            }
        } else if let Some(msg) = message.data::<WidgetMessage>() {
            if message.destination() == self.handle()
                || self.has_descendant(message.destination(), ui)
            {
                match msg {
                    WidgetMessage::MouseLeave => {
                        if self.is_selected {
                            ui.send_message(WidgetMessage::background(
                                self.handle(),
                                MessageDirection::ToWidget,
                                self.selected_brush.clone(),
                            ));
                        }
                    }
                    WidgetMessage::MouseDown { .. } if self.is_pressable => {
                        ui.send_message(WidgetMessage::background(
                            self.handle(),
                            MessageDirection::ToWidget,
                            self.pressed_brush.clone(),
                        ));
                    }
                    WidgetMessage::MouseUp { .. } => {
                        if self.is_selected {
                            ui.send_message(WidgetMessage::background(
                                self.handle(),
                                MessageDirection::ToWidget,
                                self.selected_brush.clone(),
                            ));
                        } else {
                            self.border.palette.set_normal(self.handle(), ui);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Creates [`Decorator`] widget instances and adds them to the user interface.
pub struct DecoratorBuilder {
    border_builder: BorderBuilder,
    pressed_brush: Brush,
    selected_brush: Brush,
    pressable: bool,
    selected: bool,
}

impl DecoratorBuilder {
    /// Creates a new decorator builder.
    pub fn new(border_builder: BorderBuilder) -> Self {
        Self {
            border_builder,
            pressed_brush: BRUSH_LIGHTEST,
            selected_brush: BRUSH_BRIGHT,
            pressable: true,
            selected: false,
        }
    }

    /// Sets a desired brush for `Normal` state.
    ///
    /// ## Backward compatibility
    ///
    /// For future use you should use the `with_palette` along with
    /// a `WidgetPaletteBuilder` on the underlying widget
    pub fn with_normal_brush(mut self, brush: Brush) -> Self {
        self.border_builder.widget_builder.palette.background_normal = Some(brush);
        self
    }

    /// Sets a desired brush for `Hovered` state.
    ///
    /// ## Backward compatibility
    ///
    /// For future use you should use the `with_palette` along with
    /// a `WidgetPaletteBuilder` on the underlying widget
    pub fn with_hover_brush(mut self, brush: Brush) -> Self {
        self.border_builder.widget_builder.palette.background_hover = Some(brush);

        self
    }

    /// Sets a desired brush for `Pressed` state.
    pub fn with_pressed_brush(mut self, brush: Brush) -> Self {
        self.pressed_brush = brush;
        self
    }

    /// Sets a desired brush for `Selected` state.
    pub fn with_selected_brush(mut self, brush: Brush) -> Self {
        self.selected_brush = brush;
        self
    }

    /// Sets whether the decorator is pressable or not.
    pub fn with_pressable(mut self, pressable: bool) -> Self {
        self.pressable = pressable;
        self
    }

    /// Sets whether the decorator is selected or not.
    pub fn with_selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Finishes decorator instance building.
    pub fn build(mut self, ui: &mut BuildContext) -> Handle<UiNode> {
        let selected_brush = self.selected_brush;

        if self
            .border_builder
            .widget_builder
            .palette
            .foreground_normal
            .is_none()
        {
            self.border_builder.widget_builder.palette.foreground_normal = Some(BRUSH_DARKER);
        }

        let mut border = self.border_builder.build_border();

        if self.selected {
            border.set_background(selected_brush.clone());
        }
        let node = UiNode::new(Decorator {
            border,

            pressed_brush: self.pressed_brush,
            selected_brush,
            is_selected: self.selected,
            is_pressable: self.pressable,
        });
        ui.add_node(node)
    }
}
