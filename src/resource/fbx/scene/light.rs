use crate::{
    core::{
        pool::Handle,
        color::Color,
    },
    resource::fbx::{
        document::{
            FbxNodeContainer,
            FbxNode
        }
    },
    utils::log::Log,
    scene::light::{
        LightKind,
        PointLight,
        SpotLight,
        Light
    }
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
    pub(in crate::resource::fbx) fn read(light_node_handle: Handle<FbxNode>, nodes: &FbxNodeContainer) -> Result<Self, String> {
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
                    light.falloff_cone_angle_delta = (prop.get_attrib(4)?.as_f64()? as f32).to_radians() - light.hotspot_cone_angle;
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
                            Log::writeln(format!("FBX: Unknown light type {}, fallback to Point!", type_code));
                            FbxLightType::Point
                        }
                    };
                }
                _ => ()
            }
        }

        Ok(light)
    }

    pub fn convert(&self) -> Light {
        let light_kind = match self.actual_type {
            FbxLightType::Point | FbxLightType::Directional | FbxLightType::Area | FbxLightType::Volume => {
                LightKind::Point(PointLight::new(self.radius))
            }
            FbxLightType::Spot => {
                LightKind::Spot(SpotLight::new(self.radius, self.hotspot_cone_angle, self.falloff_cone_angle_delta))
            }
        };

        let mut light = Light::new(light_kind);

        light.set_color(Color::opaque(self.color.r, self.color.g, self.color.b));

        light
    }
}