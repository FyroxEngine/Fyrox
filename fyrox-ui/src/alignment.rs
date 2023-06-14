//! Alignment defines relative location of the widget to its parent widget. There are two kinds of alignment:
//! [`HorizontalAlignment`] and [`VerticalAlignment`]. Check the docs for them for more info.

/// Horizontal alignment defines relative location and size of the widget to its parent widget along horizontal
/// (X) axis.
#[derive(Copy, Clone, PartialEq, Debug, Eq, Default)]
pub enum HorizontalAlignment {
    /// Tells the widget to take all available space along horizontal axis and stay at left side of the
    /// parent widget. This is default vertical alignment for all widgets.
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

/// Horizontal alignment defines relative location and size of the widget to its parent widget along vertical
/// (Y) axis.
#[derive(Copy, Clone, PartialEq, Debug, Eq, Default)]
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
