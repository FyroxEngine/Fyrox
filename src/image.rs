use std::{
    sync::Arc,
};
use crate::{
    core::{
        pool::Handle
    },
    UINode,
    draw::{
        DrawingContext,
        CommandKind,
    },
    widget::Widget,
    widget::WidgetBuilder,
    Control,
    UINodeContainer,
    Builder,
    draw::{Texture, CommandTexture}
};

pub struct Image<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    texture: Option<Arc<Texture>>,
}

impl<M, C: 'static + Control<M, C>> Image<M, C> {
    pub fn new(widget: Widget<M, C>) -> Self {
        Self {
            widget,
            texture: None,
        }
    }

    pub fn set_texture(&mut self, texture: Arc<Texture>) {
        self.texture = Some(texture);
    }
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for Image<M, C> {
    fn widget(&self) -> &Widget<M, C> {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget<M, C> {
        &mut self.widget
    }

    fn raw_copy(&self) -> UINode<M, C> {
        UINode::Image(Self {
            widget: self.widget.raw_copy(),
            texture: self.texture.clone(),
        })
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        let bounds = self.widget.get_screen_bounds();
        drawing_context.push_rect_filled(&bounds, None);
        let texture = self.texture
            .as_ref()
            .map_or(CommandTexture::None, |t| CommandTexture::Texture(t.clone()));
        drawing_context.commit(CommandKind::Geometry, self.widget.background(), texture);
    }
}

pub struct ImageBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    texture: Option<Arc<Texture>>,
}

impl<M, C: 'static + Control<M, C>> ImageBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            texture: None,
        }
    }

    pub fn with_texture(mut self, texture: Arc<Texture>) -> Self {
        self.texture = Some(texture);
        self
    }

    pub fn with_opt_texture(mut self, texture: Option<Arc<Texture>>) -> Self {
        self.texture = texture;
        self
    }
}

impl<M, C: 'static + Control<M, C>> Builder<M, C> for ImageBuilder<M, C> {
    fn build(self, ui: &mut dyn UINodeContainer<M, C>) -> Handle<UINode<M, C>> {
        let image = Image {
            widget: self.widget_builder.build(),
            texture: self.texture,
        };

        ui.add_node(UINode::Image(image))
    }
}