use crate::{
    brush::Brush,
    core::{algebra::Vector2, color::Color, pool::Handle},
    define_constructor,
    draw::DrawingContext,
    formatted_text::{FormattedText, FormattedTextBuilder, WrapMode},
    message::{MessageDirection, UiMessage},
    ttf::SharedFont,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, HorizontalAlignment, UiNode, UserInterface, VerticalAlignment,
};
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone, PartialEq)]
pub enum TextMessage {
    Text(String),
    Wrap(WrapMode),
    Font(SharedFont),
    VerticalAlignment(VerticalAlignment),
    HorizontalAlignment(HorizontalAlignment),
    Shadow(bool),
    ShadowDilation(f32),
    ShadowBrush(Brush),
    ShadowOffset(Vector2<f32>),
}

impl TextMessage {
    define_constructor!(TextMessage:Text => fn text(String), layout: false);
    define_constructor!(TextMessage:Wrap=> fn wrap(WrapMode), layout: false);
    define_constructor!(TextMessage:Font => fn font(SharedFont), layout: false);
    define_constructor!(TextMessage:VerticalAlignment => fn vertical_alignment(VerticalAlignment), layout: false);
    define_constructor!(TextMessage:HorizontalAlignment => fn horizontal_alignment(HorizontalAlignment), layout: false);
    define_constructor!(TextMessage:Shadow => fn shadow(bool), layout: false);
    define_constructor!(TextMessage:ShadowDilation => fn shadow_dilation(f32), layout: false);
    define_constructor!(TextMessage:ShadowBrush => fn shadow_brush(Brush), layout: false);
    define_constructor!(TextMessage:ShadowOffset => fn shadow_offset(Vector2<f32>), layout: false);
}

#[derive(Clone)]
pub struct Text {
    pub widget: Widget,
    pub formatted_text: RefCell<FormattedText>,
}

crate::define_widget_deref!(Text);

impl Control for Text {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn measure_override(&self, _: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        self.formatted_text
            .borrow_mut()
            .set_constraint(available_size)
            .set_brush(self.widget.foreground())
            .build()
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        let bounds = self.widget.bounding_rect();
        drawing_context.draw_text(
            self.clip_bounds(),
            bounds.position,
            &self.formatted_text.borrow(),
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle() {
            if let Some(msg) = message.data::<TextMessage>() {
                let mut text_ref = self.formatted_text.borrow_mut();
                match msg {
                    TextMessage::Text(text) => {
                        text_ref.set_text(text);
                        drop(text_ref);
                        self.invalidate_layout();
                    }
                    &TextMessage::Wrap(wrap) => {
                        if text_ref.wrap_mode() != wrap {
                            text_ref.set_wrap(wrap);
                            drop(text_ref);
                            self.invalidate_layout();
                        }
                    }
                    TextMessage::Font(font) => {
                        if &text_ref.get_font() != font {
                            text_ref.set_font(font.clone());
                            drop(text_ref);
                            self.invalidate_layout();
                        }
                    }
                    &TextMessage::HorizontalAlignment(horizontal_alignment) => {
                        if text_ref.horizontal_alignment() != horizontal_alignment {
                            text_ref.set_horizontal_alignment(horizontal_alignment);
                            drop(text_ref);
                            self.invalidate_layout();
                        }
                    }
                    &TextMessage::VerticalAlignment(vertical_alignment) => {
                        if text_ref.vertical_alignment() != vertical_alignment {
                            text_ref.set_vertical_alignment(vertical_alignment);
                            drop(text_ref);
                            self.invalidate_layout();
                        }
                    }
                    &TextMessage::Shadow(shadow) => {
                        if text_ref.shadow != shadow {
                            text_ref.set_shadow(shadow);
                            drop(text_ref);
                            self.invalidate_layout();
                        }
                    }
                    TextMessage::ShadowBrush(brush) => {
                        if &text_ref.shadow_brush != brush {
                            text_ref.set_shadow_brush(brush.clone());
                            drop(text_ref);
                            self.invalidate_layout();
                        }
                    }
                    &TextMessage::ShadowDilation(dilation) => {
                        if text_ref.shadow_dilation != dilation {
                            text_ref.set_shadow_dilation(dilation);
                            drop(text_ref);
                            self.invalidate_layout();
                        }
                    }
                    &TextMessage::ShadowOffset(offset) => {
                        if text_ref.shadow_offset != offset {
                            text_ref.set_shadow_offset(offset);
                            drop(text_ref);
                            self.invalidate_layout();
                        }
                    }
                }
            }
        }
    }
}

impl Text {
    pub fn wrap_mode(&self) -> WrapMode {
        self.formatted_text.borrow().wrap_mode()
    }

