use crate::{
    border::{Border, BorderBuilder},
    brush::{Brush, GradientPoint},
    core::{algebra::Vector2, color::Color, pool::Handle},
    define_constructor,
    draw::DrawingContext,
    message::{MessageDirection, UiMessage},
    widget::{Widget, WidgetMessage},
    BuildContext, Control, NodeHandleMapping, UiNode, UserInterface, BRUSH_BRIGHT, BRUSH_LIGHT,
    BRUSH_LIGHTER, BRUSH_LIGHTEST, COLOR_DARKEST, COLOR_LIGHTEST,
};
use std::any::{Any, TypeId};
use std::{
    ops::{Deref, DerefMut},
    sync::mpsc::Sender,
};

#[derive(Debug, Clone, PartialEq)]
pub enum DecoratorMessage {
    Select(bool),
    HoverBrush(Brush),
    NormalBrush(Brush),
    PressedBrush(Brush),
    SelectedBrush(Brush),
}

impl DecoratorMessage {
    define_constructor!(DecoratorMessage:Select => fn select(bool), layout: false);
    define_constructor!(DecoratorMessage:HoverBrush => fn hover_brush(Brush), layout: false);
    define_constructor!(DecoratorMessage:NormalBrush => fn normal_brush(Brush), layout: false);
    define_constructor!(DecoratorMessage:PressedBrush => fn pressed_brush(Brush), layout: false);
    define_constructor!(DecoratorMessage:SelectedBrush => fn selected_brush(Brush), layout: false);
}

/// A visual element that changes its appearance by listening specific events.
/// It can has "pressed", "hover", "selected" or normal appearance:
///
/// `Pressed` - enables on mouse down message.
/// `Selected` - whether decorator selected or not.
/// `Hovered` - mouse is over decorator.
/// `Normal` - not selected, pressed, hovered.
///
/// This element is widely used to provide some generic visual behaviour for various
/// widgets. For example it used to decorate button, items in items control.
#[derive(Clone)]
pub struct Decorator {
    border: Border,
    normal_brush: Brush,
    hover_brush: Brush,
    pressed_brush: Brush,
    selected_brush: Brush,
    disabled_brush: Brush,
    is_selected: bool,
    is_pressable: bool,
}

impl Decorator {
    pub fn border(&self) -> &Border {
        &self.border
    }

    pub fn normal_brush(&self) -> &Brush {
        &self.normal_brush
    }

    pub fn hover_brush(&self) -> &Brush {
        &self.hover_brush
    }

    pub fn pressed_brush(&self) -> &Brush {
        &self.pressed_brush
    }

    pub fn selected_brush(&self) -> &Brush {
        &self.selected_brush
    }

    pub fn disabled_brush(&self) -> &Brush {
        &self.disabled_brush
    }

    pub fn is_pressable(&self) -> bool {
        self.is_pressable
    }

    pub fn is_selected(&self) -> bool {
        self.is_pressable
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
                            ui.send_message(WidgetMessage::background(
                                self.handle(),
                                MessageDirection::ToWidget,
                                self.normal_brush.clone(),
                            ));
                        }
                    }
                }
                DecoratorMessage::HoverBrush(brush) => {
                    self.hover_brush = brush.clone();
                    if self.is_mouse_directly_over {
                        ui.send_message(WidgetMessage::background(
                            self.handle(),
                            MessageDirection::ToWidget,
                            self.hover_brush.clone(),
                        ));
                    }
                }
                DecoratorMessage::NormalBrush(brush) => {
                    self.normal_brush = brush.clone();
                    if !self.is_selected && !self.is_mouse_directly_over {
                        ui.send_message(WidgetMessage::background(
                            self.handle(),
                            MessageDirection::ToWidget,
                            self.normal_brush.clone(),
                        ));
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
                        } else {
                            ui.send_message(WidgetMessage::background(
                                self.handle(),
                                MessageDirection::ToWidget,
                                self.normal_brush.clone(),
                            ));
                        }
                    }
                    WidgetMessage::MouseEnter => {
                        ui.send_message(WidgetMessage::background(
                            self.handle(),
                            MessageDirection::ToWidget,
                            self.hover_brush.clone(),
                        ));
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
                            ui.send_message(WidgetMessage::background(
                                self.handle(),
                                MessageDirection::ToWidget,
                                self.normal_brush.clone(),
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

pub struct DecoratorBuilder {
    border_builder: BorderBuilder,
    normal_brush: Option<Brush>,
    hover_brush: Option<Brush>,
    pressed_brush: Option<Brush>,
    selected_brush: Option<Brush>,
    disabled_brush: Option<Brush>,
    pressable: bool,
}

impl DecoratorBuilder {
    pub fn new(border_builder: BorderBuilder) -> Self {
        Self {
            border_builder,
            normal_brush: None,
            hover_brush: None,
            pressed_brush: None,
            selected_brush: None,
            disabled_brush: None,
            pressable: true,
        }
    }

    pub fn with_normal_brush(mut self, brush: Brush) -> Self {
        self.normal_brush = Some(brush);
        self
    }

    pub fn with_hover_brush(mut self, brush: Brush) -> Self {
        self.hover_brush = Some(brush);
        self
    }

    pub fn with_pressed_brush(mut self, brush: Brush) -> Self {
        self.pressed_brush = Some(brush);
        self
    }

    pub fn with_selected_brush(mut self, brush: Brush) -> Self {
        self.selected_brush = Some(brush);
        self
    }

    pub fn with_disabled_brush(mut self, brush: Brush) -> Self {
        self.disabled_brush = Some(brush);
        self
    }

    pub fn with_pressable(mut self, pressable: bool) -> Self {
        self.pressable = pressable;
        self
    }

    pub fn build(mut self, ui: &mut BuildContext) -> Handle<UiNode> {
        let normal_brush = self.normal_brush.unwrap_or(BRUSH_LIGHT);

        if self.border_builder.widget_builder.foreground.is_none() {
            self.border_builder.widget_builder.foreground = Some(Brush::LinearGradient {
                from: Vector2::new(0.5, 0.0),
                to: Vector2::new(0.5, 1.0),
                stops: vec![
                    GradientPoint {
                        stop: 0.0,
                        color: COLOR_LIGHTEST,
                    },
                    GradientPoint {
                        stop: 0.25,
                        color: COLOR_LIGHTEST,
                    },
                    GradientPoint {
                        stop: 1.0,
                        color: COLOR_DARKEST,
                    },
                ],
            });
        }

        let mut border = self.border_builder.build_border();

        border.set_background(normal_brush.clone());

        let node = UiNode::new(Decorator {
            border,
            normal_brush,
            hover_brush: self.hover_brush.unwrap_or(BRUSH_LIGHTER),
            pressed_brush: self.pressed_brush.unwrap_or(BRUSH_LIGHTEST),
            selected_brush: self.selected_brush.unwrap_or(BRUSH_BRIGHT),
            disabled_brush: self
                .disabled_brush
                .unwrap_or_else(|| Brush::Solid(Color::opaque(50, 50, 50))),
            is_selected: false,
            is_pressable: self.pressable,
        });
        ui.add_node(node)
    }
}
