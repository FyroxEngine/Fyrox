use crate::{
    brush::Brush,
    color::draw_checker_board,
    core::{algebra::Vector2, color::Color, math::Rect, pool::Handle},
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
    UvRect(Rect<f32>),
    CheckerboardBackground(bool),
}

impl ImageMessage {
    define_constructor!(ImageMessage:Texture => fn texture(Option<SharedTexture>), layout: false);
    define_constructor!(ImageMessage:Flip => fn flip(bool), layout: false);
    define_constructor!(ImageMessage:UvRect => fn uv_rect(Rect<f32>), layout: false);
    define_constructor!(ImageMessage:CheckerboardBackground => fn checkerboard_background(bool), layout: false);
}

#[derive(Clone)]
pub struct Image {
    pub widget: Widget,
    pub texture: Option<SharedTexture>,
    pub flip: bool,
    pub uv_rect: Rect<f32>,
    pub checkerboard_background: bool,
}

crate::define_widget_deref!(Image);

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

        if self.checkerboard_background {
            draw_checker_board(bounds, self.clip_bounds(), 8.0, drawing_context);
        }

        if self.texture.is_some() || !self.checkerboard_background {
            let tex_coords = if self.flip {
                Some([
                    Vector2::new(self.uv_rect.position.x, self.uv_rect.position.y),
                    Vector2::new(
                        self.uv_rect.position.x + self.uv_rect.size.x,
                        self.uv_rect.position.y,
                    ),
                    Vector2::new(
                        self.uv_rect.position.x + self.uv_rect.size.x,
                        self.uv_rect.position.y - self.uv_rect.size.y,
                    ),
                    Vector2::new(
                        self.uv_rect.position.x,
                        self.uv_rect.position.y - self.uv_rect.size.y,
                    ),
                ])
            } else {
                Some([
                    Vector2::new(self.uv_rect.position.x, self.uv_rect.position.y),
                    Vector2::new(
                        self.uv_rect.position.x + self.uv_rect.size.x,
                        self.uv_rect.position.y,
                    ),
                    Vector2::new(
                        self.uv_rect.position.x + self.uv_rect.size.x,
                        self.uv_rect.position.y + self.uv_rect.size.y,
                    ),
                    Vector2::new(
                        self.uv_rect.position.x,
                        self.uv_rect.position.y + self.uv_rect.size.y,
                    ),
                ])
            };
            drawing_context.push_rect_filled(&bounds, tex_coords.as_ref());
            let texture = self
                .texture
                .as_ref()
                .map_or(CommandTexture::None, |t| CommandTexture::Texture(t.clone()));
            drawing_context.commit(self.clip_bounds(), self.widget.background(), texture, None);
        }
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
                    ImageMessage::UvRect(uv_rect) => {
                        self.uv_rect = *uv_rect;
                    }
                    ImageMessage::CheckerboardBackground(value) => {
                        self.checkerboard_background = *value;
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
    uv_rect: Rect<f32>,
    checkerboard_background: bool,
}

impl ImageBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            texture: None,
            flip: false,
            uv_rect: Rect::new(0.0, 0.0, 1.0, 1.0),
            checkerboard_background: false,
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

    pub fn with_uv_rect(mut self, uv_rect: Rect<f32>) -> Self {
        self.uv_rect = uv_rect;
        self
    }

    pub fn with_checkerboard_background(mut self, checkerboard_background: bool) -> Self {
        self.checkerboard_background = checkerboard_background;
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
            uv_rect: self.uv_rect,
            checkerboard_background: self.checkerboard_background,
        };
        UiNode::new(image)
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        ctx.add_node(self.build_node())
    }
}
