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

//! Animation allows you to change properties of arbitrary objects at runtime using a set of key frames.
//! See [`Animation`] docs for more info.

#![warn(missing_docs)]
#![allow(clippy::doc_lazy_continuation)]

use crate::{
    core::{
        algebra::{UnitQuaternion, Vector3},
        math::wrapf,
        pool::{ErasedHandle, Handle, Pool, Ticket},
        reflect::prelude::*,
        type_traits::prelude::*,
        uuid::{uuid, Uuid},
        visitor::{Visit, VisitResult, Visitor},
        ImmutableString, NameProvider,
    },
    track::Track,
};
use fxhash::FxHashMap;
use fyrox_resource::{Resource, ResourceData};
use std::{
    collections::VecDeque,
    error::Error,
    fmt::Debug,
    hash::Hash,
    ops::{Index, IndexMut, Range},
    path::Path,
};
use value::{nlerp, TrackValue, ValueBinding};

use crate::container::TrackDataContainer;
use crate::track::TrackBinding;
pub use fyrox_core as core;
use fyrox_resource::untyped::ResourceKind;
pub use pose::{AnimationPose, NodePose};
pub use signal::{AnimationEvent, AnimationSignal};

pub mod container;
pub mod machine;
pub mod pose;
pub mod signal;
pub mod spritesheet;
pub mod track;
pub mod value;

/// A container for animation tracks. Multiple animations can share the same container to reduce
/// memory consumption. It could be extremely useful in case of many instances of a little amount
/// of kinds of animated models.
#[derive(Default, Debug, Reflect, Clone, PartialEq, TypeUuidProvider)]
#[type_uuid(id = "044d9f7c-5c6c-4b29-8de9-d0d975a48256")]
pub struct AnimationTracksData {
    /// Tracks of the animation. See [`Track`] docs for more info.
    pub tracks: Vec<Track>,
}

impl AnimationTracksData {
    /// Adds new track to the animation. Animation can have unlimited number of tracks, each track is responsible
    /// for animation of a single scene node.
    pub fn add_track(&mut self, track: Track) {
        self.tracks.push(track);
    }

    /// Removes a track at given index.
    pub fn remove_track(&mut self, index: usize) -> Track {
        self.tracks.remove(index)
    }

    /// Inserts a track at given index.
    pub fn insert_track(&mut self, index: usize, track: Track) {
        self.tracks.insert(index, track)
    }

    /// Removes last track from the list of tracks of the animation.
    pub fn pop_track(&mut self) -> Option<Track> {
        self.tracks.pop()
    }

    /// Returns a reference to tracks container.
    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    /// Returns a mutable reference to the track container.
    pub fn tracks_mut(&mut self) -> &mut [Track] {
        &mut self.tracks
    }

    /// Removes all tracks from the animation for which the given `filter` closure returns `false`. Could be useful
    /// to remove undesired animation tracks.
    pub fn retain_tracks<F>(&mut self, filter: F)
    where
        F: FnMut(&Track) -> bool,
    {
        self.tracks.retain(filter)
    }
}

impl Visit for AnimationTracksData {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.tracks.visit(name, visitor)
    }
}

impl ResourceData for AnimationTracksData {
    fn type_uuid(&self) -> Uuid {
        <AnimationTracksData as TypeUuidProvider>::type_uuid()
    }

    fn save(&mut self, _path: &Path) -> Result<(), Box<dyn Error>> {
        // TODO
        Ok(())
    }

    fn can_be_saved(&self) -> bool {
        true
    }
}

/// A resource that holds animation tracks. This resource can be shared across multiple animations.
pub type AnimationTracksDataResource = Resource<AnimationTracksData>;

