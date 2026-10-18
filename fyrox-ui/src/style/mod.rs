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

//! Style allows to change the visual appearance of widgets in a centralized manner. It can be considered
//! as a storage for properties that define visual appearance. See [`Style`] docs for more info
//! and usage examples.

pub mod resource;

use crate::{
    Thickness,
    brush::Brush,
    button::Button,
    check_box::CheckBox,
    core::{ImmutableString, color::Color, reflect::prelude::*, visitor::prelude::*},
    dropdown_list::DropdownList,
    style::resource::{StyleResource, StyleResourceError},
    toggle::ToggleButton,
};
use fyrox_resource::untyped::ResourceKind;
use fyrox_resource::{
    io::ResourceIo,
    manager::{BuiltInResource, ResourceManager},
};
use fyrox_texture::TextureResource;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use std::{
    any::{Any, TypeId},
    sync::LazyLock,
};
use std::{path::Path, sync::Arc};
use strum_macros::{AsRefStr, EnumString, VariantNames};

/// A set of potential values for styled properties.
#[derive(Visit, PartialEq, Reflect, Debug, Clone, AsRefStr, EnumString, VariantNames)]
#[reflect(type_uuid = "85b8c1e4-03a2-4a28-acb4-1850d1a29227")]
pub enum StyleProperty {
    /// A numeric property.
    Number(f32),
    /// A thickness property that defines the width of four sides of a rectangles.
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

impl StyleProperty {
    /// Returns type id of the actual value stored in the property. The set of potential types is
    /// finite (see [`StyleProperty`] declaration).
    pub fn value_type_id(&self) -> TypeId {
        match self {
            StyleProperty::Number(v) => v.type_id(),
            StyleProperty::Thickness(v) => v.type_id(),
            StyleProperty::Color(v) => v.type_id(),
            StyleProperty::Brush(v) => v.type_id(),
            StyleProperty::Texture(v) => v.type_id(),
        }
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

/// Default style of the library.
pub static DEFAULT_STYLE: LazyLock<BuiltInResource<Style>> = LazyLock::new(|| {
    BuiltInResource::new_no_source(
        "Default Style",
        StyleResource::new_ok(
            uuid!("1e0716e8-e728-491c-a65b-ca11b15048be"),
            ResourceKind::External,
            Style::dark_style(),
        ),
    )
});

/// Light style of the library.
pub static LIGHT_STYLE: LazyLock<BuiltInResource<Style>> = LazyLock::new(|| {
    BuiltInResource::new_no_source(
        "Light Style",
        StyleResource::new_ok(
            uuid!("05141b18-2a27-4fe3-ae6e-7af11c2e7471"),
            ResourceKind::External,
            Style::light_style(),
        ),
    )
});

/// Name of a property in a style table.
#[derive(Clone, Debug, Default, PartialEq, Hash, Reflect)]
#[reflect(type_uuid = "c4aed405-4999-4381-bd74-2fd08eec2785")]
pub struct StyledPropertyName(pub ImmutableString);

impl Visit for StyledPropertyName {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.0.visit(name, visitor)
    }
}

impl Deref for StyledPropertyName {
    type Target = ImmutableString;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for StyledPropertyName {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A property, that can be bound to a style table. There are two kinds of styled properties:
///
/// 1) [`StyledProperty::External`] - the value of the property is stored in a style table and can be fetched
/// by a certain name. This kind of property forces the UI system to fetch the actual value from a
/// style resource. Style resource is stored in a widget in which the property is located. External
/// properties are a bit slower than the direct ones (see below), but that's the price of flexibility.
/// 2) [`StyledProperty::Value`] - the value of the property itself.
///
/// To get the value of the property, use [`Self::get`] or [`Self::get_or_default`] methods. See their
/// documentation for more info.
#[derive(Clone, Debug, Reflect, AsRefStr, VariantNames)]
#[reflect(
    bounds = "T: Reflect + Clone + PartialEq",
    type_uuid = "71e84137-2832-42a1-bcb5-f8ee95784997"
)]
pub enum StyledProperty<T> {
    /// A value stored in a style table with the specified name.
    External {
        /// Name of the property in a style table.
        #[reflect(display_name = "Property Name")]
        name: StyledPropertyName,
    },
    /// The value itself, not bound to a style table.
    Value(T),
}

// TODO: Replace this with derived impl in Fyrox 2.0 and remove backward compatibility.
impl<T> Visit for StyledProperty<T>
where
    T: Visit + Default,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        // Backward compatibility.
        if visitor.is_reading() {
            let mut value = T::default();
            if value.visit(name, visitor).is_ok() {
                *self = Self::Value(value);
                return Ok(());
            }
        }

