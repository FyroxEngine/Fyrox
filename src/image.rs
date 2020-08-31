use crate::message::{ImageMessage, UiMessageData};
use crate::{
    brush::Brush,
    core::math::vec2::Vec2,
    core::{color::Color, pool::Handle},
    draw::{CommandKind, DrawingContext},
    draw::{CommandTexture, Texture},
    message::UiMessage,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UINode, UserInterface,
};
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

pub struct Image<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    texture: Option<Arc<Texture>>,
    flip: bool,
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

impl<M: 'static, C: 'static + Control<M, C>> Image<M, C> {
    pub fn new(widget: Widget<M, C>) -> Self {
        Self {
            widget,
            texture: None,
            flip: false,
        }
    }

    pub fn set_texture(&mut self, texture: Arc<Texture>) {
        self.texture = Some(texture);
    }
}

impl<M: 'static, C: 'static + Control<M, C>> Clone for Image<M, C> {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.raw_copy(),
            texture: self.texture.clone(),
            flip: false,
        }
    }
}

impl<M: 'static, C: 'static + Control<M, C>> Control<M, C> for Image<M, C> {
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
        let texture = self
            .texture
            .as_ref()
            .map_or(CommandTexture::None, |t| CommandTexture::Texture(t.clone()));
        drawing_context.commit(CommandKind::Geometry, self.widget.background(), texture);
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);

        if message.destination == self.handle {
            if let UiMessageData::Image(msg) = &message.data {
                match msg {
                    ImageMessage::Texture(tex) => {
                        self.texture = tex.clone();
                    }
                    &ImageMessage::Flip(flip) => {
                        self.flip = flip;
                    }
                }
            }
        }
    }
}

pub struct ImageBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    texture: Option<Arc<Texture>>,
    flip: bool,
}

impl<M: 'static, C: 'static + Control<M, C>> ImageBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            texture: None,
            flip: false,
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

    pub fn build_node(mut self) -> UINode<M, C> {
        if self.widget_builder.background.is_none() {
            self.widget_builder.background = Some(Brush::Solid(Color::WHITE))
        }

        let image = Image {
            widget: self.widget_builder.build(),
            texture: self.texture,
            flip: self.flip,
        };
        UINode::Image(image)
    }

    pub fn build(self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        ctx.add_node(self.build_node())
    }
}
