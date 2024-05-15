//! 3D and 2D Game Engine.

#![doc(
    html_logo_url = "https://fyrox.rs/assets/logos/logo.png",
    html_favicon_url = "https://fyrox.rs/assets/logos/logo.png"
)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::from_over_into)]
#![allow(clippy::approx_constant)]

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
pub use tbc;
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

    #[test]
    fn test_assembly_names() {
        let var = ImmutableString::new("Foobar");
        let base = BaseBuilder::new().build_base();
        let widget = WidgetBuilder::new().build();
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
