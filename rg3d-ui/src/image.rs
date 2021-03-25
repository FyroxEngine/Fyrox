use crate::draw::Draw;
use crate::{
    brush::Brush,
    core::{algebra::Vector2, color::Color, pool::Handle},
    draw::{CommandTexture, DrawingContext, SharedTexture},
    message::{ImageMessage, MessageData, UiMessage, UiMessageData},
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UINode, UserInterface,
};
use std::ops::{Deref, DerefMut};

#[derive(Clone)]
pub struct Image<M: MessageData, C: Control<M, C>> {
    widget: Widget<M, C>,
    texture: Option<SharedTexture>,
    flip: bool,
}

crate::define_widget_deref!(Image<M, C>);

impl<M: MessageData, C: Control<M, C>> Image<M, C> {
    pub fn new(widget: Widget<M, C>) -> Self {
        Self {
            widget,
            texture: None,
            flip: false,
        }
    }

    pub fn set_texture(&mut self, texture: SharedTexture) {
        self.texture = Some(texture);
    }
}

impl<M: MessageData, C: Control<M, C>> Control<M, C> for Image<M, C> {
    fn draw(&self, drawing_context: &mut DrawingContext) {
        let bounds = self.widget.screen_bounds();
        let tex_coords = if self.flip {
            Some([
                Vector2::new(0.0, 0.0),
                Vector2::new(1.0, 0.0),
                Vector2::new(1.0, -1.0),
                Vector2::new(0.0, -1.0),
            ])
        } else {
            None
        };
        drawing_context.push_rect_filled(&bounds, tex_coords.as_ref());
        let texture = self
            .texture
            .as_ref()
            .map_or(CommandTexture::None, |t| CommandTexture::Texture(t.clone()));
        drawing_context.commit(self.clip_bounds(), self.widget.background(), texture, None);
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle {
            if let UiMessageData::Image(msg) = &message.data() {
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

pub struct ImageBuilder<M: MessageData, C: Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    texture: Option<SharedTexture>,
    flip: bool,
}

impl<M: MessageData, C: Control<M, C>> ImageBuilder<M, C> {
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

    pub fn with_texture(mut self, texture: SharedTexture) -> Self {
        self.texture = Some(texture);
        self
    }

    pub fn with_opt_texture(mut self, texture: Option<SharedTexture>) -> Self {
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
