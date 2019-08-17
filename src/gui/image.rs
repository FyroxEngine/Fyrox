use crate::{
    utils::{
        pool::Handle,
        rcpool::RcHandle
    },
    resource::Resource,
    gui::node::UINode
};

pub struct Image {
    pub(in crate::gui) owner_handle: Handle<UINode>,
    texture: RcHandle<Resource>,
}