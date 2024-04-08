//! A module that contains everything related to numeric values of animation tracks. See [`TrackValue`] docs
//! for more info.

use crate::core::{
    algebra::{Unit, UnitQuaternion, Vector2, Vector3, Vector4},
    math::lerpf,
    num_traits::AsPrimitive,
    reflect::prelude::*,
    visitor::prelude::*,
};
use fyrox_core::log::Log;
use std::fmt::{Debug, Display, Formatter};

/// An actual type of a property value.
#[derive(Visit, Reflect, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ValueType {
    /// `bool`
    Bool,
    /// `f32`
    F32,
    /// `f64`
    F64,
    /// `u64`
    U64,
    /// `i64`
    I64,
    /// `u32`
    U32,
    /// `i32`
    I32,
    /// `u16`
    U16,
    /// `i16`
    I16,
    /// `u8`
    U8,
    /// `i8`
    I8,

    /// `Vector2<bool>`
    Vector2Bool,
    /// `Vector2<f32>`
    Vector2F32,
    /// `Vector2<f64>`
    Vector2F64,
    /// `Vector2<u64>`
    Vector2U64,
    /// `Vector2<i64>`
    Vector2I64,
    /// `Vector2<u32>`
    Vector2U32,
    /// `Vector2<i32>`
    Vector2I32,
    /// `Vector2<u16>`
    Vector2U16,
    /// `Vector2<i16>`
    Vector2I16,
    /// `Vector2<u8>`
    Vector2U8,
    /// `Vector2<i8>`
    Vector2I8,

    /// `Vector3<bool>`
    Vector3Bool,
    /// `Vector3<f32>`
    Vector3F32,
    /// `Vector3<f64>`
    Vector3F64,
    /// `Vector3<u64>`
    Vector3U64,
    /// `Vector3<i64>`
    Vector3I64,
    /// `Vector3<u32>`
    Vector3U32,
    /// `Vector3<i32>`
    Vector3I32,
    /// `Vector3<u16>`
    Vector3U16,
    /// `Vector3<i16>`
    Vector3I16,
    /// `Vector3<u8>`
    Vector3U8,
    /// `Vector3<i8>`
    Vector3I8,

    /// `Vector4<bool>`
    Vector4Bool,
    /// `Vector4<f32>`
    Vector4F32,
    /// `Vector4<f64>`
    Vector4F64,
    /// `Vector4<u64>`
    Vector4U64,
    /// `Vector4<i64>`
    Vector4I64,
    /// `Vector4<u32>`
    Vector4U32,
    /// `Vector4<i32>`
    Vector4I32,
    /// `Vector4<u16>`
    Vector4U16,
    /// `Vector4<i16>`
    Vector4I16,
    /// `Vector4<u8>`
    Vector4U8,
    /// `Vector4<i8>`
    Vector4I8,

    /// `UnitQuaternion<f32>`
    UnitQuaternionF32,
    /// `UnitQuaternion<f64>`
    UnitQuaternionF64,
}

impl Default for ValueType {
    fn default() -> Self {
        Self::F32
    }
}

/// A real value that can be produced by an animation track. Animations always operate on real numbers (`f32`) for any kind
/// of machine numeric types (including `bool`). This is needed to be able to blend values; final blending result is then
/// converted to an actual machine type of a target property.
#[derive(Clone, Debug, PartialEq)]
pub enum TrackValue {
    /// A real number.
    Real(f32),

    /// A 2-dimensional vector of real values.
    Vector2(Vector2<f32>),

    /// A 3-dimensional vector of real values.
    Vector3(Vector3<f32>),

    /// A 4-dimensional vector of real values.
    Vector4(Vector4<f32>),

    /// A quaternion that represents some rotation.
    UnitQuaternion(UnitQuaternion<f32>),
}

impl TrackValue {
    /// Mixes (blends) the current value with an other value using the given weight. Blending is possible only if the types
    /// are the same.
    pub fn blend_with(&mut self, other: &Self, weight: f32) {
        match (self, other) {
            (Self::Real(a), Self::Real(b)) => *a = lerpf(*a, *b, weight),
            (Self::Vector2(a), Self::Vector2(b)) => *a = a.lerp(b, weight),
            (Self::Vector3(a), Self::Vector3(b)) => *a = a.lerp(b, weight),
            (Self::Vector4(a), Self::Vector4(b)) => *a = a.lerp(b, weight),
            (Self::UnitQuaternion(a), Self::UnitQuaternion(b)) => *a = nlerp(*a, b, weight),
            _ => (),
        }
    }