        fn id<T>(me: &StyledProperty<T>) -> u32
        where
            T: Visit + Default,
        {
            match me {
                StyledProperty::External { .. } => 0u32,
                StyledProperty::Value(_) => 1u32,
            }
        }
        fn from_id<T>(id: u32) -> Result<StyledProperty<T>, String>
        where
            T: Visit + Default,
        {
            match id {
                0u32 => Ok(StyledProperty::External {
                    name: Default::default(),
                }),
                1u32 => Ok(StyledProperty::Value(Default::default())),
                _ => Err(format!(
                    "Unknown ID for type `{0}`: `{1}`",
                    "StyledProperty", id
                )),
            }
        }

        let mut region = visitor.enter_region(name)?;

        let mut id = id(self);
        id.visit("Id", &mut region)?;

        if region.is_reading() {
            *self = from_id(id)?;
        }
        match self {
            StyledProperty::External { name } => {
                name.visit("Name", &mut region)?;
            }
            StyledProperty::Value(f0) => {
                f0.visit("0", &mut region)?;
            }
        }
        Ok(())
    }
}

impl<T: Default> FromStr for StyledProperty<T> {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "External" => Ok(Self::External {
                name: Default::default(),
            }),
            "Value" => Ok(Self::Value(T::default())),
            _ => Err(()),
        }
    }
}

impl<T: Default> Default for StyledProperty<T> {
    fn default() -> Self {
        Self::Value(T::default())
    }
}

impl<T> From<T> for StyledProperty<T> {
    fn from(value: T) -> Self {
        Self::Value(value)
    }
}

impl<T: PartialEq> PartialEq for StyledProperty<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Value(a), Self::Value(b)) => a.eq(b),
            (Self::External { name: name_a }, Self::External { name: name_b }) => name_a == name_b,
            _ => false,
        }
    }
}

impl<T> StyledProperty<T> {
    /// Creates a new styled property with the given value.
    pub fn new_value(property: T) -> Self {
        Self::Value(property)
    }

    /// Creates a new styled property whose value is stored in a style.
    pub fn new_external(name: impl Into<ImmutableString>) -> Self {
        Self::External {
            name: StyledPropertyName(name.into()),
        }
    }

    /// Tries to fetch the value of this property. `style` arg is mandatory for [`Self::External`]
    /// properties to be correctly resolved. See [`Style::get`] documentation for more info.
    pub fn get(&self, style: &Option<StyleResource>) -> Option<T>
    where
        T: Clone,
        StyleProperty: IntoPrimitive<T>,
    {
        match self {
            StyledProperty::External { name } => match style {
                Some(style_resource) => {
                    let style_data = style_resource.data_ref();
                    style_data.as_loaded_ref()?.get(name.0.clone())
                }
                None => DEFAULT_STYLE.resource.data_ref().get(name.0.clone()),
            },
            StyledProperty::Value(value) => Some(value.clone()),
        }
    }

    /// Same as [`Self::get`], but returns default value if there's no value set for the property.
    pub fn get_or_default(&self, style: &Option<StyleResource>) -> T
    where
        T: Default + Clone,
        StyleProperty: IntoPrimitive<T>,
    {
        self.get(style).unwrap_or_default()
    }
}

/// Named style property container.
#[derive(Visit, PartialEq, Reflect, Clone, Default, Debug)]
#[reflect(type_uuid = "6238f37c-c067-4dd1-be67-6a8bb8853a59")]
pub struct StylePropertyContainer {
    /// Name of the property.
    pub name: ImmutableString,
    /// The actual value of the property.
    pub value: StyleProperty,
}

