//! Sprite sheet animation is used to create simple key frame animation using single image with
//! series of frames.

#![warn(missing_docs)]

use crate::{
    core::{
        algebra::Vector2,
        math::Rect,
        reflect::prelude::*,
        uuid::{uuid, Uuid},
        uuid_provider,
        visitor::prelude::*,
        TypeUuidProvider,
    },
    spritesheet::signal::Signal,
};
use std::collections::vec_deque::VecDeque;
use strum_macros::{AsRefStr, EnumString, VariantNames};

pub mod signal;

/// Trait for anything that can be used as a texture.
pub trait SpriteSheetTexture: PartialEq + Clone + Visit + Reflect + 'static {}

impl<T: PartialEq + Clone + Visit + Reflect + 'static> SpriteSheetTexture for T {}

/// Animation playback status.
#[derive(Visit, Reflect, Copy, Clone, Eq, PartialEq, Debug, AsRefStr, EnumString, VariantNames)]
pub enum Status {
    /// Animation is playing.
    Playing,

    /// Animation is stopped. Stopped animation is guaranteed to be either at beginning or at end frames (depending on speed).
    /// When an animation is stopped manually via ([`SpriteSheetAnimation::stop()`], the animation will be rewound to beginning.
    Stopped,

    /// Animation is paused. Playback can be resumed by [`SpriteSheetAnimation::play()`].
    Paused,
}

uuid_provider!(Status = "74a31122-a7a8-476c-ab87-77e53cf0523c");

impl Default for Status {
    fn default() -> Self {
        Self::Stopped
    }
}

/// Some animation event.
#[derive(Visit, Reflect, Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Event {
    /// A signal with an id was hit.
    Signal(u64),
}

impl Default for Event {
    fn default() -> Self {
        Self::Signal(0)
    }
}

/// Container for a sprite sheet animation frames.
#[derive(Reflect, Visit, Clone, Debug, PartialEq, Eq)]
pub struct SpriteSheetFramesContainer<T>
where
    T: SpriteSheetTexture,
{
    size: Vector2<u32>,
    frames: Vec<Vector2<u32>>,
    #[visit(optional)]
    texture: Option<T>,
}

impl<T> SpriteSheetFramesContainer<T>
where
    T: SpriteSheetTexture,
{
    /// Adds a frame to the container.
    pub fn push(&mut self, bounds: Vector2<u32>) {
        self.frames.push(bounds)
    }

    /// Removes a frame from the container.
    pub fn remove(&mut self, index: usize) -> Vector2<u32> {
        self.frames.remove(index)
    }

    /// Returns total amount of frames in the container.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Returns `true` if the container is empty, `false` - otherwise.
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Tries to get a reference to a frame with given index.
    pub fn get(&self, index: usize) -> Option<&Vector2<u32>> {
        self.frames.get(index)
    }

    /// Sets new container size. It does not affect frames!
    pub fn set_size(&mut self, size: Vector2<u32>) {
        self.size = Vector2::new(size.x.max(1), size.y.max(1));
    }

    /// Returns size of the container.
    pub fn size(&self) -> Vector2<u32> {
        self.size
    }

    /// Sorts frames by their position. `(x,y)` will be converted to index and then used for
    /// sorting. This method ensures that the frames will be ordered from the left top corner
    /// to right bottom corner line-by-line.
    pub fn sort_by_position(&mut self) {
        self.frames.sort_by_key(|p| p.y * self.size.x + p.x)
    }

    /// Returns an iterator that yields frames position.
    pub fn iter(&self) -> impl Iterator<Item = &Vector2<u32>> {
        self.frames.iter()
    }

    /// Returns an iterator that yields frames position.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Vector2<u32>> {
        self.frames.iter_mut()
    }

    /// Returns current texture of the container. To set a texture use sprite sheet animation methods.
    pub fn texture(&self) -> Option<T> {
        self.texture.clone()
    }
}

impl<T> Default for SpriteSheetFramesContainer<T>
where
    T: SpriteSheetTexture,
{
    fn default() -> Self {
        Self {
            size: Vector2::new(1, 1),
            frames: vec![],
            texture: None,
        }
    }
}

/// Sprite sheet animation is an animation based on key frames, where each key frame is packed into single image. Usually, all key
/// frames have the same size, but this is not mandatory.
#[derive(Visit, Reflect, Clone, Debug)]
pub struct SpriteSheetAnimation<T>
where
    T: SpriteSheetTexture,
{
    #[visit(rename = "Frames")]
    frames_container: SpriteSheetFramesContainer<T>,
    current_frame: f32,
    speed: f32,
    status: Status,
    looping: bool,
    signals: Vec<Signal>,
    #[visit(optional)]
    #[reflect(setter = "set_texture")]
    texture: Option<T>,
    #[reflect(hidden)]
    #[visit(skip)]
    events: VecDeque<Event>,
    #[visit(optional)]
    max_event_capacity: usize,
}

