use crate::{
    utils::{
        pool::Handle,
    },
    resource::Resource,
    gui::node::UINode
};
use std::{
    rc::Weak,
    cell::RefCell
};

pub struct Image {
    pub(in crate::gui) owner_handle: Handle<UINode>,
    texture: Weak<RefCell<Resource>>,
}