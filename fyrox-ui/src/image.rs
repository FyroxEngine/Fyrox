use crate::{
    brush::Brush,
    core::{algebra::Vector2, color::Color, pool::Handle},
    define_constructor,
    draw::{CommandTexture, Draw, DrawingContext, SharedTexture},
    message::{MessageDirection, UiMessage},
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UiNode, UserInterface,
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone, PartialEq)]
pub enum ImageMessage {
    Texture(Option<SharedTexture>),
    Flip(bool),
}

impl ImageMessage {
    define_constructor!(ImageMessage:Texture => fn texture(Option<SharedTexture>), layout: false);
    define_constructor!(ImageMessage:Flip => fn flip(bool), layout: false);
}

#[derive(Clone)]
pub struct Image {
    widget: Widget,
    texture: Option<SharedTexture>,
    flip: bool,
}

crate::define_widget_deref!(Image);

impl Image {
    pub fn new(widget: Widget) -> Self {
        Self {
            widget,
            texture: None,
            flip: false,
        }
    }

    pub fn set_texture(&mut self, texture: SharedTexture) {
        self.texture = Some(texture);
    }

    pub fn texture(&self) -> Option<SharedTexture> {
        self.texture.clone()
    }
}

impl Control for Image {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        let bounds = self.widget.bounding_rect();
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

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<ImageMessage>() {
            if message.destination() == self.handle {
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

pub struct ImageBuilder {
    widget_builder: WidgetBuilder,
    texture: Option<SharedTexture>,
    flip: bool,
}

impl ImageBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
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

    pub fn build_node(mut self) -> UiNode {
        if self.widget_builder.background.is_none() {
            self.widget_builder.background = Some(Brush::Solid(Color::WHITE))
        }

        let image = Image {
            widget: self.widget_builder.build(),
            texture: self.texture,
            flip: self.flip,
        };
        UiNode::new(image)
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        ctx.add_node(self.build_node())
    }
}
