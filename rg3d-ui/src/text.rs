use crate::core::algebra::Vector2;
use crate::message::MessageData;
use crate::ttf::SharedFont;
use crate::{
    brush::Brush,
    core::{color::Color, pool::Handle},
    draw::DrawingContext,
    formatted_text::{FormattedText, FormattedTextBuilder},
    message::UiMessage,
    message::{TextMessage, UiMessageData},
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, HorizontalAlignment, UINode, UserInterface, VerticalAlignment,
};
use std::{
    cell::RefCell,
    ops::{Deref, DerefMut},
};

#[derive(Clone)]
pub struct Text<M: MessageData, C: Control<M, C>> {
    widget: Widget<M, C>,
    formatted_text: RefCell<FormattedText>,
}

crate::define_widget_deref!(Text<M, C>);

impl<M: MessageData, C: Control<M, C>> Control<M, C> for Text<M, C> {
    fn measure_override(
        &self,
        _: &UserInterface<M, C>,
        available_size: Vector2<f32>,
    ) -> Vector2<f32> {
        self.formatted_text
            .borrow_mut()
            .set_constraint(available_size)
            .set_brush(self.widget.foreground())
            .build()
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        let bounds = self.widget.screen_bounds();
        drawing_context.draw_text(
            self.clip_bounds(),
            bounds.position,
            &self.formatted_text.borrow(),
        );
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle() {
            if let UiMessageData::Text(msg) = &message.data() {
                match msg {
                    TextMessage::Text(text) => {
                        self.formatted_text.borrow_mut().set_text(text);
                        self.invalidate_layout();
                    }
                    &TextMessage::Wrap(wrap) => {
                        if self.formatted_text.borrow().is_wrap() != wrap {
                            self.formatted_text.borrow_mut().set_wrap(wrap);
                            self.invalidate_layout();
                        }
                    }
                    TextMessage::Font(font) => {
                        self.formatted_text.borrow_mut().set_font(font.clone());
                        self.invalidate_layout();
                    }
                    &TextMessage::HorizontalAlignment(horizontal_alignment) => {
                        self.formatted_text
                            .borrow_mut()
                            .set_horizontal_alignment(horizontal_alignment);
                        self.invalidate_layout();
                    }
                    &TextMessage::VerticalAlignment(vertical_alignment) => {
                        self.formatted_text
                            .borrow_mut()
                            .set_vertical_alignment(vertical_alignment);
                        self.invalidate_layout();
                    }
                }
            }
        }
    }
}

impl<M: MessageData, C: Control<M, C>> Text<M, C> {
    pub fn new(widget: Widget<M, C>) -> Self {
        Self {
            widget,
            formatted_text: RefCell::new(
                FormattedTextBuilder::new()
                    .with_font(crate::DEFAULT_FONT.clone())
                    .build(),
            ),
        }
    }

    pub fn is_wrap(&self) -> bool {
        self.formatted_text.borrow().is_wrap()
    }

    pub fn text(&self) -> String {
        self.formatted_text.borrow().text()
    }

    pub fn font(&self) -> SharedFont {
        self.formatted_text.borrow().get_font().unwrap()
    }

    pub fn vertical_alignment(&self) -> VerticalAlignment {
        self.formatted_text.borrow().vertical_alignment()
    }

    pub fn horizontal_alignment(&self) -> HorizontalAlignment {
        self.formatted_text.borrow().horizontal_alignment()
    }
}

pub struct TextBuilder<M: MessageData, C: Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    text: Option<String>,
    font: Option<SharedFont>,
    vertical_text_alignment: VerticalAlignment,
    horizontal_text_alignment: HorizontalAlignment,
    wrap: bool,
}

impl<M: MessageData, C: Control<M, C>> TextBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            text: None,
            font: None,
            vertical_text_alignment: VerticalAlignment::Top,
            horizontal_text_alignment: HorizontalAlignment::Left,
            wrap: false,
        }
    }

    pub fn with_text<P: AsRef<str>>(mut self, text: P) -> Self {
        self.text = Some(text.as_ref().to_owned());
        self
    }

    pub fn with_font(mut self, font: SharedFont) -> Self {
        self.font = Some(font);
        self
    }

    pub fn with_opt_font(mut self, font: Option<SharedFont>) -> Self {
        self.font = font;
        self
    }

    pub fn with_vertical_text_alignment(mut self, valign: VerticalAlignment) -> Self {
        self.vertical_text_alignment = valign;
        self
    }

    pub fn with_horizontal_text_alignment(mut self, halign: HorizontalAlignment) -> Self {
        self.horizontal_text_alignment = halign;
        self
    }

    pub fn with_wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn build(mut self, ui: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        let font = if let Some(font) = self.font {
            font
        } else {
            crate::DEFAULT_FONT.clone()
        };

        if self.widget_builder.foreground.is_none() {
            self.widget_builder.foreground = Some(Brush::Solid(Color::opaque(220, 220, 220)));
        }

        let text = Text {
            widget: self.widget_builder.build(),
            formatted_text: RefCell::new(
                FormattedTextBuilder::new()
                    .with_text(self.text.unwrap_or_default())
                    .with_vertical_alignment(self.vertical_text_alignment)
                    .with_horizontal_alignment(self.horizontal_text_alignment)
                    .with_font(font)
                    .with_wrap(self.wrap)
                    .build(),
            ),
        };
        ui.add_node(UINode::Text(text))
    }
}
