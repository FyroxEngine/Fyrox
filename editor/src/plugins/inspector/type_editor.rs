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

use crate::{Message, message::MessageSender};
use fyrox::gui::widget::WidgetMessage;
use fyrox::{
    asset::{Resource, TypedResourceData, manager::ResourceManager},
    core::{pool::Handle, reflect::Reflect, some_or_return},
    gui::{
        UiNode, UserInterface,
        button::{Button, ButtonBuilder, ButtonMessage},
        message::UiMessage,
        widget::WidgetBuilder,
    },
};
use std::marker::PhantomData;
use std::{any::TypeId, path::PathBuf};

pub struct InspectorTypeEditorContext<'a> {
    pub object: &'a dyn Reflect,
    pub ui: &'a mut UserInterface,
    pub resource_manager: &'a ResourceManager,
    pub message_sender: &'a MessageSender,
}

pub trait InspectorTypeEditor: 'static {
    fn supports_type(&self, type_id: TypeId) -> bool;
    fn create_ui(&mut self, ctx: InspectorTypeEditorContext) -> Handle<UiNode>;
    fn destroy_ui(&mut self, ctx: InspectorTypeEditorContext);
    fn on_sync_to_model(&mut self, ctx: InspectorTypeEditorContext);
    fn on_ui_message(&mut self, message: &mut UiMessage, ctx: InspectorTypeEditorContext);
}

pub struct ResourceTypeEditor<T>
where
    T: TypedResourceData,
{
    open: Handle<Button>,
    phantom_data: PhantomData<T>,
}

impl<T> Default for ResourceTypeEditor<T>
where
    T: TypedResourceData,
{
    fn default() -> Self {
        Self {
            open: Default::default(),
            phantom_data: Default::default(),
        }
    }
}

impl<T> ResourceTypeEditor<T>
where
    T: TypedResourceData,
{
    fn path(&self, object: &dyn Reflect, resource_manager: &ResourceManager) -> Option<PathBuf> {
        let ui_resource = object.downcast_ref::<Resource<T>>()?;
        resource_manager.resource_path(ui_resource)
    }
}

impl<T: TypedResourceData> InspectorTypeEditor for ResourceTypeEditor<T> {
    fn supports_type(&self, type_id: TypeId) -> bool {
        TypeId::of::<Resource<T>>() == type_id
    }

    fn create_ui(&mut self, ctx: InspectorTypeEditorContext) -> Handle<UiNode> {
        let path = some_or_return!(self.path(ctx.object, ctx.resource_manager), Handle::NONE);
        self.open = ButtonBuilder::new(WidgetBuilder::new())
            .with_text(&format!("Open {}", path.display()))
            .build(&mut ctx.ui.build_ctx());
        self.open.to_base()
    }

    fn destroy_ui(&mut self, ctx: InspectorTypeEditorContext) {
        ctx.ui.send(self.open, WidgetMessage::Remove);
    }

    fn on_sync_to_model(&mut self, _ctx: InspectorTypeEditorContext) {}

    fn on_ui_message(&mut self, message: &mut UiMessage, ctx: InspectorTypeEditorContext) {
        let path = some_or_return!(self.path(ctx.object, ctx.resource_manager));
        if let Some(ButtonMessage::Click) = message.data_from(self.open) {
            ctx.message_sender.send(Message::LoadScene(path));
        }
    }
}
