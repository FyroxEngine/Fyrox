pub mod graph;
pub mod widget;

use crate::fyrox::{core::type_traits::prelude::*, gui::UserInterface};
use crate::{
    command::CommandContext, message::MessageSender, scene::Selection,
    ui_scene::clipboard::Clipboard,
};

#[derive(ComponentProvider)]
pub struct UiSceneContext {
    pub ui: &'static mut UserInterface,
    #[component(include)]
    pub selection: &'static mut Selection,
    #[component(include)]
    pub message_sender: MessageSender,
    pub clipboard: &'static mut Clipboard,
}

impl UiSceneContext {
    pub fn exec<'a, F>(
        ui: &'a mut UserInterface,
        selection: &'a mut Selection,
        message_sender: MessageSender,
        clipboard: &'a mut Clipboard,
        func: F,
    ) where
        F: FnOnce(&mut UiSceneContext),
    {
        // SAFETY: Temporarily extend lifetime to 'static and execute external closure with it.
        // The closure accepts this extended context by reference, so there's no way it escapes to
        // outer world. The initial lifetime is still preserved by this function call.
        func(unsafe {
            &mut Self {
                ui: std::mem::transmute::<&'a mut _, &'static mut _>(ui),
                selection: std::mem::transmute::<&'a mut _, &'static mut _>(selection),
                message_sender,
                clipboard: std::mem::transmute::<&'a mut _, &'static mut _>(clipboard),
            }
        });
    }
}

impl CommandContext for UiSceneContext {}
