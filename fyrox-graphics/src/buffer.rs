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

use crate::error::FrameworkError;
use bytemuck::Pod;
use fyrox_core::{array_as_u8_slice, array_as_u8_slice_mut};
use std::any::Any;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum BufferKind {
    Vertex,
    Index,
    Uniform,
    PixelRead,
    PixelWrite,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum BufferUsage {
    StreamDraw,
    StreamRead,
    StreamCopy,
    StaticDraw,
    StaticRead,
    StaticCopy,
    DynamicDraw,
    DynamicRead,
    DynamicCopy,
}

pub trait Buffer: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn usage(&self) -> BufferUsage;
    fn kind(&self) -> BufferKind;
    fn size(&self) -> usize;
    fn write_data(&self, data: &[u8]) -> Result<(), FrameworkError>;
    fn read_data(&self, data: &mut [u8]) -> Result<(), FrameworkError>;
}

impl dyn Buffer {
    pub fn write_data_of_type<T: Pod>(&self, data: &[T]) -> Result<(), FrameworkError> {
        let data = array_as_u8_slice(data);
        Buffer::write_data(self, data)
    }

    pub fn read_data_of_type<T: Pod>(&self, data: &mut [T]) -> Result<(), FrameworkError> {
        let data = array_as_u8_slice_mut(data);
        Buffer::read_data(self, data)
    }
}
