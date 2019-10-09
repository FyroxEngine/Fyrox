use rg3d_core::{
    color::Color,
    visitor::{Visit, Visitor, VisitResult},
};

#[derive(Clone)]
pub enum LightKind {
    Spot,
    Point,
}

impl LightKind {
    pub fn new(id: u32) -> Result<Self, String> {
        match id {
            0 => Ok(LightKind::Spot),
            1 => Ok(LightKind::Point),
            _ => Err(format!("Invalid light kind {}", id))
        }
    }

    pub fn id(&self) -> u32 {
        match self {
            LightKind::Spot => 0,
            LightKind::Point => 1,
        }
    }
}

pub struct Light {
    kind: LightKind,
    radius: f32,
    color: Color,
    cone_angle: f32,
    cone_angle_cos: f32,
}

impl Default for Light {
    fn default() -> Self {
        Self {
            kind: LightKind::Point,
            radius: 10.0,
            color: Color::white(),
            cone_angle: std::f32::consts::PI,
            cone_angle_cos: -1.0,
        }
    }
}

impl Visit for Light {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut kind = self.kind.id();
        kind.visit("KindId", visitor)?;
        if visitor.is_reading() {
            self.kind = LightKind::new(kind)?;
        }

        // TODO: These properties can be taken from resource if light was
        // created from resource.
        self.radius.visit("Radius", visitor)?;
        self.color.visit("Color", visitor)?;
        self.cone_angle.visit("ConeAngle", visitor)?;
        self.cone_angle_cos.visit("ConeAngleCos", visitor)?;

        visitor.leave_region()
    }
}

impl Light {
    pub fn new(kind: LightKind) -> Self {
        Self {
            kind,
            radius: 10.0,
            color: Color::white(),
            cone_angle: std::f32::consts::PI,
            cone_angle_cos: -1.0,
        }
    }

    #[inline]
    pub fn set_radius(&mut self, radius: f32) {
        self.radius = radius;
    }

    #[inline]
    pub fn get_radius(&self) -> f32 {
        self.radius
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
    pub fn get_cone_angle_cos(&self) -> f32 {
        self.cone_angle_cos
    }

    #[inline]
    pub fn get_kind(&self) -> &LightKind {
        &self.kind
    }

    #[inline]
    pub fn set_cone_angle(&mut self, cone_angle: f32) {
        self.cone_angle = cone_angle;
        self.cone_angle_cos = cone_angle.cos();
    }
}

impl Clone for Light {
    fn clone(&self) -> Self {
        Self {
            kind: self.kind.clone(),
            radius: self.radius,
            color: self.color,
            cone_angle: self.cone_angle,
            cone_angle_cos: self.cone_angle_cos,
        }
    }
}