/// # Overview
///
/// Animation allows you to change properties of arbitrary entities at runtime using a set of key frames. Animation
/// consists of multiple tracks, where each track is bound to a property of an entity. A track can animate
/// any numeric properties, starting from numbers (including `bool`) end ending by 2/3/4 dimensional vectors.
/// Each component (number, x/y/z/w vector components) is stored in a _parametric curve_ (see
/// [`crate::core::math::curve::Curve`] docs for more info). Every parametric curve contains zero or more _key frames_.
/// Graphically this could be represented like so:
///
/// ```text
///                                          Timeline
///                                             v
///   Time   > |---------------|------------------------------------>
///            |               |
///   Track1 > | node.position |
///            |   X curve     |..1..........5...........10..........
///            |   Y curve     |..2.........-2..................1....  < Curve key frames
///            |   Z curve     |..1..........9......................4
///            |_______________|
///   Track2   | node.property |
///            | ............  |.....................................
///            | ............  |.....................................
///            | ............  |.....................................
/// ```
///
/// Each key frame is just a real number with interpolation mode. Interpolation mode tells the engine how to
/// calculate intermediate values between key frames. There are three kinds of interpolation used in animations
/// (you can skip "boring math" if you want):
///
/// - **Constant** - intermediate value will be calculated using leftmost value of two. Constant "interpolation" is
/// usually used to create step-like behaviour, the most common case is to "interpolate" two boolean values.
/// - **Linear** - intermediate value will be calculated using linear interpolation `i = left + (right - left) / t`,
/// where `t = (time_position - left) / (right - left)`. `t` is always in `0..1` range. Linear interpolation is usually
/// used to create "straight" transitions between two values.
/// - **Cubic** - intermediate value will be calculated using Hermite cubic spline:
/// `i = (2t^3 - 3t^2 + 1) * left + (t^3 - 2t^2 + t) * left_tangent + (-2t^3 + 3t^2) * right + (t^3 - t^2) * right_tangent`,
/// where `t = (time_position - left) / (right - left)` (`t` is always in `0..1` range), `left_tangent` and `right_tangent`
/// is usually a `tan(angle)`. Cubic interpolation is usually used to create "smooth" transitions between two values.
///
/// # Track binding
///
/// Each track is always bound to a property in a node, either by its name or by a special binding. The name is used to fetch the
/// property using reflection, the special binding is a faster way of fetching built-in properties. It is usually used to animate
/// position, scale and rotation (these are the most common properties available in every scene node).
///
/// # Time slice and looping
///
/// While key frames on the curves can be located at arbitrary position in time, animations usually plays a specific time slice.
/// By default, each animation will play on a given time slice infinitely - it is called _animation looping_, it works in both
/// playback directions.
///
/// # Speed
///
/// You can vary playback speed in wide range, by default every animation has playback speed multiplier set to 1.0. The multiplier
/// tells how faster (>1) or slower (<1) the animation needs to be played. Negative speed multiplier values will reverse playback.
///
/// # Enabling or disabling animations
///
/// Sometimes there's a need to disable/enable an animation or check if it is enabled or not, you can do this by using the pair
/// of respective methods - [`Animation::set_enabled`] and [`Animation::is_enabled`].
///
/// # Signals
///
/// Signal is a named marker on specific time position on the animation timeline. Signal will emit an event if the animation playback
/// time passes signal's position from left-to-right (or vice versa depending on playback direction). Signals are usually used to
/// attach some specific actions to a position in time. For example, you can have a walking animation and you want to emit sounds
/// when character's feet touch ground. In this case you need to add a few signals at times when each foot touches the ground.
/// After that all you need to do is to fetch animation events one-by-one and emit respective sounds. See [`AnimationSignal`] docs
/// for more info and examples.
///
/// # Examples
///
/// Usually, animations are created from the editor or some external tool and then imported in the engine.
/// However, sometimes there's a need for procedural animations. Use the following example code as
/// a guide **only** if you need to create procedural animations:
///
/// ```rust
/// # use fyrox_animation::{
/// #     container::{TrackDataContainer, TrackValueKind},
/// #     track::Track,
/// #     value::ValueBinding,
/// #     Animation,
/// #     core::{
/// #         math::curve::{Curve, CurveKey, CurveKeyKind},
/// #         pool::Handle,
/// #     },
/// # };
/// # use fyrox_animation::track::TrackBinding;
/// # use fyrox_core::pool::ErasedHandle;
/// fn create_animation(target: ErasedHandle) -> Animation<ErasedHandle> {
///     let mut frames_container = TrackDataContainer::new(TrackValueKind::Vector3);
///
///     // We'll animate only X coordinate (at index 0).
///     frames_container.curves_mut()[0] = Curve::from(vec![
///         CurveKey::new(0.5, 2.0, CurveKeyKind::Linear),
///         CurveKey::new(0.75, 1.0, CurveKeyKind::Linear),
///         CurveKey::new(1.0, 3.0, CurveKeyKind::Linear),
///     ]);
///
///     // Create a track that will animate the node using the curve above.
///     let mut track = Track::new(frames_container, ValueBinding::Position);
///
///     // Finally create an animation and set its time slice and turn it on.
///     let mut animation = Animation::default();
///     animation.add_track_with_binding(TrackBinding::new(target), track);
///     animation.set_time_slice(0.0..1.0);
///     animation.set_enabled(true);
///
///     animation
/// }
///
/// // Create the animation.
/// let mut animation = create_animation(Default::default());
///
/// // Emulate some ticks (like it was updated from the main loop of your game).
/// for _ in 0..10 {
///     animation.tick(1.0 / 60.0);
/// }
/// ```
///
/// The code above creates a simple animation that moves a node along X axis in various ways. The usage of the animation
/// is only for the sake of completeness of the example. In the real games you need to add the animation to an animation
/// player scene node and it will do the job for you.
#[derive(Debug, Reflect, PartialEq)]
pub struct Animation<T: EntityId> {
    name: ImmutableString,
    tracks_data: AnimationTracksDataResource,
    track_bindings: FxHashMap<Uuid, TrackBinding<T>>,
    time_position: f32,
    time_slice: Range<f32>,
    speed: f32,
    looped: bool,
    enabled: bool,
    signals: Vec<AnimationSignal>,
    root_motion_settings: Option<RootMotionSettings<T>>,
    max_event_capacity: usize,

    #[reflect(hidden)]
    root_motion: Option<RootMotion>,
    // Non-serialized
    #[reflect(hidden)]
    pose: AnimationPose<T>,
    // Non-serialized
    #[reflect(hidden)]
    events: VecDeque<AnimationEvent>,
}

