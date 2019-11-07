use rg3d_core::{
    color::Color,
    visitor::{
        Visit,
        Visitor,
        VisitResult
    }
};
use crate::scene::base::{BaseBuilder, Base, AsBase};

#[derive(Clone)]
pub struct SpotLight {
    cone_angle: f32,
    cone_angle_cos: f32,
    distance: f32,
}

impl Default for SpotLight {
    fn default() -> Self {
        Self {
            cone_angle: std::f32::consts::FRAC_PI_4,
            cone_angle_cos: std::f32::consts::FRAC_PI_4.cos(),
            distance: 10.0
        }
    }
}

impl SpotLight {
    pub fn new(distance: f32, cone_angle: f32) -> Self {
        Self {
            cone_angle,
            cone_angle_cos: cone_angle.cos(),
            distance
        }
    }

    #[inline]
    pub fn get_cone_angle_cos(&self) -> f32 {
        self.cone_angle_cos
    }

    #[inline]
    pub fn get_cone_angle(&self) -> f32 {
        self.cone_angle
    }

    #[inline]
    pub fn set_cone_angle(&mut self, cone_angle: f32) {
        self.cone_angle = cone_angle;
        self.cone_angle_cos = cone_angle.cos();
    }

    #[inline]
    pub fn set_distance(&mut self, distance: f32) {
        self.distance = distance.abs();
    }

    #[inline]
    pub fn get_distance(&self) -> f32 {
        self.distance
    }
}

impl Visit for SpotLight {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.cone_angle.visit("ConeAngle", visitor)?;
        self.distance.visit("Distance", visitor)?;

        if visitor.is_reading() {
            self.cone_angle_cos = self.cone_angle.cos();
        }

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
    pub fn get_radius(&self) -> f32 {
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
    Spot(SpotLight),
    Point(PointLight),
}

impl LightKind {
    pub fn new(id: u32) -> Result<Self, String> {
        match id {
            0 => Ok(LightKind::Spot(Default::default())),
            1 => Ok(LightKind::Point(Default::default())),
            _ => Err(format!("Invalid light kind {}", id))
        }
    }

    pub fn id(&self) -> u32 {
        match self {
            LightKind::Spot(_) => 0,
            LightKind::Point(_) => 1,
        }
    }
}

impl Visit for LightKind {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        match self {
            LightKind::Spot(spot_light) => spot_light.visit(name, visitor),
            LightKind::Point(point_light) => point_light.visit(name, visitor),
        }
    }
}

#[derive(Clone)]
pub struct Light {
    base: Base,
    kind: LightKind,
    color: Color,
    cast_shadows: bool,
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

        visitor.leave_region()
    }
}

impl Light {
    pub fn new(kind: LightKind) -> Self {
        Self {
            kind,
            .. Default::default()
        }
    }

    #[inline]
    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    #[inline]
    pub fn get_color(&self) -> Color {
        self.color
    }

    #[inline]
    pub fn get_kind(&self) -> &LightKind {
        &self.kind
    }

    #[inline]
    pub fn get_kind_mut(&mut self) -> &mut LightKind {
        &mut self.kind
    }
}

pub struct LightBuilder {
    base_builder: BaseBuilder,
    kind: LightKind,
    color: Color,
    cast_shadows: bool,
}

impl LightBuilder {
    pub fn new(kind: LightKind, base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            kind,
            color: Color::WHITE,
            cast_shadows: true,
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

    pub fn build(self) -> Light {
        Light {
            base: self.base_builder.build(),
            kind: self.kind,
            color: self.color,
            cast_shadows: self.cast_shadows
        }
    }
}