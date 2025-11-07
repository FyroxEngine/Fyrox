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

use crate::core::reflect::prelude::*;
use fyrox_core::uuid_provider;
use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, EnumString, VariantNames};

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct BloomSettings {
    /// Whether to use bloom effect.
    pub use_bloom: bool,

    /// A threshold value for luminance of a pixel to be considered "very bright". Only pixels
    /// that passed this check (>=) will be included in the bloom render target and will have the glow
    /// effect.
    pub threshold: f32,
}

impl Default for BloomSettings {
    fn default() -> Self {
        Self {
            use_bloom: true,
            threshold: 1.01,
        }
    }
}

/// Quality settings allows you to find optimal balance between performance and
/// graphics quality.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct QualitySettings {
    /// Point shadows
    /// Size of cube map face of shadow map texture in pixels.
    pub point_shadow_map_size: usize,
    /// Use or not percentage close filtering (smoothing) for point shadows.
    pub point_soft_shadows: bool,
    /// Point shadows enabled or not.
    pub point_shadows_enabled: bool,
    /// Maximum distance from camera to draw shadows.
    pub point_shadows_distance: f32,
    /// Point shadow map precision. Allows you to select compromise between
    /// quality and performance.
    pub point_shadow_map_precision: ShadowMapPrecision,
    /// Point shadows fade out range.
    /// Specifies the distance from the camera at which point shadows start to fade out.
    /// Shadows beyond this distance will gradually become less visible.
    pub point_shadows_fade_out_range: f32,

    /// Spot shadows
    /// Size of square shadow map texture in pixels
    pub spot_shadow_map_size: usize,
    /// Use or not percentage close filtering (smoothing) for spot shadows.
    pub spot_soft_shadows: bool,
    /// Spot shadows enabled or not.
    pub spot_shadows_enabled: bool,
    /// Maximum distance from camera to draw shadows.
    pub spot_shadows_distance: f32,
    /// Spot shadow map precision. Allows you to select compromise between
    /// quality and performance.
    pub spot_shadow_map_precision: ShadowMapPrecision,
    /// Specifies the distance from the camera at which spot shadows start to fade out.
    /// Shadows beyond this distance will gradually become less visible.
    pub spot_shadows_fade_out_range: f32,

    /// Cascaded-shadow maps settings.
    pub csm_settings: CsmSettings,

    /// Whether to use screen space ambient occlusion or not.
    pub use_ssao: bool,
    /// Radius of sampling hemisphere used in SSAO, it defines much ambient
    /// occlusion will be in your scene.
    pub ssao_radius: f32,

    /// Global switch to enable or disable light scattering. Each light can have
    /// its own scatter switch, but this one is able to globally disable scatter.
    pub light_scatter_enabled: bool,

    /// Whether to use Fast Approximate AntiAliasing or not.
    pub fxaa: bool,

    /// Whether to use Parallax Mapping or not.
    pub use_parallax_mapping: bool,

    /// Whether to use occlusion culling for geometry or not. Warning: this is experimental feature
    /// that may have bugs and unstable behavior. Disabled by default.
    #[serde(default)]
    pub use_occlusion_culling: bool,

    /// Whether to use occlusion culling for light sources or not. Warning: this is experimental
    /// feature that may have bugs and unstable behavior. Disabled by default.
    #[serde(default)]
    pub use_light_occlusion_culling: bool,

    #[serde(default)]
    pub bloom_settings: BloomSettings,
}

impl Default for QualitySettings {
    fn default() -> Self {
        Self::high()
    }
}

impl QualitySettings {
    /// Highest possible graphics quality. Requires very powerful GPU.
    pub fn ultra() -> Self {
        Self {
            point_shadow_map_size: 2048,
            point_shadows_distance: 20.0,
            point_shadows_enabled: true,
            point_soft_shadows: true,
            point_shadows_fade_out_range: 1.0,

            spot_shadow_map_size: 2048,
            spot_shadows_distance: 20.0,
            spot_shadows_enabled: true,
            spot_soft_shadows: true,
            spot_shadows_fade_out_range: 1.0,

            use_ssao: true,
            ssao_radius: 0.5,

            light_scatter_enabled: true,

            point_shadow_map_precision: ShadowMapPrecision::Full,
            spot_shadow_map_precision: ShadowMapPrecision::Full,

            fxaa: true,

            bloom_settings: Default::default(),

            use_parallax_mapping: true,

            csm_settings: Default::default(),

            use_occlusion_culling: false,
            use_light_occlusion_culling: false,
        }
    }

