use fyrox::renderer::CsmSettings;
use fyrox::{
    core::{
        inspect::{Inspect, PropertyInfo},
        reflect::Reflect,
    },
    gui::inspector::{FieldKind, PropertyChanged},
    renderer::QualitySettings,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Debug, Clone, Inspect, Reflect)]
pub struct GraphicsSettings {
    pub quality: QualitySettings,
    pub z_near: f32,
    pub z_far: f32,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            quality: Default::default(),
            z_near: 0.025,
            z_far: 128.0,
        }
    }
}

fn handle_csm_settings_property_changed(
    settings: &mut CsmSettings,
    property_changed: &PropertyChanged,
) -> bool {
    if let FieldKind::Object(ref args) = property_changed.value {
        return match property_changed.name.as_ref() {
            CsmSettings::ENABLED => args.try_override(&mut settings.enabled),
            CsmSettings::SIZE => args.try_override(&mut settings.size),
            CsmSettings::PRECISION => args.try_override(&mut settings.precision),
            CsmSettings::PCF => args.try_override(&mut settings.pcf),
            _ => false,
        };
    }
    false
}

fn handle_quality_property_changed(
    settings: &mut QualitySettings,
    property_changed: &PropertyChanged,
) -> bool {
    match property_changed.value {
        FieldKind::Object(ref args) => {
            return match property_changed.name.as_ref() {
                QualitySettings::POINT_SHADOW_MAP_SIZE => {
                    args.try_override(&mut settings.point_shadow_map_size)
                }
                QualitySettings::POINT_SOFT_SHADOWS => {
                    args.try_override(&mut settings.point_soft_shadows)
                }
                QualitySettings::POINT_SHADOWS_ENABLED => {
                    args.try_override(&mut settings.point_shadows_enabled)
                }
                QualitySettings::POINT_SHADOWS_DISTANCE => {
                    args.try_override(&mut settings.point_shadows_distance)
                }
                QualitySettings::POINT_SHADOW_MAP_PRECISION => {
                    args.try_override(&mut settings.point_shadow_map_precision)
                }

                QualitySettings::SPOT_SHADOW_MAP_SIZE => {
                    args.try_override(&mut settings.spot_shadow_map_size)
                }
                QualitySettings::SPOT_SOFT_SHADOWS => {
                    args.try_override(&mut settings.spot_soft_shadows)
                }
                QualitySettings::SPOT_SHADOWS_ENABLED => {
                    args.try_override(&mut settings.spot_shadows_enabled)
                }
                QualitySettings::SPOT_SHADOWS_DISTANCE => {
                    args.try_override(&mut settings.spot_shadows_distance)
                }
                QualitySettings::SPOT_SHADOW_MAP_PRECISION => {
                    args.try_override(&mut settings.spot_shadow_map_precision)
                }

                QualitySettings::USE_SSAO => args.try_override(&mut settings.use_ssao),
                QualitySettings::SSAO_RADIUS => args.try_override(&mut settings.ssao_radius),

                QualitySettings::LIGHT_SCATTER_ENABLED => {
                    args.try_override(&mut settings.light_scatter_enabled)
                }

                QualitySettings::FXAA => args.try_override(&mut settings.fxaa),

                QualitySettings::USE_PARALLAX_MAPPING => {
                    args.try_override(&mut settings.use_parallax_mapping)
                }

                QualitySettings::USE_BLOOM => args.try_override(&mut settings.use_bloom),
                _ => false,
            };
        }
        FieldKind::Inspectable(ref inner) => {
            return match property_changed.name.as_ref() {
                QualitySettings::CSM_SETTINGS => {
                    return handle_csm_settings_property_changed(
                        &mut settings.csm_settings,
                        &**inner,
                    )
                }
                _ => false,
            }
        }
        _ => {}
    }
    false
}

impl GraphicsSettings {
    pub fn handle_property_changed(&mut self, property_changed: &PropertyChanged) -> bool {
        match property_changed.value {
            FieldKind::Object(ref args) => {
                return match property_changed.name.as_ref() {
                    Self::Z_NEAR => args.try_override(&mut self.z_near),
                    Self::Z_FAR => args.try_override(&mut self.z_far),
                    _ => false,
                };
            }
            FieldKind::Inspectable(ref inner) => {
                return match property_changed.name.as_ref() {
                    Self::QUALITY => {
                        return handle_quality_property_changed(&mut self.quality, &**inner)
                    }
                    _ => false,
                }
            }
            _ => {}
        }
        false
    }
}
