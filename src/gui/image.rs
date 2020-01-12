use std::{
    sync::{
        Mutex,
        Arc
    },
    collections::HashMap
};
use crate::{
    core::{
        pool::Handle
    },
    gui::{
        UINode,
        draw::{
            DrawingContext,
            CommandKind,
            CommandTexture
        },
        widget::Widget,
        widget::WidgetBuilder,
        Control,
        ControlTemplate,
        UINodeContainer,
        Builder
    },
    resource::texture::Texture,
};

pub struct Image {
    widget: Widget,
    texture: Option<Arc<Mutex<Texture>>>,
}

impl Image {
    pub fn new(widget: Widget) -> Self {
        Self {
            widget,
            texture: None
        }
    }

    pub fn set_texture(&mut self, texture: Arc<Mutex<Texture>>) {
        self.texture = Some(texture);
    }
}

impl Control for Image {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget {
        &mut self.widget
    }

    fn raw_copy(&self) -> Box<dyn Control> {
        Box::new(Self {
            widget: *self.widget.raw_copy().downcast::<Widget>().unwrap_or_else(|_| panic!()),
            texture: self.texture.clone()
        })
    }

    fn resolve(&mut self, _: &ControlTemplate, _: &HashMap<Handle<UINode>, Handle<UINode>>) {

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
}

impl Builder for ImageBuilder {
    fn build(self, ui: &mut dyn UINodeContainer) -> Handle<UINode> {
        let image = Image {
            widget: self.widget_builder.build(),
            texture: self.texture,
        };

        ui.add_node(Box::new(image))
    }
}