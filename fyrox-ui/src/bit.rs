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

//! A widget that shows numeric value as a set of individual bits allowing switching separate bits.

use crate::{
    brush::Brush,
    core::{
        algebra::Vector2,
        color::Color,
        math::Rect,
        num_traits::{Euclid, NumCast, One, Zero},
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        uuid::uuid,
        visitor::prelude::*,
    },
    define_constructor,
    draw::{CommandTexture, Draw, DrawingContext},
    message::{ButtonState, UiMessage},
    utils::load_image,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, MessageDirection, MouseButton, UiNode, UserInterface, WidgetMessage,
};
use fyrox_graph::constructor::{ConstructorProvider, GraphNodeConstructor};
use fyrox_texture::TextureResource;
use std::{
    fmt::Debug,
    mem,
    ops::{BitAnd, BitOr, Deref, DerefMut, Not, Shl},
    sync::LazyLock,
};

static BIT_ICONS: LazyLock<Option<TextureResource>> =
    LazyLock::new(|| load_image(include_bytes!("resources/bits.png")));

const BIT_SIZE: f32 = 16.0;
const BYTE_GAP: f32 = 8.0;
const ROW_GAP: f32 = 4.0;

const ON_NORMAL: Brush = Brush::Solid(Color::DARK_GRAY);
const ON_HOVER: Brush = Brush::Solid(Color::LIGHT_BLUE);
const OFF_HOVER: Brush = Brush::Solid(Color::DARK_SLATE_BLUE);

pub trait BitContainer:
    BitAnd<Output = Self>
    + BitOr<Output = Self>
    + Clone
    + Copy
    + Default
    + One
    + Shl<Output = Self>
    + NumCast
    + Not<Output = Self>
    + Zero
    + PartialEq
    + Debug
    + Reflect
    + Visit
    + Send
    + TypeUuidProvider
    + 'static
{
}

