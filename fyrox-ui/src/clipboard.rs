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

use fyrox_core::blank_reflect_ref;
use fyrox_core::reflect::Reflect;

#[derive(Default, Debug)]
pub struct ClipboardContainer(Option<Box<dyn Reflect>>);

impl ClipboardContainer {
    pub fn replace_content(&mut self, value: Option<Box<dyn Reflect>>) -> Option<Box<dyn Reflect>> {
        std::mem::replace(&mut self.0, value)
    }

    pub fn set_content_raw(&mut self, value: Box<dyn Reflect>) -> Option<Box<dyn Reflect>> {
        self.0.replace(value)
    }

    pub fn set_content<T: Reflect>(&mut self, value: T) -> Option<Box<dyn Reflect>> {
        self.0.replace(Box::new(value))
    }

    pub fn content_ref_raw(&self) -> Option<&dyn Reflect> {
        self.0.as_deref()
    }

    pub fn content_ref<T: Reflect>(&self) -> Option<&T> {
        self.0.as_ref()?.downcast_ref()
    }

    pub fn content_mut<T: Reflect>(&mut self) -> Option<&mut T> {
        self.0.as_mut()?.downcast_mut()
    }

    pub fn try_take_content<T: Reflect>(&mut self) -> Option<T> {
        if let Some(value) = self.0.take() {
            match value.downcast::<T>() {
                Ok(typed) => Some(*typed),
                Err(untyped) => {
                    self.0.replace(untyped);
                    None
                }
            }
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.0 = None;
    }
}

impl Reflect for &'static mut ClipboardContainer {
    blank_reflect_ref!("494991b3-f492-4af0-bf9e-060b24c032f0");
}
