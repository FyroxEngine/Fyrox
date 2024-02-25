//! Alignment defines relative location of the widget to its parent widget. There are two kinds of alignment:
//! [`HorizontalAlignment`] and [`VerticalAlignment`]. Check the docs for them for more info.

use crate::core::{reflect::prelude::*, visitor::prelude::*};
use fyrox_core::uuid_provider;
use strum_macros::{AsRefStr, EnumString, VariantNames};

/// Horizontal alignment defines relative location and size of the widget to its parent widget along horizontal
/// (X) axis.
#[derive(
    Copy, Clone, PartialEq, Debug, Eq, Default, Reflect, Visit, AsRefStr, EnumString, VariantNames,
)]
pub enum HorizontalAlignment {
    /// Tells the widget to take all available space along horizontal axis and stay at left side of the
    /// parent widget. This is default horizontal alignment for all widgets.
    #[default]
    Stretch,
    /// Tells the widget to stay at the left side of the parent widget and take as less space as
    /// possible (shrink-to-fit).
    Left,
    /// Tells the widget to stay at the center of the parent widget and take as less space as possible
    /// (shrink-to-fit).
    Center,
    /// Tells the widget to stay at the right side of the parent widget and take as less space as
    /// possible (shrink-to-fit).
    Right,
}

uuid_provider!(HorizontalAlignment = "ef571515-ec16-47ad-bfe3-ddc259e2c7d3");

/// Horizontal alignment defines relative location and size of the widget to its parent widget along vertical
/// (Y) axis.
#[derive(
    Copy, Clone, PartialEq, Debug, Eq, Default, Reflect, Visit, AsRefStr, EnumString, VariantNames,
)]
pub enum VerticalAlignment {
    /// Tells the widget to take all available space along vertical axis and stay at top side of the
    /// parent widget. This is default vertical alignment for all widgets.
    #[default]
    Stretch,
    /// Tells the widget to stay at the top side of the parent widget and take as less space as
    /// possible (shrink-to-fit).
    Top,
    /// Tells the widget to stay at the center of the parent widget and take as less space as possible
    /// (shrink-to-fit).
    Center,
    /// Tells the widget to stay at the bottom side of the parent widget and take as less space as
    /// possible (shrink-to-fit).
    Bottom,
}

uuid_provider!(VerticalAlignment = "8555dc0d-c9b7-4c49-816a-a7f610a6886d");