impl<T> BitContainer for T where
    T: BitAnd<Output = Self>
        + BitOr<Output = Self>
        + Clone
        + Copy
        + Default
        + One
        + Shl<Output = Self>
        + NumCast
        + Not<Output = Self>
        + Zero
        + PartialEq
        + Debug
        + Reflect
        + Visit
        + Send
        + TypeUuidProvider
        + 'static
{
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BitFieldMessage<T: BitContainer> {
    Value(T),
}

impl<T: BitContainer> BitFieldMessage<T> {
    define_constructor!(BitFieldMessage:Value => fn value(T), layout: false);
}

impl<T: BitContainer> ConstructorProvider<UiNode, UserInterface> for BitField<T> {
    fn constructor() -> GraphNodeConstructor<UiNode, UserInterface> {
        GraphNodeConstructor::new::<Self>()
            .with_variant(format!("Bit Field<{}>", std::any::type_name::<T>()), |ui| {
                BitFieldBuilder::<T>::new(WidgetBuilder::new())
                    .build(&mut ui.build_ctx())
                    .into()
            })
            .with_group("Bit")
    }
}

#[derive(Default, Clone, Reflect, Visit, Debug, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct BitField<T>
where
    T: BitContainer,
{
    pub widget: Widget,
    pub value: T,
    #[visit(skip)]
    #[reflect(hidden)]
    current_bit: usize,
    #[visit(skip)]
    #[reflect(hidden)]
    bit_state: BitState,
    #[visit(skip)]
    #[reflect(hidden)]
    current_value: bool,
}

impl<T> Deref for BitField<T>
where
    T: BitContainer,
{
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<T> DerefMut for BitField<T>
where
    T: BitContainer,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
enum BitState {
    #[default]
    Normal,
    Hovered,
    Pressed,
}

#[must_use]
fn set_bit_value<T: BitContainer>(value: T, index: usize, bit_value: bool) -> T {
    if bit_value {
        set_bit(value, index)
    } else {
        reset_bit(value, index)
    }
}

#[must_use]
fn set_bit<T: BitContainer>(value: T, index: usize) -> T {
    value | (T::one() << T::from(index).unwrap_or_default())
}

#[must_use]
fn reset_bit<T: BitContainer>(value: T, index: usize) -> T {
    value & !(T::one() << T::from(index).unwrap_or_default())
}

#[must_use]
fn is_bit_set<T: BitContainer>(value: T, index: usize) -> bool {
    value & (T::one() << T::from(index).unwrap_or_default()) != T::zero()
}

fn byte_width(width: f32) -> usize {
    let byte_size = BIT_SIZE * 8.0;
    let col_stride = byte_size + BYTE_GAP;
    ((width - byte_size) / col_stride).floor().max(0.0) as usize + 1
}

fn bit_to_rect(index: usize, width: usize) -> Rect<f32> {
    let (byte_index, bit_index) = index.div_rem_euclid(&8);
    let (byte_y, byte_x) = byte_index.div_rem_euclid(&width);
    let row_stride = BIT_SIZE + ROW_GAP;
    let col_stride = BIT_SIZE * 8.0 + BYTE_GAP;
    let x = col_stride * byte_x as f32 + BIT_SIZE * bit_index as f32;
    let y = row_stride * byte_y as f32;
    Rect::new(x, y, BIT_SIZE, BIT_SIZE)
}

fn position_to_bit(position: Vector2<f32>, size: usize, width: f32) -> Option<usize> {
    let byte_width = byte_width(width);
    (0..size).find(|&i| bit_to_rect(i, byte_width).contains(position))
}

impl<T> TypeUuidProvider for BitField<T>
where
    T: BitContainer,
{
    fn type_uuid() -> Uuid {
        combine_uuids(
            uuid!("6c19b266-18be-46d2-bfd3-f1dc9cb3f36c"),
            T::type_uuid(),
        )
    }
}

impl<T> Control for BitField<T>
where
    T: BitContainer,
{
    fn measure_override(&self, _ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        let size = mem::size_of::<T>();
        let byte_size = BIT_SIZE * 8.0;
        let width = available_size.x;
        let byte_width = if width.is_finite() {
            byte_width(width)
        } else {
            2
        };
        let (byte_height, rem) = size.div_rem_euclid(&byte_width);
        let byte_height = byte_height + if rem > 0 { 1 } else { 0 };
        let byte_height = byte_height.max(1);
        let byte_width = byte_width.min(size);
        let width = byte_width as f32 * byte_size + (byte_width - 1) as f32 * BYTE_GAP;
        let height = byte_height as f32 * BIT_SIZE + (byte_height - 1) as f32 * ROW_GAP;
        Vector2::new(width, height)
    }
    fn draw(&self, ctx: &mut DrawingContext) {
        let value = self.value;
        let width = self.actual_local_size().x;
        let byte_width = byte_width(width);
        let bit_count = mem::size_of::<T>() * 8;
        for i in 0..bit_count {
            if (self.current_bit != i || self.bit_state == BitState::Normal) && is_bit_set(value, i)
            {
                self.draw_bit_background(i, byte_width, ctx);
            }
        }
        ctx.commit(self.clip_bounds(), ON_NORMAL, CommandTexture::None, None);
        for i in 0..bit_count {
            if (self.current_bit != i || self.bit_state == BitState::Normal)
                && !is_bit_set(value, i)
            {
                self.draw_bit_background(i, byte_width, ctx);
            }
        }
        ctx.commit(
            self.clip_bounds(),
            Brush::Solid(Color::TRANSPARENT),
            CommandTexture::None,
            None,
        );
        if self.bit_state != BitState::Normal {
            let i = self.current_bit;
            self.draw_bit_background(i, byte_width, ctx);
            if is_bit_set(value, i) {
                ctx.commit(self.clip_bounds(), ON_HOVER, CommandTexture::None, None);
            } else {
                ctx.commit(self.clip_bounds(), OFF_HOVER, CommandTexture::None, None);
            }
        }
        for i in 0..bit_count {
            if is_bit_set(value, i) {
                self.draw_bit_foreground(i, byte_width, ctx);
            }
        }
        ctx.commit(
            self.clip_bounds(),
            Brush::Solid(Color::WHITE),
            CommandTexture::None,
            None,
        );
        for i in 0..bit_count {
            if is_bit_set(value, i) {
                self.draw_bit_icon(i, byte_width, ctx);
            }
        }
        ctx.commit(
            self.clip_bounds(),
            Brush::Solid(Color::BLACK),
            CommandTexture::Texture(BIT_ICONS.clone().unwrap()),
            None,
        );
        for i in 0..bit_count {
            if !is_bit_set(value, i) {
                self.draw_bit_icon(i, byte_width, ctx);
            }
        }
        ctx.commit(
            self.clip_bounds(),
            Brush::Solid(Color::GRAY),
            CommandTexture::Texture(BIT_ICONS.clone().unwrap()),
            None,
        );
    }
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(WidgetMessage::MouseMove { pos, state }) = message.data() {
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::FromWidget
            {
                let pos = self.screen_to_local(*pos);
                let size = mem::size_of::<T>() * 8;
                self.bit_state = BitState::Normal;
                if let Some(bit_index) = position_to_bit(pos, size, self.actual_local_size().x) {
                    self.current_bit = bit_index;
                    let mut new_value = self.value;
                    match state.left {
                        ButtonState::Pressed => {
                            new_value = set_bit_value(new_value, bit_index, self.current_value);
                            self.bit_state = BitState::Pressed;
                        }
                        ButtonState::Released => {
                            self.bit_state = BitState::Hovered;
                        }
                    }

                    if new_value != self.value {
                        ui.send_message(BitFieldMessage::value(
                            self.handle,
                            MessageDirection::ToWidget,
                            new_value,
                        ));
                    }
                }
            }
        } else if let Some(WidgetMessage::MouseDown { pos, button }) = message.data() {
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::FromWidget
            {
                let pos = self.screen_to_local(*pos);
                let size = mem::size_of::<T>() * 8;
                self.bit_state = BitState::Normal;
                if let Some(bit_index) = position_to_bit(pos, size, self.actual_local_size().x) {
                    self.current_bit = bit_index;
                    match button {
                        MouseButton::Left => {
                            message.set_handled(true);
                            self.bit_state = BitState::Pressed;
                            self.current_value = !is_bit_set(self.value, bit_index);
                            let new_value =
                                set_bit_value(self.value, bit_index, self.current_value);
                            self.bit_state = BitState::Pressed;

                            ui.send_message(BitFieldMessage::value(
                                self.handle,
                                MessageDirection::ToWidget,
                                new_value,
                            ));
                        }
                        MouseButton::Right => {
                            message.set_handled(true);
                            self.bit_state = BitState::Hovered;
                            let new_value = if is_bit_set(self.value, bit_index) {
                                !(T::one() << T::from(bit_index).unwrap_or_default())
                            } else {
                                T::one() << T::from(bit_index).unwrap_or_default()
                            };

                            ui.send_message(BitFieldMessage::value(
                                self.handle,
                                MessageDirection::ToWidget,
                                new_value,
                            ));
                        }
                        _ => (),
                    }
                }
            }
        } else if let Some(WidgetMessage::MouseLeave)
        | Some(WidgetMessage::MouseUp {
            button: MouseButton::Left,
            ..
        }) = message.data()
        {
            if message.destination() == self.handle() {
                self.bit_state = BitState::Normal;
            }
        } else if let Some(BitFieldMessage::Value(value)) = message.data() {
            if message.destination() == self.handle
                && message.direction() == MessageDirection::ToWidget
                && *value != self.value
            {
                self.value = *value;
                ui.send_message(message.reverse());
            }
        }
    }
}

impl<T> BitField<T>
where
    T: BitContainer,
{
    fn screen_to_local(&self, position: Vector2<f32>) -> Vector2<f32> {
        let trans = self.visual_transform();
        let Some(trans) = trans.try_inverse() else {
            return position;
        };
        trans.transform_point(&position.into()).coords
    }
    fn draw_bit_background(&self, index: usize, width: usize, ctx: &mut DrawingContext) {
        let rect = bit_to_rect(index, width);
        ctx.push_rect_filled(&rect, None);
    }
    fn draw_bit_foreground(&self, index: usize, width: usize, ctx: &mut DrawingContext) {
        let rect = bit_to_rect(index, width);
        ctx.push_rect(&rect, 1.0);
    }
    fn draw_bit_icon(&self, index: usize, width: usize, ctx: &mut DrawingContext) {
        let rect = bit_to_rect(index, width);
        let center = rect.center();
        let rect = Rect::new(center.x - 4.0, center.y - 4.0, 8.0, 8.0);
        let i = index % 32;
        let u = i as f32 / 32.0;
        let u1 = (i + 1) as f32 / 32.0;
        let t0 = Vector2::new(u, 0.0);
        let t1 = Vector2::new(u1, 0.0);
        let t2 = Vector2::new(u1, 1.0);
        let t3 = Vector2::new(u, 1.0);
        ctx.push_rect_filled(&rect, Some(&[t0, t1, t2, t3]));
    }
}

pub struct BitFieldBuilder<T>
where
    T: BitContainer,
{
    widget_builder: WidgetBuilder,
    value: T,
}

impl<T> BitFieldBuilder<T>
where
    T: BitContainer,
{
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            value: T::default(),
        }
    }

    pub fn with_value(mut self, value: T) -> Self {
        self.value = value;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let canvas = BitField {
            widget: self.widget_builder.build(ctx),
            value: self.value,
            current_bit: 0,
            bit_state: BitState::Normal,
            current_value: false,
        };
        ctx.add_node(UiNode::new(canvas))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::bit::{byte_width, BitFieldBuilder};
    use crate::{test::test_widget_deletion, widget::WidgetBuilder};

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| BitFieldBuilder::<usize>::new(WidgetBuilder::new()).build(ctx));
    }

    #[test]
    fn test_byte_width() {
        assert_eq!(byte_width(0.0), 1);
        assert_eq!(byte_width(8.0 * BIT_SIZE), 1);
        assert_eq!(byte_width(16.0 * BIT_SIZE), 1);
        assert_eq!(byte_width(16.0 * BIT_SIZE + BYTE_GAP), 2);
        assert_eq!(byte_width(24.0 * BIT_SIZE + 2.0 * BYTE_GAP), 3);
        assert_eq!(byte_width(32.0 * BIT_SIZE + 2.0 * BYTE_GAP), 3);
        assert_eq!(byte_width(32.0 * BIT_SIZE + 3.0 * BYTE_GAP), 4);
        assert_eq!(byte_width(40.0 * BIT_SIZE + 4.0 * BYTE_GAP), 5);
    }
}
