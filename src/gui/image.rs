use rg3d_core::pool::Handle;
use std::sync::{Mutex, Arc};
use crate::{
    resource::texture::Texture,
    gui::{
        node::UINode,
        EventSource,
        event::UIEvent
    }
};

pub struct Image {
    pub(in crate::gui) owner_handle: Handle<UINode>,
    texture: Arc<Mutex<Texture>>,
}

impl EventSource for Image {
    fn emit_event(&mut self) -> Option<UIEvent> {
        None
    }
}