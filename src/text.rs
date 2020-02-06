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
    UserInterface,
};
use std::{
    collections::HashMap,
    sync::{
        Mutex,
        Arc,
    },
    cell::RefCell,
};

pub struct Text {
    widget: Widget,
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
            formatted_text: self.formatted_text.clone(),
        })
    }

    fn resolve(&mut self, _: &ControlTemplate, _: &HashMap<Handle<UINode>, Handle<UINode>>) {}

    fn measure_override(&self, _: &UserInterface, available_size: Vec2) -> Vec2 {
        self.formatted_text
            .borrow_mut()
            .set_constraint(available_size)
            .set_color(self.widget.foreground())
            .build()
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        let bounds = self.widget.get_screen_bounds();
        drawing_context.draw_text(Vec2::new(bounds.x, bounds.y), &self.formatted_text.borrow());
    }
}

impl Text {
    pub fn new(widget: Widget) -> Self {
        Self {
            widget,
            formatted_text: RefCell::new(FormattedTextBuilder::new()
                .with_font(crate::DEFAULT_FONT.clone())
                .build()),
        }
    }

    pub fn set_text<P: AsRef<str>>(&mut self, text: P) -> &mut Self {
        self.formatted_text.borrow_mut().set_text(text);
        self
    }

    pub fn set_wrap(&mut self, wrap: bool) -> &mut Self {
        self.formatted_text
            .borrow_mut()
            .set_wrap(wrap);
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
            .set_font( font);
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
        self
    }

    pub fn horizontal_alignment(&self) -> HorizontalAlignment {
        self.formatted_text
            .borrow()
            .horizontal_alignment()
    }
}

pub struct TextBuilder {
    widget_builder: WidgetBuilder,
    text: Option<String>,
    font: Option<Arc<Mutex<Font>>>,
    vertical_text_alignment: VerticalAlignment,
    horizontal_text_alignment: HorizontalAlignment,
}

impl TextBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            text: None,
            font: None,
            vertical_text_alignment: VerticalAlignment::Top,
            horizontal_text_alignment: HorizontalAlignment::Left,
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
            formatted_text: RefCell::new(FormattedTextBuilder::new()
                .with_text(self.text.unwrap_or_default())
                .with_vertical_alignment(self.vertical_text_alignment)
                .with_horizontal_alignment(self.horizontal_text_alignment)
                .with_font(font)
                .build()),
        }))
    }
}