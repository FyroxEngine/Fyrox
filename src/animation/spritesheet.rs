//! Sprite sheet animation is used to create simple key frame animation using single image with
//! series of frames.

#![warn(missing_docs)]

use crate::core::{
    algebra::Vector2, inspect::prelude::*, math::Rect, reflect::Reflect, visitor::prelude::*,
};
use std::ops::{Deref, DerefMut};
use strum_macros::{AsRefStr, EnumString, EnumVariantNames};

/// Animation playback status.
#[derive(
    Visit,
    Reflect,
    Inspect,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Debug,
    AsRefStr,
    EnumString,
    EnumVariantNames,
)]
pub enum Status {
    /// Animation is playing.
    Playing,

    /// Animation is stopped. Stopped animation is guaranteed to be either at beginning or at end frames (depending on speed).
    /// When an animation is stopped manually via ([`SpriteSheetAnimation::stop()`], the animation will be rewound to beginning.
    Stopped,

    /// Animation is paused. Playback can be resumed by [`SpriteSheetAnimation::play()`].
    Paused,
}

impl Default for Status {
    fn default() -> Self {
        Self::Stopped
    }
}

/// Frame bounds, represented in normalized coordinates (range [0; 1]). Normalized coordinates represents fractions of some
/// other coordinates, which means `[0; 0]` corresponds to top-left corner of a texture and `[1; 1]` corresponds to right-bottom
/// corner.
#[derive(Visit, Reflect, Inspect, Default, Clone, Debug, PartialEq)]
pub struct FrameBounds(pub Rect<f32>);

impl FrameBounds {
    /// Creates new frame bounds using given position and size.
    #[inline]
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self(Rect::new(x, y, w, h))
    }
}

impl Deref for FrameBounds {
    type Target = Rect<f32>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for FrameBounds {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Sprite sheet animation is an animation based on key frames, where each key frame is packed into single image. Usually, all key
/// frames have the same size, but this is not mandatory.
#[derive(Visit, Reflect, Inspect, Clone, Debug)]
pub struct SpriteSheetAnimation {
    frames: Vec<FrameBounds>,
    current_frame: f32,
    speed: f32,
    status: Status,
    looping: bool,
}

impl Default for SpriteSheetAnimation {
    fn default() -> Self {
        Self {
            frames: Default::default(),
            current_frame: 0.0,
            speed: 10.0,
            status: Default::default(),
            looping: true,
        }
    }
}

/// Sprite sheet source image parameters defines how to interpret an image. It defines size of each frame,
/// total size of an image, frame range to use, etc.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageParameters {
    /// Width of an image in pixels.
    pub width: usize,

    /// Height of an image in pixels.
    pub height: usize,

    /// Width of every frame in an image.
    pub frame_width: usize,

    /// Height of every frame in an image.
    pub frame_height: usize,

    /// Index of a first frame at which a produced animation should start.
    pub first_frame: usize,

    /// Index of a last frame at which a produced animation should end.
    pub last_frame: usize,

    /// Defines how to interpret the image - is it pack in rows of frames or columns of frames.
    pub column_major: bool,
}

impl SpriteSheetAnimation {
    /// Creates new empty animation.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates sprite sheet animation using given image parameters. The method is used to create animation
    /// for particular range in an image. For example, you have the following sprite sheet:
    ///
    /// ```text
    /// 128 pixels wide
    /// _________________
    /// | 0 | 1 | 2 | 3 |  
    /// |___|___|___|___|
    /// | 4 | 5 | 6 | 7 |  128 pixels tall
    /// |___|___|___|___|
    /// | 8 | 9 |10 |11 |
    /// |___|___|___|___|
    /// ```
    ///
    /// Let's assume that there could be three animations:
    /// - 0..3 - run
    /// - 4..6 - idle
    /// - 7..11 - attack
    ///
    /// and you want to extract all three animations as separate animations. In this case you could do something
    /// like this:
    ///
    /// ```rust
    /// # use fyrox::{
    /// #      animation::spritesheet::{ImageParameters, SpriteSheetAnimation},
    /// #      core::math::Rect,
    /// # };
    /// fn extract_animations() {
    ///     let run = SpriteSheetAnimation::new_from_image_parameters(ImageParameters {
    ///         width: 128,
    ///         height: 128,
    ///         frame_width: 32,
    ///         frame_height: 32,
    ///         first_frame: 0,
    ///         last_frame: 4,
    ///         column_major: false,
    ///     });
    ///
    ///     let idle = SpriteSheetAnimation::new_from_image_parameters(ImageParameters {
    ///         width: 128,
    ///         height: 128,
    ///         frame_width: 32,
    ///         frame_height: 32,
    ///         first_frame: 4,
    ///         last_frame: 7,
    ///         column_major: false,
    ///     });
    ///
    ///     let attack = SpriteSheetAnimation::new_from_image_parameters(ImageParameters {
    ///         width: 128,
    ///         height: 128,
    ///         frame_width: 32,
    ///         frame_height: 32,
    ///         first_frame: 7,
    ///         last_frame: 12,
    ///         column_major: false,
    ///     });
    ///  }
    /// ```
    ///
    /// If frames if your sprite sheet are ordered in column-major fashion (when you count them from top-left corner to bottom-left corner and then
    /// starting from new column, etc.), you should set `column_major` parameter to true.
    pub fn new_from_image_parameters(params: ImageParameters) -> Self {
        let ImageParameters {
            width,
            height,
            frame_width,
            frame_height,
            first_frame,
            last_frame,
            column_major,
        } = params;

        let normalized_frame_width = frame_width as f32 / width as f32;
        let normalized_frame_height = frame_height as f32 / height as f32;

        let width_in_frames = width / frame_width;
        let height_in_frames = height / frame_height;

        let frames = (first_frame..last_frame)
            .map(|n| {
                let x = if column_major {
                    n / width_in_frames
                } else {
                    n % width_in_frames
                };
                let y = if column_major {
                    n % height_in_frames
                } else {
                    n / height_in_frames
                };

                FrameBounds(Rect {
                    position: Vector2::new(
                        x as f32 * normalized_frame_width,
                        y as f32 * normalized_frame_height,
                    ),
                    size: Vector2::new(normalized_frame_width, normalized_frame_height),
                })
            })
            .collect::<Vec<_>>();

        Self {
            frames,
            ..Default::default()
        }
    }

