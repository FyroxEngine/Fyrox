use crate::{
    core::{
        color::Color,
        visitor::{
            Visit,
            Visitor,
            VisitResult,
        },
        math::vec3::Vec3
    },
    scene::base::{
        BaseBuilder,
        Base,
        AsBase,
    },
};

pub const DEFAULT_SCATTER: Vec3 = Vec3::new(0.03, 0.03, 0.03);

#[derive(Clone)]
pub struct SpotLight {
    hotspot_cone_angle: f32,
    falloff_angle_delta: f32,
    distance: f32,
}

impl Default for SpotLight {
    fn default() -> Self {
        Self {
            hotspot_cone_angle: 90.0f32.to_radians(),
            falloff_angle_delta: 5.0f32.to_radians(),
            distance: 10.0,
        }
    }
}

impl SpotLight {
    pub fn new(distance: f32, hotspot_cone_angle: f32, falloff_angle_delta: f32) -> Self {
        Self {
            hotspot_cone_angle: hotspot_cone_angle.abs(),
            falloff_angle_delta: falloff_angle_delta.abs(),
            distance,
        }
    }

    #[inline]
    pub fn hotspot_cone_angle(&self) -> f32 {
        self.hotspot_cone_angle
    }

    #[inline]
    pub fn set_hotspot_cone_angle(&mut self, cone_angle: f32) -> &mut Self {
        self.hotspot_cone_angle = cone_angle.abs();
        self
    }

    #[inline]
    pub fn set_falloff_angle_delta(&mut self, delta: f32) -> &mut Self {
        self.falloff_angle_delta = delta;
        self
    }

    #[inline]
    pub fn falloff_angle_delta(&self) -> f32 {
        self.falloff_angle_delta
    }

    #[inline]
    pub fn full_cone_angle(&self) -> f32 {
        self.hotspot_cone_angle + self.falloff_angle_delta
    }

    #[inline]
    pub fn set_distance(&mut self, distance: f32) -> &mut Self {
        self.distance = distance.abs();
        self
    }

    #[inline]
    pub fn distance(&self) -> f32 {
        self.distance
    }
}

impl Visit for SpotLight {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.hotspot_cone_angle.visit("HotspotConeAngle", visitor)?;
        self.falloff_angle_delta.visit("FalloffAngleDelta", visitor)?;
        self.distance.visit("Distance", visitor)?;

        visitor.leave_region()
    }
}

#[derive(Clone)]
pub struct PointLight {
    radius: f32
}

impl PointLight {
    pub fn new(radius: f32) -> Self {
        Self {
            radius
        }
    }

    #[inline]
    pub fn set_radius(&mut self, radius: f32) {
        self.radius = radius.abs();
    }

    #[inline]
    pub fn radius(&self) -> f32 {
        self.radius
    }
}

impl Visit for PointLight {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.radius.visit("Radius", visitor)?;

        visitor.leave_region()
    }
}

impl Default for PointLight {
    fn default() -> Self {
        Self {
            radius: 10.0
        }
    }
}

#[derive(Clone)]
pub enum LightKind {
    Directional,
    Spot(SpotLight),
    Point(PointLight),
}

impl LightKind {
    pub fn new(id: u32) -> Result<Self, String> {
        match id {
            0 => Ok(LightKind::Spot(Default::default())),
            1 => Ok(LightKind::Point(Default::default())),
            2 => Ok(LightKind::Directional),
            _ => Err(format!("Invalid light kind {}", id))
        }
    }

    pub fn id(&self) -> u32 {
        match self {
            LightKind::Spot(_) => 0,
            LightKind::Point(_) => 1,
            LightKind::Directional => 2,
        }
    }
}

impl Visit for LightKind {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        match self {
            LightKind::Spot(spot_light) => spot_light.visit(name, visitor),
            LightKind::Point(point_light) => point_light.visit(name, visitor),
            LightKind::Directional => Ok(())
        }
    }
}

#[derive(Clone)]
pub struct Light {
    base: Base,
    kind: LightKind,
    color: Color,
    cast_shadows: bool,
    scatter: Vec3,
    scatter_enabled: bool,
}

impl AsBase for Light {
    fn base(&self) -> &Base {
        &self.base
    }

    fn base_mut(&mut self) -> &mut Base {
        &mut self.base
    }
}

impl Default for Light {
    fn default() -> Self {
        Self {
            base: Default::default(),
            kind: LightKind::Point(Default::default()),
            color: Color::WHITE,
            cast_shadows: true,
            scatter: Vec3::new(0.03, 0.03, 0.03),
            scatter_enabled: true
        }
    }
}

impl Visit for Light {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut kind_id = self.kind.id();
        kind_id.visit("KindId", visitor)?;
        if visitor.is_reading() {
            self.kind = LightKind::new(kind_id)?;
        }
        self.kind.visit("Kind", visitor)?;
        self.color.visit("Color", visitor)?;
        self.base.visit("Base", visitor)?;
        self.cast_shadows.visit("CastShadows", visitor)?;
        self.scatter.visit("ScatterFactor", visitor)?;
        self.scatter_enabled.visit("ScatterEnabled", visitor)?;

        visitor.leave_region()
    }
}

impl Light {
    pub fn new(kind: LightKind) -> Self {
        Self {
            kind,
            ..Default::default()
        }
    }

    #[inline]
    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    #[inline]
    pub fn color(&self) -> Color {
        self.color
    }

    #[inline]
    pub fn kind(&self) -> &LightKind {
        &self.kind
    }

    #[inline]
    pub fn kind_mut(&mut self) -> &mut LightKind {
        &mut self.kind
    }

    #[inline]
    pub fn set_cast_shadows(&mut self, value: bool) {
        self.cast_shadows = value;
    }

    #[inline]
    pub fn is_cast_shadows(&self) -> bool {
        self.cast_shadows
    }

    #[inline]
    pub fn set_scatter(&mut self, f: Vec3) {
        self.scatter = f;
    }

    #[inline]
    pub fn scatter(&self) -> Vec3 {
        self.scatter
    }

    #[inline]
    pub fn enable_scatter(&mut self, state: bool) {
        self.scatter_enabled = state;
    }

    #[inline]
    pub fn is_scatter_enabled(&self) -> bool {
        self.scatter_enabled
    }
}

pub struct LightBuilder {
    base_builder: BaseBuilder,
    kind: LightKind,
    color: Color,
    cast_shadows: bool,
    scatter_factor: Vec3,
    scatter_enabled: bool,
}

impl LightBuilder {
    pub fn new(kind: LightKind, base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            kind,
            color: Color::WHITE,
            cast_shadows: true,
            scatter_factor: DEFAULT_SCATTER,
            scatter_enabled: true
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn cast_shadows(mut self, cast_shadows: bool) -> Self {
        self.cast_shadows = cast_shadows;
        self
    }

    pub fn with_scatter_factor(mut self, f: Vec3) -> Self {
        self.scatter_factor = f;
        self
    }

    pub fn with_scatter_enabled(mut self, state: bool) -> Self {
        self.scatter_enabled = state;
        self
    }

    pub fn build(self) -> Light {
        Light {
            base: self.base_builder.build(),
            kind: self.kind,
            color: self.color,
            cast_shadows: self.cast_shadows,
            scatter: self.scatter_factor,
            scatter_enabled: self.scatter_enabled
        }
    }
}