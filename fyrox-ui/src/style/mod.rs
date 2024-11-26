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

pub mod resource;

use crate::{
    brush::Brush,
    style::resource::{StyleResource, StyleResourceError},
    Thickness,
};
use fxhash::FxHashMap;
use fyrox_core::{
    color::Color, reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*,
    ImmutableString, Uuid,
};
use fyrox_resource::{io::ResourceIo, manager::BuiltInResource};
use std::path::Path;

#[derive(Visit, Reflect, Debug, Clone)]
pub enum StyleProperty {
    Number(f32),
    Thickness(Thickness),
    Color(Color),
    Brush(Brush),
}

impl Default for StyleProperty {
    fn default() -> Self {
        Self::Number(0.0)
    }
}

pub trait IntoPrimitive<T> {
    fn into_primitive(self) -> Option<T>;
}

macro_rules! impl_casts {
    ($ty:ty => $var:ident) => {
        impl From<$ty> for StyleProperty {
            fn from(value: $ty) -> Self {
                Self::$var(value)
            }
        }

        impl IntoPrimitive<$ty> for StyleProperty {
            fn into_primitive(self) -> Option<$ty> {
                if let StyleProperty::$var(value) = self {
                    Some(value)
                } else {
                    None
                }
            }
        }
    };
}

impl_casts!(f32 => Number);
impl_casts!(Thickness => Thickness);
impl_casts!(Color => Color);
impl_casts!(Brush => Brush);

#[derive(Visit, Reflect, Default, Debug, TypeUuidProvider)]
#[type_uuid(id = "38a63b49-d765-4c01-8fb5-202cc43d607e")]
pub struct Style {
    parent: Option<StyleResource>,
    variables: FxHashMap<ImmutableString, StyleProperty>,
}

impl Style {
    pub const BRUSH_DARKEST: &'static str = "Global.Brush.Darkest";
    pub const BRUSH_DARKER: &'static str = "Global.Brush.Darker";
    pub const BRUSH_DARK: &'static str = "Global.Brush.Dark";
    pub const BRUSH_PRIMARY: &'static str = "Global.Brush.Primary";
    pub const BRUSH_LIGHT: &'static str = "Global.Brush.Light";
    pub const BRUSH_LIGHTER: &'static str = "Global.Brush.Lighter";
    pub const BRUSH_LIGHTEST: &'static str = "Global.Brush.Lightest";
    pub const BRUSH_BRIGHT: &'static str = "Global.Brush.Bright";
    pub const BRUSH_BRIGHTEST: &'static str = "Global.Brush.Brightest";
    pub const BRUSH_BRIGHT_BLUE: &'static str = "Global.Brush.BrightBlue";
    pub const BRUSH_DIM_BLUE: &'static str = "Global.Brush.DimBlue";
    pub const BRUSH_TEXT: &'static str = "Global.Brush.Text";
    pub const BRUSH_FOREGROUND: &'static str = "Global.Brush.Foreground";

    pub fn default_style() -> Style {
        let mut style = Self::default();
        style
            .set(Self::BRUSH_DARKEST, Brush::Solid(Color::opaque(20, 20, 20)))
            .set(Self::BRUSH_DARKER, Brush::Solid(Color::opaque(30, 30, 30)))
            .set(Self::BRUSH_DARK, Brush::Solid(Color::opaque(40, 40, 40)))
            .set(Self::BRUSH_PRIMARY, Brush::Solid(Color::opaque(50, 50, 50)))
            .set(Self::BRUSH_LIGHT, Brush::Solid(Color::opaque(70, 70, 70)))
            .set(Self::BRUSH_LIGHTER, Brush::Solid(Color::opaque(85, 85, 85)))
            .set(
                Self::BRUSH_LIGHTEST,
                Brush::Solid(Color::opaque(100, 100, 100)),
            )
            .set(
                Self::BRUSH_BRIGHT,
                Brush::Solid(Color::opaque(130, 130, 130)),
            )
            .set(
                Self::BRUSH_BRIGHTEST,
                Brush::Solid(Color::opaque(160, 160, 160)),
            )
            .set(
                Self::BRUSH_BRIGHT_BLUE,
                Brush::Solid(Color::opaque(80, 118, 178)),
            )
            .set(
                Self::BRUSH_DIM_BLUE,
                Brush::Solid(Color::opaque(66, 99, 149)),
            )
            .set(Self::BRUSH_TEXT, Brush::Solid(Color::opaque(220, 220, 220)))
            .set(Self::BRUSH_FOREGROUND, Brush::Solid(Color::WHITE));
        style
    }

    pub fn set_parent(&mut self, parent: Option<StyleResource>) {
        self.parent = parent;
    }

    pub fn set(
        &mut self,
        name: impl Into<ImmutableString>,
        property: impl Into<StyleProperty>,
    ) -> &mut Self {
        self.variables.insert(name.into(), property.into());
        self
    }

    pub fn get_raw(&self, name: impl Into<ImmutableString>) -> Option<StyleProperty> {
        let name = name.into();
        if let Some(property) = self.variables.get(&name) {
            return Some(property.clone());
        } else if let Some(parent) = self.parent.as_ref() {
            let state = parent.state();
            if let Some(data) = state.data_ref() {
                return data.get_raw(name);
            }
        }
        None
    }

    pub fn get<P>(&self, name: impl Into<ImmutableString>) -> Option<P>
    where
        StyleProperty: IntoPrimitive<P>,
    {
        self.get_raw(name)
            .and_then(|property| property.into_primitive())
    }

    pub fn get_or_default<P>(&self, name: impl Into<ImmutableString>) -> P
    where
        P: Default,
        StyleProperty: IntoPrimitive<P>,
    {
        self.get_raw(name)
            .and_then(|property| property.into_primitive())
            .unwrap_or_default()
    }

    pub async fn from_file(path: &Path, io: &dyn ResourceIo) -> Result<Self, StyleResourceError> {
        let bytes = io.load_file(path).await?;
        let mut visitor = Visitor::load_from_memory(&bytes)?;
        let mut style = Style::default();
        style.visit("Style", &mut visitor)?;
        Ok(style)
    }
}
