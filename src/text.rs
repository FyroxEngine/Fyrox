use crate::{
    core::{
        pool::Handle,
        math::vec2::Vec2,
        color::Color
    },
    VerticalAlignment,
    HorizontalAlignment,
    draw::DrawingContext,
    formatted_text::{
        FormattedText,
        FormattedTextBuilder,
    },
    widget::{
        Widget,
        WidgetBuilder,
    },
    UINode,
    Control,
    ttf::Font,
    UserInterface,
    brush::Brush,
    message::UiMessage
};
use std::{
    sync::{
        Mutex,
        Arc,
    },
    cell::RefCell,
};
use std::ops::{Deref, DerefMut};

pub struct Text<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    formatted_text: RefCell<FormattedText>,
}

impl<M: 'static, C: 'static + Control<M, C>> Deref for Text<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> DerefMut for Text<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for Text<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        UINode::Text(Self {
            widget: self.widget.raw_copy(),
            formatted_text: self.formatted_text.clone(),
        })
    }

    fn measure_override(&self, _: &UserInterface<M, C>, available_size: Vec2) -> Vec2 {
        self.formatted_text
            .borrow_mut()
            .set_constraint(available_size)
            .set_brush(self.widget.foreground())
            .build()
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        let bounds = self.widget.screen_bounds();
        drawing_context.draw_text(Vec2::new(bounds.x, bounds.y), &self.formatted_text.borrow());
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_routed_message(ui, message);
    }
}

impl<M, C: 'static + Control<M, C>> Text<M, C> {
    pub fn new(widget: Widget<M, C>) -> Self {
        Self {
            widget,
            formatted_text: RefCell::new(FormattedTextBuilder::new()
                .with_font(crate::DEFAULT_FONT.clone())
                .build()),
        }
    }

    pub fn set_text<P: AsRef<str>>(&mut self, text: P) -> &mut Self {
        self.formatted_text.borrow_mut().set_text(text);
        self.widget.invalidate_layout();
        self
    }

    pub fn set_wrap(&mut self, wrap: bool) -> &mut Self {
        self.formatted_text
            .borrow_mut()
            .set_wrap(wrap);
        self.widget.invalidate_layout();
        self
    }

    pub fn is_wrap(&self) -> bool {
        self.formatted_text
            .borrow()
            .is_wrap()
    }

    pub fn text(&self) -> String {
        self.formatted_text
            .borrow()
            .text()
    }

    pub fn set_font(&mut self, font: Arc<Mutex<Font>>) -> &mut Self {
        self.formatted_text
            .borrow_mut()
            .set_font(font);
        self.widget.invalidate_layout();
        self
    }

    pub fn font(&self) -> Arc<Mutex<Font>> {
        self.formatted_text
            .borrow()
            .get_font()
            .unwrap()
    }

    pub fn set_vertical_alignment(&mut self, valign: VerticalAlignment) -> &mut Self {
        self.formatted_text
            .borrow_mut()
            .set_vertical_alignment(valign);
        self.widget.invalidate_layout();
        self
    }

    pub fn vertical_alignment(&self) -> VerticalAlignment {
        self.formatted_text
            .borrow()
            .vertical_alignment()
    }

    pub fn set_horizontal_alignment(&mut self, halign: HorizontalAlignment) -> &mut Self {
        self.formatted_text
            .borrow_mut()
            .set_horizontal_alignment(halign);
        self.widget.invalidate_layout();
        self
    }

    pub fn horizontal_alignment(&self) -> HorizontalAlignment {
        self.formatted_text
            .borrow()
            .horizontal_alignment()
    }
}

pub struct TextBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    text: Option<String>,
    font: Option<Arc<Mutex<Font>>>,
    vertical_text_alignment: VerticalAlignment,
    horizontal_text_alignment: HorizontalAlignment,
    wrap: bool
}

impl<M, C: 'static + Control<M, C>> TextBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            text: None,
            font: None,
            vertical_text_alignment: VerticalAlignment::Top,
            horizontal_text_alignment: HorizontalAlignment::Left,
            wrap: false
        }
    }

    pub fn with_text<P: AsRef<str>>(mut self, text: P) -> Self {
        self.text = Some(text.as_ref().to_owned());
        self
    }

    pub fn with_font(mut self, font: Arc<Mutex<Font>>) -> Self {
        self.font = Some(font);
        self
    }

    pub fn with_opt_font(mut self, font: Option<Arc<Mutex<Font>>>) -> Self {
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

    pub fn build(mut self, ui: &mut UserInterface<M, C>) -> Handle<UINode<M, C>> {
        let font = if let Some(font) = self.font {
            font
        } else {
            crate::DEFAULT_FONT.clone()
        };

        if self.widget_builder.foreground.is_none() {
            self.widget_builder.foreground = Some(Brush::Solid(Color::opaque(220, 220, 220)));
        }

        let handle = ui.add_node(UINode::Text(Text {
            widget: self.widget_builder.build(ui.sender()),
            formatted_text: RefCell::new(FormattedTextBuilder::new()
                .with_text(self.text.unwrap_or_default())
                .with_vertical_alignment(self.vertical_text_alignment)
                .with_horizontal_alignment(self.horizontal_text_alignment)
                .with_font(font)
                .with_wrap(self.wrap)
                .build()),
        }));

        ui.flush_messages();

        handle
    }
}
