use crate::{
    gui::{
        builder::CommonBuilderFields,
        VerticalAlignment,
        HorizontalAlignment,
        Drawable,
        draw::DrawingContext,
        node::{
            UINode,
            UINodeKind,
        },
        UserInterface,
        formatted_text::{FormattedText, FormattedTextBuilder}
    },
    resource::ttf::Font,
};
use rg3d_core::{
    color::Color,
    math::{
        Rect,
        vec2::Vec2,
    },
    pool::{Handle},
};
use std::{
    cell::RefCell,
    rc::Rc,
};

pub struct Text {
    pub(in crate::gui) owner_handle: Handle<UINode>,
    need_update: bool,
    text: String,
    font: Rc<RefCell<Font>>,
    vertical_alignment: VerticalAlignment,
    horizontal_alignment: HorizontalAlignment,
    formatted_text: Option<FormattedText>,
}

impl Drawable for Text {
    fn draw(&mut self, drawing_context: &mut DrawingContext, bounds: &Rect<f32>, color: Color) {
        if self.need_update {
            let formatted_text = FormattedTextBuilder::reuse(self.formatted_text.take().unwrap())
                .with_size(Vec2::make(bounds.w, bounds.h))
                .with_text(self.text.as_str())
                .with_color(color)
                .with_horizontal_alignment(self.horizontal_alignment)
                .with_vertical_alignment(self.vertical_alignment)
                .build();
            self.formatted_text = Some(formatted_text);
            self.need_update = true; // TODO
        }
        drawing_context.draw_text(Vec2::make(bounds.x, bounds.y), self.formatted_text.as_ref().unwrap());
    }
}

impl Text {
    pub fn new(font: Rc<RefCell<Font>>) -> Text {
        Text {
            owner_handle: Handle::none(),
            text: String::new(),
            need_update: true,
            vertical_alignment: VerticalAlignment::Top,
            horizontal_alignment: HorizontalAlignment::Left,
            formatted_text: Some(FormattedTextBuilder::new(font.clone()).build()),
            font,
        }
    }

    pub fn set_text(&mut self, text: &str) -> &mut Self {
        self.text.clear();
        self.text += text;
        self.need_update = true;
        self
    }

    pub fn get_text(&self) -> &str {
        self.text.as_str()
    }

    pub fn set_font(&mut self, font: Rc<RefCell<Font>>) -> &mut Self {
        self.font = font;
        self.need_update = true;
        self
    }

    pub fn set_vertical_alignment(&mut self, valign: VerticalAlignment) -> &mut Self {
        self.vertical_alignment = valign;
        self
    }

    pub fn set_horizontal_alignment(&mut self, halign: HorizontalAlignment) -> &mut Self {
        self.horizontal_alignment = halign;
        self
    }
}


pub struct TextBuilder {
    text: Option<String>,
    font: Option<Rc<RefCell<Font>>>,
    common: CommonBuilderFields,
    vertical_text_alignment: Option<VerticalAlignment>,
    horizontal_text_alignment: Option<HorizontalAlignment>,
}


impl Default for TextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TextBuilder {
    pub fn new() -> Self {
        Self {
            text: None,
            font: None,
            vertical_text_alignment: None,
            horizontal_text_alignment: None,
            common: CommonBuilderFields::new(),
        }
    }

    impl_default_builder_methods!();

    pub fn with_text(mut self, text: &str) -> Self {
        self.text = Some(text.to_owned());
        self
    }

    pub fn with_font(mut self, font: Rc<RefCell<Font>>) -> Self {
        self.font = Some(font);
        self
    }

    pub fn build(mut self, ui: &mut UserInterface) -> Handle<UINode> {
        let mut text = Text::new( if let Some(font) = self.font {
            font
        } else {
            ui.default_font.clone()
        });

        if let Some(txt) = self.text {
            text.set_text(txt.as_str());
        }
        if let Some(valign) = self.vertical_text_alignment {
            text.set_vertical_alignment(valign);
        }
        if let Some(halign) = self.horizontal_text_alignment {
            text.set_horizontal_alignment(halign);
        }
        let handle = ui.add_node(UINode::new(UINodeKind::Text(text)));
        self.common.apply(ui, handle);
        handle
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