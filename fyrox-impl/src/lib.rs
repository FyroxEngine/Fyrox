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

//! 3D/2D Game Engine.
//!
//! Tutorials can be found [here](https://fyrox-book.github.io/tutorials/tutorials.html)

#![doc(
    html_logo_url = "https://fyrox.rs/assets/logos/logo.png",
    html_favicon_url = "https://fyrox.rs/assets/logos/logo.png"
)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::from_over_into)]
#![allow(clippy::approx_constant)]
#![allow(clippy::doc_lazy_continuation)]
#![allow(clippy::mutable_key_type)]

pub mod engine;
pub mod material;
pub mod plugin;
pub mod renderer;
pub mod resource;
pub mod scene;
pub mod script;
pub mod utils;

pub use crate::core::rand;
pub use fxhash;
pub use lazy_static;
pub use walkdir;
pub use winit::*;

#[doc(inline)]
pub use fyrox_animation as generic_animation;

#[doc(inline)]
pub use fyrox_graph as graph;

#[doc(inline)]
pub use fyrox_core as core;

#[doc(inline)]
pub use fyrox_resource as asset;

#[doc(inline)]
pub use fyrox_ui as gui;

#[doc(inline)]
pub use fyrox_autotile as autotile;

/// Defines a builder's `with_xxx` method.
#[macro_export]
macro_rules! define_with {
    ($(#[$attr:meta])* fn $name:ident($field:ident: $ty:ty)) => {
        $(#[$attr])*
        pub fn $name(mut self, value: $ty) -> Self {
            self.$field = value;
            self
        }
    };
}

#[cfg(test)]
mod test {
    use crate::scene::base::{Base, BaseBuilder};
    use fyrox_core::reflect::Reflect;
    use fyrox_core::ImmutableString;
    use fyrox_sound::source::Status;
    use fyrox_ui::widget::{Widget, WidgetBuilder};
    use fyrox_ui::UserInterface;

    #[test]
    fn test_assembly_names() {
        let mut ui = UserInterface::new(Default::default());
        let var = ImmutableString::new("Foobar");
        let base = BaseBuilder::new().build_base();
        let widget = WidgetBuilder::new().build(&ui.build_ctx());
        let status = Status::Stopped;

        assert_eq!(var.assembly_name(), "fyrox-core");
        assert_eq!(base.assembly_name(), "fyrox-impl");
        assert_eq!(widget.assembly_name(), "fyrox-ui");
        assert_eq!(status.assembly_name(), "fyrox-sound");

        assert_eq!(ImmutableString::type_assembly_name(), "fyrox-core");
        assert_eq!(Base::type_assembly_name(), "fyrox-impl");
        assert_eq!(Widget::type_assembly_name(), "fyrox-ui");
        assert_eq!(Status::type_assembly_name(), "fyrox-sound");
    }
}