    /// Adds new frame.
    pub fn add_frame(&mut self, frame: FrameBounds) {
        self.frames.push(frame);
    }

    /// Remove a frame at given index.
    pub fn remove_frame(&mut self, index: usize) -> Option<FrameBounds> {
        if index < self.frames.len() {
            self.current_frame = self.current_frame.min(self.frames.len() as f32);
            Some(self.frames.remove(index))
        } else {
            None
        }
    }

    /// Updates animation playback using given time step.
    pub fn update(&mut self, dt: f32) {
        if self.status != Status::Playing {
            return;
        }

        if self.frames.is_empty() {
            self.status = Status::Stopped;
            return;
        }

        self.current_frame += self.speed * dt;
        if self.current_frame >= self.frames.len() as f32 {
            if self.looping {
                // Continue playing from beginning.
                self.current_frame = 0.0;
            } else {
                // Keep on last frame and stop.
                self.current_frame = self.frames.len().saturating_sub(1) as f32;
                self.status = Status::Stopped;
            }
        } else if self.current_frame <= 0.0 {
            if self.looping {
                // Continue playing from end.
                self.current_frame = self.frames.len().saturating_sub(1) as f32;
            } else {
                // Keep on first frame and stop.
                self.current_frame = 0.0;
                self.status = Status::Stopped;
            }
        }
    }

    /// Returns current frame index.
    pub fn current_frame(&self) -> usize {
        self.current_frame as usize
    }

    /// Tries to fetch UV rectangle at current frame. Returns `None` if animation is empty.
    pub fn current_frame_uv_rect(&self) -> Option<&FrameBounds> {
        self.frames.get(self.current_frame())
    }

    /// Sets current frame of the animation. Input value will be clamped to [0; frame_count] range.
    pub fn set_current_frame(&mut self, current_frame: usize) {
        self.current_frame = current_frame.min(self.frames.len()) as f32;
    }

    /// Returns true if the animation is looping, false - otherwise.
    pub fn is_looping(&self) -> bool {
        self.looping
    }

    /// Continue animation from beginning (or end in case of negative speed) when ended or stop.
    pub fn set_looping(&mut self, looping: bool) {
        self.looping = looping;
    }

    /// Returns playback speed in frames per second.
    pub fn speed(&self) -> f32 {
        self.speed
    }

    /// Sets playback speed in frames per second. The speed can be negative, in this case animation
    /// will play in reverse.
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
    }

    /// Sets current frame index to the first frame in the animation.
    pub fn rewind_to_beginning(&mut self) {
        self.current_frame = 0.0;
    }

    /// Sets current frame index to the last frame in the animation.
    pub fn rewind_to_end(&mut self) {
        self.current_frame = self.frames.len().saturating_sub(1) as f32;
    }

    /// Returns current status of the animation.
    pub fn status(&self) -> Status {
        self.status
    }

    /// Starts animation playback.
    pub fn play(&mut self) {
        self.status = Status::Playing;
    }

    /// Returns `true` if the animation is playing, `false` - otherwise.
    pub fn is_playing(&self) -> bool {
        self.status == Status::Playing
    }

    /// Stops animation playback, rewinds animation to the beginning.
    pub fn stop(&mut self) {
        self.status = Status::Stopped;
        self.rewind_to_beginning();
    }

    /// Returns `true` if the animation is stopped, `false` - otherwise.
    pub fn is_stopped(&self) -> bool {
        self.status == Status::Stopped
    }

