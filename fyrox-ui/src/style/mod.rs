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

#![warn(missing_docs)]

//! Style allows to change visual appearance of widgets in centralized manner. It can be considered
//! as a storage for properties, that defines visual appearance. See [`Style`] docs for more info
//! and usage examples.

pub mod resource;

use crate::{
    brush::Brush,
    button::Button,
    check_box::CheckBox,
    core::{
        color::Color, reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*,
        ImmutableString, Uuid,
    },
    dropdown_list::DropdownList,
    style::resource::{StyleResource, StyleResourceError, StyleResourceExt},
    toggle::ToggleButton,
    Thickness,
};
use fxhash::FxHashMap;
use fyrox_resource::untyped::ResourceKind;
use fyrox_resource::{
    io::ResourceIo,
    manager::{BuiltInResource, ResourceManager},
};
use fyrox_texture::TextureResource;
use lazy_static::lazy_static;
use std::{
    ops::{Deref, DerefMut},
    path::Path,
    sync::Arc,
};

/// A set of potential values for styled properties.
#[derive(Visit, Reflect, Debug, Clone)]
pub enum StyleProperty {
    /// A numeric property.
    Number(f32),
    /// A thickness property, that defines width of four sides of a rectangles.
    Thickness(Thickness),
    /// A color property.
    Color(Color),
    /// A brush property, that defines how to render an arbitrary surface (solid, with gradient, etc.).
    Brush(Brush),
    /// A texture property. Could be used together with [`crate::image::Image`] widget or [`crate::nine_patch::NinePatch`]
    /// widget.
    Texture(TextureResource),
}

impl Default for StyleProperty {
    fn default() -> Self {
        Self::Number(0.0)
    }
}

/// A trait that provides a method that translates [`StyleProperty`] into a specific primitive value.
pub trait IntoPrimitive<T> {
    /// Tries to convert self into a primitive value `T`.
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
impl_casts!(TextureResource => Texture);

lazy_static! {
    /// Default style of the library.
    pub static ref DEFAULT_STYLE: BuiltInResource<Style> = BuiltInResource::new_no_source("__DEFAULT_STYLE__",
        StyleResource::new_ok(uuid!("1e0716e8-e728-491c-a65b-ca11b15048be"), ResourceKind::External, Style::dark_style())
    );
}

/// A property, that can bind its value to a style. Why can't we just fetch the actual value from
/// the style and why do we need to store the value as well? The answer is flexibility. In this
/// approach style becomes not necessary and the value can be hardcoded. Also, the values of such
/// properties can be updated individually.
#[derive(Clone, Debug, Default, Reflect)]
pub struct StyledProperty<T> {
    /// Property value.
    pub property: T,
    /// Name of the property in a style table.
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
    /// Creates a new styled property with the given value and the name.
    pub fn new(property: T, name: impl Into<ImmutableString>) -> Self {
        Self {
            property,
            name: name.into(),
        }
    }

