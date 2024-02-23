use crate::{
    border::BorderBuilder,
    brush::Brush,
    core::{
        algebra::Vector2,
        color::{Color, Hsv},
        math::Rect,
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        visitor::prelude::*,
    },
    define_constructor,
    draw::{CommandTexture, Draw, DrawingContext},
    grid::{Column, GridBuilder, Row},
    message::{MessageDirection, MouseButton, UiMessage},
    numeric::{NumericUpDownBuilder, NumericUpDownMessage},
    popup::{Placement, PopupBuilder, PopupMessage},
    text::TextBuilder,
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, Orientation, Thickness, UiNode, UserInterface, VerticalAlignment,
};
use fyrox_core::uuid_provider;
use fyrox_graph::BaseSceneGraph;
use std::{
    ops::{Deref, DerefMut},
    sync::mpsc::Sender,
};

pub mod gradient;

#[derive(Debug, Clone, PartialEq)]
pub enum HueBarMessage {
    /// Sets new hue value.
    Hue(f32),

    /// Sets new orientation
    Orientation(Orientation),
}

impl HueBarMessage {
    define_constructor!(HueBarMessage:Hue => fn hue(f32), layout: false);
    define_constructor!(HueBarMessage:Orientation => fn orientation(Orientation), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum AlphaBarMessage {
    /// Sets new hue value.
    Alpha(f32),

    /// Sets new orientation
    Orientation(Orientation),
}

impl AlphaBarMessage {
    define_constructor!(AlphaBarMessage:Alpha => fn alpha(f32), layout: false);
    define_constructor!(AlphaBarMessage:Orientation => fn orientation(Orientation), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum SaturationBrightnessFieldMessage {
    /// Sets new hue value on the field.
    Hue(f32),

    /// Sets new saturation value on the field.
    Saturation(f32),

    /// Sets new brightness value on the field.
    Brightness(f32),
}

impl SaturationBrightnessFieldMessage {
    define_constructor!(SaturationBrightnessFieldMessage:Hue => fn hue(f32), layout: false);
    define_constructor!(SaturationBrightnessFieldMessage:Saturation => fn saturation(f32), layout: false);
    define_constructor!(SaturationBrightnessFieldMessage:Brightness => fn brightness(f32), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum ColorPickerMessage {
    /// Sets color in RGB.
    ///
    /// Direction: **To/From Widget**.
    Color(Color),

    /// Sets color in HSV.
    ///
    /// Direction: **To Widget**.
    Hsv(Hsv),
}

impl ColorPickerMessage {
    define_constructor!(ColorPickerMessage:Color => fn color(Color), layout: false);
    define_constructor!(ColorPickerMessage:Hsv => fn hsv(Hsv), layout: false);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorFieldMessage {
    Color(Color),
}

impl ColorFieldMessage {
    define_constructor!(ColorFieldMessage:Color => fn color(Color), layout: false);
}

#[derive(Default, Clone, Debug, Visit, Reflect, ComponentProvider)]
pub struct AlphaBar {
    pub widget: Widget,
    pub orientation: Orientation,
    pub alpha: f32,
    pub is_picking: bool,
}

crate::define_widget_deref!(AlphaBar);

impl AlphaBar {
    fn alpha_at(&self, mouse_pos: Vector2<f32>) -> f32 {
        let relative_pos = mouse_pos - self.screen_position();
        let k = match self.orientation {
            Orientation::Vertical => relative_pos.y / self.actual_local_size().y,
            Orientation::Horizontal => relative_pos.x / self.actual_local_size().x,
        };
        k.clamp(0.0, 1.0) * 255.0
    }
}

fn push_gradient_rect(
    drawing_context: &mut DrawingContext,
    bounds: &Rect<f32>,
    orientation: Orientation,
    prev_k: f32,
    prev_color: Color,
    curr_k: f32,
    curr_color: Color,
) {
    match orientation {
        Orientation::Vertical => {
            drawing_context.push_triangle_multicolor([
                (
                    Vector2::new(bounds.x(), bounds.y() + bounds.h() * prev_k),
                    prev_color,
                ),
                (
                    Vector2::new(bounds.x() + bounds.w(), bounds.y() + bounds.h() * prev_k),
                    prev_color,
                ),
                (
                    Vector2::new(bounds.x(), bounds.y() + bounds.h() * curr_k),
                    curr_color,
                ),
            ]);
            drawing_context.push_triangle_multicolor([
                (
                    Vector2::new(bounds.x() + bounds.w(), bounds.y() + bounds.h() * prev_k),
                    prev_color,
                ),
                (
                    Vector2::new(bounds.x() + bounds.w(), bounds.y() + bounds.h() * curr_k),
                    curr_color,
                ),
                (
                    Vector2::new(bounds.x(), bounds.y() + bounds.h() * curr_k),
                    curr_color,
                ),
            ]);
        }
        Orientation::Horizontal => {
            drawing_context.push_triangle_multicolor([
                (
                    Vector2::new(bounds.x() + bounds.w() * prev_k, bounds.y()),
                    prev_color,
                ),
                (
                    Vector2::new(bounds.x() + bounds.w() * curr_k, bounds.y()),
                    curr_color,
                ),
                (
                    Vector2::new(bounds.x() + bounds.w() * prev_k, bounds.y() + bounds.h()),
                    prev_color,
                ),
            ]);
            drawing_context.push_triangle_multicolor([
                (
                    Vector2::new(bounds.x() + bounds.w() * curr_k, bounds.y()),
                    curr_color,
                ),
                (
                    Vector2::new(bounds.x() + bounds.w() * curr_k, bounds.y() + bounds.h()),
                    curr_color,
                ),
                (
                    Vector2::new(bounds.x() + bounds.w() * prev_k, bounds.y() + bounds.h()),
                    prev_color,
                ),
            ]);
        }
    }
}

const CHECKERBOARD_SIZE: f32 = 6.0;

pub fn draw_checker_board(
    bounds: Rect<f32>,
    clip_bounds: Rect<f32>,
    size: f32,
    drawing_context: &mut DrawingContext,
) {
    let h_amount = (bounds.w() / size).ceil() as usize;
    let v_amount = (bounds.h() / size).ceil() as usize;
    for y in 0..v_amount {
        for x in 0..h_amount {
            let rect = Rect::new(
                bounds.x() + x as f32 * size,
                bounds.y() + y as f32 * size,
                size,
                size,
            );
            let color = if (x + y) & 1 == 0 {
                Color::opaque(127, 127, 127)
            } else {
                Color::WHITE
            };
            drawing_context.push_rect_multicolor(&rect, [color; 4]);
        }
    }
    drawing_context.commit(
        clip_bounds,
        Brush::Solid(Color::WHITE),
        CommandTexture::None,
        None,
    );
}

uuid_provider!(AlphaBar = "956d4cae-7953-486b-99da-a9b852c2e144");

impl Control for AlphaBar {
    fn draw(&self, drawing_context: &mut DrawingContext) {
        let bounds = self.bounding_rect();

        // Draw checker board first.
        draw_checker_board(
            bounds,
            self.clip_bounds(),
            CHECKERBOARD_SIZE,
            drawing_context,
        );

        // Then draw alpha gradient.
        for alpha in 1..255 {
            let prev_color = Color::from_rgba(0, 0, 0, alpha - 1);
            let curr_color = Color::from_rgba(0, 0, 0, alpha);
            let prev_k = (alpha - 1) as f32 / 255.0;
            let curr_k = alpha as f32 / 255.0;
            push_gradient_rect(
                drawing_context,
                &bounds,
                self.orientation,
                prev_k,
                prev_color,
                curr_k,
                curr_color,
            );
        }

        let k = self.alpha / 255.0;
        match self.orientation {
            Orientation::Vertical => drawing_context.push_rect_multicolor(
                &Rect::new(bounds.x(), bounds.y() + bounds.h() * k, bounds.w(), 1.0),
                [Color::WHITE; 4],
            ),
            Orientation::Horizontal => drawing_context.push_rect_multicolor(
                &Rect::new(bounds.x() + k * bounds.w(), bounds.y(), 1.0, bounds.h()),
                [Color::WHITE; 4],
            ),
        }

        drawing_context.commit(
            self.clip_bounds(),
            Brush::Solid(Color::WHITE),
            CommandTexture::None,
            None,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle {
            if let Some(msg) = message.data::<WidgetMessage>() {
                if message.direction() == MessageDirection::FromWidget {
                    match *msg {
                        WidgetMessage::MouseDown { button, .. } => {
                            if button == MouseButton::Left {
                                self.is_picking = true;
                                ui.capture_mouse(self.handle);
                            }
                        }
                        WidgetMessage::MouseMove { pos, .. } => {
                            if self.is_picking {
                                ui.send_message(AlphaBarMessage::alpha(
                                    self.handle,
                                    MessageDirection::ToWidget,
                                    self.alpha_at(pos),
                                ))
                            }
                        }
                        WidgetMessage::MouseUp { button, .. } => {
                            if self.is_picking && button == MouseButton::Left {
                                self.is_picking = false;
                                ui.release_mouse_capture();
                            }
                        }
                        _ => (),
                    }
                }
            } else if let Some(msg) = message.data::<AlphaBarMessage>() {
                if message.direction() == MessageDirection::ToWidget {
                    match *msg {
                        AlphaBarMessage::Alpha(alpha) => {
                            if self.alpha != alpha {
                                self.alpha = alpha;
                                ui.send_message(message.reverse());
                            }
                        }
                        AlphaBarMessage::Orientation(orientation) => {
                            if self.orientation != orientation {
                                self.orientation = orientation;
                                ui.send_message(message.reverse());
                            }
                        }
                    }
                }
            }
        }
    }
}

pub struct AlphaBarBuilder {
    widget_builder: WidgetBuilder,
    orientation: Orientation,
    alpha: f32,
}

impl AlphaBarBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            orientation: Orientation::Vertical,
            alpha: 255.0,
        }
    }

    pub fn with_alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha;
        self
    }

    pub fn build(self, ui: &mut BuildContext) -> Handle<UiNode> {
        let canvas = AlphaBar {
            widget: self.widget_builder.build(),
            orientation: self.orientation,
            alpha: self.alpha,
            is_picking: false,
        };
        ui.add_node(UiNode::new(canvas))
    }
}

#[derive(Default, Clone, Debug, Visit, Reflect, ComponentProvider)]
pub struct HueBar {
    pub widget: Widget,
    pub orientation: Orientation,
    pub is_picking: bool,
    pub hue: f32,
}

crate::define_widget_deref!(HueBar);

impl HueBar {
    fn hue_at(&self, mouse_pos: Vector2<f32>) -> f32 {
        let relative_pos = mouse_pos - self.screen_position();
        let k = match self.orientation {
            Orientation::Vertical => relative_pos.y / self.actual_local_size().y,
            Orientation::Horizontal => relative_pos.x / self.actual_local_size().x,
        };
        k.clamp(0.0, 1.0) * 360.0
    }
}

uuid_provider!(HueBar = "af28f977-85e7-4c9e-9a61-7f208844acb5");

impl Control for HueBar {
    fn draw(&self, drawing_context: &mut DrawingContext) {
        let bounds = self.bounding_rect();
        for hue in 1..360 {
            let prev_color = Color::from(Hsv::new((hue - 1) as f32, 100.0, 100.0));
            let curr_color = Color::from(Hsv::new(hue as f32, 100.0, 100.0));
            let prev_k = (hue - 1) as f32 / 360.0;
            let curr_k = hue as f32 / 360.0;
            push_gradient_rect(
                drawing_context,
                &bounds,
                self.orientation,
                prev_k,
                prev_color,
                curr_k,
                curr_color,
            );
        }

        let k = self.hue / 360.0;
        match self.orientation {
            Orientation::Vertical => drawing_context.push_rect_multicolor(
                &Rect::new(bounds.x(), bounds.y() + bounds.h() * k, bounds.w(), 1.0),
                [Color::BLACK; 4],
            ),
            Orientation::Horizontal => drawing_context.push_rect_multicolor(
                &Rect::new(bounds.x() + k * bounds.w(), bounds.y(), 1.0, bounds.h()),
                [Color::BLACK; 4],
            ),
        }

        drawing_context.commit(
            self.clip_bounds(),
            Brush::Solid(Color::WHITE),
            CommandTexture::None,
            None,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle {
            if let Some(msg) = message.data::<WidgetMessage>() {
                if message.direction() == MessageDirection::FromWidget {
                    match *msg {
                        WidgetMessage::MouseDown { button, .. } => {
                            if button == MouseButton::Left {
                                self.is_picking = true;
                                ui.capture_mouse(self.handle);
                            }
                        }
                        WidgetMessage::MouseMove { pos, .. } => {
                            if self.is_picking {
                                ui.send_message(HueBarMessage::hue(
                                    self.handle,
                                    MessageDirection::ToWidget,
                                    self.hue_at(pos),
                                ))
                            }
                        }
                        WidgetMessage::MouseUp { button, .. } => {
                            if self.is_picking && button == MouseButton::Left {
                                self.is_picking = false;
                                ui.release_mouse_capture();
                            }
                        }
                        _ => (),
                    }
                }
            } else if let Some(msg) = message.data::<HueBarMessage>() {
                if message.direction() == MessageDirection::ToWidget {
                    match *msg {
                        HueBarMessage::Hue(hue) => {
                            if self.hue != hue {
                                self.hue = hue;
                                ui.send_message(message.reverse());
                            }
                        }
                        HueBarMessage::Orientation(orientation) => {
                            if self.orientation != orientation {
                                self.orientation = orientation;
                                ui.send_message(message.reverse());
                            }
                        }
                    }
                }
            }
        }
    }
}

pub struct HueBarBuilder {
    widget_builder: WidgetBuilder,
    orientation: Orientation,
    hue: f32,
}

impl HueBarBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            orientation: Orientation::Vertical,
            hue: 0.0, // Red
        }
    }

    pub fn with_hue(mut self, hue: f32) -> Self {
        self.hue = hue;
        self
    }

    pub fn with_orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn build(self, ui: &mut BuildContext) -> Handle<UiNode> {
        let bar = HueBar {
            widget: self.widget_builder.build(),
            orientation: self.orientation,
            is_picking: false,
            hue: self.hue,
        };
        ui.add_node(UiNode::new(bar))
    }
}

#[derive(Default, Clone, Debug, Visit, Reflect, ComponentProvider)]
pub struct SaturationBrightnessField {
    pub widget: Widget,
    pub is_picking: bool,
    pub hue: f32,
    pub saturation: f32,
    pub brightness: f32,
}

crate::define_widget_deref!(SaturationBrightnessField);

impl SaturationBrightnessField {
    fn saturation_at(&self, mouse_pos: Vector2<f32>) -> f32 {
        ((mouse_pos.x - self.screen_position().x) / self.screen_bounds().w()).clamp(0.0, 1.0)
            * 100.0
    }

    fn brightness_at(&self, mouse_pos: Vector2<f32>) -> f32 {
        100.0
            - ((mouse_pos.y - self.screen_position().y) / self.screen_bounds().h()).clamp(0.0, 1.0)
                * 100.0
    }
}

uuid_provider!(SaturationBrightnessField = "ab6bfad5-0c4b-42a5-8da5-fc5687b1afc7");

impl Control for SaturationBrightnessField {
    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        let size = self.deref().arrange_override(ui, final_size);
        // Make sure field is always square.
        ui.send_message(WidgetMessage::width(
            self.handle,
            MessageDirection::ToWidget,
            final_size.y,
        ));
        size
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        let bounds = self.bounding_rect();

        drawing_context.push_rect_multicolor(
            &bounds,
            [
                Color::from(Hsv::new(self.hue, 0.0, 100.0)),
                Color::from(Hsv::new(self.hue, 100.0, 100.0)),
                Color::from(Hsv::new(self.hue, 100.0, 0.0)),
                Color::from(Hsv::new(self.hue, 0.0, 0.0)),
            ],
        );
        drawing_context.commit(
            self.clip_bounds(),
            Brush::Solid(Color::WHITE),
            CommandTexture::None,
            None,
        );

        // Indicator must be drawn separately, otherwise it may be drawn incorrectly.
        let origin = Vector2::new(
            bounds.x() + self.saturation / 100.0 * bounds.w(),
            bounds.y() + (100.0 - self.brightness) / 100.0 * bounds.h(),
        );
        drawing_context.push_circle_filled(
            origin,
            3.0,
            10,
            Color::from(Hsv::new(360.0 - self.hue, 100.0, 100.0)),
        );
        drawing_context.commit(
            self.clip_bounds(),
            Brush::Solid(Color::WHITE),
            CommandTexture::None,
            None,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle {
            if let Some(msg) = message.data::<WidgetMessage>() {
                if message.direction() == MessageDirection::FromWidget {
                    match *msg {
                        WidgetMessage::MouseDown { button, .. } => {
                            if button == MouseButton::Left {
                                self.is_picking = true;
                                ui.capture_mouse(self.handle);
                            }
                        }
                        WidgetMessage::MouseMove { pos, .. } => {
                            if self.is_picking {
                                ui.send_message(SaturationBrightnessFieldMessage::brightness(
                                    self.handle,
                                    MessageDirection::ToWidget,
                                    self.brightness_at(pos),
                                ));

                                ui.send_message(SaturationBrightnessFieldMessage::saturation(
                                    self.handle,
                                    MessageDirection::ToWidget,
                                    self.saturation_at(pos),
                                ));
                            }
                        }
                        WidgetMessage::MouseUp { button, .. } => {
                            if self.is_picking && button == MouseButton::Left {
                                self.is_picking = false;
                                ui.release_mouse_capture();
                            }
                        }
                        _ => (),
                    }
                }
            } else if let Some(msg) = message.data::<SaturationBrightnessFieldMessage>() {
                if message.direction() == MessageDirection::ToWidget {
                    match *msg {
                        SaturationBrightnessFieldMessage::Hue(hue) => {
                            let clamped = hue.clamp(0.0, 360.0);
                            if self.hue != clamped {
                                self.hue = clamped;
                                ui.send_message(SaturationBrightnessFieldMessage::hue(
                                    self.handle,
                                    MessageDirection::FromWidget,
                                    self.hue,
                                ));
                            }
                        }
                        SaturationBrightnessFieldMessage::Saturation(saturation) => {
                            let clamped = saturation.clamp(0.0, 100.0);
                            if self.saturation != clamped {
                                self.saturation = clamped;
                                ui.send_message(SaturationBrightnessFieldMessage::saturation(
                                    self.handle,
                                    MessageDirection::FromWidget,
                                    self.saturation,
                                ));
                            }
                        }
                        SaturationBrightnessFieldMessage::Brightness(brightness) => {
                            let clamped = brightness.clamp(0.0, 100.0);
                            if self.brightness != clamped {
                                self.brightness = clamped;
                                ui.send_message(SaturationBrightnessFieldMessage::brightness(
                                    self.handle,
                                    MessageDirection::FromWidget,
                                    self.brightness,
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
}

pub struct SaturationBrightnessFieldBuilder {
    widget_builder: WidgetBuilder,
    hue: f32,
    saturation: f32,
    brightness: f32,
}

impl SaturationBrightnessFieldBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            hue: 0.0,
            saturation: 100.0,
            brightness: 100.0,
        }
    }

    pub fn with_hue(mut self, hue: f32) -> Self {
        self.hue = hue;
        self
    }

    pub fn with_saturation(mut self, saturation: f32) -> Self {
        self.saturation = saturation;
        self
    }

    pub fn with_brightness(mut self, brightness: f32) -> Self {
        self.brightness = brightness;
        self
    }

    pub fn build(self, ui: &mut BuildContext) -> Handle<UiNode> {
        let bar = SaturationBrightnessField {
            widget: self.widget_builder.build(),
            is_picking: false,
            saturation: self.saturation,
            brightness: self.brightness,
            hue: self.hue,
        };
        ui.add_node(UiNode::new(bar))
    }
}

#[derive(Default, Clone, Debug, Visit, Reflect, ComponentProvider)]
pub struct ColorPicker {
    pub widget: Widget,
    pub hue_bar: Handle<UiNode>,
    pub alpha_bar: Handle<UiNode>,
    pub saturation_brightness_field: Handle<UiNode>,
    pub red: Handle<UiNode>,
    pub green: Handle<UiNode>,
    pub blue: Handle<UiNode>,
    pub alpha: Handle<UiNode>,
    pub hue: Handle<UiNode>,
    pub saturation: Handle<UiNode>,
    pub brightness: Handle<UiNode>,
    pub color_mark: Handle<UiNode>,
    pub color: Color,
    pub hsv: Hsv,
}

crate::define_widget_deref!(ColorPicker);

fn mark_handled(message: UiMessage) -> UiMessage {
    message.set_handled(true);
    message
}

impl ColorPicker {
    fn sync_fields(&self, ui: &mut UserInterface, color: Color, hsv: Hsv) {
        ui.send_message(mark_handled(NumericUpDownMessage::value(
            self.hue,
            MessageDirection::ToWidget,
            hsv.hue(),
        )));

        ui.send_message(mark_handled(NumericUpDownMessage::value(
            self.saturation,
            MessageDirection::ToWidget,
            hsv.saturation(),
        )));

        ui.send_message(mark_handled(NumericUpDownMessage::value(
            self.brightness,
            MessageDirection::ToWidget,
            hsv.brightness(),
        )));

        ui.send_message(mark_handled(NumericUpDownMessage::value(
            self.red,
            MessageDirection::ToWidget,
            color.r as f32,
        )));

        ui.send_message(mark_handled(NumericUpDownMessage::value(
            self.green,
            MessageDirection::ToWidget,
            color.g as f32,
        )));

        ui.send_message(mark_handled(NumericUpDownMessage::value(
            self.blue,
            MessageDirection::ToWidget,
            color.b as f32,
        )));

        ui.send_message(mark_handled(NumericUpDownMessage::value(
            self.alpha,
            MessageDirection::ToWidget,
            color.a as f32,
        )));

        ui.send_message(mark_handled(WidgetMessage::background(
            self.color_mark,
            MessageDirection::ToWidget,
            Brush::Solid(color),
        )));
    }
}

uuid_provider!(ColorPicker = "b7a5d650-5b77-4938-83c1-37f3fe107885");

impl Control for ColorPicker {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(&HueBarMessage::Hue(hue)) = message.data::<HueBarMessage>() {
            if message.destination() == self.hue_bar
                && message.direction() == MessageDirection::FromWidget
            {
                ui.send_message(SaturationBrightnessFieldMessage::hue(
                    self.saturation_brightness_field,
                    MessageDirection::ToWidget,
                    hue,
                ));

                let mut hsv = self.hsv;
                hsv.set_hue(hue);
                ui.send_message(ColorPickerMessage::hsv(
                    self.handle,
                    MessageDirection::ToWidget,
                    hsv,
                ));
            }
        } else if let Some(&AlphaBarMessage::Alpha(alpha)) = message.data::<AlphaBarMessage>() {
            if message.destination() == self.alpha_bar
                && message.direction() == MessageDirection::FromWidget
            {
                ui.send_message(ColorPickerMessage::color(
                    self.handle,
                    MessageDirection::ToWidget,
                    Color::from_rgba(self.color.r, self.color.g, self.color.b, alpha as u8),
                ));
            }
        } else if let Some(msg) = message.data::<SaturationBrightnessFieldMessage>() {
            if message.destination() == self.saturation_brightness_field
                && message.direction() == MessageDirection::FromWidget
            {
                match *msg {
                    SaturationBrightnessFieldMessage::Brightness(brightness) => {
                        let mut hsv = self.hsv;
                        hsv.set_brightness(brightness);
                        ui.send_message(ColorPickerMessage::hsv(
                            self.handle,
                            MessageDirection::ToWidget,
                            hsv,
                        ));
                    }
                    SaturationBrightnessFieldMessage::Saturation(saturation) => {
                        let mut hsv = self.hsv;
                        hsv.set_saturation(saturation);
                        ui.send_message(ColorPickerMessage::hsv(
                            self.handle,
                            MessageDirection::ToWidget,
                            hsv,
                        ));
                    }
                    _ => {}
                }
            }
        } else if let Some(&NumericUpDownMessage::Value(value)) =
            message.data::<NumericUpDownMessage<f32>>()
        {
            if message.direction() == MessageDirection::FromWidget && !message.handled() {
                if message.destination() == self.hue {
                    ui.send_message(HueBarMessage::hue(
                        self.hue_bar,
                        MessageDirection::ToWidget,
                        value,
                    ));
                } else if message.destination() == self.saturation {
                    ui.send_message(SaturationBrightnessFieldMessage::saturation(
                        self.saturation_brightness_field,
                        MessageDirection::ToWidget,
                        value,
                    ));
                } else if message.destination() == self.brightness {
                    ui.send_message(SaturationBrightnessFieldMessage::brightness(
                        self.saturation_brightness_field,
                        MessageDirection::ToWidget,
                        value,
                    ));
                } else if message.destination() == self.red {
                    ui.send_message(ColorPickerMessage::color(
                        self.handle,
                        MessageDirection::ToWidget,
                        Color::from_rgba(value as u8, self.color.g, self.color.b, self.color.a),
                    ));
                } else if message.destination() == self.green {
                    ui.send_message(ColorPickerMessage::color(
                        self.handle,
                        MessageDirection::ToWidget,
                        Color::from_rgba(self.color.r, value as u8, self.color.b, self.color.a),
                    ));
                } else if message.destination() == self.blue {
                    ui.send_message(ColorPickerMessage::color(
                        self.handle,
                        MessageDirection::ToWidget,
                        Color::from_rgba(self.color.r, self.color.g, value as u8, self.color.a),
                    ));
                } else if message.destination() == self.alpha {
                    ui.send_message(ColorPickerMessage::color(
                        self.handle,
                        MessageDirection::ToWidget,
                        Color::from_rgba(self.color.r, self.color.g, self.color.b, value as u8),
                    ));
                }
            }
        } else if let Some(msg) = message.data::<ColorPickerMessage>() {
            if message.destination() == self.handle
                && message.direction() == MessageDirection::ToWidget
            {
                match *msg {
                    ColorPickerMessage::Color(color) => {
                        if self.color != color {
                            self.color = color;
                            self.hsv = Hsv::from(color);

                            self.sync_fields(ui, color, self.hsv);

                            ui.send_message(message.reverse());
                        }
                    }
                    ColorPickerMessage::Hsv(hsv) => {
                        if self.hsv != hsv {
                            self.hsv = hsv;
                            let opaque = Color::from(hsv);
                            self.color =
                                Color::from_rgba(opaque.r, opaque.g, opaque.b, self.color.a);

                            self.sync_fields(ui, self.color, hsv);

                            ui.send_message(message.reverse());
                        }
                    }
                }
            }
        }
    }
}

pub struct ColorPickerBuilder {
    widget_builder: WidgetBuilder,
    color: Color,
}

fn make_text_mark(ctx: &mut BuildContext, text: &str, row: usize, column: usize) -> Handle<UiNode> {
    TextBuilder::new(
        WidgetBuilder::new()
            .with_vertical_alignment(VerticalAlignment::Center)
            .on_row(row)
            .on_column(column),
    )
    .with_text(text)
    .build(ctx)
}

fn make_input_field(
    ctx: &mut BuildContext,
    value: f32,
    max_value: f32,
    row: usize,
    column: usize,
) -> Handle<UiNode> {
    NumericUpDownBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::uniform(1.0))
            .on_row(row)
            .on_column(column),
    )
    .with_value(value)
    .with_min_value(0.0)
    .with_max_value(max_value)
    .with_precision(0)
    .with_step(1.0)
    .build(ctx)
}

impl ColorPickerBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            color: Color::WHITE,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let hue_bar;
        let alpha_bar;
        let saturation_brightness_field;
        let red;
        let green;
        let blue;
        let hue;
        let saturation;
        let brightness;
        let color_mark;
        let alpha;
        let hsv = Hsv::from(self.color);

        let numerics_grid = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .with_child(make_text_mark(ctx, "R", 0, 0))
                .with_child({
                    red = make_input_field(ctx, self.color.r as f32, 255.0, 0, 1);
                    red
                })
                .with_child(make_text_mark(ctx, "G", 1, 0))
                .with_child({
                    green = make_input_field(ctx, self.color.g as f32, 255.0, 1, 1);
                    green
                })
                .with_child(make_text_mark(ctx, "B", 2, 0))
                .with_child({
                    blue = make_input_field(ctx, self.color.b as f32, 255.0, 2, 1);
                    blue
                })
                .with_child(make_text_mark(ctx, "H", 0, 2))
                .with_child({
                    hue = make_input_field(ctx, hsv.hue(), 360.0, 0, 3);
                    hue
                })
                .with_child(make_text_mark(ctx, "S", 1, 2))
                .with_child({
                    saturation = make_input_field(ctx, hsv.saturation(), 100.0, 1, 3);
                    saturation
                })
                .with_child(make_text_mark(ctx, "B", 2, 2))
                .with_child({
                    brightness = make_input_field(ctx, hsv.brightness(), 100.0, 2, 3);
                    brightness
                })
                .with_child(make_text_mark(ctx, "A", 3, 0))
                .with_child({
                    alpha = make_input_field(ctx, self.color.a as f32, 255.0, 3, 1);
                    alpha
                }),
        )
        .add_column(Column::strict(10.0))
        .add_column(Column::stretch())
        .add_column(Column::strict(10.0))
        .add_column(Column::stretch())
        .add_row(Row::strict(25.0))
        .add_row(Row::strict(25.0))
        .add_row(Row::strict(25.0))
        .add_row(Row::strict(25.0))
        .add_row(Row::stretch())
        .build(ctx);

        let widget = self
            .widget_builder
            .with_child(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            saturation_brightness_field = SaturationBrightnessFieldBuilder::new(
                                WidgetBuilder::new()
                                    .with_margin(Thickness::uniform(1.0))
                                    .on_column(0),
                            )
                            .build(ctx);
                            saturation_brightness_field
                        })
                        .with_child({
                            hue_bar = HueBarBuilder::new(
                                WidgetBuilder::new()
                                    .with_margin(Thickness::uniform(1.0))
                                    .on_column(1),
                            )
                            .build(ctx);
                            hue_bar
                        })
                        .with_child({
                            alpha_bar = AlphaBarBuilder::new(
                                WidgetBuilder::new()
                                    .with_margin(Thickness::uniform(1.0))
                                    .on_column(2),
                            )
                            .with_alpha(self.color.a as f32)
                            .build(ctx);
                            alpha_bar
                        })
                        .with_child(
                            GridBuilder::new(
                                WidgetBuilder::new()
                                    .on_column(3)
                                    .with_child({
                                        color_mark = BorderBuilder::new(
                                            WidgetBuilder::new()
                                                .on_row(0)
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .build(ctx);
                                        color_mark
                                    })
                                    .with_child(numerics_grid),
                            )
                            .add_row(Row::strict(25.0))
                            .add_row(Row::stretch())
                            .add_column(Column::stretch())
                            .build(ctx),
                        ),
                )
                .add_column(Column::stretch())
                .add_column(Column::strict(20.0))
                .add_column(Column::strict(20.0))
                .add_column(Column::strict(100.0))
                .add_row(Row::auto())
                .build(ctx),
            )
            .build();

        let picker = ColorPicker {
            widget,
            hue_bar,
            saturation_brightness_field,
            red,
            green,
            blue,
            hue,
            saturation,
            brightness,
            color: self.color,
            color_mark,
            hsv,
            alpha_bar,
            alpha,
        };
        ctx.add_node(UiNode::new(picker))
    }
}