/// Style is a simple container for a named properties. Styles can be based on some other styles, thus
/// allowing cascaded styling. Such cascading allows to define some base style with common properties
/// and then create any number of derived styles. For example, you can define a style for Button widget
/// with corner radius, font size, border thickness and then create two derived styles for light and
/// dark themes that will define colors and brushes. Light or dark theme does not affect all of those
/// base properties, but has different colors.
///
/// Styles can contain only specific types of properties (see [`StyleProperty`] enumeration), any
/// more complex properties can be built using these primitives.
///
/// There are three major ways of widget styling:
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
///     // The button will have a corner radius of 6.0 points and border thickness of 3.0 points on
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
///
/// ## Limitations
///
/// Style cannot change widget's hierarchy nor add or remove widgets, use prefabs to do that instead.
/// Style can be used only to change properties of widgets in a visual tree. The set of property types
/// is limited to those that listed in [`StyleProperty`].
#[derive(Visit, PartialEq, Reflect, Clone, Default, Debug)]
#[reflect(type_uuid = "38a63b49-d765-4c01-8fb5-202cc43d607e")]
pub struct Style {
    parent: Option<StyleResource>,
    properties: Vec<StylePropertyContainer>,
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
    /// The name of the highlight brush used to highlight widgets with keyboard focus.
    pub const BRUSH_HIGHLIGHT: &'static str = "Global.Brush.Highlight";
    /// The name of the font size property.
    pub const FONT_SIZE: &'static str = "Global.Font.Size";
    /// The name of the normal state brush of the `ok` context action.
    pub const BRUSH_OK_NORMAL: &'static str = "Global.Brush.Ok.Normal";
    /// The name of the pressed state brush of the `ok` context action.
    pub const BRUSH_OK_PRESSED: &'static str = "Global.Brush.Ok.Pressed";
    /// The name of the hover state brush of the `ok` context action.
    pub const BRUSH_OK_HOVER: &'static str = "Global.Brush.Ok.Hover";
    /// The name of the normal state brush of the `cancel` context action.
    pub const BRUSH_CANCEL_NORMAL: &'static str = "Global.Brush.Cancel.Normal";
    /// The name of the pressed state brush of the `cancel` context action.
    pub const BRUSH_CANCEL_PRESSED: &'static str = "Global.Brush.Cancel.Pressed";
    /// The name of the hover state brush of the `cancel` context action.
    pub const BRUSH_CANCEL_HOVER: &'static str = "Global.Brush.Cancel.Hover";

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
                Self::BRUSH_HIGHLIGHT,
                Brush::Solid(Color::opaque(80, 118, 178)),
            )
            .set(
                Self::BRUSH_DIM_BLUE,
                Brush::Solid(Color::opaque(66, 99, 149)),
            )
            .set(Self::BRUSH_TEXT, Brush::Solid(Color::opaque(190, 190, 190)))
            .set(Self::BRUSH_FOREGROUND, Brush::Solid(Color::WHITE))
            .set(Self::BRUSH_INFORMATION, Brush::Solid(Color::ANTIQUE_WHITE))
            .set(Self::BRUSH_WARNING, Brush::Solid(Color::GOLD))
            .set(Self::BRUSH_ERROR, Brush::Solid(Color::RED))
            .set(Self::BRUSH_OK, Brush::Solid(Color::GREEN))
            .set(
                Self::BRUSH_OK_NORMAL,
                Brush::Solid(Color::opaque(0, 130, 0)),
            )
            .set(Self::BRUSH_OK_HOVER, Brush::Solid(Color::opaque(0, 150, 0)))
            .set(
                Self::BRUSH_OK_PRESSED,
                Brush::Solid(Color::opaque(0, 170, 0)),
            )
            .set(
                Self::BRUSH_CANCEL_NORMAL,
                Brush::Solid(Color::opaque(130, 0, 0)),
            )
            .set(
                Self::BRUSH_CANCEL_HOVER,
                Brush::Solid(Color::opaque(150, 0, 0)),
            )
            .set(
                Self::BRUSH_CANCEL_PRESSED,
                Brush::Solid(Color::opaque(170, 0, 0)),
            );
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
                Self::BRUSH_HIGHLIGHT,
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

    /// Sets the parent style for this style. Parent style will be used in an attempt to fetch properties
    /// that aren't present in this style.
    pub fn set_parent(&mut self, parent: Option<StyleResource>) {
        self.parent = parent;
    }

    /// Returns parent style of this style.
    pub fn parent(&self) -> Option<&StyleResource> {
        self.parent.as_ref()
    }

    /// Returns an index of the variable with the given name.
    pub fn index_of(&self, name: &ImmutableString) -> Option<usize> {
        self.properties
            .binary_search_by(|v| v.name.cached_hash().cmp(&name.cached_hash()))
            .ok()
    }

    /// Checks if there's a variable with the given name.
    pub fn contains(&self, name: &ImmutableString) -> bool {
        self.index_of(name).is_some()
    }

    /// Merges current style with some other style. This method does not overwrite existing values,
    /// instead it only adds missing values from the other style.
    pub fn merge(&mut self, other: &Self) -> &mut Self {
        for other_property in other.properties.iter() {
            if !self.contains(&other_property.name) {
                self.set(other_property.name.clone(), other_property.value.clone());
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
        value: impl Into<StyleProperty>,
    ) -> &mut Self {
        let name = name.into();
        let value = value.into();

        if let Some(existing_index) = self.index_of(&name) {
            self.properties[existing_index] = StylePropertyContainer { name, value };
        } else {
            let index = self
                .properties
                .partition_point(|h| h.name.cached_hash() < name.cached_hash());
            self.properties
                .insert(index, StylePropertyContainer { name, value });
        }

        self
    }

    /// Tries to fetch a property with the given name. If the property is not found, this method will
    /// try to search in the parent style (the search is recursive).
    pub fn get_raw(&self, name: impl Into<ImmutableString>) -> Option<StyleProperty> {
        let name = name.into();
        let index = self.index_of(&name)?;
        if let Some(container) = self.properties.get(index) {
            return Some(container.value.clone());
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
    /// all, this method will return its default value (defined by [`Default`] trait implementation).
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
        StyledProperty::new_external(name)
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

    /// Returns an immutable reference to the internal container with the style properties.
    /// Keep in mind that the returned container contains only the properties of the current
    /// style! Properties of the parent style(s) should be obtained separately.
    pub fn inner(&self) -> &Vec<StylePropertyContainer> {
        &self.properties
    }

    /// Collects all the properties in the current and ancestor style chain. Returns a hash map with
    /// all property values with their names. Basically, this method merges all the styles with their
    /// ancestor style chain.
    pub fn all_properties(&self) -> Self {
        let mut properties = self
            .parent
            .as_ref()
            .map(|parent| parent.data_ref().all_properties())
            .unwrap_or_default();
        for property in self.properties.iter() {
            properties.set(property.name.clone(), property.value.clone());
        }
        properties
    }
}

#[cfg(test)]
mod test {
    use crate::brush::Brush;
    use crate::style::resource::StyleResource;
    use crate::style::{Style, StyledProperty};
    use fyrox_core::ImmutableString;
    use fyrox_core::color::Color;

    fn make_test_style() -> Style {
        let mut style = Style::default();
        style
            .set("A", 0.2f32)
            .set("D", 0.1f32)
            .set("B", Brush::Solid(Color::WHITE))
            .set("C", Brush::Solid(Color::WHITE));
        style
    }

    fn make_test_style_resource() -> StyleResource {
        StyleResource::new_embedded(make_test_style())
    }

    #[test]
    fn test_style() {
        let style = make_test_style();
        assert_eq!(style.index_of(&ImmutableString::new("A")), Some(3));
        assert_eq!(style.index_of(&ImmutableString::new("B")), Some(2));
        assert_eq!(style.index_of(&ImmutableString::new("C")), Some(1));
        assert_eq!(style.index_of(&ImmutableString::new("D")), Some(0));
    }

    #[test]
    fn test_styled_property() {
        let style = Some(make_test_style_resource());
        let styled_prop = StyledProperty::<f32>::new_external("A");
        assert_eq!(styled_prop.get_or_default(&style), 0.2f32);
        let styled_prop = StyledProperty::<f32>::new_external("D");
        assert_eq!(styled_prop.get_or_default(&style), 0.1f32);
        let styled_prop = StyledProperty::<Brush>::new_external("B");
        assert_eq!(
            styled_prop.get_or_default(&style),
            Brush::Solid(Color::WHITE)
        );
        let styled_prop = StyledProperty::<Brush>::new_external("C");
        assert_eq!(
            styled_prop.get_or_default(&style),
            Brush::Solid(Color::WHITE)
        );
        let styled_prop = StyledProperty::<f32>::new_external("E");
        assert_eq!(styled_prop.get(&style), None);
        assert_eq!(styled_prop.get_or_default(&style), 0.0f32);
    }
}
