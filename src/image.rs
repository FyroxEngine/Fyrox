use std::{
    sync::Arc,
};
use crate::{
    brush::Brush,
    core::{
        pool::Handle,
        color::Color
    },
    UINode,
    draw::{
        DrawingContext,
        CommandKind,
    },
    widget::{
        WidgetBuilder,
        Widget
    },
    Control,
    draw::{Texture, CommandTexture},
    UserInterface,
    message::UiMessage,
};
use std::ops::{Deref, DerefMut};
use rg3d_core::math::vec2::Vec2;

pub struct Image<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    texture: Option<Arc<Texture>>,
    flip: bool
}

impl<M: 'static, C: 'static + Control<M, C>> Deref for Image<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> DerefMut for Image<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M, C: 'static + Control<M, C>> Image<M, C> {
    pub fn new(widget: Widget<M, C>) -> Self {
        Self {
            widget,
            texture: None,
            flip: false
        }
    }

    pub fn set_texture(&mut self, texture: Arc<Texture>) {
        self.texture = Some(texture);
    }
}

impl<M, C: 'static + Control<M, C>> Clone for Image<M, C> {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.raw_copy(),
            texture: self.texture.clone(),
            flip: false
        }
    }
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for Image<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        UINode::Image(self.clone())
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        let bounds = self.widget.screen_bounds();
        let tex_coords = if self.flip {
            Some([
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(1.0, -1.0),
                Vec2::new(0.0, -1.0),
            ])
        } else {
            None
        };
        drawing_context.push_rect_filled(&bounds, tex_coords.as_ref());
        let texture = self.texture
            .as_ref()
            .map_or(CommandTexture::None, |t| CommandTexture::Texture(t.clone()));
        drawing_context.commit(CommandKind::Geometry, self.widget.background(), texture);
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_routed_message(ui, message);
    }
}

pub struct ImageBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    texture: Option<Arc<Texture>>,
    flip: bool
}

impl<M, C: 'static + Control<M, C>> ImageBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            texture: None,
            flip: false
        }
    }

    pub fn with_flip(mut self, flip: bool) -> Self {
        self.flip = flip;
        self
    }

    pub fn with_texture(mut self, texture: Arc<Texture>) -> Self {
        self.texture = Some(texture);
        self
    }

    pub fn with_opt_texture(mut self, texture: Option<Arc<Texture>>) -> Self {
        self.texture = texture;
        self
    }

    pub fn build(mut self, ui: &mut UserInterface<M, C>) -> Handle<UINode<M, C>> {
        if self.widget_builder.background.is_none() {
            self.widget_builder.background = Some(Brush::Solid(Color::WHITE))
        }

        let image = Image {
            widget: self.widget_builder.build(ui.sender()),
            texture: self.texture,
            flip: self.flip
        };

        let handle = ui.add_node(UINode::Image(image));

        ui.flush_messages();

        handle
    }
}