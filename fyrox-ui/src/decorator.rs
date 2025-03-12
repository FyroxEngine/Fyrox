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

//! A visual element that is used to highlight standard states of interactive widgets. It has "pressed", "hover",
//! "selected", "normal" appearances. See [`Decorator`] docs for more info and usage examples.

#![warn(missing_docs)]

use crate::style::resource::StyleResourceExt;
use crate::style::{Style, StyledProperty};
use crate::widget::WidgetBuilder;
use crate::{
    border::{Border, BorderBuilder},
    brush::Brush,
    core::{
        algebra::Vector2, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        visitor::prelude::*,
    },
    define_constructor,
    draw::DrawingContext,
    message::{MessageDirection, UiMessage},
    widget::{Widget, WidgetMessage},
    BuildContext, Control, UiNode, UserInterface,
};

use fyrox_core::uuid_provider;
use fyrox_core::variable::InheritableVariable;
use fyrox_graph::constructor::{ConstructorProvider, GraphNodeConstructor};
use std::ops::{Deref, DerefMut};

/// A set of messages that is used to modify [`Decorator`] widgets state.
#[derive(Debug, Clone, PartialEq)]
pub enum DecoratorMessage {
    /// This message is used to switch a decorator in a `Selected` state or not.
    Select(bool),
    /// Sets a new brush for `Hovered` state.
    HoverBrush(StyledProperty<Brush>),
    /// Sets a new brush for `Normal` state.
    NormalBrush(StyledProperty<Brush>),
    /// Sets a new brush for `Pressed` state.
    PressedBrush(StyledProperty<Brush>),
    /// Sets a new brush for `Selected` state.
    SelectedBrush(StyledProperty<Brush>),
}

impl DecoratorMessage {
    define_constructor!(
        /// Creates a [`DecoratorMessage::Select`] message.
        DecoratorMessage:Select => fn select(bool), layout: false
    );
    define_constructor!(
        /// Creates a [`DecoratorMessage::HoverBrush`] message.
        DecoratorMessage:HoverBrush => fn hover_brush(StyledProperty<Brush>), layout: false
    );
    define_constructor!(
        /// Creates a [`DecoratorMessage::NormalBrush`] message.
        DecoratorMessage:NormalBrush => fn normal_brush(StyledProperty<Brush>), layout: false
    );
    define_constructor!(
        /// Creates a [`DecoratorMessage::PressedBrush`] message.
        DecoratorMessage:PressedBrush => fn pressed_brush(StyledProperty<Brush>), layout: false
    );
    define_constructor!(
        /// Creates a [`DecoratorMessage::SelectedBrush`] message.
        DecoratorMessage:SelectedBrush => fn selected_brush(StyledProperty<Brush>), layout: false
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
///         .with_hover_brush(Brush::Solid(Color::opaque(0, 255, 0)).into())
///         .build(ctx)
/// }
/// ```
#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct Decorator {
    /// Base widget of the decorator.
    #[component(include)]
    pub border: Border,
    /// Current brush used for `Normal` state.
    pub normal_brush: InheritableVariable<StyledProperty<Brush>>,
    /// Current brush used for `Hovered` state.
    pub hover_brush: InheritableVariable<StyledProperty<Brush>>,
    /// Current brush used for `Pressed` state.
    pub pressed_brush: InheritableVariable<StyledProperty<Brush>>,
    /// Current brush used for `Selected` state.
    pub selected_brush: InheritableVariable<StyledProperty<Brush>>,
    /// Whether the decorator is in `Selected` state or not.
    pub is_selected: InheritableVariable<bool>,
    /// Whether the decorator should react to mouse clicks and switch its state to `Pressed` or not.
    pub is_pressable: InheritableVariable<bool>,
}

impl ConstructorProvider<UiNode, UserInterface> for Decorator {
    fn constructor() -> GraphNodeConstructor<UiNode, UserInterface> {
        GraphNodeConstructor::new::<Self>()
            .with_variant("Decorator", |ui| {
                DecoratorBuilder::new(BorderBuilder::new(
                    WidgetBuilder::new().with_name("Decorator"),
                ))
                .build(&mut ui.build_ctx())
                .into()
            })
            .with_group("Visual")
    }
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

uuid_provider!(Decorator = "bb4b60aa-c657-4ed6-8db6-d7f374397c73");

impl Control for Decorator {
    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        self.border.measure_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        self.border.arrange_override(ui, final_size)
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        self.border.draw(drawing_context)
    }