    /// Tries to perform a numeric type casting of the current value to some other and returns a boxed value, that can
    /// be used to set the value using reflection.
    pub fn numeric_type_cast(&self, value_type: ValueType) -> Option<Box<dyn Reflect>> {
        fn convert_vec2<T>(vec2: &Vector2<f32>) -> Vector2<T>
        where
            f32: AsPrimitive<T>,
            T: Copy + 'static,
        {
            Vector2::new(vec2.x.as_(), vec2.y.as_())
        }

        fn convert_vec3<T>(vec3: &Vector3<f32>) -> Vector3<T>
        where
            f32: AsPrimitive<T>,
            T: Copy + 'static,
        {
            Vector3::new(vec3.x.as_(), vec3.y.as_(), vec3.z.as_())
        }

        fn convert_vec4<T>(vec4: &Vector4<f32>) -> Vector4<T>
        where
            f32: AsPrimitive<T>,
            T: Copy + 'static,
        {
            Vector4::new(vec4.x.as_(), vec4.y.as_(), vec4.z.as_(), vec4.w.as_())
        }

        match self {
            TrackValue::Real(real) => match value_type {
                ValueType::Bool => Some(Box::new(real.ne(&0.0))),
                ValueType::F32 => Some(Box::new(*real)),
                ValueType::F64 => Some(Box::new(*real as f64)),
                ValueType::U64 => Some(Box::new(*real as u64)),
                ValueType::I64 => Some(Box::new(*real as i64)),
                ValueType::U32 => Some(Box::new(*real as u32)),
                ValueType::I32 => Some(Box::new(*real as i32)),
                ValueType::U16 => Some(Box::new(*real as u16)),
                ValueType::I16 => Some(Box::new(*real as i16)),
                ValueType::U8 => Some(Box::new(*real as u8)),
                ValueType::I8 => Some(Box::new(*real as i8)),
                _ => None,
            },
            TrackValue::Vector2(vec2) => match value_type {
                ValueType::Vector2Bool => {
                    Some(Box::new(Vector2::new(vec2.x.ne(&0.0), vec2.y.ne(&0.0))))
                }
                ValueType::Vector2F32 => Some(Box::new(*vec2)),
                ValueType::Vector2F64 => Some(Box::new(convert_vec2::<f64>(vec2))),
                ValueType::Vector2U64 => Some(Box::new(convert_vec2::<u64>(vec2))),
                ValueType::Vector2I64 => Some(Box::new(convert_vec2::<i64>(vec2))),
                ValueType::Vector2U32 => Some(Box::new(convert_vec2::<u32>(vec2))),
                ValueType::Vector2I32 => Some(Box::new(convert_vec2::<i32>(vec2))),
                ValueType::Vector2U16 => Some(Box::new(convert_vec2::<u16>(vec2))),
                ValueType::Vector2I16 => Some(Box::new(convert_vec2::<i16>(vec2))),
                ValueType::Vector2U8 => Some(Box::new(convert_vec2::<u8>(vec2))),
                ValueType::Vector2I8 => Some(Box::new(convert_vec2::<i8>(vec2))),
                _ => None,
            },
            TrackValue::Vector3(vec3) => match value_type {
                ValueType::Vector3Bool => Some(Box::new(Vector3::new(
                    vec3.x.ne(&0.0),
                    vec3.y.ne(&0.0),
                    vec3.z.ne(&0.0),
                ))),
                ValueType::Vector3F32 => Some(Box::new(*vec3)),
                ValueType::Vector3F64 => Some(Box::new(convert_vec3::<f64>(vec3))),
                ValueType::Vector3U64 => Some(Box::new(convert_vec3::<u64>(vec3))),
                ValueType::Vector3I64 => Some(Box::new(convert_vec3::<i64>(vec3))),
                ValueType::Vector3U32 => Some(Box::new(convert_vec3::<u32>(vec3))),
                ValueType::Vector3I32 => Some(Box::new(convert_vec3::<i32>(vec3))),
                ValueType::Vector3U16 => Some(Box::new(convert_vec3::<u16>(vec3))),
                ValueType::Vector3I16 => Some(Box::new(convert_vec3::<i16>(vec3))),
                ValueType::Vector3U8 => Some(Box::new(convert_vec3::<u8>(vec3))),
                ValueType::Vector3I8 => Some(Box::new(convert_vec3::<i8>(vec3))),
                _ => None,
            },
            TrackValue::Vector4(vec4) => match value_type {
                ValueType::Vector4Bool => Some(Box::new(Vector4::new(
                    vec4.x.ne(&0.0),
                    vec4.y.ne(&0.0),
                    vec4.z.ne(&0.0),
                    vec4.w.ne(&0.0),
                ))),
                ValueType::Vector4F32 => Some(Box::new(*vec4)),
                ValueType::Vector4F64 => Some(Box::new(convert_vec4::<f64>(vec4))),
                ValueType::Vector4U64 => Some(Box::new(convert_vec4::<u64>(vec4))),
                ValueType::Vector4I64 => Some(Box::new(convert_vec4::<i64>(vec4))),
                ValueType::Vector4U32 => Some(Box::new(convert_vec4::<u32>(vec4))),
                ValueType::Vector4I32 => Some(Box::new(convert_vec4::<i32>(vec4))),
                ValueType::Vector4U16 => Some(Box::new(convert_vec4::<u16>(vec4))),
                ValueType::Vector4I16 => Some(Box::new(convert_vec4::<i16>(vec4))),
                ValueType::Vector4U8 => Some(Box::new(convert_vec4::<u8>(vec4))),
                ValueType::Vector4I8 => Some(Box::new(convert_vec4::<i8>(vec4))),
                _ => None,
            },
            TrackValue::UnitQuaternion(quat) => match value_type {
                ValueType::UnitQuaternionF32 => Some(Box::new(*quat)),
                ValueType::UnitQuaternionF64 => Some(Box::new(quat.cast::<f64>())),
                _ => None,
            },
        }
    }
}