    /// Tries to sync the property value with its respective value in the given style. This method
    /// will fail if the given style does not contain the property.
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

/// Style is a simple container for a named properties. Styles can be based off some other style, thus
/// allowing cascaded styling. Such cascading allows to define some base style with common properties
/// and then create any amount of derived styles. For example, you can define a style for Button widget
/// with corner radius, font size, border thickness and then create two derived styles for light and
/// dark themes that will define colors and brushes. Light or dark theme does not affect all of those
/// base properties, but has different colors.
///
/// Styles can contain only specific types of properties (see [`StyleProperty`] enumeration), any
/// more complex properties can be built using these primitives.
///
/// There are three major ways of widgets styling:
///
/// 1) During widget building stage - this way involves [`crate::BuildContext`]'s style field. This
/// field defines a style for all widgets that will be built with the context.
/// 2) Message-based style changes - this way is based on [`crate::widget::WidgetMessage::Style`] message
/// that can be sent to a particular widget (or hierarchy) to force them to update styled properties.
/// 3) Global style changes - this way is based on [`crate::UserInterface::set_style`] method, that
/// sends the specified style to all widgets, forcing them to update styled properties.
///
/// The most used methods are 1 and 3. The following examples should clarify how to use these
/// approaches.
///
/// ## Examples
///
/// The following example shows how to use a style during widget building stage:
///
/// ```rust
/// # use fyrox_ui::{
/// #     button::{Button, ButtonBuilder},
/// #     style::{resource::StyleResource, Style},
/// #     widget::WidgetBuilder,
/// #     Thickness, UserInterface,
/// # };
/// #
/// fn build_with_style(ui: &mut UserInterface) {
///     // The context will use UI style by default. You can override it using `ui.set_style(..)`.
///     let ctx = &mut ui.build_ctx();
///
///     // Create a style resource first and assign it to the build context. All widgets built with
///     // the context will use this style.
///     let style = Style::light_style()
///         .with(Button::CORNER_RADIUS, 6.0f32)
///         .with(Button::BORDER_THICKNESS, Thickness::uniform(3.0));
///
///     ctx.style = StyleResource::new_embedded(style);
///
///     // The button will have corner radius of 6.0 points and border thickness of 3.0 points on
///     // each side.
///     ButtonBuilder::new(WidgetBuilder::new()).build(ctx);
/// }
/// ```
///
/// To change UI style globally after it was built, use something like this:
///
/// ```rust
/// use fyrox_ui::{
///     button::Button,
///     style::{resource::StyleResource, Style},
///     Thickness, UserInterface,
/// };
///
/// fn apply_style(ui: &mut UserInterface) {
///     let style = Style::light_style()
///         .with(Button::CORNER_RADIUS, 3.0f32)
///         .with(Button::BORDER_THICKNESS, Thickness::uniform(1.0));
///
///     ui.set_style(StyleResource::new_embedded(style));
/// }
/// ```
#[derive(Visit, Reflect, Default, Debug, TypeUuidProvider)]
#[type_uuid(id = "38a63b49-d765-4c01-8fb5-202cc43d607e")]
pub struct Style {
    parent: Option<StyleResource>,
    variables: FxHashMap<ImmutableString, StyleProperty>,
}

impl Style {
    /// The name of the darkest brush.
    pub const BRUSH_DARKEST: &'static str = "Global.Brush.Darkest";
    /// The name of the darker brush.
    pub const BRUSH_DARKER: &'static str = "Global.Brush.Darker";
    /// The name of the dark brush.
    pub const BRUSH_DARK: &'static str = "Global.Brush.Dark";
    /// The name of the primary brush that is used for the major amount of surface.
    pub const BRUSH_PRIMARY: &'static str = "Global.Brush.Primary";
    /// The name of the slightly lighter primary brush.
    pub const BRUSH_LIGHTER_PRIMARY: &'static str = "Global.Brush.LighterPrimary";
    /// The name of the light brush.
    pub const BRUSH_LIGHT: &'static str = "Global.Brush.Light";
    /// The name of the lighter brush.
    pub const BRUSH_LIGHTER: &'static str = "Global.Brush.Lighter";
    /// The name of the lightest brush.
    pub const BRUSH_LIGHTEST: &'static str = "Global.Brush.Lightest";
    /// The name of the bright brush.
    pub const BRUSH_BRIGHT: &'static str = "Global.Brush.Bright";
    /// The name of the brightest brush.
    pub const BRUSH_BRIGHTEST: &'static str = "Global.Brush.Brightest";
    /// The name of the bright blue brush.
    pub const BRUSH_BRIGHT_BLUE: &'static str = "Global.Brush.BrightBlue";
    /// The name of the dim blue brush.
    pub const BRUSH_DIM_BLUE: &'static str = "Global.Brush.DimBlue";
    /// The name of the text brush.
    pub const BRUSH_TEXT: &'static str = "Global.Brush.Text";
    /// The name of the foreground brush.
    pub const BRUSH_FOREGROUND: &'static str = "Global.Brush.Foreground";
    /// The name of the information brush.
    pub const BRUSH_INFORMATION: &'static str = "Global.Brush.Information";
    /// The name of the warning brush.
    pub const BRUSH_WARNING: &'static str = "Global.Brush.Warning";
    /// The name of the error brush.
    pub const BRUSH_ERROR: &'static str = "Global.Brush.Error";
    /// The name of the ok brush.
    pub const BRUSH_OK: &'static str = "Global.Brush.Ok";
    /// The name of the font size property.
    pub const FONT_SIZE: &'static str = "Global.Font.Size";

    fn base_style() -> Style {
        let mut style = Self::default();

        style
            .set(Self::FONT_SIZE, 14.0f32)
            .merge(&Button::style())
            .merge(&CheckBox::style())
            .merge(&DropdownList::style())
            .merge(&ToggleButton::style());

        style
    }

    /// Creates a new dark style.
    pub fn dark_style() -> Style {
        let mut style = Self::base_style();
        style
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
            .set(Self::BRUSH_OK, Brush::Solid(Color::GREEN));
        style
    }

