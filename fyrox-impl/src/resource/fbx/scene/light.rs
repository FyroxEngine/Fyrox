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

use crate::{
    core::{
        color::Color,
        log::{Log, MessageKind},
        pool::Handle,
    },
    resource::fbx::document::{FbxNode, FbxNodeContainer},
    scene::{
        base::BaseBuilder,
        graph::Graph,
        light::{
            directional::DirectionalLightBuilder, point::PointLightBuilder, spot::SpotLightBuilder,
            BaseLightBuilder,
        },
        node::Node,
    },
};

pub enum FbxLightType {
    Point = 0,
    Directional = 1,
    Spot = 2,
    Area = 3,
    Volume = 4,
}

pub struct FbxLight {
    actual_type: FbxLightType,
    color: Color,
    radius: f32,
    hotspot_cone_angle: f32,
    falloff_cone_angle_delta: f32,
}

impl FbxLight {
    pub(in crate::resource::fbx) fn read(
        light_node_handle: Handle<FbxNode>,
        nodes: &FbxNodeContainer,
    ) -> Result<Self, String> {
        let mut light = Self {
            actual_type: FbxLightType::Point,
            color: Color::WHITE,
            radius: 10.0,
            hotspot_cone_angle: 90.0f32.to_radians(),
            falloff_cone_angle_delta: 5.0f32.to_radians(),
        };

        let props = nodes.get_by_name(light_node_handle, "Properties70")?;
        for prop_handle in props.children() {
            let prop = nodes.get(*prop_handle);
            match prop.get_attrib(0)?.as_string().as_str() {
                "DecayStart" => light.radius = prop.get_attrib(4)?.as_f64()? as f32,
                "Color" => {
                    let r = (prop.get_attrib(4)?.as_f64()? * 255.0) as u8;
                    let g = (prop.get_attrib(5)?.as_f64()? * 255.0) as u8;
                    let b = (prop.get_attrib(6)?.as_f64()? * 255.0) as u8;
                    light.color = Color::from_rgba(r, g, b, 255);
                }
                "HotSpot" => {
                    light.hotspot_cone_angle = (prop.get_attrib(4)?.as_f64()? as f32).to_radians();
                }
                "Cone angle" => {
                    light.falloff_cone_angle_delta = (prop.get_attrib(4)?.as_f64()? as f32)
                        .to_radians()
                        - light.hotspot_cone_angle;
                }
                "LightType" => {
                    let type_code = prop.get_attrib(4)?.as_i32()?;
                    light.actual_type = match type_code {
                        0 => FbxLightType::Point,
                        1 => FbxLightType::Directional,
                        2 => FbxLightType::Spot,
                        3 => FbxLightType::Area,
                        4 => FbxLightType::Volume,
                        _ => {
                            Log::writeln(
                                MessageKind::Warning,
                                format!("FBX: Unknown light type {type_code}, fallback to Point!"),
                            );
                            FbxLightType::Point
                        }
                    };
                }
                _ => (),
            }
        }

        Ok(light)
    }

    pub fn convert(&self, base: BaseBuilder, graph: &mut Graph) -> Handle<Node> {
        match self.actual_type {
            FbxLightType::Point | FbxLightType::Area | FbxLightType::Volume => {
                PointLightBuilder::new(
                    BaseLightBuilder::new(base).with_color(self.color.to_opaque()),
                )
                .with_radius(self.radius)
                .build(graph)
            }
            FbxLightType::Spot => SpotLightBuilder::new(
                BaseLightBuilder::new(base).with_color(self.color.to_opaque()),
            )
            .with_distance(self.radius)
            .with_hotspot_cone_angle(self.hotspot_cone_angle)
            .with_falloff_angle_delta(self.falloff_cone_angle_delta)
            .build(graph),

            FbxLightType::Directional => DirectionalLightBuilder::new(
                BaseLightBuilder::new(base).with_color(self.color.to_opaque()),
            )
            .build(graph),
        }
    }
}