#[derive(Visit, Default)]
struct OldTrack<T: EntityId> {
    binding: ValueBinding,
    frames: TrackDataContainer,
    enabled: bool,
    node: T,
    id: Uuid,
}

impl<T: EntityId> Visit for Animation<T> {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        // Backward compatibility.
        let mut old_tracks = Vec::<OldTrack<T>>::new();
        if region.is_reading() {
            if old_tracks.visit("Tracks", &mut region).is_ok() {
                let mut tracks_data = AnimationTracksData::default();
                for old_track in old_tracks {
                    self.track_bindings.insert(
                        old_track.id,
                        TrackBinding {
                            enabled: old_track.enabled,
                            target: old_track.node,
                        },
                    );
                    tracks_data.tracks.push(Track {
                        binding: old_track.binding,
                        frames: old_track.frames,
                        id: old_track.id,
                    });
                }
                self.tracks_data = AnimationTracksDataResource::new_ok(
                    Uuid::new_v4(),
                    ResourceKind::Embedded,
                    tracks_data,
                );
            } else {
                self.tracks_data.visit("TracksData", &mut region)?;
                self.track_bindings.visit("TrackBindings", &mut region)?;
            }
        } else {
            self.tracks_data.visit("TracksData", &mut region)?;
            self.track_bindings.visit("TrackBindings", &mut region)?;
        }

        let _ = self.name.visit("Name", &mut region);
        self.time_position.visit("TimePosition", &mut region)?;
        let _ = self.time_slice.visit("TimeSlice", &mut region);
        self.speed.visit("Speed", &mut region)?;
        self.looped.visit("Looped", &mut region)?;
        self.enabled.visit("Enabled", &mut region)?;
        self.signals.visit("Signals", &mut region)?;
        let _ = self
            .max_event_capacity
            .visit("MaxEventCapacity", &mut region);
        let _ = self
            .root_motion_settings
            .visit("RootMotionSettings", &mut region);

        Ok(())
    }
}

impl<T: EntityId> TypeUuidProvider for Animation<T> {
    fn type_uuid() -> Uuid {
        uuid!("aade8e9d-e2cf-401d-a4d1-59c6943645f3")
    }
}

/// Identifier of an entity, that can be animated.
pub trait EntityId:
    Default + Send + Copy + Reflect + Visit + PartialEq + Eq + Hash + Debug + Ord + PartialEq + 'static
{
}

impl<T: Reflect> EntityId for Handle<T> {}
impl EntityId for ErasedHandle {}

/// Root motion settings. It allows you to set a node (root) from which the motion will be taken
/// as well as filter out some unnecessary parts of the motion (i.e. do not extract motion on
/// Y axis).
#[derive(Default, Debug, Clone, PartialEq, Reflect, Visit)]
pub struct RootMotionSettings<T: EntityId> {
    /// A handle to a node which movement will be extracted and put in root motion field of an animation
    /// to which these settings were set to.
    pub node: T,
    /// Keeps X part of the translational part of the motion.
    pub ignore_x_movement: bool,
    /// Keeps Y part of the translational part of the motion.
    pub ignore_y_movement: bool,
    /// Keeps Z part of the translational part of the motion.
    pub ignore_z_movement: bool,
    /// Keeps rotational part of the motion.
    pub ignore_rotations: bool,
}

/// Motion of a root node of an hierarchy of nodes. It contains relative rotation and translation in local
/// space of the node. To transform this data into velocity and orientation you need to multiply these
/// parts with some global transform, usually with the global transform of the mesh that is being animated.
#[derive(Default, Debug, Clone, PartialEq)]
pub struct RootMotion {
    /// Relative offset between current and a previous frame of an animation.
    pub delta_position: Vector3<f32>,
    /// Relative rotation between current and a previous frame of an animation.
    pub delta_rotation: UnitQuaternion<f32>,

    prev_position: Vector3<f32>,
    position_offset_remainder: Option<Vector3<f32>>,

    prev_rotation: UnitQuaternion<f32>,
    rotation_remainder: Option<UnitQuaternion<f32>>,
}

impl RootMotion {
    /// Blend this motion with some other using `weight` as a proportion.
    pub fn blend_with(&mut self, other: &RootMotion, weight: f32) {
        self.delta_position = self.delta_position.lerp(&other.delta_position, weight);
        self.delta_rotation = nlerp(self.delta_rotation, &other.delta_rotation, weight);
    }
}

impl<T: EntityId> NameProvider for Animation<T> {
    fn name(&self) -> &str {
        &self.name
    }
}

impl<T: EntityId> Clone for Animation<T> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            tracks_data: self.tracks_data.clone(),
            speed: self.speed,
            time_position: self.time_position,
            looped: self.looped,
            enabled: self.enabled,
            pose: Default::default(),
            signals: self.signals.clone(),
            root_motion_settings: self.root_motion_settings.clone(),
            events: Default::default(),
            time_slice: self.time_slice.clone(),
            root_motion: self.root_motion.clone(),
            max_event_capacity: 32,
            track_bindings: self.track_bindings.clone(),
        }
    }
}

