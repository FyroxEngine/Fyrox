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

use crate::button::Button;
use crate::style::resource::StyleResourceExt;
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
use lazy_static::lazy_static;
use std::ops::{Deref, DerefMut};
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

lazy_static! {
    pub static ref DEFAULT_STYLE: BuiltInResource<Style> = BuiltInResource::new_no_source(
        StyleResource::new_ok("__DEFAULT_STYLE__".into(), Style::dark_style())
    );
}

#[derive(Clone, Debug, Default, Reflect)]
pub struct StyledProperty<T> {
    pub property: T,
    #[reflect(hidden)]
    pub name: ImmutableString,
}

impl<T> From<T> for StyledProperty<T> {
    fn from(property: T) -> Self {
        Self {
            property,
            name: Default::default(),
        }
    }
}

impl<T: PartialEq> PartialEq for StyledProperty<T> {
    fn eq(&self, other: &Self) -> bool {
        self.property == other.property
    }
}

impl<T> StyledProperty<T> {
    pub fn new(property: T, name: impl Into<ImmutableString>) -> Self {
        Self {
            property,
            name: name.into(),
        }
    }

    pub fn update(&mut self, style: &StyleResource)
    where
        StyleProperty: IntoPrimitive<T>,
    {
        if let Some(property) = style.get(self.name.clone()) {
            self.property = property;
        }
    }
}

impl<T> Deref for StyledProperty<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.property
    }
}

impl<T> DerefMut for StyledProperty<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.property
    }
}

impl<T: Visit> Visit for StyledProperty<T> {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.property.visit(name, visitor)
    }
}

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
    pub const BRUSH_LIGHTER_PRIMARY: &'static str = "Global.Brush.LighterPrimary";
    pub const BRUSH_LIGHT: &'static str = "Global.Brush.Light";
    pub const BRUSH_LIGHTER: &'static str = "Global.Brush.Lighter";
    pub const BRUSH_LIGHTEST: &'static str = "Global.Brush.Lightest";
    pub const BRUSH_BRIGHT: &'static str = "Global.Brush.Bright";
    pub const BRUSH_BRIGHTEST: &'static str = "Global.Brush.Brightest";
    pub const BRUSH_BRIGHT_BLUE: &'static str = "Global.Brush.BrightBlue";
    pub const BRUSH_DIM_BLUE: &'static str = "Global.Brush.DimBlue";
    pub const BRUSH_TEXT: &'static str = "Global.Brush.Text";
    pub const BRUSH_FOREGROUND: &'static str = "Global.Brush.Foreground";
    pub const BRUSH_INFORMATION: &'static str = "Global.Brush.Information";
    pub const BRUSH_WARNING: &'static str = "Global.Brush.Warning";
    pub const BRUSH_ERROR: &'static str = "Global.Brush.Error";
    pub const BRUSH_OK: &'static str = "Global.Brush.Ok";

    pub fn dark_style() -> Style {
        let mut style = Self::default();
        style
            // Global
            .set(Self::BRUSH_DARKEST, Brush::Solid(Color::repeat_opaque(20)))
            .set(Self::BRUSH_DARKER, Brush::Solid(Color::repeat_opaque(30)))
            .set(Self::BRUSH_DARK, Brush::Solid(Color::repeat_opaque(40)))
            .set(Self::BRUSH_PRIMARY, Brush::Solid(Color::repeat_opaque(50)))
            .set(
                Self::BRUSH_LIGHTER_PRIMARY,
                Brush::Solid(Color::repeat_opaque(60)),
            )
            .set(Self::BRUSH_LIGHT, Brush::Solid(Color::repeat_opaque(70)))
            .set(Self::BRUSH_LIGHTER, Brush::Solid(Color::repeat_opaque(85)))
            .set(
                Self::BRUSH_LIGHTEST,
                Brush::Solid(Color::repeat_opaque(100)),
            )
            .set(Self::BRUSH_BRIGHT, Brush::Solid(Color::repeat_opaque(130)))
            .set(
                Self::BRUSH_BRIGHTEST,
                Brush::Solid(Color::repeat_opaque(160)),
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
            .set(Self::BRUSH_FOREGROUND, Brush::Solid(Color::WHITE))
            .set(Self::BRUSH_INFORMATION, Brush::Solid(Color::ANTIQUE_WHITE))
            .set(Self::BRUSH_WARNING, Brush::Solid(Color::GOLD))
            .set(Self::BRUSH_ERROR, Brush::Solid(Color::RED))
            .set(Self::BRUSH_OK, Brush::Solid(Color::GREEN))
            // Button
            .set(Button::CORNER_RADIUS, 4.0f32)
            .set(Button::BORDER_THICKNESS, Thickness::uniform(1.0));
        style
    }

    pub fn light_style() -> Style {
        let mut style = Self::default();
        style
            // Global
            .set(Self::BRUSH_DARKEST, Brush::Solid(Color::repeat_opaque(140)))
            .set(Self::BRUSH_DARKER, Brush::Solid(Color::repeat_opaque(150)))
            .set(Self::BRUSH_DARK, Brush::Solid(Color::repeat_opaque(160)))
            .set(Self::BRUSH_PRIMARY, Brush::Solid(Color::repeat_opaque(170)))
            .set(
                Self::BRUSH_LIGHTER_PRIMARY,
                Brush::Solid(Color::repeat_opaque(180)),
            )
            .set(Self::BRUSH_LIGHT, Brush::Solid(Color::repeat_opaque(190)))
            .set(Self::BRUSH_LIGHTER, Brush::Solid(Color::repeat_opaque(205)))
            .set(
                Self::BRUSH_LIGHTEST,
                Brush::Solid(Color::repeat_opaque(220)),
            )
            .set(Self::BRUSH_BRIGHT, Brush::Solid(Color::repeat_opaque(40)))
            .set(
                Self::BRUSH_BRIGHTEST,
                Brush::Solid(Color::repeat_opaque(30)),
            )
            .set(
                Self::BRUSH_BRIGHT_BLUE,
                Brush::Solid(Color::opaque(80, 118, 178)),
            )
            .set(
                Self::BRUSH_DIM_BLUE,
                Brush::Solid(Color::opaque(66, 99, 149)),
            )
            .set(Self::BRUSH_TEXT, Brush::Solid(Color::repeat_opaque(0)))
            .set(Self::BRUSH_FOREGROUND, Brush::Solid(Color::WHITE))
            .set(Self::BRUSH_INFORMATION, Brush::Solid(Color::ANTIQUE_WHITE))
            .set(Self::BRUSH_WARNING, Brush::Solid(Color::GOLD))
            .set(Self::BRUSH_ERROR, Brush::Solid(Color::RED))
            // Button
            .set(Button::CORNER_RADIUS, 4.0f32)
            .set(Button::BORDER_THICKNESS, Thickness::uniform(1.0));
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
