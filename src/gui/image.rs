use std::sync::{Mutex, Arc};
use crate::{
    resource::texture::Texture,
    gui::{
        Draw,
        draw::{DrawingContext, CommandKind, CommandTexture},
        widget::{Widget, AsWidget},
        Layout,
        UserInterface
    }
};
use rg3d_core::math::vec2::Vec2;

pub struct Image {
    widget: Widget,
    texture: Arc<Mutex<Texture>>,
}

impl AsWidget for Image {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget {
        &mut self.widget
    }
}

impl Draw for Image {
    fn draw(&mut self, drawing_context: &mut DrawingContext) {
        let bounds = self.widget.get_screen_bounds();
        drawing_context.push_rect_filled(&bounds, None, self.widget.color);
        drawing_context.commit(CommandKind::Geometry, CommandTexture::Texture(self.texture.clone()))
    }
}

impl Layout for Image {
    fn measure_override(&self, ui: &UserInterface, available_size: Vec2) -> Vec2 {
        self.widget.measure_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vec2) -> Vec2 {
        self.widget.arrange_override(ui, final_size)
    }
}