impl<T: EntityId> Animation<T> {
    /// Gets the maximum capacity of events.
    pub fn get_max_event_capacity(&self) -> usize {
        self.max_event_capacity
    }

    /// Sets the maximum capacity of events.
    pub fn set_max_event_capacity(&mut self, max_event_capacity: usize) {
        self.max_event_capacity = max_event_capacity;
    }

    /// Sets a new name for the animation. The name then could be used to find the animation in a container.
    pub fn set_name<S: AsRef<str>>(&mut self, name: S) {
        self.name = ImmutableString::new(name);
    }

    /// Returns current name of the animation.
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    /// Sets a new source of data for animation tracks. Keep in mind, that all existing track bindings
    /// stored in the animation could become invalid, if the new resource does not have tracks with
    /// the same ids that the bindings has.
    pub fn set_tracks_data(&mut self, resource: AnimationTracksDataResource) {
        self.tracks_data = resource;
    }

    /// Returns a reference to the current animation tracks resource.
    pub fn tracks_data(&self) -> &AnimationTracksDataResource {
        &self.tracks_data
    }

    /// Calculates new length of the animation based on the content of its tracks. It looks for the most "right"
    /// curve key in all curves of all tracks and treats it as length of the animation. The method could be used
    /// in case if you formed animation from code using just curves and don't know the actual length of the
    /// animation.
    pub fn fit_length_to_content(&mut self) {
        let state = self.tracks_data.state();
        let Some(tracks_data) = state.data_ref() else {
            return;
        };
        self.time_slice.start = 0.0;
        for track in tracks_data.tracks.iter() {
            if track.time_length() > self.time_slice.end {
                self.time_slice.end = track.time_length();
            }
        }
    }

    /// Sets new time position of the animation. The actual time position the animation will have after the call,
    /// can be different in two reasons:
    ///
    /// - If the animation is looping and the new time position is outside of the time slice of the animation, then
    /// the actual time position will be wrapped to fit the time slice. For example, if you have an animation that has
    /// `0.0..5.0s` time slice and you trying to set `7.5s` position, the actual time position will be `2.5s` (it
    /// wraps the input value on the given time slice).
    /// - If the animation is **not** looping and the new time position is outside of the time slice of the animation,
    /// then the actual time position will be clamped to the time clice of the animation.
    pub fn set_time_position(&mut self, time: f32) -> &mut Self {
        if self.looped {
            self.time_position = wrapf(time, self.time_slice.start, self.time_slice.end);
        } else {
            self.time_position = time.clamp(self.time_slice.start, self.time_slice.end);
        }

        self
    }

    /// Sets new time slice of the animation in seconds. It defines a time interval in which the animation will
    /// be played. Current playback position will be clamped (or wrapped if the animation is looping) to fit to new
    /// bounds.
    pub fn set_time_slice(&mut self, time_slice: Range<f32>) {
        assert!(time_slice.start <= time_slice.end);

        self.time_slice = time_slice;

        // Ensure time position is in given time slice.
        self.set_time_position(self.time_position);
    }

    /// Returns current time slice of the animation.
    pub fn time_slice(&self) -> Range<f32> {
        self.time_slice.clone()
    }

    /// Rewinds the animation to the beginning.
    pub fn rewind(&mut self) -> &mut Self {
        self.set_time_position(self.time_slice.start)
    }

    /// Returns length of the animation in seconds.
    pub fn length(&self) -> f32 {
        self.time_slice.end - self.time_slice.start
    }

    /// Performs a single update tick and calculates an output pose. This method is low level, you should not use it
    /// in normal circumstances - the engine will call it for you.
    pub fn tick(&mut self, dt: f32) {
        self.update_pose();

        let current_time_position = self.time_position();
        let new_time_position = current_time_position + dt * self.speed();

        for signal in self.signals.iter_mut().filter(|s| s.enabled) {
            if self.speed >= 0.0
                && (current_time_position < signal.time && new_time_position >= signal.time)
                || self.speed < 0.0
                    && (current_time_position > signal.time && new_time_position <= signal.time)
                    && self.events.len() < self.max_event_capacity
            {
                self.events.push_back(AnimationEvent {
                    signal_id: signal.id,
                    name: signal.name.clone(),
                });
            }
        }

        let prev_time_position = current_time_position;

        self.set_time_position(new_time_position);

        self.update_root_motion(prev_time_position);
    }

