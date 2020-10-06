use crate::message::{MessageData, MessageDirection};
use crate::{
    border::Border,
    border::BorderBuilder,
    brush::{Brush, GradientPoint},
    core::{
        color::Color,
        math::{vec2::Vec2, Rect},
        pool::Handle,
    },
    draw::DrawingContext,
    message::DecoratorMessage,
    message::{UiMessage, UiMessageData, WidgetMessage},
    node::UINode,
    widget::Widget,
    BuildContext, Control, NodeHandleMapping, UserInterface,
};
use std::ops::{Deref, DerefMut};

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
pub struct Decorator<M: MessageData, C: Control<M, C>> {
    border: Border<M, C>,
    normal_brush: Brush,
    hover_brush: Brush,
    pressed_brush: Brush,
    selected_brush: Brush,
    disabled_brush: Brush,
    is_selected: bool,
    pressable: bool,
}

impl<M: MessageData, C: Control<M, C>> Deref for Decorator<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.border
    }
}

impl<M: MessageData, C: Control<M, C>> DerefMut for Decorator<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.border
    }
}

impl<M: MessageData, C: Control<M, C>> Control<M, C> for Decorator<M, C> {
    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        self.border.resolve(node_map)
    }

    fn measure_override(&self, ui: &UserInterface<M, C>, available_size: Vec2) -> Vec2 {
        self.border.measure_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface<M, C>, final_size: Vec2) -> Vec2 {
        self.border.arrange_override(ui, final_size)
    }

    fn arrange(&self, ui: &UserInterface<M, C>, final_rect: &Rect<f32>) {
        self.border.arrange(ui, final_rect)
    }

    fn is_measure_valid(&self, ui: &UserInterface<M, C>) -> bool {
        self.border.is_measure_valid(ui)
    }

    fn is_arrange_valid(&self, ui: &UserInterface<M, C>) -> bool {
        self.border.is_arrange_valid(ui)
    }

    fn measure(&self, ui: &UserInterface<M, C>, available_size: Vec2) {
        self.border.measure(ui, available_size);
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        self.border.draw(drawing_context)
    }

    fn update(&mut self, dt: f32) {
        self.border.update(dt)
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.border.handle_routed_message(ui, message);

        match &message.data() {
            UiMessageData::Decorator(msg) => match msg {
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
            },
            UiMessageData::Widget(msg) => {
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
                        WidgetMessage::MouseDown { .. } if self.pressable => {
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
            _ => {}
        }
    }

    fn remove_ref(&mut self, handle: Handle<UINode<M, C>>) {
        self.border.remove_ref(handle)
    }
}

pub struct DecoratorBuilder<M: MessageData, C: Control<M, C>> {
    border_builder: BorderBuilder<M, C>,
    normal_brush: Option<Brush>,
    hover_brush: Option<Brush>,
    pressed_brush: Option<Brush>,
    selected_brush: Option<Brush>,
    disabled_brush: Option<Brush>,
    pressable: bool,
}

impl<M: MessageData, C: Control<M, C>> DecoratorBuilder<M, C> {
    pub fn new(border_builder: BorderBuilder<M, C>) -> Self {
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

    pub fn build(self, ui: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        let normal_brush = self.normal_brush.unwrap_or_else(|| Brush::LinearGradient {
            from: Vec2::new(0.5, 0.0),
            to: Vec2::new(0.5, 1.0),
            stops: vec![
                GradientPoint {
                    stop: 0.0,
                    color: Color::opaque(85, 85, 85),
                },
                GradientPoint {
                    stop: 0.46,
                    color: Color::opaque(85, 85, 85),
                },
                GradientPoint {
                    stop: 0.5,
                    color: Color::opaque(65, 65, 65),
                },
                GradientPoint {
                    stop: 0.54,
                    color: Color::opaque(75, 75, 75),
                },
                GradientPoint {
                    stop: 1.0,
                    color: Color::opaque(75, 75, 75),
                },
            ],
        });

        let mut border = self.border_builder.build_border();

        border.set_background(normal_brush.clone());

        let node = UINode::Decorator(Decorator {
            border,
            normal_brush,
            hover_brush: self.hover_brush.unwrap_or_else(|| Brush::LinearGradient {
                from: Vec2::new(0.5, 0.0),
                to: Vec2::new(0.5, 1.0),
                stops: vec![
                    GradientPoint {
                        stop: 0.0,
                        color: Color::opaque(105, 95, 85),
                    },
                    GradientPoint {
                        stop: 0.46,
                        color: Color::opaque(105, 95, 85),
                    },
                    GradientPoint {
                        stop: 0.5,
                        color: Color::opaque(85, 75, 65),
                    },
                    GradientPoint {
                        stop: 0.54,
                        color: Color::opaque(95, 85, 75),
                    },
                    GradientPoint {
                        stop: 1.0,
                        color: Color::opaque(95, 85, 75),
                    },
                ],
            }),
            pressed_brush: self.pressed_brush.unwrap_or_else(|| Brush::LinearGradient {
                from: Vec2::new(0.5, 0.0),
                to: Vec2::new(0.5, 1.0),
                stops: vec![
                    GradientPoint {
                        stop: 0.0,
                        color: Color::opaque(65, 65, 65),
                    },
                    GradientPoint {
                        stop: 0.46,
                        color: Color::opaque(65, 65, 65),
                    },
                    GradientPoint {
                        stop: 0.5,
                        color: Color::opaque(45, 45, 45),
                    },
                    GradientPoint {
                        stop: 0.54,
                        color: Color::opaque(55, 55, 55),
                    },
                    GradientPoint {
                        stop: 1.0,
                        color: Color::opaque(55, 55, 55),
                    },
                ],
            }),
            selected_brush: self
                .selected_brush
                .unwrap_or_else(|| Brush::LinearGradient {
                    from: Vec2::new(0.5, 0.0),
                    to: Vec2::new(0.5, 1.0),
                    stops: vec![
                        GradientPoint {
                            stop: 0.0,
                            color: Color::opaque(170, 108, 57),
                        },
                        GradientPoint {
                            stop: 0.46,
                            color: Color::opaque(170, 108, 57),
                        },
                        GradientPoint {
                            stop: 0.5,
                            color: Color::opaque(150, 88, 37),
                        },
                        GradientPoint {
                            stop: 0.54,
                            color: Color::opaque(160, 98, 47),
                        },
                        GradientPoint {
                            stop: 1.0,
                            color: Color::opaque(160, 98, 47),
                        },
                    ],
                }),
            disabled_brush: self
                .disabled_brush
                .unwrap_or(Brush::Solid(Color::opaque(50, 50, 50))),
            is_selected: false,
            pressable: self.pressable,
        });
        ui.add_node(node)
    }
}
