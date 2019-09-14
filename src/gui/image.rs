use crate::{
    resource::Resource,
    gui::node::UINode
};
use std::{
    rc::Weak,
    cell::RefCell
};

use rg3d_core::pool::Handle;

pub struct Image {
    pub(in crate::gui) owner_handle: Handle<UINode>,
    texture: Weak<RefCell<Resource>>,
}