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

use crate::core::{
    algebra::Vector2, color::Color, num_traits::Euclid, reflect::prelude::*,
    type_traits::prelude::*, visitor::prelude::*, ImmutableString,
};
use std::fmt::{Display, Formatter};

use super::*;
use tileset::*;

/// Since a tile map may require multiple colliders to represent the diverse ways that physics objects may interact with the tiles,
/// tile set data must allow each tile to include multiple values for its collider information.
/// These multiple collider values are associated with their collider objects by a UUID and a name.
#[derive(Clone, Default, Debug, Reflect, Visit)]
pub struct TileSetColliderLayer {
    /// The id number that identifies the collider
    pub uuid: Uuid,
    /// The name of the collider
    pub name: ImmutableString,
    /// The color that will be used to represent the collider in the editor.
    pub color: Color,
}

/// In order to allow tile properties to be easily edited, properties need to have consistent names and data types
/// across all tiles in a tile set. A tile property layer represents the association between a property name
/// and its data type, along with other information.
#[derive(Clone, Default, Debug, Reflect, Visit)]
pub struct TileSetPropertyLayer {
    /// The id number that identifies this property
    pub uuid: Uuid,
    /// The name of the property that will be shown in the editor and can be used access the value.
    pub name: ImmutableString,
    /// The data type
    pub prop_type: TileSetPropertyType,
    /// Pre-defined named values.
    pub named_values: Vec<NamedValue>,
}

/// A value with an associated name. Often certain property values will have special meanings
/// for the game that is using the values, so it is useful to be able to label those values
/// so their special meaning can be visible in the editor.
#[derive(Clone, Default, Debug, Reflect, Visit)]
pub struct NamedValue {
    /// The label associated with the value.
    pub name: String,
    /// The special value that is being named.
    pub value: NamableValue,
    /// The color to represent this value in the editor
    pub color: Color,
}

/// Named values can be either an integer or a float.
/// It would make little sense to name a string or a nine slice.
#[derive(Copy, Clone, Debug, Reflect, Visit, PartialEq)]
pub enum NamableValue {
    /// A value for an element of a nine-slice
    I8(i8),
    /// An integer value
    I32(i32),
    /// A float value
    F32(f32),
}

impl Display for NamableValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::I8(value) => write!(f, "{value}"),
            Self::I32(value) => write!(f, "{value}"),
            Self::F32(value) => write!(f, "{value}"),
        }
    }
}

impl Default for NamableValue {
    fn default() -> Self {
        Self::I32(0)
    }
}

impl NamableValue {
    /// True if this value corresponds to the given property value.
    pub fn matches(&self, other: &TileSetPropertyOptionValue) -> bool {
        match (self, other) {
            (Self::I32(x), TileSetPropertyOptionValue::I32(Some(y))) => *x == *y,
            (Self::F32(x), TileSetPropertyOptionValue::F32(Some(y))) => *x == *y,
            _ => false,
        }
    }
}

impl TileSetPropertyLayer {
    /// Find the name associated with the given value.
    pub fn value_to_name(&self, value: NamableValue) -> String {
        self.named_values
            .iter()
            .find(|v| v.value == value)
            .map(|v| v.name.clone())
            .unwrap_or_else(|| format!("{value}"))
    }
    /// Find the color associated with the given value.
    pub fn value_to_color(&self, value: NamableValue) -> Option<Color> {
        self.named_values
            .iter()
            .find(|v| v.value == value)
            .map(|v| v.color)
    }
    /// Return the index of the named value that matches the given value, if one exits.
    pub fn find_value_index_from_property(
        &self,
        value: &TileSetPropertyOptionValue,
    ) -> Option<usize> {
        self.named_values
            .iter()
            .position(|v| v.value.matches(value))
    }
    /// Return the index of the named value that matches the given value, if one exits.
    pub fn find_value_index(&self, value: NamableValue) -> Option<usize> {
        self.named_values.iter().position(|v| v.value == value)
    }
    /// Return the appropriate highlight color for the tile at the given position when the
    /// tile has the given property value and the user has selected the given element value.
    /// If the value does not have a specified highlight color within this layer, then
    /// the value is compared against the element value and it is given a highlight color
    /// to acknowledge that the value matches the element value.
    pub fn highlight_color(
        &self,
        position: Vector2<usize>,
        value: &TileSetPropertyValue,
        element_value: &TileSetPropertyValueElement,
    ) -> Option<Color> {
        use TileSetPropertyValue as PropValue;
        use TileSetPropertyValueElement as Element;
        if position != Vector2::new(1, 1) && !matches!(value, PropValue::NineSlice(_)) {
            return None;
        }
        match (value, element_value) {
            (&PropValue::I32(v0), &Element::I32(v1)) => {
                self.value_to_color(NamableValue::I32(v0)).or({
                    if v0 == v1 {
                        Some(ELEMENT_MATCH_HIGHLIGHT_COLOR)
                    } else {
                        None
                    }
                })
            }
            (&PropValue::F32(v0), &Element::F32(v1)) => {
                self.value_to_color(NamableValue::F32(v0)).or({
                    if v0 == v1 {
                        Some(ELEMENT_MATCH_HIGHLIGHT_COLOR)
                    } else {
                        None
                    }
                })
            }
            (PropValue::String(v0), Element::String(v1)) => {
                if v0 == v1 {
                    Some(ELEMENT_MATCH_HIGHLIGHT_COLOR)
                } else {
                    None
                }
            }
            (PropValue::NineSlice(v0), &Element::I8(v1)) => {
                let v = v0[PropValue::nine_position_to_index(position)];
                self.value_to_color(NamableValue::I8(v)).or({
                    if v == v1 {
                        Some(ELEMENT_MATCH_HIGHLIGHT_COLOR)
                    } else {
                        None
                    }
                })
            }
            _ => None,
        }
    }
}

