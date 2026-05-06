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

use crate::pool::ErasedHandle;
use crate::reflect::Reflect;
use std::any::TypeId;

pub trait ReflectHandle: Reflect {
    fn reflect_inner_type_id(&self) -> TypeId;
    fn reflect_inner_type_name(&self) -> &'static str;
    fn reflect_is_some(&self) -> bool;
    fn reflect_set_index(&mut self, index: u32);
    fn reflect_index(&self) -> u32;
    fn reflect_set_generation(&mut self, generation: u32);
    fn reflect_generation(&self) -> u32;
    fn reflect_as_erased(&self) -> ErasedHandle;
}