#[derive(Default, Clone, Debug, Visit, Reflect, ComponentProvider)]
pub struct ColorField {
    pub widget: Widget,
    pub popup: Handle<UiNode>,
    pub picker: Handle<UiNode>,
    pub color: Color,
}

crate::define_widget_deref!(ColorField);

uuid_provider!(ColorField = "68dec1ac-23c6-41df-bc85-499f2a82e908");

impl Control for ColorField {
    fn on_remove(&self, sender: &Sender<UiMessage>) {
        // Popup won't be deleted with the color field, because it is not the child of the field.
        // So we have to remove it manually.
        sender
            .send(WidgetMessage::remove(
                self.popup,
                MessageDirection::ToWidget,
            ))
            .unwrap();
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        let bounds = self.bounding_rect();

        drawing_context.push_rect_filled(&bounds, None);
        drawing_context.commit(
            self.clip_bounds(),
            Brush::Solid(self.color),
            CommandTexture::None,
            None,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(&WidgetMessage::MouseDown { button, .. }) = message.data::<WidgetMessage>() {
            if message.destination() == self.handle
                && message.direction() == MessageDirection::FromWidget
                && button == MouseButton::Left
            {
                ui.send_message(WidgetMessage::width(
                    self.popup,
                    MessageDirection::ToWidget,
                    self.actual_local_size().x,
                ));
                ui.send_message(PopupMessage::placement(
                    self.popup,
                    MessageDirection::ToWidget,
                    Placement::LeftBottom(self.handle),
                ));
                ui.send_message(PopupMessage::open(self.popup, MessageDirection::ToWidget));
                ui.send_message(ColorPickerMessage::color(
                    self.picker,
                    MessageDirection::ToWidget,
                    self.color,
                ));

                message.set_handled(true);
            }
        } else if let Some(&ColorFieldMessage::Color(color)) = message.data::<ColorFieldMessage>() {
            if message.destination() == self.handle
                && message.direction() == MessageDirection::ToWidget
                && self.color != color
            {
                self.color = color;
                ui.send_message(ColorPickerMessage::color(
                    self.picker,
                    MessageDirection::ToWidget,
                    self.color,
                ));
                ui.send_message(message.reverse());
            }
        }
    }

    // We have to use preview message because popup it *not* in visual tree of our control and
    // handle_routed_message won't trigger because of it.
    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        if let Some(PopupMessage::Close) = message.data::<PopupMessage>() {
            if message.destination() == self.popup {
                let picker = ui
                    .node(self.picker)
                    .cast::<ColorPicker>()
                    .expect("self.picker must be ColorPicker!");
                ui.send_message(ColorFieldMessage::color(
                    self.handle,
                    MessageDirection::ToWidget,
                    picker.color,
                ));
            }
        }
    }
}

pub struct ColorFieldBuilder {
    widget_builder: WidgetBuilder,
    color: Color,
}

impl ColorFieldBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            color: Color::WHITE,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let picker;
        let popup = PopupBuilder::new(WidgetBuilder::new())
            .with_content({
                picker = ColorPickerBuilder::new(WidgetBuilder::new())
                    .with_color(self.color)
                    .build(ctx);
                picker
            })
            .build(ctx);

        let field = ColorField {
            widget: self.widget_builder.with_preview_messages(true).build(),
            popup,
            picker,
            color: self.color,
        };
        ctx.add_node(UiNode::new(field))
    }
}
