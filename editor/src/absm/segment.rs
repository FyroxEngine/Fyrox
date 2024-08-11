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

use crate::fyrox::{
    core::{algebra::Vector2, pool::Handle, reflect::prelude::*, visitor::prelude::*},
    gui::{
        define_constructor,
        message::{MessageDirection, UiMessage},
        UiNode,
    },
};

#[derive(Debug, Clone, PartialEq)]
pub enum SegmentMessage {
    SourcePosition(Vector2<f32>),
    DestPosition(Vector2<f32>),
}

impl SegmentMessage {
    define_constructor!(SegmentMessage:SourcePosition => fn source_position(Vector2<f32>), layout: false);
    define_constructor!(SegmentMessage:DestPosition => fn dest_position(Vector2<f32>), layout: false);
}

#[derive(Debug, Clone, Reflect, Visit)]
pub struct Segment {
    pub source: Handle<UiNode>,
    pub source_pos: Vector2<f32>,
    pub dest: Handle<UiNode>,
    pub dest_pos: Vector2<f32>,
}

impl Segment {
    pub fn handle_routed_message(&mut self, self_handle: Handle<UiNode>, message: &mut UiMessage) {
        if let Some(msg) = message.data::<SegmentMessage>() {
            if message.destination() == self_handle
                && message.direction() == MessageDirection::ToWidget
            {
                match msg {
                    SegmentMessage::SourcePosition(pos) => {
                        self.source_pos = *pos;
                    }
                    SegmentMessage::DestPosition(pos) => {
                        self.dest_pos = *pos;
                    }
                }
            }
        }
    }
}
