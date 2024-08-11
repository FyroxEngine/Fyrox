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

//! A crate that allows using Fyrox as a dynamically linked library. It could be useful for fast
//! prototyping, that can save some time on avoiding potentially time-consuming static linking
//! stage.
//!
//! The crate just re-exports everything from the engine, and you can use it as Fyrox. To use the
//! crate all you need to do is re-define `fyrox` dependency in your project like so:
//!
//! ```toml
//! [dependencies.fyrox]
//! version = "0.1.0"
//! registry = "fyrox-dylib"
//! package = "fyrox-dylib"
//! ```
//!
//! You can also use the latest version from git:
//!
//! ```toml
//! [dependencies.fyrox]
//! git = "https://github.com/FyroxEngine/Fyrox"
//! package = "fyrox-dylib"
//! ```

// Just re-export everything.
pub use fyrox_impl::*;