    /// High graphics quality, includes all graphical effects. Requires powerful GPU.
    pub fn high() -> Self {
        Self {
            point_shadow_map_size: 1024,
            point_shadows_distance: 15.0,
            point_shadows_enabled: true,
            point_soft_shadows: true,
            point_shadows_fade_out_range: 1.0,

            spot_shadow_map_size: 1024,
            spot_shadows_distance: 15.0,
            spot_shadows_enabled: true,
            spot_soft_shadows: true,
            spot_shadows_fade_out_range: 1.0,

            use_ssao: true,
            ssao_radius: 0.5,

            light_scatter_enabled: true,

            point_shadow_map_precision: ShadowMapPrecision::Full,
            spot_shadow_map_precision: ShadowMapPrecision::Full,

            fxaa: true,

            bloom_settings: Default::default(),

            use_parallax_mapping: true,

            csm_settings: CsmSettings {
                enabled: true,
                size: 2048,
                precision: ShadowMapPrecision::Full,
                pcf: true,
            },

            use_occlusion_culling: false,
            use_light_occlusion_culling: false,
        }
    }

    /// Medium graphics quality, some of effects are disabled, shadows will have sharp edges.
    pub fn medium() -> Self {
        Self {
            point_shadow_map_size: 512,
            point_shadows_distance: 5.0,
            point_shadows_enabled: true,
            point_soft_shadows: false,
            point_shadows_fade_out_range: 1.0,

            spot_shadow_map_size: 512,
            spot_shadows_distance: 5.0,
            spot_shadows_enabled: true,
            spot_soft_shadows: false,
            spot_shadows_fade_out_range: 1.0,

            use_ssao: true,
            ssao_radius: 0.5,

            light_scatter_enabled: false,

            point_shadow_map_precision: ShadowMapPrecision::Half,
            spot_shadow_map_precision: ShadowMapPrecision::Half,

            fxaa: true,

            bloom_settings: Default::default(),

            use_parallax_mapping: false,

            csm_settings: CsmSettings {
                enabled: true,
                size: 512,
                precision: ShadowMapPrecision::Full,
                pcf: false,
            },

            use_occlusion_culling: false,
            use_light_occlusion_culling: false,
        }
    }

    /// Lowest graphics quality, all effects are disabled.
    pub fn low() -> Self {
        Self {
            point_shadow_map_size: 1, // Zero is unsupported.
            point_shadows_distance: 0.0,
            point_shadows_enabled: false,
            point_soft_shadows: false,
            point_shadows_fade_out_range: 1.0,

            spot_shadow_map_size: 1,
            spot_shadows_distance: 0.0,
            spot_shadows_enabled: false,
            spot_soft_shadows: false,
            spot_shadows_fade_out_range: 1.0,

            use_ssao: false,
            ssao_radius: 0.5,

            light_scatter_enabled: false,

            point_shadow_map_precision: ShadowMapPrecision::Half,
            spot_shadow_map_precision: ShadowMapPrecision::Half,

            fxaa: false,

            bloom_settings: BloomSettings {
                use_bloom: false,
                ..Default::default()
            },

            use_parallax_mapping: false,

            csm_settings: CsmSettings {
                enabled: true,
                size: 512,
                precision: ShadowMapPrecision::Half,
                pcf: false,
            },

            use_occlusion_culling: false,
            use_light_occlusion_culling: false,
        }
    }
}

/// Cascaded-shadow maps settings.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Reflect, Eq)]
pub struct CsmSettings {
    /// Whether cascaded shadow maps enabled or not.
    pub enabled: bool,

    /// Size of texture for each cascade.
    pub size: usize,

    /// Bit-wise precision for each cascade, the lower precision the better performance is,
    /// but the more artifacts may occur.
    pub precision: ShadowMapPrecision,

    /// Whether to use Percentage-Closer Filtering or not.
    pub pcf: bool,
}

impl Default for CsmSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            size: 2048,
            precision: ShadowMapPrecision::Full,
            pcf: true,
        }
    }
}

/// Shadow map precision allows you to select compromise between quality and performance.
#[derive(
    Copy,
    Clone,
    Hash,
    PartialOrd,
    PartialEq,
    Eq,
    Ord,
    Debug,
    Serialize,
    Deserialize,
    Reflect,
    AsRefStr,
    EnumString,
    VariantNames,
)]
pub enum ShadowMapPrecision {
    /// Shadow map will use 2 times less memory by switching to 16bit pixel format,
    /// but "shadow acne" may occur.
    Half,
    /// Shadow map will use 32bit pixel format. This option gives highest quality,
    /// but could be less performant than `Half`.
    Full,
}

uuid_provider!(ShadowMapPrecision = "f9b2755b-248e-46ba-bcab-473eac1acdb8");
