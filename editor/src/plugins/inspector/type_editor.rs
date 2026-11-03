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

use crate::{
    Message,
    asset::selection::SelectedResource,
    fyrox::{
        asset::{TypedResourceData, manager::ResourceManager},
        core::{pool::Handle, some_or_return},
        gui::{
            UiNode, UserInterface,
            button::{Button, ButtonBuilder, ButtonMessage},
            message::UiMessage,
            widget::{WidgetBuilder, WidgetMessage},
        },
    },
    message::MessageSender,
    scene::EntityInfo,
};
use fyrox::gui::Thickness;
use std::{any::TypeId, marker::PhantomData};

pub struct InspectorTypeEditorContext<'a> {
    pub entity_info: EntityInfo<'a>,
    pub ui: &'a mut UserInterface,
    pub resource_manager: &'a ResourceManager,
    pub message_sender: &'a MessageSender,
}

pub trait InspectorTypeEditor: 'static {
    fn supports_type(&self, entity_info: EntityInfo) -> bool;
    fn create_ui(&mut self, ctx: InspectorTypeEditorContext) -> Handle<UiNode>;
    fn destroy_ui(&mut self, ctx: InspectorTypeEditorContext);
    fn on_ui_message(&mut self, message: &mut UiMessage, ctx: InspectorTypeEditorContext);
}

pub struct ResourceTypeEditor<T>
where
    T: TypedResourceData,
{
    open: Handle<Button>,
    extension: String,
    phantom_data: PhantomData<T>,
}

impl<T> ResourceTypeEditor<T>
where
    T: TypedResourceData,
{
    pub fn new(extension: &str) -> Self {
        Self {
            open: Default::default(),
            extension: extension.to_string(),
            phantom_data: Default::default(),
        }
    }
}

impl<T: TypedResourceData> InspectorTypeEditor for ResourceTypeEditor<T> {
    fn supports_type(&self, entity_info: EntityInfo) -> bool {
        let path = some_or_return!(
            entity_info
                .selection_entity
                .downcast_ref::<SelectedResource>()
                .map(|r| r.path.clone()),
            false
        );
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        TypeId::of::<T>() == entity_info.entity.type_id() || ext == self.extension
    }

    fn create_ui(&mut self, ctx: InspectorTypeEditorContext) -> Handle<UiNode> {
        let path = some_or_return!(
            ctx.entity_info
                .selection_entity
                .downcast_ref::<SelectedResource>()
                .map(|r| r.path.clone()),
            Handle::NONE
        );
        self.open = ButtonBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(2.0))
                .with_height(24.0),
        )
        .with_text(&format!("Edit {}...", path.display()))
        .build(&mut ctx.ui.build_ctx());
        self.open.to_base()
    }

    fn destroy_ui(&mut self, ctx: InspectorTypeEditorContext) {
        ctx.ui.send(self.open, WidgetMessage::Remove);
    }

    fn on_ui_message(&mut self, message: &mut UiMessage, ctx: InspectorTypeEditorContext) {
        let path = some_or_return!(
            ctx.entity_info
                .selection_entity
                .downcast_ref::<SelectedResource>()
                .map(|r| r.path.clone())
        );
        if let Some(ButtonMessage::Click) = message.data_from(self.open) {
            ctx.message_sender.send(Message::LoadScene(path));
        }
    }
}
