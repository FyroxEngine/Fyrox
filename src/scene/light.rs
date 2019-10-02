use rg3d_core::{
    color::Color,
    visitor::{Visit, Visitor, VisitResult}
};

pub struct Light {
    radius: f32,
    color: Color,
    cone_angle: f32,
    cone_angle_cos: f32,
}

impl Default for Light {
    fn default() -> Self {
        Self::new()
    }
}

impl Visit for Light {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        // TODO: These properties can be taken from resource if light was
        // create from resource.
        self.radius.visit("Radius", visitor)?;
        self.color.visit("Color", visitor)?;
        self.cone_angle.visit("ConeAngle", visitor)?;
        self.cone_angle_cos.visit("ConeAngleCos", visitor)?;

        visitor.leave_region()
    }
}

impl Light {
    pub fn new() -> Self {
        Self {
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

    pub fn set_cone_angle(&mut self, cone_angle: f32) {
        self.cone_angle = cone_angle;
        self.cone_angle_cos = cone_angle.cos();
    }
}

impl Clone for Light {
    fn clone(&self) -> Self {
        Self {
            radius: self.radius,
            color: self.color,
            cone_angle: self.cone_angle,
            cone_angle_cos: self.cone_angle_cos,
        }
    }
}