/// Each tile property needs to store a value to indicate what type of data will
/// be stored in that property, as the data type will affect how the editor
/// allows users to edit the property on each tile.
#[derive(Copy, Clone, Default, Debug, Eq, PartialEq, Reflect, Visit)]
pub enum TileSetPropertyType {
    /// The type for integer properties.
    #[default]
    I32,
    /// The type for float properties.
    F32,
    /// The type for string properties.
    String,
    /// Nine-slice properties allow a tile to have nine separate values,
    /// one value for each of its corners, each of its edges, and its center.
    NineSlice,
}

impl TileSetPropertyType {
    /// The default value for properties of the given type.
    pub fn default_value(&self) -> TileSetPropertyValue {
        use TileSetPropertyType as PropType;
        use TileSetPropertyValue as PropValue;
        match self {
            PropType::I32 => PropValue::I32(0),
            PropType::F32 => PropValue::F32(0.0),
            PropType::String => PropValue::String(ImmutableString::default()),
            PropType::NineSlice => PropValue::NineSlice([0; 9]),
        }
    }
    /// The none value when no value is available.
    pub fn default_option_value(&self) -> TileSetPropertyOptionValue {
        use TileSetPropertyOptionValue as PropValue;
        use TileSetPropertyType as PropType;
        match self {
            PropType::I32 => PropValue::I32(None),
            PropType::F32 => PropValue::F32(None),
            PropType::String => PropValue::String(None),
            PropType::NineSlice => PropValue::NineSlice([None; 9]),
        }
    }
}

/// The data stored in a tile property.
#[derive(Clone, Debug, PartialEq, Reflect, Visit)]
pub enum TileSetPropertyValue {
    /// Integer property data.
    I32(i32),
    /// Float property data.
    F32(f32),
    /// String property data.
    String(ImmutableString),
    /// Nine-slice properties allow a tile to have nine separate values,
    /// one value for each of its corners, each of its edges, and its center.
    NineSlice([i8; 9]),
}

/// An element of data stored within a tile's property.
/// For most value types, the element is the whole of the value,
/// but a nine-slice value contains nine elements.
#[derive(Clone, Debug, PartialEq)]
pub enum TileSetPropertyValueElement {
    /// Integer property data.
    I32(i32),
    /// Float property data.
    F32(f32),
    /// String property data.
    String(ImmutableString),
    /// Nine-slice property element.
    I8(i8),
}

impl Default for TileSetPropertyValue {
    fn default() -> Self {
        Self::I32(0)
    }
}

impl Default for TileSetPropertyValueElement {
    fn default() -> Self {
        Self::I32(0)
    }
}

impl OrthoTransform for TileSetPropertyValue {
    fn x_flipped(self) -> Self {
        if let Self::NineSlice(mut v) = self {
            fn pos(x: usize, y: usize) -> usize {
                TileSetPropertyValue::nine_position_to_index(Vector2::new(x, y))
            }
            v.swap(pos(2, 0), pos(0, 0));
            v.swap(pos(2, 1), pos(0, 1));
            v.swap(pos(2, 2), pos(0, 2));
            Self::NineSlice(v)
        } else {
            self
        }
    }

    fn rotated(self, amount: i8) -> Self {
        if let Self::NineSlice(mut v) = self {
            let amount = amount.rem_euclid(4);
            nine_rotate(&mut v, amount as usize * 2);
            Self::NineSlice(v)
        } else {
            self
        }
    }
}

const fn nine_index(x: usize, y: usize) -> usize {
    y * 3 + x
}

const NINE_ROTATE_LIST: [usize; 8] = [
    nine_index(0, 0),
    nine_index(1, 0),
    nine_index(2, 0),
    nine_index(2, 1),
    nine_index(2, 2),
    nine_index(1, 2),
    nine_index(0, 2),
    nine_index(0, 1),
];

fn nine_rotate(nine: &mut [i8; 9], amount: usize) {
    let copy = *nine;
    for i in 0..(8 - amount) {
        nine[NINE_ROTATE_LIST[i + amount]] = copy[NINE_ROTATE_LIST[i]];
    }
    for i in 0..amount {
        nine[NINE_ROTATE_LIST[i]] = copy[NINE_ROTATE_LIST[8 - amount + i]];
    }
}