    /// Creates a new light style.
    pub fn light_style() -> Style {
        let mut style = Self::base_style();
        style
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
            .set(Self::BRUSH_INFORMATION, Brush::Solid(Color::ROYAL_BLUE))
            .set(
                Self::BRUSH_WARNING,
                Brush::Solid(Color::opaque(255, 242, 0)),
            )
            .set(Self::BRUSH_ERROR, Brush::Solid(Color::RED));
        style
    }

    /// The same as [`Self::set`], but takes self as value and essentially allows chained calls in
    /// builder-like style:
    ///
    /// ```rust
    /// # use fyrox_core::color::Color;
    /// # use fyrox_ui::brush::Brush;
    /// # use fyrox_ui::style::Style;
    /// Style::default()
    ///     .with("SomeProperty", 0.2f32)
    ///     .with("SomeOtherProperty", Brush::Solid(Color::WHITE));
    /// ```
    pub fn with(
        mut self,
        name: impl Into<ImmutableString>,
        property: impl Into<StyleProperty>,
    ) -> Self {
        self.set(name, property);
        self
    }

    /// Sets the parent style for this style. Parent style will be used at attempt to fetch properties
    /// that aren't present in this style.
    pub fn set_parent(&mut self, parent: Option<StyleResource>) {
        self.parent = parent;
    }

    /// Returns parent style of this style.
    pub fn parent(&self) -> Option<&StyleResource> {
        self.parent.as_ref()
    }

    /// Merges current style with some other style. This method does not overwrite existing values,
    /// instead it only adds missing values from the other style.
    pub fn merge(&mut self, other: &Self) -> &mut Self {
        for (k, v) in other.variables.iter() {
            if !self.variables.contains_key(k) {
                self.variables.insert(k.clone(), v.clone());
            }
        }
        self
    }

    /// Registers a new property with the given name and value:
    ///
    /// ```rust
    /// # use fyrox_core::color::Color;
    /// # use fyrox_ui::brush::Brush;
    /// # use fyrox_ui::style::Style;
    /// let mut style = Style::default();
    /// style
    ///     .set("SomeProperty", 0.2f32)
    ///     .set("SomeOtherProperty", Brush::Solid(Color::WHITE));
    /// ```
    pub fn set(
        &mut self,
        name: impl Into<ImmutableString>,
        property: impl Into<StyleProperty>,
    ) -> &mut Self {
        self.variables.insert(name.into(), property.into());
        self
    }

    /// Tries to fetch a property with the given name. If the property is not found, this method will
    /// try to search in the parent style (the search is recursive).
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

    /// Tries to fetch a property with the given name and perform type casting to the requested type.
    /// If the property is not found, this method will try to search in the parent style (the search
    /// is recursive).
    pub fn get<P>(&self, name: impl Into<ImmutableString>) -> Option<P>
    where
        StyleProperty: IntoPrimitive<P>,
    {
        self.get_raw(name)
            .and_then(|property| property.into_primitive())
    }

    /// Tries to fetch a property with the given name. If the property is not found, this method will
    /// try to search in the parent style (the search is recursive). If there's no such property at
    /// all, this method will return its default value (define by [`Default`] trait).
    pub fn get_or_default<P>(&self, name: impl Into<ImmutableString>) -> P
    where
        P: Default,
        StyleProperty: IntoPrimitive<P>,
    {
        self.get_raw(name)
            .and_then(|property| property.into_primitive())
            .unwrap_or_default()
    }

    /// Tries to fetch a property with the given name or, if not found, returns the given default value.
    pub fn get_or<P>(&self, name: impl Into<ImmutableString>, default: P) -> P
    where
        StyleProperty: IntoPrimitive<P>,
    {
        self.get(name).unwrap_or(default)
    }

    /// Tries to find a property with the given name or takes the default value of the property's type
    /// and wraps it into [`StyledProperty`], essentially binding the value to the style property.
    pub fn property<P>(&self, name: impl Into<ImmutableString>) -> StyledProperty<P>
    where
        P: Default,
        StyleProperty: IntoPrimitive<P>,
    {
        let name = name.into();
        StyledProperty::new(self.get_or_default(name.clone()), name)
    }

    /// Tries to load a style from the given path.
    pub async fn from_file(
        path: &Path,
        io: &dyn ResourceIo,
        resource_manager: ResourceManager,
    ) -> Result<Self, StyleResourceError> {
        let bytes = io.load_file(path).await?;
        let mut visitor = Visitor::load_from_memory(&bytes)?;
        visitor.blackboard.register(Arc::new(resource_manager));
        let mut style = Style::default();
        style.visit("Style", &mut visitor)?;
        Ok(style)
    }
}
