//! Scene-specific sprite sheet animation.

use crate::resource::texture::TextureResource;

/// Scene-specific sprite sheet animation.
pub type SpriteSheetAnimation =
    crate::generic_animation::spritesheet::SpriteSheetAnimation<TextureResource>;
/// Scene-specific sprite sheet animation frames container.
pub type SpriteSheetFramesContainer =
    crate::generic_animation::spritesheet::SpriteSheetFramesContainer<TextureResource>;

/// Standard prelude for sprite sheet animations, that contains all most commonly used types and traits.
pub mod prelude {
    pub use super::{SpriteSheetAnimation, SpriteSheetFramesContainer};
    pub use crate::generic_animation::spritesheet::{
        signal::Signal, Event, ImageParameters, Status,
    };
}
