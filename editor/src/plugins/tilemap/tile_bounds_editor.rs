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

use super::*;

use fyrox::{
    core::{pool::Handle, reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*},
    gui::{
        button::ButtonMessage,
        define_constructor, define_widget_deref,
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessage},
        stack_panel::StackPanelBuilder,
        vec::{Vec2EditorMessage, VecEditorBuilder},
        widget::{Widget, WidgetBuilder, WidgetMessage},
        BuildContext, Control, UiNode, UserInterface,
    },
    scene::tilemap::{tileset::TileBounds, OrthoTransform},
};
use std::ops::{Deref, DerefMut};

#[derive(Debug, PartialEq, Clone)]
pub enum TileBoundsMessage {
    Value(Option<TileBounds>),
    Turn(i8),
    FlipX,
    FlipY,
}

impl TileBoundsMessage {
    define_constructor!(TileBoundsMessage:Value => fn value(Option<TileBounds>), layout: false);
    define_constructor!(TileBoundsMessage:Turn => fn turn(i8), layout: false);
    define_constructor!(TileBoundsMessage:FlipX => fn flip_x(), layout: false);
    define_constructor!(TileBoundsMessage:FlipY => fn flip_y(), layout: false);
}

#[derive(Clone, Default, Debug, Visit, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "1e600103-6516-4c5a-a30b-f90f64fc9623")]
#[reflect(derived_type = "UiNode")]
pub struct TileBoundsEditor {
    widget: Widget,
    pub value: Option<TileBounds>,
    pub value_area: Handle<UiNode>,
    pub left_top: Handle<UiNode>,
    pub left_bottom: Handle<UiNode>,
    pub right_top: Handle<UiNode>,
    pub right_bottom: Handle<UiNode>,
    pub button_left: Handle<UiNode>,
    pub button_right: Handle<UiNode>,
    pub button_flip_x: Handle<UiNode>,
    pub button_flip_y: Handle<UiNode>,
}

define_widget_deref!(TileBoundsEditor);

impl TileBoundsEditor {
    fn get_field(&self, index: usize) -> Handle<UiNode> {
        match index {
            0 => self.left_bottom,
            1 => self.right_bottom,
            2 => self.right_top,
            3 => self.left_top,
            _ => panic!(),
        }
    }
    fn turn(&mut self, amount: i8, ui: &mut UserInterface) {
        if let Some(value) = self.value.clone().map(|v| v.rotated(amount)) {
            ui.send_message(TileBoundsMessage::value(
                self.handle,
                MessageDirection::ToWidget,
                Some(value),
            ));
        } else {
            ui.send_message(TileBoundsMessage::turn(
                self.handle,
                MessageDirection::FromWidget,
                amount,
            ));
        }
    }
    fn flip_x(&mut self, ui: &mut UserInterface) {
        if let Some(value) = self.value.clone().map(|v| v.x_flipped()) {
            ui.send_message(TileBoundsMessage::value(
                self.handle,
                MessageDirection::ToWidget,
                Some(value),
            ));
        } else {
            ui.send_message(TileBoundsMessage::flip_x(
                self.handle,
                MessageDirection::FromWidget,
            ));
        }
    }
    fn flip_y(&mut self, ui: &mut UserInterface) {
        if let Some(value) = self.value.clone().map(|v| v.y_flipped()) {
            ui.send_message(TileBoundsMessage::value(
                self.handle,
                MessageDirection::ToWidget,
                Some(value),
            ));
        } else {
            ui.send_message(TileBoundsMessage::flip_y(
                self.handle,
                MessageDirection::FromWidget,
            ));
        }
    }
    fn set_value(
        &mut self,
        new_value: &Option<TileBounds>,
        message: &UiMessage,
        ui: &mut UserInterface,
    ) {
        match (&self.value, new_value) {
            (None, None) => (),
            (Some(_), None) => {
                ui.send_message(WidgetMessage::visibility(
                    self.value_area,
                    MessageDirection::ToWidget,
                    false,
                ));
                self.value = None;
                ui.send_message(message.reverse());
            }
            (None, Some(v)) => {
                ui.send_message(WidgetMessage::visibility(
                    self.value_area,
                    MessageDirection::ToWidget,
                    true,
                ));
                self.value = Some(v.clone());
                for i in 0..4 {
                    ui.send_message(Vec2EditorMessage::value(
                        self.get_field(i),
                        MessageDirection::ToWidget,
                        v.get(i),
                    ));
                }
                ui.send_message(message.reverse());
            }
            (Some(v0), Some(v1)) => {
                let mut has_changed = false;
                for i in 0..4 {
                    if v0.get(i) != v1.get(i) {
                        has_changed = true;
                        ui.send_message(Vec2EditorMessage::value(
                            self.get_field(i),
                            MessageDirection::ToWidget,
                            v1.get(i),
                        ));
                    }
                }
                self.value = Some(v1.clone());
                if has_changed {
                    ui.send_message(message.reverse());
                }
            }
        }
    }
}

