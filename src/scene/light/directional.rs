//! Directional light is a light source with parallel rays, it has
//! excellent example in real life - Sun. It does not have position,
//! only direction which defined by parent light scene node.
//!
//! # Notes
//!
//! Current directional light does *not* support shadows, it is still
//! on list of features that should be implemented.

use crate::{
    core::{
        inspect::{Inspect, PropertyInfo},
        pool::Handle,
        visitor::{Visit, VisitResult, Visitor},
    },
    scene::{
        graph::Graph,
        light::{BaseLight, BaseLightBuilder, Light},
        node::Node,
    },
};
use std::ops::{Deref, DerefMut};
use strum_macros::{AsRefStr, EnumString, EnumVariantNames};

/// Maximum amount of cascades.
pub const CSM_NUM_CASCADES: usize = 3;

/// Frustum split options defines how to split camera's frustum to generate cascades.
#[derive(Inspect, Clone, Visit, Debug, AsRefStr, EnumString, EnumVariantNames)]
pub enum FrustumSplitOptions {
    /// Camera frustum will be split into a [`CSM_NUM_CASCADES`] splits where each sub-frustum
    /// will have fixed far plane location.
    ///
    /// This option allows you to set far planes very precisely, thus allowing you to set desired
    /// quality of each cascade.
    ///
    /// This is default option.
    Absolute {
        /// A fixed set of distances, where each distance sets the location of far plane of
        /// of sub-frustum. If far plane exceeds far plane of current camera, then cascade will
        /// be discarded and won't be used for rendering.
        far_planes: [f32; CSM_NUM_CASCADES],
    },
    /// Camera frustum will be split into a [`CSM_NUM_CASCADES`] splits using provided fractions.
    ///
    /// This option might give lesser quality results with camera that have large far plane, however
    /// it does not require any precise tweaking.
    Relative {
        /// A fixed set of fractions in `[0; 1]` range which defines how far the far plane of
        /// sub-frustum will be relative to camera's frustum.
        fractions: [f32; CSM_NUM_CASCADES],
    },
}

impl Default for FrustumSplitOptions {
    fn default() -> Self {
        Self::Absolute {
            far_planes: [5.0, 25.0, 64.0],
        }
    }
}

/// Cascade Shadow Mapping (CSM) options.
#[derive(Inspect, Clone, Visit, Debug)]
pub struct CsmOptions {
    /// See [`FrustumSplitOptions`].
    pub split_options: FrustumSplitOptions,

    #[inspect(min_value = 0.0, step = 0.000025)]
    shadow_bias: f32,
}

impl Default for CsmOptions {
    fn default() -> Self {
        Self {
            split_options: Default::default(),
            shadow_bias: 0.00025,
        }
    }
}

impl CsmOptions {
    /// Sets new shadow bias value. Shadow bias allows you to prevent "shadow-acne" effect by
    /// shifting values fetched from shadow map by a certain value. "Shadow acne" occur due to
    /// insufficient precision.
    pub fn set_shadow_bias(&mut self, bias: f32) {
        self.shadow_bias = bias.max(0.0);
    }

    /// Returns current shadow bias value.
    pub fn shadow_bias(&self) -> f32 {
        self.shadow_bias
    }
}

/// See module docs.
#[derive(Default, Debug, Visit, Inspect)]
pub struct DirectionalLight {
    base_light: BaseLight,
    /// See [`CsmOptions`].
    #[visit(optional)] // Backward compatibility
    pub csm_options: CsmOptions,
}

impl From<BaseLight> for DirectionalLight {
    fn from(base_light: BaseLight) -> Self {
        Self {
            base_light,
            csm_options: Default::default(),
        }
    }
}

impl Deref for DirectionalLight {
    type Target = BaseLight;

    fn deref(&self) -> &Self::Target {
        &self.base_light
    }
}

impl DerefMut for DirectionalLight {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base_light
    }
}

impl DirectionalLight {
    /// Creates a raw copy of a directional light node.
    pub fn raw_copy(&self) -> Self {
        Self {
            base_light: self.base_light.raw_copy(),
            csm_options: self.csm_options.clone(),
        }
    }

    // Prefab inheritance resolving.
    pub(crate) fn inherit(&mut self, parent: &Node) {
        self.base_light.inherit(parent);

        // TODO: Add properties. https://github.com/FyroxEngine/Fyrox/issues/282
    }
}

/// Allows you to build directional light in declarative manner.
pub struct DirectionalLightBuilder {
    base_light_builder: BaseLightBuilder,
    csm_options: CsmOptions,
}

impl DirectionalLightBuilder {
    /// Creates new builder instance.
    pub fn new(base_light_builder: BaseLightBuilder) -> Self {
        Self {
            base_light_builder,
            csm_options: Default::default(),
        }
    }

    /// Creates new instance of directional light.
    pub fn build_directional_light(self) -> DirectionalLight {
        DirectionalLight {
            base_light: self.base_light_builder.build(),
            csm_options: self.csm_options,
        }
    }

    /// Sets desired options for cascaded shadow maps.
    pub fn with_csm_options(mut self, csm_options: CsmOptions) -> Self {
        self.csm_options = csm_options;
        self
    }

    /// Creates new instance of directional light node.
    pub fn build_node(self) -> Node {
        Node::Light(Light::Directional(self.build_directional_light()))
    }

    /// Creates new instance of directional light and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
