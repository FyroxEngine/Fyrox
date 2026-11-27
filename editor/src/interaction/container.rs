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

use super::InteractionMode;
use crate::{Message, fyrox::core::uuid::Uuid, message::MessageSender};

#[derive(Default)]
pub struct InteractionModeContainer {
    // It is better to use Vec instead of HashMap here, because it keeps the order iteration of the
    // modes the same as the order in which the modes were added to the container. Performance here
    // is not an issue, because there are tiny amount of modes anyway (currently - max 5) and linear
    // search is faster in such conditions.
    container: Vec<Box<dyn InteractionMode>>,
    pub sender: Option<MessageSender>,
}

impl InteractionModeContainer {
    fn try_notify_changed(&self) {
        if let Some(sender) = self.sender.as_ref() {
            sender.send(Message::SyncInteractionModes);
        }
    }

    pub fn add<T: InteractionMode>(&mut self, mode: T) {
        self.container.push(Box::new(mode));
        self.try_notify_changed();
    }

    pub fn remove(&mut self, id: &Uuid) -> Option<Box<dyn InteractionMode>> {
        if let Some(position) = self.container.iter().position(|mode| mode.uuid() == *id) {
            self.try_notify_changed();
            Some(self.container.remove(position))
        } else {
            None
        }
    }

    pub fn remove_typed<T: InteractionMode>(&mut self) -> Option<Box<T>> {
        self.remove(&T::type_info().type_uuid)
            .and_then(|mode| mode.into_any().downcast::<T>().ok())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut dyn InteractionMode> + '_ {
        self.container.iter_mut().map(|mode| &mut **mode)
    }

    pub fn get(&self, id: &Uuid) -> Option<&dyn InteractionMode> {
        self.container
            .iter()
            .find(|mode| mode.uuid() == *id)
            .map(|mode| &**mode)
    }

    pub fn get_mut(&mut self, id: &Uuid) -> Option<&mut dyn InteractionMode> {
        self.container
            .iter_mut()
            .find(|mode| mode.uuid() == *id)
            .map(|mode| &mut **mode)
    }

    pub fn of_type<T: InteractionMode>(&self) -> Option<&T> {
        self.get(&T::type_info().type_uuid)
            .and_then(|mode| mode.as_any().downcast_ref())
    }

    pub fn of_type_mut<T: InteractionMode>(&mut self) -> Option<&mut T> {
        self.get_mut(&T::type_info().type_uuid)
            .and_then(|mode| mode.as_any_mut().downcast_mut())
    }

    pub fn drain(&mut self) -> impl Iterator<Item = Box<dyn InteractionMode>> + '_ {
        self.try_notify_changed();
        self.container.drain(..)
    }
}