    pub fn text(&self) -> String {
        self.formatted_text.borrow().text()
    }

    pub fn font(&self) -> SharedFont {
        self.formatted_text.borrow().get_font()
    }

    pub fn vertical_alignment(&self) -> VerticalAlignment {
        self.formatted_text.borrow().vertical_alignment()
    }

    pub fn horizontal_alignment(&self) -> HorizontalAlignment {
        self.formatted_text.borrow().horizontal_alignment()
    }
}

pub struct TextBuilder {
    widget_builder: WidgetBuilder,
    text: Option<String>,
    font: Option<SharedFont>,
    vertical_text_alignment: VerticalAlignment,
    horizontal_text_alignment: HorizontalAlignment,
    wrap: WrapMode,
    shadow: bool,
    shadow_brush: Brush,
    shadow_dilation: f32,
    shadow_offset: Vector2<f32>,
}

impl TextBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            text: None,
            font: None,
            vertical_text_alignment: VerticalAlignment::Top,
            horizontal_text_alignment: HorizontalAlignment::Left,
            wrap: WrapMode::NoWrap,
            shadow: false,
            shadow_brush: Brush::Solid(Color::BLACK),
            shadow_dilation: 1.0,
            shadow_offset: Vector2::new(1.0, 1.0),
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

    pub fn with_wrap(mut self, wrap: WrapMode) -> Self {
        self.wrap = wrap;
        self
    }

    /// Whether the shadow enabled or not.
    pub fn with_shadow(mut self, shadow: bool) -> Self {
        self.shadow = shadow;
        self
    }

    /// Sets desired shadow brush. It will be used to render the shadow.
    pub fn with_shadow_brush(mut self, brush: Brush) -> Self {
        self.shadow_brush = brush;
        self
    }

    /// Sets desired shadow dilation in units. Keep in mind that the dilation is absolute,
    /// not percentage-based.
    pub fn with_shadow_dilation(mut self, thickness: f32) -> Self {
        self.shadow_dilation = thickness;
        self
    }

    /// Sets desired shadow offset in units.
    pub fn with_shadow_offset(mut self, offset: Vector2<f32>) -> Self {
        self.shadow_offset = offset;
        self
    }

    pub fn build(mut self, ui: &mut BuildContext) -> Handle<UiNode> {
        let font = if let Some(font) = self.font {
            font
        } else {
            ui.default_font()
        };

        if self.widget_builder.foreground.is_none() {
            self.widget_builder.foreground = Some(Brush::Solid(Color::opaque(220, 220, 220)));
        }

        let text = Text {
            widget: self.widget_builder.build(),
            formatted_text: RefCell::new(
                FormattedTextBuilder::new(font)
                    .with_text(self.text.unwrap_or_default())
                    .with_vertical_alignment(self.vertical_text_alignment)
                    .with_horizontal_alignment(self.horizontal_text_alignment)
                    .with_wrap(self.wrap)
                    .with_shadow(self.shadow)
                    .with_shadow_brush(self.shadow_brush)
                    .with_shadow_dilation(self.shadow_dilation)
                    .with_shadow_offset(self.shadow_offset)
                    .build(),
            ),
        };
        ui.add_node(UiNode::new(text))
    }
}