/// Value binding tells the animation system to which of the many properties to set track's value. It has special
/// cases for the most used properties and a generic one for arbitrary properties. Arbitrary properties are set using
/// reflection system, while the special cases handles bindings to standard properties (such as position, scaling, or
/// rotation) for optimization. Reflection is quite slow to be used as the universal property setting mechanism.  
#[derive(Clone, Visit, Reflect, Debug, PartialEq, Eq)]
pub enum ValueBinding {
    /// A binding to position of a scene node.
    Position,
    /// A binding to scale of a scene node.
    Scale,
    /// A binding to rotation of a scene node.
    Rotation,
    /// A binding to an arbitrary property of a scene node.
    Property {
        /// A path to a property (`foo.bar.baz[1].foobar@EnumVariant.stuff`)
        name: String,
        /// Actual property type (only numeric properties are supported).
        value_type: ValueType,
    },
}

impl Display for ValueBinding {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueBinding::Position => write!(f, "Position"),
            ValueBinding::Scale => write!(f, "Scale"),
            ValueBinding::Rotation => write!(f, "Rotation"),
            ValueBinding::Property { name, .. } => write!(f, "{}", name),
        }
    }
}

/// A value that is bound to a property.
#[derive(Clone, Debug, PartialEq)]
pub struct BoundValue {
    /// A property to which the value is bound to.
    pub binding: ValueBinding,
    /// The new value for the property the binding points to.
    pub value: TrackValue,
}

impl BoundValue {
    /// Blends the current value with an other value using the given weight. See [`TrackValue::blend_with`] for
    /// more info.
    pub fn blend_with(&mut self, other: &Self, weight: f32) {
        assert_eq!(self.binding, other.binding);
        self.value.blend_with(&other.value, weight);
    }

    /// Sets a property of the given object.
    pub fn apply_to_object(
        &self,
        object: &mut dyn Reflect,
        property_name: &str,
        value_type: ValueType,
    ) {
        if let Some(casted) = self.value.numeric_type_cast(value_type) {
            let mut casted = Some(casted);
            object.as_reflect_mut(&mut |object_ref| {
                object_ref.set_field_by_path(property_name, casted.take().unwrap(), &mut |result| {
                    if let Err(err) = result {
                        match err {
                            SetFieldByPathError::InvalidPath { reason, .. } => {
                                Log::err(format!(
                                    "Failed to set property {}! Invalid path: {}",
                                    property_name, reason
                                ));
                            }
                            SetFieldByPathError::InvalidValue(_) => {
                                Log::err(format!(
                                    "Failed to set property {}! Types mismatch!",
                                    property_name
                                ));
                            }
                        }
                    }
                })
            })
        }
    }
}

/// A collection of values that are bounds to some properties.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BoundValueCollection {
    /// Actual values collection.
    pub values: Vec<BoundValue>,
}

impl BoundValueCollection {
    /// Tries to blend each value of the current collection with a respective (by binding) value in the other collection.
    /// See [`TrackValue::blend_with`] docs for more info.
    pub fn blend_with(&mut self, other: &Self, weight: f32) {
        for value in self.values.iter_mut() {
            if let Some(other_value) = other.values.iter().find(|v| v.binding == value.binding) {
                value.blend_with(other_value, weight);
            }
        }
    }
}

/// Interpolates from `a` to `b` using nlerp, including an additional check to ensure
/// that the a.dot(b) is positive to prevent the interpolation from going around the long way.
pub fn nlerp(mut a: UnitQuaternion<f32>, b: &UnitQuaternion<f32>, w: f32) -> UnitQuaternion<f32> {
    if a.dot(b) < 0.0 {
        a = negate_unit_quaternion(&a)
    }
    a.nlerp(b, w)
}

/// Negate the given quaternion by negating each of its components.
pub fn negate_unit_quaternion(a: &UnitQuaternion<f32>) -> UnitQuaternion<f32> {
    Unit::new_unchecked(-a.as_ref())
}