    fn update_root_motion(&mut self, prev_time_position: f32) {
        let state = self.tracks_data.state();
        let Some(tracks_data) = state.data_ref() else {
            return;
        };
        let tracks = &tracks_data.tracks;

        fn fetch_position_at_time(tracks: &[Track], time: f32) -> Vector3<f32> {
            tracks
                .iter()
                .find(|track| track.value_binding() == &ValueBinding::Position)
                .and_then(|track| track.fetch(time))
                .and_then(|value| {
                    if let TrackValue::Vector3(position) = value.value {
                        Some(position)
                    } else {
                        None
                    }
                })
                .unwrap_or_default()
        }

        fn fetch_rotation_at_time(tracks: &[Track], time: f32) -> UnitQuaternion<f32> {
            tracks
                .iter()
                .find(|track| track.value_binding() == &ValueBinding::Rotation)
                .and_then(|track| track.fetch(time))
                .and_then(|value| {
                    if let TrackValue::UnitQuaternion(rotation) = value.value {
                        Some(rotation)
                    } else {
                        None
                    }
                })
                .unwrap_or_default()
        }

        // If we have root motion enabled, try to extract the actual motion values. We'll take only relative motion
        // here, relative to the previous values.
        if let Some(root_motion_settings) = self.root_motion_settings.as_ref() {
            let mut prev_root_motion = self.root_motion.clone().unwrap_or_default();

            // Check if we've started another loop cycle.
            let new_loop_cycle_started = self.looped
                && (self.speed > 0.0 && self.time_position < prev_time_position
                    || self.speed < 0.0 && self.time_position > prev_time_position);

            let cycle_start_time = if self.speed > 0.0 {
                self.time_slice.start
            } else {
                self.time_slice.end
            };

            let cycle_end_time = if self.speed > 0.0 {
                self.time_slice.end
            } else {
                self.time_slice.start
            };

            let mut root_motion = RootMotion::default();
            if let Some(root_pose) = self.pose.poses_mut().get_mut(&root_motion_settings.node) {
                for bound_value in root_pose.values.values.iter_mut() {
                    match bound_value.binding {
                        ValueBinding::Position => {
                            if let TrackValue::Vector3(pose_position) = bound_value.value {
                                if new_loop_cycle_started {
                                    root_motion.prev_position =
                                        fetch_position_at_time(tracks, cycle_start_time);
                                    root_motion.position_offset_remainder = Some(
                                        fetch_position_at_time(tracks, cycle_end_time)
                                            - pose_position,
                                    );
                                } else {
                                    root_motion.prev_position = pose_position;
                                }

                                let remainder = prev_root_motion
                                    .position_offset_remainder
                                    .take()
                                    .unwrap_or_default();
                                let current_offset = pose_position - prev_root_motion.prev_position;
                                let delta = current_offset + remainder;

                                root_motion.delta_position.x =
                                    if root_motion_settings.ignore_x_movement {
                                        0.0
                                    } else {
                                        delta.x
                                    };
                                root_motion.delta_position.y =
                                    if root_motion_settings.ignore_y_movement {
                                        0.0
                                    } else {
                                        delta.y
                                    };
                                root_motion.delta_position.z =
                                    if root_motion_settings.ignore_z_movement {
                                        0.0
                                    } else {
                                        delta.z
                                    };

                                // Reset position so the root won't move.
                                let start_position =
                                    fetch_position_at_time(tracks, self.time_slice.start);

                                bound_value.value = TrackValue::Vector3(Vector3::new(
                                    if root_motion_settings.ignore_x_movement {
                                        pose_position.x
                                    } else {
                                        start_position.x
                                    },
                                    if root_motion_settings.ignore_y_movement {
                                        pose_position.y
                                    } else {
                                        start_position.y
                                    },
                                    if root_motion_settings.ignore_z_movement {
                                        pose_position.z
                                    } else {
                                        start_position.z
                                    },
                                ));
                            }
                        }
                        ValueBinding::Rotation => {
                            if let TrackValue::UnitQuaternion(pose_rotation) = bound_value.value {
                                if !root_motion_settings.ignore_rotations {
                                    if new_loop_cycle_started {
                                        root_motion.prev_rotation =
                                            fetch_rotation_at_time(tracks, cycle_start_time);
                                        root_motion.rotation_remainder = Some(
                                            fetch_rotation_at_time(tracks, cycle_end_time)
                                                .inverse()
                                                * pose_rotation,
                                        );
                                    } else {
                                        root_motion.prev_rotation = pose_rotation;
                                    }

                                    // Compute relative rotation that can be used to "turn" a node later on.
                                    let remainder = prev_root_motion
                                        .rotation_remainder
                                        .take()
                                        .unwrap_or_else(UnitQuaternion::identity);
                                    let current_relative_rotation =
                                        prev_root_motion.prev_rotation.inverse() * pose_rotation;
                                    root_motion.delta_rotation =
                                        remainder * current_relative_rotation;

                                    // Reset rotation so the root won't rotate.
                                    bound_value.value = TrackValue::UnitQuaternion(
                                        fetch_rotation_at_time(tracks, self.time_slice.start),
                                    );
                                }
                            }
                        }
                        _ => (),
                    }
                }
            }
            self.root_motion = Some(root_motion);
        }
    }

    /// Sets new root motion settings.
    pub fn set_root_motion_settings(&mut self, settings: Option<RootMotionSettings<T>>) {
        self.root_motion_settings = settings;
    }

    /// Returns a reference to the root motion settings (if any).
    pub fn root_motion_settings_ref(&self) -> Option<&RootMotionSettings<T>> {
        self.root_motion_settings.as_ref()
    }

