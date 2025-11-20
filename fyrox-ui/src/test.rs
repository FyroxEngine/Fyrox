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

use crate::message::UiMessage;
use crate::text::Text;
use crate::{
    core::{algebra::Vector2, pool::Handle},
    widget::WidgetMessage,
    BuildContext, Control, UiNode, UserInterface,
};
use fyrox_core::info;
use fyrox_core::reflect::Reflect;
use fyrox_graph::{SceneGraph, SceneGraphNode};
use uuid::Uuid;

pub trait UserInterfaceTestingExtension {
    /// Clicks at the given position.
    fn click(&mut self, position: Vector2<f32>);

    /// Tries to find a widget with the given unique id and clicks at its center.
    fn click_at(&mut self, name: Uuid);

    fn click_at_text(&mut self, uuid: Uuid, text: &str);

    fn find_by_uuid(&self, uuid: Uuid) -> Option<&UiNode>;

    fn find_by_uuid_of<T: Control>(&self, uuid: Uuid) -> Option<&T>;

    fn is_visible(&self, uuid: Uuid) -> bool {
        if let Some(node) = self.find_by_uuid(uuid) {
            node.is_globally_visible()
        } else {
            panic!("Widget {uuid} does not exist!")
        }
    }

    fn poll_all_messages(&mut self);

    fn poll_and_count(&mut self, pred: impl FnMut(&UiMessage) -> bool) -> usize;
}

fn is_enabled(mut handle: Handle<UiNode>, ui: &UserInterface) -> bool {
    while let Some(node) = ui.try_get(handle) {
        if !node.enabled() {
            return false;
        }
        handle = node.parent();
    }
    true
}

impl UserInterfaceTestingExtension for UserInterface {
    fn click(&mut self, position: Vector2<f32>) {
        self.process_os_event(&crate::message::OsEvent::CursorMoved { position });
        self.process_os_event(&crate::message::OsEvent::MouseInput {
            button: crate::message::MouseButton::Left,
            state: crate::message::ButtonState::Pressed,
        });
        self.process_os_event(&crate::message::OsEvent::MouseInput {
            button: crate::message::MouseButton::Left,
            state: crate::message::ButtonState::Released,
        });
    }

    fn click_at(&mut self, uuid: Uuid) {
        assert_ne!(uuid, Uuid::default());
        if let Some((handle, n)) = self.find_from_root(&mut |n| n.id == uuid) {
            info!("{} - bounds {:?}", uuid, n.screen_bounds());
            assert!(is_enabled(handle, self));
            assert!(n.is_globally_visible());
            let center = n.local_to_screen(n.center());
            self.click(center);
            info!(
                "Clicked at {uuid}({}:{}) at [{};{}] coords.",
                handle.index(),
                handle.generation(),
                center.x,
                center.y
            );
        } else {
            panic!("There's no widget {uuid}!")
        }
    }

    fn click_at_text(&mut self, uuid: Uuid, text: &str) {
        assert_ne!(uuid, Uuid::default());
        if let Some((start_handle, start_node)) = self.find_from_root(&mut |n| n.id == uuid) {
            info!("{} - bounds {:?}", uuid, start_node.screen_bounds());
            assert!(is_enabled(start_handle, self));
            assert!(start_node.is_globally_visible());
            if let Some((text_handle, text_node)) = self.find(start_handle, &mut |n| {
                if let Some(text_widget) = n.component_ref::<Text>() {
                    text_widget.text() == text
                } else {
                    false
                }
            }) {
                assert!(is_enabled(text_handle, self));
                assert!(text_node.is_globally_visible());
                let center = text_node.local_to_screen(text_node.center());
                self.click(center);
                info!(
                    "Clicked at {text}({}:{}) at [{};{}] coords. Found from {uuid} starting location.",
                    text_handle.index(),
                    text_handle.generation(),
                    center.x,
                    center.y
                );
            }
        } else {
            panic!("There's no widget {uuid}!")
        }
    }

    fn find_by_uuid(&self, uuid: Uuid) -> Option<&UiNode> {
        self.find_from_root(&mut |n| n.id == uuid).map(|(_, n)| n)
    }

    fn find_by_uuid_of<T: Control>(&self, uuid: Uuid) -> Option<&T> {
        self.find_from_root(&mut |n| n.id == uuid)
            .and_then(|(_, n)| n.cast())
    }

    fn poll_all_messages(&mut self) {
        while let Some(msg) = self.poll_message() {
            if let Some(widget) = self.try_get(msg.destination()) {
                let ty = Reflect::type_name(widget);
                info!("[{ty}]{msg:?}");
            }
        }
        let screen_size = self.screen_size();
        self.update(screen_size, 1.0 / 60.0, &Default::default());
    }

    fn poll_and_count(&mut self, mut pred: impl FnMut(&UiMessage) -> bool) -> usize {
        let mut num = 0;
        while let Some(msg) = self.poll_message() {
            if let Some(widget) = self.try_get(msg.destination()) {
                let ty = Reflect::type_name(widget);
                info!("[{ty}]{msg:?}");
            }

            if pred(&msg) {
                num += 1;
            }
        }
        let screen_size = self.screen_size();
        self.update(screen_size, 1.0 / 60.0, &Default::default());
        num
    }
}

pub fn test_widget_deletion(constructor: impl FnOnce(&mut BuildContext) -> Handle<UiNode>) {
    let screen_size = Vector2::new(100.0, 100.0);
    let mut ui = UserInterface::new(screen_size);
    let widget = constructor(&mut ui.build_ctx());
    ui.send(widget, WidgetMessage::Remove);
    ui.update(screen_size, 1.0 / 60.0, &Default::default());
    while ui.poll_message().is_some() {}
    // Only root node must be alive.
    assert_eq!(ui.nodes().alive_count(), 1);
}