impl Control for TileBoundsEditor {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);
        if let Some(Vec2EditorMessage::<u32>::Value(v)) = message.data() {
            if message.direction() == MessageDirection::FromWidget {
                for i in 0..4 {
                    if self.get_field(i) == message.destination() {
                        let mut value = self.value.clone().unwrap_or_default();
                        *value.get_mut(i) = *v;
                        // This does not trigger a Value FromWidget message from the VecEditor
                        // because the values of the fields have not changed.
                        ui.send_message(TileBoundsMessage::value(
                            self.handle,
                            MessageDirection::ToWidget,
                            Some(value),
                        ));
                    }
                }
            }
        } else if let Some(TileBoundsMessage::Value(v)) = message.data() {
            if message.direction() == MessageDirection::ToWidget {
                self.set_value(v, message, ui);
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.button_left {
                self.turn(1, ui)
            } else if message.destination() == self.button_right {
                self.turn(-1, ui)
            } else if message.destination() == self.button_flip_x {
                self.flip_x(ui)
            } else if message.destination() == self.button_flip_y {
                self.flip_y(ui)
            }
        }
    }
}

pub struct TileBoundsEditorBuilder {
    widget_builder: WidgetBuilder,
}

impl TileBoundsEditorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self { widget_builder }
    }
    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let left_top = VecEditorBuilder::<u32, 2>::new(WidgetBuilder::new()).build(ctx);
        let right_top =
            VecEditorBuilder::<u32, 2>::new(WidgetBuilder::new().on_column(1)).build(ctx);
        let left_bottom =
            VecEditorBuilder::<u32, 2>::new(WidgetBuilder::new().on_row(1)).build(ctx);
        let right_bottom =
            VecEditorBuilder::<u32, 2>::new(WidgetBuilder::new().on_row(1).on_column(1)).build(ctx);
        let value_area = GridBuilder::new(
            WidgetBuilder::new()
                .with_visibility(false)
                .with_child(left_top)
                .with_child(right_top)
                .with_child(left_bottom)
                .with_child(right_bottom),
        )
        .add_column(Column::stretch())
        .add_column(Column::stretch())
        .add_row(Row::auto())
        .add_row(Row::auto())
        .build(ctx);
        let width = 20.0;
        let height = 20.0;
        let left_button = make_drawing_mode_button(
            ctx,
            width,
            height,
            TURN_LEFT_IMAGE.clone(),
            "Rotate left 90 degrees.",
            Some(0),
        );
        let right_button = make_drawing_mode_button(
            ctx,
            width,
            height,
            TURN_RIGHT_IMAGE.clone(),
            "Rotate right 90 degrees.",
            Some(0),
        );
        let flip_x_button = make_drawing_mode_button(
            ctx,
            width,
            height,
            FLIP_X_IMAGE.clone(),
            "Flip along the x-axis.",
            Some(0),
        );
        let flip_y_button = make_drawing_mode_button(
            ctx,
            width,
            height,
            FLIP_Y_IMAGE.clone(),
            "Flip along the y-axis.",
            Some(0),
        );
        let buttons = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_child(left_button)
                .with_child(right_button)
                .with_child(flip_x_button)
                .with_child(flip_y_button),
        )
        .with_orientation(fyrox::gui::Orientation::Horizontal)
        .build(ctx);
        let content = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_child(value_area)
                .with_child(buttons),
        )
        .build(ctx);
        ctx.add_node(UiNode::new(TileBoundsEditor {
            widget: self.widget_builder.with_child(content).build(ctx),
            value: None,
            value_area,
            left_top,
            right_top,
            left_bottom,
            right_bottom,
            button_left: left_button,
            button_right: right_button,
            button_flip_x: flip_x_button,
            button_flip_y: flip_y_button,
        }))
    }
}
