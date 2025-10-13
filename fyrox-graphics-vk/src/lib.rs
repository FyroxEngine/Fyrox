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

//! Vulkan-based graphics server implementation for Fyrox Game Engine.

pub mod buffer;
pub mod command;
pub mod device;
pub mod framebuffer;
pub mod geometry_buffer;
pub mod instance;
pub mod memory;
pub mod program;
pub mod query;
pub mod read_buffer;
pub mod sampler;
pub mod server;
pub mod swapchain;
pub mod texture;

/// Trait for converting Fyrox graphics types to Vulkan constants.
pub trait ToVkType<T> {
    /// Convert to Vulkan type.
    fn to_vk(self) -> T;
}

/// Utility function to convert C string to Rust string.
pub unsafe fn cstr_to_string(cstr: *const i8) -> String {
    std::ffi::CStr::from_ptr(cstr)
        .to_string_lossy()
        .into_owned()
}

/// Utility function to create a null-terminated C string from a Rust string.
pub fn to_cstring(s: &str) -> std::ffi::CString {
    std::ffi::CString::new(s).expect("Failed to create CString")
}
