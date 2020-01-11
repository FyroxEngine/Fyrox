use std::sync::{Mutex, Arc};
use crate::core::{
    pool::Handle
};
use crate::{
    gui::{
        UINode,
        draw::{DrawingContext, CommandKind, CommandTexture},
        widget::{Widget},
        UserInterface,
        widget::WidgetBuilder
    },
    resource::texture::Texture,
};
use crate::gui::Control;

pub struct Image {
    widget: Widget,
    texture: Option<Arc<Mutex<Texture>>>,
}

impl Control for Image {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget {
        &mut self.widget
    }

    fn draw(&mut self, drawing_context: &mut DrawingContext) {
        let bounds = self.widget.get_screen_bounds();
        drawing_context.push_rect_filled(&bounds, None, self.widget.background());
        let texture = self.texture.as_ref().map_or(CommandTexture::None,
                                          |t| { CommandTexture::Texture(t.clone()) });
        drawing_context.commit(CommandKind::Geometry, texture);
    }
}

pub struct ImageBuilder {
    widget_builder: WidgetBuilder,
    texture: Option<Arc<Mutex<Texture>>>,
}

impl ImageBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            texture: None,
        }
    }

    pub fn with_texture(mut self, texture: Arc<Mutex<Texture>>) -> Self {
        self.texture = Some(texture);
        self
    }

    pub fn with_opt_texture(mut self, texture: Option<Arc<Mutex<Texture>>>) -> Self {
        self.texture = texture;
        self
    }

    pub fn build(self, ui: &mut UserInterface) -> Handle<UINode> {
        let image = Image {
            widget: self.widget_builder.build(),
            texture: self.texture,
        };

        ui.add_node(image)
    }
}