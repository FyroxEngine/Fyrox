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

//! A kinematic character controller for player/NPC movement (walking, climbing, sliding).
//! See [`KinematicCharacterController`] docs for more info.

use crate::{
    core::{
        algebra::Vector2, num_traits::FloatConst, pool::Handle, reflect::prelude::*,
        visitor::prelude::*,
    },
    scene::{
        dim2::{
            self,
            physics::{filter_by_predicate, QueryFilter},
        },
        graph::{
            physics::character::{CharacterAutostep, CharacterLength},
            Graph,
        },
    },
};
use fyrox_graph::SceneGraph;
use rapier2d::{
    geometry::{Collider, ColliderHandle, Shape},
    math::Pose,
    na::Isometry2,
};

/// A kinematic character controller for player/NPC movement (walking, climbing, sliding).
///
/// This provides classic game character movement: walking on floors, sliding on slopes,
/// climbing stairs, and snapping to ground. It's kinematic (not physics-based), meaning
/// you control movement directly rather than applying forces.
///
/// TODO: Example
#[derive(Visit, Reflect, Debug, Clone)]
#[reflect(type_uuid = "42995595-7e06-41b5-b8fc-8aa9b7feaf36", non_comparable)]
pub struct KinematicCharacterController {
    /// The direction that goes "up". Used to determine where the floor is, and the floor’s angle.
    pub up: Vector2<f32>,
    /// A small gap to preserve between the character and its surroundings.
    ///
    /// This value should not be too large to avoid visual artifacts, but shouldn’t be too small
    /// (must not be zero) to improve numerical stability of the character controller.
    pub offset: CharacterLength,
    /// Should the character try to slide against the floor if it hits it?
    pub slide: bool,
    /// Should the character automatically step over small obstacles? (disabled by default)
    ///
    /// Note that autostepping is currently a very computationally expensive feature, so it
    /// is disabled by default.
    pub autostep: Option<CharacterAutostep>,
    /// The maximum angle (radians) between the floor’s normal and the `up` vector that the
    /// character is able to climb.
    pub max_slope_climb_angle: f32,
    /// The minimum angle (radians) between the floor’s normal and the `up` vector before the
    /// character starts to slide down automatically.
    pub min_slope_slide_angle: f32,
    /// Should the character be automatically snapped to the ground if the distance between
    /// the ground and its feed are smaller than the specified threshold?
    pub snap_to_ground: Option<CharacterLength>,
    /// Increase this number if your character appears to get stuck when sliding against surfaces.
    ///
    /// This is a small distance applied to the movement toward the contact normals of shapes hit
    /// by the character controller. This helps shape-casting not getting stuck in an always-penetrating
    /// state during the sliding calculation.
    ///
    /// This value should remain fairly small since it can introduce artificial "bumps" when sliding
    /// along a flat surface.
    pub normal_nudge_factor: f32,
}

/// The effective movement computed by the character controller.
#[derive(Debug)]
pub struct CharacterMovement {
    /// The movement to apply.
    pub translation: Vector2<f32>,
    /// Is the character touching the ground after applying `EffectiveKinematicMovement::translation`?
    pub grounded: bool,
    /// Is the character sliding down a slope due to slope angle being larger than `min_slope_slide_angle`?
    pub is_sliding_down_slope: bool,
}

impl Default for KinematicCharacterController {
    fn default() -> Self {
        Self {
            up: Vector2::y(),
            offset: CharacterLength::Relative(0.01),
            slide: true,
            autostep: None,
            max_slope_climb_angle: f32::FRAC_PI_4(),
            min_slope_slide_angle: f32::FRAC_PI_4(),
            snap_to_ground: Some(CharacterLength::Relative(0.2)),
            normal_nudge_factor: 1.0e-4,
        }
    }
}

impl KinematicCharacterController {
    /// Computes the possible movement for a shape.
    pub fn move_shape(
        &self,
        dt: f32,
        shape: &dyn Shape,
        position: Isometry2<f32>,
        desired_translation: Vector2<f32>,
        graph: &Graph,
        filter: QueryFilter,
    ) -> CharacterMovement {
        let controller = rapier2d::control::KinematicCharacterController {
            up: self.up.into(),
            offset: self.offset.into(),
            slide: self.slide,
            autostep: self.autostep.map(Into::into),
            max_slope_climb_angle: self.max_slope_climb_angle,
            min_slope_slide_angle: self.min_slope_slide_angle,
            snap_to_ground: self.snap_to_ground.map(Into::into),
            normal_nudge_factor: self.normal_nudge_factor,
        };

        let predicate = |handle: ColliderHandle, _: &Collider| -> bool {
            filter_by_predicate(filter.predicate, handle, graph, &graph.physics2d.colliders)
        };
        let filter = filter.to_native(graph, &predicate);
        let query_pipeline = graph.physics2d.query_pipeline(filter);
        let movement = controller.move_shape(
            dt,
            &query_pipeline,
            shape,
            &Pose::from(position),
            desired_translation.into(),
            |_| {},
        );
        CharacterMovement {
            translation: movement.translation.into(),
            grounded: movement.grounded,
            is_sliding_down_slope: movement.is_sliding_down_slope,
        }
    }

    /// Same as [`Self::move_shape`], but tries to use the shape of the specified collider. Can
    /// return [`None`] if the `collider_handle` is invalid.
    pub fn move_collider_shape(
        &self,
        dt: f32,
        collider_handle: Handle<dim2::collider::Collider>,
        position: Isometry2<f32>,
        desired_translation: Vector2<f32>,
        graph: &Graph,
        filter: QueryFilter,
    ) -> Option<CharacterMovement> {
        let collider = graph
            .physics2d
            .colliders
            .get(graph.try_get(collider_handle).ok()?.native.get())?;
        Some(self.move_shape(
            dt,
            collider.shape(),
            position,
            desired_translation,
            graph,
            filter,
        ))
    }
}
