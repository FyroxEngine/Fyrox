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

//! A mixin that provides selection functionality for a widget.

use crate::fyrox::{
    core::pool::Handle,
    core::{reflect::prelude::*, visitor::prelude::*},
    gui::message::{MessageDirection, MouseButton, UiMessage},
    gui::widget::WidgetMessage,
    gui::{define_constructor, UiNode, UserInterface},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectableMessage {
    Select(bool),
}

impl SelectableMessage {
    define_constructor!(SelectableMessage:Select => fn select(bool), layout: false);
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Visit, Reflect)]
pub struct Selectable {
    pub selected: bool,
}

impl Selectable {
    pub fn handle_routed_message(
        &mut self,
        self_handle: Handle<UiNode>,
        ui: &mut UserInterface,
        message: &mut UiMessage,
    ) {
        if let Some(msg) = message.data::<WidgetMessage>() {
            match msg {
                WidgetMessage::MouseDown { button, .. } => {
                    if (*button == MouseButton::Left || *button == MouseButton::Right)
                        && !self.selected
                    {
                        ui.send_message(SelectableMessage::select(
                            self_handle,
                            MessageDirection::ToWidget,
                            true,
                        ));

                        ui.capture_mouse(self_handle);
                    }
                }
                WidgetMessage::MouseUp { button, .. } => {
                    if *button == MouseButton::Left || *button == MouseButton::Right {
                        ui.release_mouse_capture();
                    }
                }
                _ => {}
            }
        } else if let Some(SelectableMessage::Select(selected)) = message.data() {
            if message.destination() == self_handle
                && message.direction() == MessageDirection::ToWidget
                && self.selected != *selected
            {
                self.selected = *selected;
                ui.send_message(message.reverse());
            }
        }
    }
}
