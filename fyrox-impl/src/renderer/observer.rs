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

#![allow(missing_docs)] // TODO

use crate::{
    core::{
        algebra::{Matrix4, Point3, Vector2, Vector3},
        math::{frustum::Frustum, Rect},
        pool::Handle,
    },
    graphics::gpu_texture::CubeMapFace,
    renderer::utils::CubeMapFaceDescriptor,
    scene::{
        camera::{Camera, ColorGradingLut, Exposure, PerspectiveProjection, Projection},
        collider::BitMask,
        node::Node,
        probe::ReflectionProbe,
        Scene,
    },
};
use fyrox_texture::TextureResource;

/// Observer position contains all the data, that describes an observer position in 3D space. It
/// could be a real camera, light source's "virtual camera" that is used for shadow mapping, etc.
#[derive(Clone, Default)]
pub struct ObserverPosition {
    /// World-space position of the observer.
    pub translation: Vector3<f32>,
    /// Position of the near clipping plane.
    pub z_near: f32,
    /// Position of the far clipping plane.
    pub z_far: f32,
    /// The view matrix of the observer.
    pub view_matrix: Matrix4<f32>,
    /// Projection matrix of the observer.
    pub projection_matrix: Matrix4<f32>,
    pub view_projection_matrix: Matrix4<f32>,
}

impl ObserverPosition {
    pub fn from_camera(camera: &Camera) -> Self {
        Self {
            translation: camera.global_position(),
            z_near: camera.projection().z_near(),
            z_far: camera.projection().z_far(),
            view_matrix: camera.view_matrix(),
            projection_matrix: camera.projection_matrix(),
            view_projection_matrix: camera.view_projection_matrix(),
        }
    }
}

/// Collections of observers in a scene.
#[derive(Default)]
pub struct ObserversCollection {
    /// Camera observers.
    pub cameras: Vec<Observer>,
    /// Reflection probes, rendered first.
    pub reflection_probes: Vec<Observer>,
}

impl ObserversCollection {
    pub fn from_scene(scene: &Scene, frame_size: Vector2<f32>) -> Self {
        let mut observers = Self::default();
        for node in scene.graph.linear_iter() {
            if node.is_globally_enabled() {
                if let Some(camera) = node.cast::<Camera>() {
                    if camera.is_enabled() {
                        observers
                            .cameras
                            .push(Observer::from_camera(camera, frame_size));
                    }
                } else if let Some(probe) = node.cast::<ReflectionProbe>() {
                    if probe.updated.get() {
                        continue;
                    }
                    probe.updated.set(true);

                    let projection = Projection::Perspective(PerspectiveProjection {
                        fov: 90.0f32.to_radians(),
                        z_near: *probe.z_near,
                        z_far: *probe.z_far,
                    });
                    let resolution = probe.resolution() as f32;
                    let cube_size = Vector2::repeat(probe.resolution() as f32);
                    let projection_matrix = projection.matrix(cube_size);

                    for cube_face in CubeMapFaceDescriptor::cube_faces() {
                        let translation = probe.global_rendering_position();
                        let view_matrix = Matrix4::look_at_rh(
                            &Point3::from(translation),
                            &Point3::from(translation + cube_face.look),
                            &cube_face.up,
                        );
                        let view_projection_matrix = projection_matrix * view_matrix;
                        observers.reflection_probes.push(Observer {
                            handle: node.handle(),
                            cube_map_face: Some(cube_face.face),
                            render_target: Some(probe.render_target().clone()),
                            position: ObserverPosition {
                                translation,
                                z_near: *probe.z_near,
                                z_far: *probe.z_far,
                                view_matrix,
                                projection_matrix,
                                view_projection_matrix,
                            },
                            environment_map: None,
                            render_mask: *probe.render_mask,
                            projection: projection.clone(),
                            color_grading_lut: None,
                            color_grading_enabled: false,
                            exposure: Default::default(),
                            viewport: Rect::new(0, 0, resolution as i32, resolution as i32),
                            frustum: Frustum::from_view_projection_matrix(view_projection_matrix)
                                .unwrap_or_default(),
                        })
                    }
                }
            }
        }
        observers
    }
}

/// An Observer holds all the information required to render a scene from a particular point of view,
/// including everything except the actual scene to be rendered.
/// Observers are naturally constructed using the data from some camera in the scene.
pub struct Observer {
    /// The handle of the camera that was used to create this Observer.
    pub handle: Handle<Node>,
    pub cube_map_face: Option<CubeMapFace>,
    pub render_target: Option<TextureResource>,
    pub position: ObserverPosition,
    pub environment_map: Option<TextureResource>,
    pub render_mask: BitMask,
    pub projection: Projection,
    pub color_grading_lut: Option<ColorGradingLut>,
    pub color_grading_enabled: bool,
    pub exposure: Exposure,
    pub viewport: Rect<i32>,
    pub frustum: Frustum,
}

impl Observer {
    pub fn from_camera(camera: &Camera, mut frame_size: Vector2<f32>) -> Self {
        if let Some(render_target) = camera.render_target() {
            if let Some(size) = render_target
                .data_ref()
                .as_loaded_ref()
                .and_then(|rt| rt.kind().rectangle_size().map(|size| size.cast::<f32>()))
            {
                frame_size = size;
            }
        }
        Observer {
            handle: camera.handle(),
            environment_map: camera.environment_map(),
            render_mask: *camera.render_mask,
            projection: camera.projection().clone(),
            position: ObserverPosition::from_camera(camera),
            render_target: camera.render_target().cloned(),
            color_grading_lut: camera.color_grading_lut(),
            color_grading_enabled: camera.color_grading_enabled(),
            exposure: camera.exposure(),
            viewport: camera.viewport_pixels(frame_size),
            frustum: camera.frustum(),
            cube_map_face: None,
        }
    }
}
