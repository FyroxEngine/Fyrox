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

pub struct Image<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    texture: Option<Arc<Texture>>,
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
        }
    }
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for Image<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        UINode::Image(self.clone())
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        let bounds = self.widget.screen_bounds();
        drawing_context.push_rect_filled(&bounds, None);
        let texture = self.texture
            .as_ref()
            .map_or(CommandTexture::None, |t| CommandTexture::Texture(t.clone()));
        drawing_context.commit(CommandKind::Geometry, self.widget.background(), texture);
    }

    fn handle_routed_message(&mut self, self_handle: Handle<UINode<M, C>>, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_routed_message(self_handle, ui, message);
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

    pub fn build(mut self, ui: &mut UserInterface<M, C>) -> Handle<UINode<M, C>> {
        if self.widget_builder.background.is_none() {
            self.widget_builder.background = Some(Brush::Solid(Color::WHITE))
        }

        let image = Image {
            widget: self.widget_builder.build(),
            texture: self.texture,
        };

        let handle = ui.add_node(UINode::Image(image));

        ui.flush_messages();

        handle
    }
}