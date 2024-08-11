// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

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