impl<T: SpriteSheetTexture> PartialEq for SpriteSheetAnimation<T> {
    fn eq(&self, other: &Self) -> bool {
        self.frames_container == other.frames_container
            && self.current_frame == other.current_frame
            && self.speed == other.speed
            && self.looping == other.looping
            && self.signals == other.signals
            && self.texture == other.texture
    }
}

impl<T> TypeUuidProvider for SpriteSheetAnimation<T>
where
    T: SpriteSheetTexture,
{
    fn type_uuid() -> Uuid {
        uuid!("1fa13feb-a16d-4539-acde-672aaeb0f62b")
    }
}

impl<T> Default for SpriteSheetAnimation<T>
where
    T: SpriteSheetTexture,
{
    fn default() -> Self {
        Self {
            frames_container: Default::default(),
            current_frame: 0.0,
            speed: 10.0,
            status: Default::default(),
            looping: true,
            signals: Default::default(),
            texture: None,
            events: Default::default(),
            max_event_capacity: 32,
        }
    }
}

/// Sprite sheet source image parameters defines how to interpret an image. It defines size of each frame,
/// total size of an image, frame range to use, etc.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageParameters {
    /// Width of an image in pixels.
    pub width: u32,

    /// Height of an image in pixels.
    pub height: u32,

    /// Width of every frame in an image.
    pub frame_width: u32,

    /// Height of every frame in an image.
    pub frame_height: u32,

    /// Index of a first frame at which a produced animation should start.
    pub first_frame: u32,

    /// Index of a last frame at which a produced animation should end.
    pub last_frame: u32,

    /// Defines how to interpret the image - is it pack in rows of frames or columns of frames.
    pub column_major: bool,
}