impl TileSetPropertyValue {
    /// The default value for property values of this one's type.
    pub fn make_default(&self) -> TileSetPropertyValue {
        match self {
            TileSetPropertyValue::I32(_) => TileSetPropertyValue::I32(0),
            TileSetPropertyValue::F32(_) => TileSetPropertyValue::F32(0.0),
            TileSetPropertyValue::String(_) => {
                TileSetPropertyValue::String(ImmutableString::default())
            }
            TileSetPropertyValue::NineSlice(_) => {
                TileSetPropertyValue::NineSlice(Default::default())
            }
        }
    }
    /// Converts an x,y position into index in 0..9. Both x and y must be within 0..3.
    #[inline]
    pub fn nine_position_to_index(position: Vector2<usize>) -> usize {
        if position.y > 2 || position.x > 2 {
            panic!("Illegal nine slice position: {:?}", position);
        }
        position.y * 3 + position.x
    }
    /// Converts an index in 0..9 into an x,y position within a tile's nine slice value.
    #[inline]
    pub fn index_to_nine_position(index: usize) -> Vector2<usize> {
        let (y, x) = index.div_rem_euclid(&3);
        Vector2::new(x, y)
    }
    /// Update this value to match the given value, wherever that value is not None.
    /// Wherever the given value is None, no change is made to this value.
    pub fn set_from(&mut self, value: &TileSetPropertyOptionValue) {
        use TileSetPropertyOptionValue as OptValue;
        use TileSetPropertyValue as PropValue;
        match (self, value) {
            (PropValue::I32(x0), OptValue::I32(Some(x1))) => *x0 = *x1,
            (PropValue::F32(x0), OptValue::F32(Some(x1))) => *x0 = *x1,
            (PropValue::String(x0), OptValue::String(Some(x1))) => *x0 = x1.clone(),
            (PropValue::NineSlice(arr0), OptValue::NineSlice(arr1)) => {
                for (x0, x1) in arr0.iter_mut().zip(arr1.iter()) {
                    if let Some(v) = x1 {
                        *x0 = *v;
                    }
                }
            }
            _ => (),
        }
    }
}

/// A representation of data stored in a tile property, or the absence of that data
/// when the data is unknown.
#[derive(Clone, Debug, PartialEq, Reflect, Visit)]
pub enum TileSetPropertyOptionValue {
    /// Integer property data.
    I32(Option<i32>),
    /// Float property data.
    F32(Option<f32>),
    /// String property data.
    String(Option<ImmutableString>),
    /// Nine-slice properties allow a tile to have nine separate values,
    /// one value for each of its corners, each of its edges, and its center.
    NineSlice([Option<i8>; 9]),
}

impl Default for TileSetPropertyOptionValue {
    fn default() -> Self {
        Self::I32(None)
    }
}

impl From<TileSetPropertyValue> for TileSetPropertyOptionValue {
    fn from(value: TileSetPropertyValue) -> Self {
        use TileSetPropertyOptionValue as OValue;
        use TileSetPropertyValue as Value;
        match value {
            Value::I32(x) => OValue::I32(Some(x)),
            Value::F32(x) => OValue::F32(Some(x)),
            Value::String(x) => OValue::String(Some(x)),
            Value::NineSlice(arr) => OValue::NineSlice(arr.map(Some)),
        }
    }
}

impl From<TileSetPropertyOptionValue> for TileSetPropertyValue {
    fn from(value: TileSetPropertyOptionValue) -> Self {
        use TileSetPropertyOptionValue as OValue;
        use TileSetPropertyValue as Value;
        match value {
            OValue::I32(x) => Value::I32(x.unwrap_or_default()),
            OValue::F32(x) => Value::F32(x.unwrap_or_default()),
            OValue::String(x) => Value::String(x.unwrap_or_default()),
            OValue::NineSlice(arr) => Value::NineSlice(arr.map(Option::unwrap_or_default)),
        }
    }
}

impl TileSetPropertyOptionValue {
    /// Combines this value with the given value, replacing the content of this value with None
    /// wherever it differs from the given value.
    pub fn intersect(&mut self, value: &TileSetPropertyValue) {
        use TileSetPropertyOptionValue as OptValue;
        use TileSetPropertyValue as PropValue;
        match self {
            OptValue::I32(x0) => {
                if let Some(x) = x0 {
                    if *value != PropValue::I32(*x) {
                        *x0 = None
                    }
                }
            }
            OptValue::F32(x0) => {
                if let Some(x) = x0 {
                    if *value != PropValue::F32(*x) {
                        *x0 = None
                    }
                }
            }
            OptValue::String(x0) => {
                if let Some(x) = x0 {
                    if let PropValue::String(x1) = value {
                        if *x != *x1 {
                            *x0 = None
                        }
                    } else {
                        *x0 = None
                    }
                }
            }
            OptValue::NineSlice(arr0) => {
                if let PropValue::NineSlice(arr1) = value {
                    for (x0, x1) in arr0.iter_mut().zip(arr1.iter()) {
                        if let Some(x) = x0 {
                            if *x != *x1 {
                                *x0 = None
                            }
                        }
                    }
                } else {
                    *arr0 = [None; 9];
                }
            }
        }
    }
}