    /// Returns a reference to the root motion settings (if any).
    pub fn root_motion_settings_mut(&mut self) -> Option<&mut RootMotionSettings<T>> {
        self.root_motion_settings.as_mut()
    }

    /// Returns a reference to the root motion (if any).
    pub fn root_motion(&self) -> Option<&RootMotion> {
        self.root_motion.as_ref()
    }

    /// Extracts a first event from the events queue of the animation.
    pub fn pop_event(&mut self) -> Option<AnimationEvent> {
        self.events.pop_front()
    }

    /// Returns a reference to inner events queue. It is useful when you need to iterate over the events, but
    /// don't extract them from the queue.
    pub fn events_ref(&self) -> &VecDeque<AnimationEvent> {
        &self.events
    }

    /// Return a mutable reference to inner events queue. Provides you a full controls over animation events,
    /// you can even manually inject events in the queue.
    pub fn events_mut(&mut self) -> &mut VecDeque<AnimationEvent> {
        &mut self.events
    }

    /// Takes the events queue and returns it to the caller, leaving the internal queue empty.
    pub fn take_events(&mut self) -> VecDeque<AnimationEvent> {
        std::mem::take(&mut self.events)
    }

    /// Returns current time position of the animation. The time position is guaranteed to be in the range of
    /// current time slice of the animation.
    pub fn time_position(&self) -> f32 {
        self.time_position
    }

    /// Sets new speed multiplier for the animation. By default it is set to 1.0. Negative values can be used
    /// to play the animation in reverse.
    pub fn set_speed(&mut self, speed: f32) -> &mut Self {
        self.speed = speed;
        self
    }

    /// Returns speed multiplier of the animation.
    pub fn speed(&self) -> f32 {
        self.speed
    }

    /// Enables or disables looping of the animation.
    pub fn set_loop(&mut self, state: bool) -> &mut Self {
        self.looped = state;
        self
    }

    /// Returns `true` if the animation is looping, `false` - otherwise.
    pub fn is_loop(&self) -> bool {
        self.looped
    }

    /// Returns `true` if the animation was played until the end of current time slice of the animation, `false` -
    /// otherwise. Looping animations will always return `false`.
    pub fn has_ended(&self) -> bool {
        !self.looped && (self.time_position - self.time_slice.end).abs() <= f32::EPSILON
    }

    /// Enables or disables the animation, disabled animations does not updated and their output pose will remain
    /// the same. By default every animation is enabled.
    pub fn set_enabled(&mut self, enabled: bool) -> &mut Self {
        self.enabled = enabled;
        self
    }

    /// Returns `true` if the animation is enabled, `false` - otherwise.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Adds a new track to the animation track data container and binds it with the specified target.
    /// Keep in mind, that this method will modify potentially shared track data container, which might
    /// affect other animations using it.
    pub fn add_track_with_binding(&mut self, binding: TrackBinding<T>, track: Track) {
        let mut state = self.tracks_data.state();
        let Some(tracks_data) = state.data() else {
            return;
        };
        let id = track.id();
        tracks_data.tracks.push(track);
        self.track_bindings.insert(id, binding);
    }

    /// Removes last track from the current tracks data resource and the respective binding to it
    /// from the animation. This method will fail if the resource is not loaded, or if there's no
    /// tracks in it. It will also fail if there's no respective binding to the track in the
    /// animation.
    ///
    /// Keep in mind, that this method modifies the tracks data resource, which might be used by
    /// some other animation.
    pub fn pop_track_with_binding(&mut self) -> Option<(TrackBinding<T>, Track)> {
        let mut state = self.tracks_data.state();
        let tracks_data = state.data()?;
        let track = tracks_data.tracks.pop()?;
        let binding = self.track_bindings.remove(&track.id())?;
        Some((binding, track))
    }

    /// Removes the specified track from the current tracks data resource and the respective binding
    /// to it from the animation. This method will fail if the resource is not loaded, or if there's
    /// no tracks in it. It will also fail if there's no respective binding to the track in the
    /// animation.
    ///
    /// Keep in mind, that this method modifies the tracks data resource, which might be used by
    /// some other animation.
    pub fn remove_track_with_binding(&mut self, index: usize) -> Option<(TrackBinding<T>, Track)> {
        let mut state = self.tracks_data.state();
        let tracks_data = state.data()?;
        let track = tracks_data.tracks.remove(index);
        let binding = self.track_bindings.remove(&track.id())?;
        Some((binding, track))
    }

    /// Inserts a new track in the tracks data resource and creates a new binding to it.
    ///
    /// Keep in mind, that this method modifies the tracks data resource, which might be used by
    /// some other animation.
    pub fn insert_track_with_binding(
        &mut self,
        index: usize,
        binding: TrackBinding<T>,
        track: Track,
    ) {
        let mut state = self.tracks_data.state();
        let Some(tracks_data) = state.data() else {
            return;
        };
        assert!(self.track_bindings.insert(track.id(), binding).is_none());
        tracks_data.tracks.insert(index, track);
    }

