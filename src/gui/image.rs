use crate::{
    gui::node::UINode
};

use rg3d_core::pool::Handle;
use std::sync::{Mutex, Arc};
use crate::resource::texture::Texture;

pub struct Image {
    pub(in crate::gui) owner_handle: Handle<UINode>,
    texture: Arc<Mutex<Texture>>,
}