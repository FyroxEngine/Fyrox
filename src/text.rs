use crate::{
    core::{
        pool::Handle,
        math::vec2::Vec2,
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
        ControlTemplate,
        UINodeContainer,
        Builder,
    ttf::Font,
};
use std::{
    collections::HashMap,
    sync::{
        Mutex,
        Arc
    }
};
use std::cell::{Cell, RefCell};

pub struct Text {
    widget: Widget,
    need_update: Cell<bool>,
    text: String,
    font: Arc<Mutex<Font>>,
    vertical_alignment: VerticalAlignment,
    horizontal_alignment: HorizontalAlignment,
    formatted_text: RefCell<FormattedText>,
}

impl Control for Text {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget {
        &mut self.widget
    }

    fn raw_copy(&self) -> Box<dyn Control> {
        Box::new(Self {
            widget: *self.widget.raw_copy().downcast::<Widget>().unwrap_or_else(|_| panic!()),
            need_update: self.need_update.clone(),
            text: self.text.clone(),
            font: self.font.clone(),
            vertical_alignment: self.vertical_alignment,
            horizontal_alignment: self.horizontal_alignment,
            formatted_text: RefCell::new(FormattedTextBuilder::new()
                .with_font(self.font.clone())
                .build()),
        })
    }

    fn resolve(&mut self, _: &ControlTemplate, _: &HashMap<Handle<UINode>, Handle<UINode>>) {}

    fn draw(&self, drawing_context: &mut DrawingContext) {
        let bounds = self.widget.get_screen_bounds();
        if self.need_update.get() {
            let mut text = self.formatted_text.borrow_mut();
            text.set_size(Vec2::new(bounds.w, bounds.h));
            text.set_text(self.text.as_str());
            text.set_color(self.widget.foreground());
            text.set_horizontal_alignment(self.horizontal_alignment);
            text.set_vertical_alignment(self.vertical_alignment);
            text.build();
            self.need_update.set(true); // TODO
        }
        drawing_context.draw_text(Vec2::new(bounds.x, bounds.y), &self.formatted_text.borrow());
    }
}

impl Text {
    pub fn new(widget: Widget) -> Self {
        Self {
            widget,
            need_update: Cell::new(true),
            text: "".to_string(),
            formatted_text: RefCell::new(FormattedTextBuilder::new()
                .with_font(crate::DEFAULT_FONT.clone())
                .build()),
            font: crate::DEFAULT_FONT.clone(),
            vertical_alignment: VerticalAlignment::Stretch,
            horizontal_alignment: HorizontalAlignment::Stretch,
        }
    }

    pub fn set_text<P: AsRef<str>>(&mut self, text: P) -> &mut Self {
        self.text.clear();
        self.text += text.as_ref();
        self.need_update.set(true);
        self
    }

    pub fn text(&self) -> &str {
        self.text.as_str()
    }

    pub fn set_font(&mut self, font: Arc<Mutex<Font>>) -> &mut Self {
        self.font = font;
        self.need_update.set(true);
        self
    }

    pub fn font(&self) -> Arc<Mutex<Font>> {
        self.font.clone()
    }

    pub fn set_vertical_alignment(&mut self, valign: VerticalAlignment) -> &mut Self {
        self.vertical_alignment = valign;
        self
    }

    pub fn vertical_alignment(&self) -> VerticalAlignment {
        self.vertical_alignment
    }

    pub fn set_horizontal_alignment(&mut self, halign: HorizontalAlignment) -> &mut Self {
        self.horizontal_alignment = halign;
        self
    }

    pub fn horizontal_alignment(&self) -> HorizontalAlignment {
        self.horizontal_alignment
    }
}

pub struct TextBuilder {
    widget_builder: WidgetBuilder,
    text: Option<String>,
    font: Option<Arc<Mutex<Font>>>,
    vertical_text_alignment: Option<VerticalAlignment>,
    horizontal_text_alignment: Option<HorizontalAlignment>,
}

impl TextBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            text: None,
            font: None,
            vertical_text_alignment: None,
            horizontal_text_alignment: None,
        }
    }

    pub fn with_text(mut self, text: &str) -> Self {
        self.text = Some(text.to_owned());
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
        self.vertical_text_alignment = Some(valign);
        self
    }

    pub fn with_horizontal_text_alignment(mut self, halign: HorizontalAlignment) -> Self {
        self.horizontal_text_alignment = Some(halign);
        self
    }
}

impl Builder for TextBuilder {
    fn build(self, ui: &mut dyn UINodeContainer) -> Handle<UINode> {
        let font = if let Some(font) = self.font {
            font
        } else {
            crate::DEFAULT_FONT.clone()
        };

        ui.add_node(Box::new(Text {
            widget: self.widget_builder.build(),
            text: self.text.unwrap_or_default(),
            need_update: Cell::new(true),
            vertical_alignment: self.vertical_text_alignment.unwrap_or(VerticalAlignment::Top),
            horizontal_alignment: self.horizontal_text_alignment.unwrap_or(HorizontalAlignment::Left),
            formatted_text: RefCell::new(FormattedTextBuilder::new().with_font(font.clone()).build()),
            font,
        }))
    }
}