    /// Adds a new animation signal to the animation. See [`AnimationSignal`] docs for more info and examples.
    pub fn add_signal(&mut self, signal: AnimationSignal) -> &mut Self {
        self.signals.push(signal);
        self
    }

    /// Removes last animation signal from the container of the animation.
    pub fn pop_signal(&mut self) -> Option<AnimationSignal> {
        self.signals.pop()
    }

    /// Inserts a new animation signal at given position.
    pub fn insert_signal(&mut self, index: usize, signal: AnimationSignal) {
        self.signals.insert(index, signal)
    }

    /// Removes an animation signal at given index.
    pub fn remove_signal(&mut self, index: usize) -> AnimationSignal {
        self.signals.remove(index)
    }

    /// Returns a reference to the animation signals container.
    pub fn signals(&self) -> &[AnimationSignal] {
        &self.signals
    }

    /// Returns a mutable reference to the inner animation signals container, allowing you to modify the signals.
    pub fn signals_mut(&mut self) -> &mut [AnimationSignal] {
        &mut self.signals
    }

    /// Tries to find all tracks that refer to a given node and enables or disables them.
    pub fn set_node_track_enabled(&mut self, handle: T, enabled: bool) {
        for track in self.track_bindings.values_mut() {
            if track.target() == handle {
                track.set_enabled(enabled);
            }
        }
    }

    /// Returns a reference to the current set of track bindings used by the animation. The returned
    /// hash map contains `(track_id -> binding)` pairs.
    pub fn track_bindings(&self) -> &FxHashMap<Uuid, TrackBinding<T>> {
        &self.track_bindings
    }

    /// Returns a reference to the current set of track bindings used by the animation. The returned
    /// hash map contains `(track_id -> binding)` pairs.
    pub fn track_bindings_mut(&mut self) -> &mut FxHashMap<Uuid, TrackBinding<T>> {
        &mut self.track_bindings
    }

    /// Tries to find a layer by its name. Returns index of the signal and its reference.
    #[inline]
    pub fn find_signal_by_name_ref<S: AsRef<str>>(
        &self,
        name: S,
    ) -> Option<(usize, &AnimationSignal)> {
        core::find_by_name_ref(self.signals.iter().enumerate(), name)
    }

    /// Tries to find a signal by its name. Returns index of the signal and its reference.
    #[inline]
    pub fn find_signal_by_name_mut<S: AsRef<str>>(
        &mut self,
        name: S,
    ) -> Option<(usize, &mut AnimationSignal)> {
        core::find_by_name_mut(self.signals.iter_mut().enumerate(), name)
    }

    /// Returns `true` if there's a signal with given name and id.
    #[inline]
    pub fn has_signal<S: AsRef<str>>(&self, name: S, id: Uuid) -> bool {
        self.find_signal_by_name_ref(name)
            .is_some_and(|(_, s)| s.id == id)
    }

    /// Removes all tracks from the animation.
    pub fn remove_tracks(&mut self) {
        self.track_bindings.clear();
    }

    fn update_pose(&mut self) {
        let state = self.tracks_data.state();
        let Some(tracks_data) = state.data_ref() else {
            return;
        };

        self.pose.reset();
        for track in tracks_data.tracks.iter() {
            let Some(binding) = self.track_bindings.get(&track.id()) else {
                continue;
            };

            if binding.is_enabled() {
                if let Some(bound_value) = track.fetch(self.time_position) {
                    self.pose.add_to_node_pose(binding.target(), bound_value);
                }
            }
        }
    }

    /// Returns current pose of the animation (a final result that can be applied to a scene graph).
    pub fn pose(&self) -> &AnimationPose<T> {
        &self.pose
    }
}

impl<T: EntityId> Default for Animation<T> {
    fn default() -> Self {
        Self {
            name: Default::default(),
            tracks_data: Resource::new_ok(
                Uuid::default(),
                ResourceKind::Embedded,
                AnimationTracksData::default(),
            ),
            speed: 1.0,
            time_position: 0.0,
            enabled: true,
            looped: true,
            pose: Default::default(),
            signals: Default::default(),
            root_motion_settings: None,
            events: Default::default(),
            time_slice: Default::default(),
            root_motion: None,
            max_event_capacity: 32,
            track_bindings: Default::default(),
        }
    }
}

/// A container for animations. It is a tiny wrapper around [`Pool`], you should never create the container yourself,
/// it is managed by the engine.
#[derive(Debug, Clone, Reflect, PartialEq)]
pub struct AnimationContainer<T: EntityId> {
    pool: Pool<Animation<T>>,
}

impl<T: EntityId> Default for AnimationContainer<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: EntityId> AnimationContainer<T> {
    /// Creates an empty animation container.
    pub fn new() -> Self {
        Self { pool: Pool::new() }
    }

    /// Returns a total amount of animations in the container.
    #[inline]
    pub fn alive_count(&self) -> u32 {
        self.pool.alive_count()
    }