    /// Puts animation playback on pause.
    pub fn pause(&mut self) {
        self.status = Status::Paused;
    }

    /// Returns `true` if the animation is paused, `false` - otherwise.
    pub fn is_paused(&self) -> bool {
        self.status == Status::Paused
    }
}

#[cfg(test)]
mod test {
    use crate::animation::spritesheet::{
        FrameBounds, ImageParameters, SpriteSheetAnimation, Status,
    };

    #[test]
    fn test_sprite_sheet_one_row() {
        let animation = SpriteSheetAnimation::new_from_image_parameters(ImageParameters {
            width: 128,
            height: 128,
            frame_width: 32,
            frame_height: 32,
            first_frame: 0,
            last_frame: 4,
            column_major: false,
        });
        assert_eq!(animation.frames[0], FrameBounds::new(0.0, 0.0, 0.25, 0.25));
        assert_eq!(animation.frames[1], FrameBounds::new(0.25, 0.0, 0.25, 0.25));
        assert_eq!(animation.frames[2], FrameBounds::new(0.5, 0.0, 0.25, 0.25));
        assert_eq!(animation.frames[3], FrameBounds::new(0.75, 0.0, 0.25, 0.25));
    }

    #[test]
    fn test_sprite_sheet_one_column() {
        let animation = SpriteSheetAnimation::new_from_image_parameters(ImageParameters {
            width: 128,
            height: 128,
            frame_width: 32,
            frame_height: 32,
            first_frame: 0,
            last_frame: 4,
            column_major: true,
        });
        assert_eq!(animation.frames[0], FrameBounds::new(0.0, 0.0, 0.25, 0.25));
        assert_eq!(animation.frames[1], FrameBounds::new(0.0, 0.25, 0.25, 0.25));
        assert_eq!(animation.frames[2], FrameBounds::new(0.0, 0.5, 0.25, 0.25));
        assert_eq!(animation.frames[3], FrameBounds::new(0.0, 0.75, 0.25, 0.25));
    }

    #[test]
    fn test_sprite_sheet_row_partial() {
        let animation = SpriteSheetAnimation::new_from_image_parameters(ImageParameters {
            width: 128,
            height: 128,
            frame_width: 32,
            frame_height: 32,
            first_frame: 2,
            last_frame: 6,
            column_major: false,
        });
        assert_eq!(animation.frames[0], FrameBounds::new(0.5, 0.0, 0.25, 0.25));
        assert_eq!(animation.frames[1], FrameBounds::new(0.75, 0.0, 0.25, 0.25));
        assert_eq!(animation.frames[2], FrameBounds::new(0.0, 0.25, 0.25, 0.25));
        assert_eq!(
            animation.frames[3],
            FrameBounds::new(0.25, 0.25, 0.25, 0.25)
        );
    }

    #[test]
    fn test_sprite_sheet_column_partial() {
        let animation = SpriteSheetAnimation::new_from_image_parameters(ImageParameters {
            width: 128,
            height: 128,
            frame_width: 32,
            frame_height: 32,
            first_frame: 2,
            last_frame: 6,
            column_major: true,
        });
        assert_eq!(animation.frames[0], FrameBounds::new(0.0, 0.5, 0.25, 0.25));
        assert_eq!(animation.frames[1], FrameBounds::new(0.0, 0.75, 0.25, 0.25));
        assert_eq!(animation.frames[2], FrameBounds::new(0.25, 0.0, 0.25, 0.25));
        assert_eq!(
            animation.frames[3],
            FrameBounds::new(0.25, 0.25, 0.25, 0.25)
        );
    }

    #[test]
    fn test_sprite_sheet_playback() {
        let mut animation = SpriteSheetAnimation::new_from_image_parameters(ImageParameters {
            width: 128,
            height: 128,
            frame_width: 32,
            frame_height: 32,
            first_frame: 2,
            last_frame: 6,
            column_major: true,
        });

        animation.speed = 1.0; // 1 FPS
        animation.looping = false;

        assert_eq!(animation.status, Status::Stopped);

        animation.play();

        assert_eq!(animation.status, Status::Playing);

        let expected_output = [
            FrameBounds::new(0.0, 0.5, 0.25, 0.25),
            FrameBounds::new(0.0, 0.75, 0.25, 0.25),
            FrameBounds::new(0.25, 0.0, 0.25, 0.25),
            FrameBounds::new(0.25, 0.25, 0.25, 0.25),
        ];

        for expected_frame in &expected_output {
            assert_eq!(animation.current_frame_uv_rect(), Some(expected_frame));
            animation.update(1.0);
        }

        assert_eq!(animation.status, Status::Stopped);

        animation.speed = -1.0; // Play in reverse.

        animation.play();

        for expected_frame in expected_output.iter().rev() {
            assert_eq!(animation.current_frame_uv_rect(), Some(expected_frame));
            animation.update(1.0);
        }
    }
}