impl<T> SpriteSheetAnimation<T>
where
    T: SpriteSheetTexture,
{
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
    /// # use fyrox_animation::{
    /// #      spritesheet::{ImageParameters, SpriteSheetAnimation},
    /// #      core::math::Rect,
    /// # };
    /// # use fyrox_core::{reflect::prelude::*, visitor::prelude::*};
    /// #
    /// #[derive(PartialEq, Clone, Reflect, Visit, Debug)]
    /// struct MyTexture {}
    ///
    /// fn extract_animations() {
    ///     let run = SpriteSheetAnimation::<MyTexture>::new_from_image_parameters(ImageParameters {
    ///         width: 128,
    ///         height: 128,
    ///         frame_width: 32,
    ///         frame_height: 32,
    ///         first_frame: 0,
    ///         last_frame: 4,
    ///         column_major: false,
    ///     });
    ///
    ///     let idle = SpriteSheetAnimation::<MyTexture>::new_from_image_parameters(ImageParameters {
    ///         width: 128,
    ///         height: 128,
    ///         frame_width: 32,
    ///         frame_height: 32,
    ///         first_frame: 4,
    ///         last_frame: 7,
    ///         column_major: false,
    ///     });
    ///
    ///     let attack = SpriteSheetAnimation::<MyTexture>::new_from_image_parameters(ImageParameters {
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

                Vector2::new(x, y)
            })
            .collect::<Vec<_>>();

        Self {
            frames_container: SpriteSheetFramesContainer {
                frames,
                size: Vector2::new(width_in_frames, height_in_frames),
                texture: None,
            },
            ..Default::default()
        }
    }

    /// Creates new animation with given frames container.
    pub fn with_container(container: SpriteSheetFramesContainer<T>) -> Self {
        Self {
            frames_container: container,
            ..Default::default()
        }
    }

    /// Sets new texture for the animation.
    pub fn set_texture(&mut self, texture: Option<T>) -> Option<T> {
        self.frames_container.texture.clone_from(&texture);
        std::mem::replace(&mut self.texture, texture)
    }

    /// Returns current texture of the animation.
    pub fn texture(&self) -> Option<T> {
        self.texture.clone()
    }

    /// Gets the maximum capacity of events.
    pub fn get_max_event_capacity(&self) -> usize {
        self.max_event_capacity
    }

    /// Sets the maximum capacity of events.
    pub fn set_max_event_capacity(&mut self, max_event_capacity: usize) {
        self.max_event_capacity = max_event_capacity;
    }

    /// Returns a shared reference to inner frames container.
    pub fn frames(&self) -> &SpriteSheetFramesContainer<T> {
        &self.frames_container
    }

    /// Returns a mutable reference to inner frames container.
    pub fn frames_mut(&mut self) -> &mut SpriteSheetFramesContainer<T> {
        &mut self.frames_container
    }

    /// Adds new frame.
    pub fn add_frame(&mut self, frame: Vector2<u32>) {
        self.frames_container.push(frame);
    }

    /// Remove a frame at given index.
    pub fn remove_frame(&mut self, index: usize) -> Option<Vector2<u32>> {
        if index < self.frames_container.len() {
            self.current_frame = self.current_frame.min(self.frames_container.len() as f32);
            Some(self.frames_container.remove(index))
        } else {
            None
        }
    }

    /// Updates animation playback using given time step.
    pub fn update(&mut self, dt: f32) {
        if self.status != Status::Playing {
            return;
        }

        if self.frames_container.is_empty() {
            self.status = Status::Stopped;
            return;
        }

        let next_frame = self.current_frame + self.speed * dt;

        for signal in self.signals.iter_mut().filter(|s| s.enabled) {
            let signal_frame = signal.frame as f32;

            if (self.speed >= 0.0
                && (self.current_frame < signal_frame && next_frame >= signal_frame)
                || self.speed < 0.0
                    && (self.current_frame > signal_frame && next_frame <= signal_frame))
                && self.events.len() < self.max_event_capacity
            {
                self.events.push_back(Event::Signal(signal.id));
            }
        }

        self.current_frame = next_frame;
        if self.current_frame >= self.frames_container.len() as f32 {
            if self.looping {
                // Continue playing from beginning.
                self.current_frame = 0.0;
            } else {
                // Keep on last frame and stop.
                self.current_frame = self.frames_container.len().saturating_sub(1) as f32;
                self.status = Status::Stopped;
            }
        } else if self.current_frame <= 0.0 {
            if self.looping {
                // Continue playing from end.
                self.current_frame = self.frames_container.len().saturating_sub(1) as f32;
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

    /// Tries to fetch UV rectangle at given frame. Returns `None` if animation is empty.
    pub fn frame_uv_rect(&self, i: usize) -> Option<Rect<f32>> {
        assert_ne!(self.frames_container.size.x, 0);
        assert_ne!(self.frames_container.size.y, 0);

        self.frames_container.get(i).map(|pos| Rect {
            position: Vector2::new(
                pos.x as f32 / self.frames_container.size.x as f32,
                pos.y as f32 / self.frames_container.size.y as f32,
            ),
            size: Vector2::new(
                1.0 / self.frames_container.size.x as f32,
                1.0 / self.frames_container.size.y as f32,
            ),
        })
    }

    /// Tries to fetch UV rectangle at current frame. Returns `None` if animation is empty.
    pub fn current_frame_uv_rect(&self) -> Option<Rect<f32>> {
        self.frame_uv_rect(self.current_frame())
    }

    /// Sets current frame of the animation. Input value will be clamped to [0; frame_count] range.
    pub fn set_current_frame(&mut self, current_frame: usize) {
        self.current_frame = current_frame.min(self.frames_container.len()) as f32;
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
        self.current_frame = self.frames_container.len().saturating_sub(1) as f32;
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

    /// Adds new animation signal to the animation.
    pub fn add_signal(&mut self, signal: Signal) {
        self.signals.push(signal)
    }

    /// Removes animation signal by given id.
    pub fn remove_signal(&mut self, id: u64) {
        self.signals.retain(|s| s.id != id)
    }

    /// Pops animation event from internal queue.
    pub fn pop_event(&mut self) -> Option<Event> {
        self.events.pop_front()
    }
}

#[cfg(test)]
mod test {
    use crate::spritesheet::{
        signal::Signal, Event, ImageParameters, SpriteSheetAnimation, Status,
    };
    use fyrox_core::{algebra::Vector2, math::Rect, reflect::prelude::*, visitor::prelude::*};

    #[derive(PartialEq, Clone, Reflect, Visit, Debug)]
    struct MyTexture {}

    #[test]
    fn test_sprite_sheet_one_row() {
        let animation =
            SpriteSheetAnimation::<MyTexture>::new_from_image_parameters(ImageParameters {
                width: 128,
                height: 128,
                frame_width: 32,
                frame_height: 32,
                first_frame: 0,
                last_frame: 4,
                column_major: false,
            });
        assert_eq!(
            animation.frame_uv_rect(0),
            Some(Rect::new(0.0, 0.0, 0.25, 0.25))
        );
        assert_eq!(
            animation.frame_uv_rect(1),
            Some(Rect::new(0.25, 0.0, 0.25, 0.25))
        );
        assert_eq!(
            animation.frame_uv_rect(2),
            Some(Rect::new(0.5, 0.0, 0.25, 0.25))
        );
        assert_eq!(
            animation.frame_uv_rect(3),
            Some(Rect::new(0.75, 0.0, 0.25, 0.25))
        );
    }

    #[test]
    fn test_sprite_sheet_one_column() {
        let animation =
            SpriteSheetAnimation::<MyTexture>::new_from_image_parameters(ImageParameters {
                width: 128,
                height: 128,
                frame_width: 32,
                frame_height: 32,
                first_frame: 0,
                last_frame: 4,
                column_major: true,
            });
        assert_eq!(
            animation.frame_uv_rect(0),
            Some(Rect::new(0.0, 0.0, 0.25, 0.25))
        );
        assert_eq!(
            animation.frame_uv_rect(1),
            Some(Rect::new(0.0, 0.25, 0.25, 0.25))
        );
        assert_eq!(
            animation.frame_uv_rect(2),
            Some(Rect::new(0.0, 0.5, 0.25, 0.25))
        );
        assert_eq!(
            animation.frame_uv_rect(3),
            Some(Rect::new(0.0, 0.75, 0.25, 0.25))
        );
    }

    #[test]
    fn test_sprite_sheet_row_partial() {
        let animation =
            SpriteSheetAnimation::<MyTexture>::new_from_image_parameters(ImageParameters {
                width: 128,
                height: 128,
                frame_width: 32,
                frame_height: 32,
                first_frame: 2,
                last_frame: 6,
                column_major: false,
            });
        assert_eq!(
            animation.frame_uv_rect(0),
            Some(Rect::new(0.5, 0.0, 0.25, 0.25))
        );
        assert_eq!(
            animation.frame_uv_rect(1),
            Some(Rect::new(0.75, 0.0, 0.25, 0.25))
        );
        assert_eq!(
            animation.frame_uv_rect(2),
            Some(Rect::new(0.0, 0.25, 0.25, 0.25))
        );
        assert_eq!(
            animation.frame_uv_rect(3),
            Some(Rect::new(0.25, 0.25, 0.25, 0.25))
        );
    }

    #[test]
    fn test_sprite_sheet_column_partial() {
        let animation =
            SpriteSheetAnimation::<MyTexture>::new_from_image_parameters(ImageParameters {
                width: 128,
                height: 128,
                frame_width: 32,
                frame_height: 32,
                first_frame: 2,
                last_frame: 6,
                column_major: true,
            });
        assert_eq!(
            animation.frame_uv_rect(0),
            Some(Rect::new(0.0, 0.5, 0.25, 0.25))
        );
        assert_eq!(
            animation.frame_uv_rect(1),
            Some(Rect::new(0.0, 0.75, 0.25, 0.25))
        );
        assert_eq!(
            animation.frame_uv_rect(2),
            Some(Rect::new(0.25, 0.0, 0.25, 0.25))
        );
        assert_eq!(
            animation.frame_uv_rect(3),
            Some(Rect::new(0.25, 0.25, 0.25, 0.25))
        );
    }

    #[test]
    fn test_sprite_sheet_playback() {
        let mut animation =
            SpriteSheetAnimation::<MyTexture>::new_from_image_parameters(ImageParameters {
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
            Rect::new(0.0, 0.5, 0.25, 0.25),
            Rect::new(0.0, 0.75, 0.25, 0.25),
            Rect::new(0.25, 0.0, 0.25, 0.25),
            Rect::new(0.25, 0.25, 0.25, 0.25),
        ];

        for &expected_frame in &expected_output {
            assert_eq!(animation.current_frame_uv_rect(), Some(expected_frame));
            animation.update(1.0);
        }

        assert_eq!(animation.status, Status::Stopped);

        animation.speed = -1.0; // Play in reverse.

        animation.play();

        for &expected_frame in expected_output.iter().rev() {
            assert_eq!(animation.current_frame_uv_rect(), Some(expected_frame));
            animation.update(1.0);
        }
    }

    #[test]
    fn test_signals() {
        let mut animation = SpriteSheetAnimation::<MyTexture>::new();

        animation.add_frame(Vector2::new(0, 0));
        animation.add_frame(Vector2::new(1, 0));
        animation.add_frame(Vector2::new(2, 0));

        animation.set_speed(1.0);
        animation.set_looping(false);
        animation.play();

        animation.add_signal(Signal {
            id: 0,
            frame: 1,
            enabled: true,
        });

        animation.add_signal(Signal {
            id: 1,
            frame: 1,
            enabled: false,
        });

        animation.add_signal(Signal {
            id: 2,
            frame: 2,
            enabled: true,
        });

        for _ in 0..3 {
            animation.update(1.0);
        }

        assert_eq!(animation.pop_event(), Some(Event::Signal(0)));
        // Disable signals does not produce any events.
        assert_eq!(animation.pop_event(), Some(Event::Signal(2)));
        // Only two should appear.
        assert_eq!(animation.pop_event(), None);
    }
}