    fn update(&mut self, dt: f32, ui: &mut UserInterface) {
        self.border.update(dt, ui)
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.border.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<DecoratorMessage>() {
            match msg {
                &DecoratorMessage::Select(value) => {
                    if *self.is_selected != value {
                        self.is_selected.set_value_and_mark_modified(value);

                        ui.send_message(WidgetMessage::background(
                            self.handle(),
                            MessageDirection::ToWidget,
                            if *self.is_selected {
                                (*self.selected_brush).clone()
                            } else {
                                (*self.normal_brush).clone()
                            },
                        ));
                    }
                }
                DecoratorMessage::HoverBrush(brush) => {
                    self.hover_brush.set_value_and_mark_modified(brush.clone());
                    if self.has_descendant(ui.picked_node, ui) {
                        ui.send_message(WidgetMessage::background(
                            self.handle(),
                            MessageDirection::ToWidget,
                            (*self.hover_brush).clone(),
                        ));
                    }
                }
                DecoratorMessage::NormalBrush(brush) => {
                    self.normal_brush.set_value_and_mark_modified(brush.clone());
                    if !*self.is_selected && !self.has_descendant(ui.picked_node, ui) {
                        ui.send_message(WidgetMessage::background(
                            self.handle(),
                            MessageDirection::ToWidget,
                            (*self.normal_brush).clone(),
                        ));
                    }
                }
                DecoratorMessage::PressedBrush(brush) => {
                    self.pressed_brush
                        .set_value_and_mark_modified(brush.clone());
                }
                DecoratorMessage::SelectedBrush(brush) => {
                    self.selected_brush
                        .set_value_and_mark_modified(brush.clone());
                    if *self.is_selected {
                        ui.send_message(WidgetMessage::background(
                            self.handle(),
                            MessageDirection::ToWidget,
                            (*self.selected_brush).clone(),
                        ));
                    }
                }
            }
        } else if let Some(msg) = message.data::<WidgetMessage>() {
            if message.destination() == self.handle()
                || self.has_descendant(message.destination(), ui)
            {
                match msg {
                    WidgetMessage::MouseLeave => {
                        ui.send_message(WidgetMessage::background(
                            self.handle(),
                            MessageDirection::ToWidget,
                            if *self.is_selected {
                                (*self.selected_brush).clone()
                            } else {
                                (*self.normal_brush).clone()
                            },
                        ));
                    }
                    WidgetMessage::MouseEnter => {
                        ui.send_message(WidgetMessage::background(
                            self.handle(),
                            MessageDirection::ToWidget,
                            if *self.is_selected {
                                (*self.selected_brush).clone()
                            } else {
                                (*self.hover_brush).clone()
                            },
                        ));
                    }
                    WidgetMessage::MouseDown { .. } if *self.is_pressable => {
                        ui.send_message(WidgetMessage::background(
                            self.handle(),
                            MessageDirection::ToWidget,
                            (*self.pressed_brush).clone(),
                        ));
                    }
                    WidgetMessage::MouseUp { .. } => {
                        if *self.is_selected {
                            ui.send_message(WidgetMessage::background(
                                self.handle(),
                                MessageDirection::ToWidget,
                                (*self.selected_brush).clone(),
                            ));
                        } else {
                            ui.send_message(WidgetMessage::background(
                                self.handle(),
                                MessageDirection::ToWidget,
                                (*self.normal_brush).clone(),
                            ));
                        }
                    }
                    _ => {}
                }
            }

            if message.destination() == self.handle() {
                if let WidgetMessage::Style(style) = msg {
                    self.normal_brush.update(style);
                    self.hover_brush.update(style);
                    self.pressed_brush.update(style);
                    self.selected_brush.update(style);
                }
            }
        }
    }
}

/// Creates [`Decorator`] widget instances and adds them to the user interface.
pub struct DecoratorBuilder {
    border_builder: BorderBuilder,
    normal_brush: Option<StyledProperty<Brush>>,
    hover_brush: Option<StyledProperty<Brush>>,
    pressed_brush: Option<StyledProperty<Brush>>,
    selected_brush: Option<StyledProperty<Brush>>,
    pressable: bool,
    selected: bool,
}

impl DecoratorBuilder {
    /// Creates a new decorator builder.
    pub fn new(border_builder: BorderBuilder) -> Self {
        Self {
            normal_brush: None,
            hover_brush: None,
            pressed_brush: None,
            selected_brush: None,
            pressable: true,
            selected: false,
            border_builder,
        }
    }

    /// Sets a desired brush for `Normal` state.
    pub fn with_normal_brush(mut self, brush: StyledProperty<Brush>) -> Self {
        self.normal_brush = Some(brush);
        self
    }

    /// Sets a desired brush for `Hovered` state.
    pub fn with_hover_brush(mut self, brush: StyledProperty<Brush>) -> Self {
        self.hover_brush = Some(brush);
        self
    }

    /// Sets a desired brush for `Pressed` state.
    pub fn with_pressed_brush(mut self, brush: StyledProperty<Brush>) -> Self {
        self.pressed_brush = Some(brush);
        self
    }

    /// Sets a desired brush for `Selected` state.
    pub fn with_selected_brush(mut self, brush: StyledProperty<Brush>) -> Self {
        self.selected_brush = Some(brush);
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
    pub fn build(mut self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let normal_brush = self
            .normal_brush
            .unwrap_or_else(|| ctx.style.property::<Brush>(Style::BRUSH_LIGHT));
        let hover_brush = self
            .hover_brush
            .unwrap_or_else(|| ctx.style.property::<Brush>(Style::BRUSH_LIGHTER));
        let pressed_brush = self
            .pressed_brush
            .unwrap_or_else(|| ctx.style.property::<Brush>(Style::BRUSH_LIGHTEST));
        let selected_brush = self
            .selected_brush
            .unwrap_or_else(|| ctx.style.property::<Brush>(Style::BRUSH_BRIGHT));

        if self.border_builder.widget_builder.foreground.is_none() {
            let brush = ctx.style.property(Style::BRUSH_DARKER);
            self.border_builder.widget_builder.foreground = Some(brush);
        }

        let mut border = self.border_builder.build_border(ctx);

        if self.selected {
            *border.background = selected_brush.clone();
        } else {
            *border.background = normal_brush.clone();
        }

        let node = UiNode::new(Decorator {
            border,
            normal_brush: normal_brush.into(),
            hover_brush: hover_brush.into(),
            pressed_brush: pressed_brush.into(),
            selected_brush: selected_brush.into(),
            is_selected: self.selected.into(),
            is_pressable: self.pressable.into(),
        });
        ctx.add_node(node)
    }
}

#[cfg(test)]
mod test {
    use crate::border::BorderBuilder;
    use crate::decorator::DecoratorBuilder;
    use crate::{test::test_widget_deletion, widget::WidgetBuilder};

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| {
            DecoratorBuilder::new(BorderBuilder::new(WidgetBuilder::new())).build(ctx)
        });
    }
}