    /// Returns an iterator yielding a references to animations in the container.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &Animation<T>> {
        self.pool.iter()
    }

    /// Returns an iterator yielding a pair (handle, reference) to animations in the container.
    #[inline]
    pub fn pair_iter(&self) -> impl Iterator<Item = (Handle<Animation<T>>, &Animation<T>)> {
        self.pool.pair_iter()
    }

    /// Returns an iterator yielding a pair (handle, reference) to animations in the container.
    #[inline]
    pub fn pair_iter_mut(
        &mut self,
    ) -> impl Iterator<Item = (Handle<Animation<T>>, &mut Animation<T>)> {
        self.pool.pair_iter_mut()
    }

    /// Returns an iterator yielding a references to animations in the container.
    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Animation<T>> {
        self.pool.iter_mut()
    }

    /// Adds a new animation to the container and returns its handle.
    #[inline]
    pub fn add(&mut self, animation: Animation<T>) -> Handle<Animation<T>> {
        self.pool.spawn(animation)
    }

    /// Tries to remove an animation from the container by its handle.
    #[inline]
    pub fn remove(&mut self, handle: Handle<Animation<T>>) -> Option<Animation<T>> {
        self.pool.try_free(handle)
    }

    /// Extracts animation from container and reserves its handle. It is used to temporarily take
    /// ownership over animation, and then put animation back using given ticket.
    pub fn take_reserve(
        &mut self,
        handle: Handle<Animation<T>>,
    ) -> (Ticket<Animation<T>>, Animation<T>) {
        self.pool.take_reserve(handle)
    }

    /// Puts animation back by given ticket.
    pub fn put_back(
        &mut self,
        ticket: Ticket<Animation<T>>,
        animation: Animation<T>,
    ) -> Handle<Animation<T>> {
        self.pool.put_back(ticket, animation)
    }

    /// Makes animation handle vacant again.
    pub fn forget_ticket(&mut self, ticket: Ticket<Animation<T>>) {
        self.pool.forget_ticket(ticket)
    }

    /// Removes all animations.
    #[inline]
    pub fn clear(&mut self) {
        self.pool.clear()
    }

    /// Tries to borrow a reference to an animation in the container. Panics if the handle is invalid.
    #[inline]
    pub fn get(&self, handle: Handle<Animation<T>>) -> &Animation<T> {
        self.pool.borrow(handle)
    }

    /// Tries to borrow a mutable reference to an animation in the container. Panics if the handle is invalid.
    #[inline]
    pub fn get_mut(&mut self, handle: Handle<Animation<T>>) -> &mut Animation<T> {
        self.pool.borrow_mut(handle)
    }

    /// Tries to borrow a reference to an animation in the container.
    #[inline]
    pub fn try_get(&self, handle: Handle<Animation<T>>) -> Option<&Animation<T>> {
        self.pool.try_borrow(handle)
    }

    /// Tries to borrow a mutable reference to an animation in the container.
    #[inline]
    pub fn try_get_mut(&mut self, handle: Handle<Animation<T>>) -> Option<&mut Animation<T>> {
        self.pool.try_borrow_mut(handle)
    }

    /// Tries to find an animation by its name in the container.
    #[inline]
    pub fn find_by_name_ref<S: AsRef<str>>(
        &self,
        name: S,
    ) -> Option<(Handle<Animation<T>>, &Animation<T>)> {
        core::find_by_name_ref(self.pool.pair_iter(), name)
    }

    /// Tries to find an animation by its name in the container.
    #[inline]
    pub fn find_by_name_mut<S: AsRef<str>>(
        &mut self,
        name: S,
    ) -> Option<(Handle<Animation<T>>, &mut Animation<T>)> {
        core::find_by_name_mut(self.pool.pair_iter_mut(), name)
    }

    /// Removes every animation from the container that does not satisfy a particular condition represented by the given
    /// closue.
    #[inline]
    pub fn retain<P>(&mut self, pred: P)
    where
        P: FnMut(&Animation<T>) -> bool,
    {
        self.pool.retain(pred)
    }

    /// Removes queued animation events from every animation in the container.
    ///
    /// # Potential use cases
    ///
    /// Sometimes there is a need to use animation events only from one frame, in this case you should clear events each frame.
    /// This situation might come up when you have multiple animations with signals, but at each frame not every event gets
    /// processed. This might result in unwanted side effects, like multiple attack events may result in huge damage in a single
    /// frame.
    pub fn clear_animation_events(&mut self) {
        for animation in self.pool.iter_mut() {
            animation.events.clear();
        }
    }
}

impl<T: EntityId> Visit for AnimationContainer<T> {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        if visitor.is_reading() && self.pool.get_capacity() != 0 {
            panic!("Animation pool must be empty on load!");
        }

        let mut region = visitor.enter_region(name)?;

        self.pool.visit("Pool", &mut region)?;

        Ok(())
    }
}

impl<T: EntityId> Index<Handle<Animation<T>>> for AnimationContainer<T> {
    type Output = Animation<T>;

    fn index(&self, index: Handle<Animation<T>>) -> &Self::Output {
        &self.pool[index]
    }
}

impl<T: EntityId> IndexMut<Handle<Animation<T>>> for AnimationContainer<T> {
    fn index_mut(&mut self, index: Handle<Animation<T>>) -> &mut Self::Output {
        &mut self.pool[index]
    }
}
