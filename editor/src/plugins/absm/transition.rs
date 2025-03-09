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

use crate::fyrox::core::pool::ErasedHandle;
use crate::fyrox::{
    core::{
        algebra::Vector2,
        color::{Color, Hsv},
        math::Rect,
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        uuid_provider,
        visitor::prelude::*,
    },
    gui::{
        brush::Brush,
        define_constructor, define_widget_deref,
        draw::{CommandTexture, Draw, DrawingContext},
        message::{MessageDirection, UiMessage},
        widget::{Widget, WidgetBuilder, WidgetMessage},
        BuildContext, Control, UiNode, UserInterface,
    },
};
use crate::plugins::absm::{
    segment::Segment,
    selectable::{Selectable, SelectableMessage},
};
use crate::utils::fetch_node_center;

use fyrox::gui::style::resource::StyleResourceExt;
use fyrox::gui::style::Style;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionMessage {
    Activate,
}

impl TransitionMessage {
    define_constructor!(TransitionMessage:Activate => fn activate(), layout: false);
}

#[derive(Clone, Debug, Visit, Reflect, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct TransitionView {
    widget: Widget,
    pub segment: Segment,
    pub model_handle: ErasedHandle,
    #[component(include)]
    selectable: Selectable,
    activity_factor: f32,
}

impl TransitionView {
    fn handle_selection_change(&self, ui: &UserInterface) {
        ui.send_message(WidgetMessage::foreground(
            self.handle(),
            MessageDirection::ToWidget,
            if self.selectable.selected {
                ui.style.property(Style::BRUSH_BRIGHT)
            } else {
                ui.style.property(Style::BRUSH_LIGHTER)
            },
        ));
    }
}

define_widget_deref!(TransitionView);

pub fn draw_transition(
    drawing_context: &mut DrawingContext,
    clip_bounds: Rect<f32>,
    brush: Brush,
    source_pos: Vector2<f32>,
    dest_pos: Vector2<f32>,
) {
    drawing_context.push_line(source_pos, dest_pos, 4.0);

    let axis = (dest_pos - source_pos).normalize();
    let center = (dest_pos + source_pos).scale(0.5);
    let perp = Vector2::new(axis.y, -axis.x).normalize();

    let size = 18.0;

    drawing_context.push_triangle_filled([
        center + axis.scale(size),
        center + perp.scale(size * 0.5),
        center - perp.scale(size * 0.5),
    ]);

    drawing_context.commit(clip_bounds, brush, CommandTexture::None, None);
}

uuid_provider!(TransitionView = "01798aee-8fe5-4480-a69d-8e5b95c3cc96");

impl Control for TransitionView {
    fn draw(&self, drawing_context: &mut DrawingContext) {
        let brush = if let Brush::Solid(color) = self.foreground() {
            Brush::Solid(color + Color::from(Hsv::new(180.0, 100.0, 50.0 * self.activity_factor)))
        } else {
            drawing_context.style.get_or_default(Style::BRUSH_LIGHTER)
        };

        draw_transition(
            drawing_context,
            self.clip_bounds(),
            brush,
            self.segment.source_pos,
            self.segment.dest_pos,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);
        self.selectable
            .handle_routed_message(self.handle(), ui, message);
        self.segment.handle_routed_message(self.handle(), message);

        if let Some(msg) = message.data::<WidgetMessage>() {
            match msg {
                WidgetMessage::MouseEnter => {
                    ui.send_message(WidgetMessage::foreground(
                        self.handle(),
                        MessageDirection::ToWidget,
                        ui.style.property(Style::BRUSH_LIGHTEST),
                    ));
                }
                WidgetMessage::MouseLeave => {
                    self.handle_selection_change(ui);
                }
                _ => (),
            }
        } else if let Some(SelectableMessage::Select(_)) = message.data() {
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::FromWidget
            {
                self.handle_selection_change(ui);
            }
        } else if let Some(TransitionMessage::Activate) = message.data() {
            self.activity_factor = 1.0;
        }
    }

    fn update(&mut self, dt: f32, _ui: &mut UserInterface) {
        // Slowly fade.
        self.activity_factor = (self.activity_factor - dt).max(0.0);
    }
}

pub struct TransitionBuilder {
    widget_builder: WidgetBuilder,
    source: Handle<UiNode>,
    dest: Handle<UiNode>,
}

impl TransitionBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            source: Default::default(),
            dest: Default::default(),
        }
    }

    pub fn with_source(mut self, source: Handle<UiNode>) -> Self {
        self.source = source;
        self
    }

    pub fn with_dest(mut self, dest: Handle<UiNode>) -> Self {
        self.dest = dest;
        self
    }

    pub fn build(self, model_handle: ErasedHandle, ctx: &mut BuildContext) -> Handle<UiNode> {
        let transition = TransitionView {
            widget: self
                .widget_builder
                .with_foreground(ctx.style.property(Style::BRUSH_LIGHTEST))
                .with_clip_to_bounds(false)
                .with_need_update(true)
                .build(ctx),
            segment: Segment {
                source: self.source,
                source_pos: fetch_node_center(self.source, ctx),
                dest: self.dest,
                dest_pos: fetch_node_center(self.dest, ctx),
            },
            model_handle,
            selectable: Selectable::default(),
            activity_factor: 0.0,
        };

        ctx.add_node(UiNode::new(transition))
    }
}

#[cfg(test)]
mod test {
    use crate::plugins::absm::transition::TransitionBuilder;
    use fyrox::{gui::test::test_widget_deletion, gui::widget::WidgetBuilder};

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| {
            TransitionBuilder::new(WidgetBuilder::new()).build(Default::default(), ctx)
        });
    }
}
