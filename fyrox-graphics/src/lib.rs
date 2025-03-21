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

#![allow(clippy::too_many_arguments)]

pub use fyrox_core as core;
use std::fmt::Debug;

use crate::core::{reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*};
use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, EnumString, VariantNames};

pub mod buffer;
pub mod error;
pub mod framebuffer;
pub mod geometry_buffer;
pub mod gl;
pub mod gpu_program;
pub mod gpu_texture;
pub mod query;
pub mod read_buffer;
pub mod sampler;
pub mod server;
pub mod stats;
pub mod uniform;

#[macro_export]
macro_rules! define_shared_wrapper {
    ($name:ident<$ty:ty>) => {
        #[derive(Clone)]
        #[doc(hidden)]
        pub struct $name(pub std::rc::Rc<$ty>);

        impl std::ops::Deref for $name {
            type Target = $ty;

            fn deref(&self) -> &Self::Target {
                self.0.deref()
            }
        }
    };
}

/// A set of possible polygon filling modes.
#[derive(
    Copy,
    Clone,
    PartialOrd,
    PartialEq,
    Hash,
    Debug,
    Deserialize,
    Visit,
    Eq,
    Reflect,
    AsRefStr,
    EnumString,
    VariantNames,
    TypeUuidProvider,
    Default,
)]
#[type_uuid(id = "47aff01a-7daa-427c-874c-87464a7ffe28")]
pub enum PolygonFillMode {
    /// Only vertices of polygons are rendered. Their size is 1px by default.
    Point,
    /// Only edges of polygons are rendered using 1px lines. This mode is useful for wireframe
    /// drawing.
    Line,
    /// The entire polygon surface is rendered. This is default rendering mode.
    #[default]
    Fill,
}

/// A function used to compare two values. Usually it is used for depth and stencil testing.
#[derive(
    Copy,
    Clone,
    PartialOrd,
    PartialEq,
    Eq,
    Ord,
    Hash,
    Visit,
    Serialize,
    Deserialize,
    Debug,
    Reflect,
    AsRefStr,
    EnumString,
    VariantNames,
    Default,
)]
pub enum CompareFunc {
    /// Never passes.
    Never,

    /// Passes if the incoming value is less than the stored value.
    Less,

    /// Passes if the incoming value is equal to the stored value.
    Equal,

    /// Passes if the incoming value is less than or equal to the stored value.
    #[default]
    LessOrEqual,

    /// Passes if the incoming value is greater than the stored value.
    Greater,

    /// Passes if the incoming value is not equal to the stored value.
    NotEqual,

    /// Passes if the incoming value is greater than or equal to the stored value.
    GreaterOrEqual,

    /// Always passes.
    Always,
}

/// Defines a set values (per color) for blending operation.
#[derive(
    Copy,
    Clone,
    Hash,
    PartialOrd,
    PartialEq,
    Eq,
    Ord,
    Serialize,
    Deserialize,
    Visit,
    Debug,
    Reflect,
    AsRefStr,
    EnumString,
    VariantNames,
    Default,
)]
pub enum BlendFactor {
    #[default]
    Zero,
    One,
    SrcColor,
    OneMinusSrcColor,
    DstColor,
    OneMinusDstColor,
    SrcAlpha,
    OneMinusSrcAlpha,
    DstAlpha,
    OneMinusDstAlpha,
    ConstantColor,
    OneMinusConstantColor,
    ConstantAlpha,
    OneMinusConstantAlpha,
    SrcAlphaSaturate,
    Src1Color,
    OneMinusSrc1Color,
    Src1Alpha,
    OneMinusSrc1Alpha,
}

/// Defines an operation used in blending equation.
#[derive(
    Copy,
    Clone,
    Hash,
    PartialOrd,
    PartialEq,
    Eq,
    Ord,
    Serialize,
    Deserialize,
    Visit,
    Debug,
    Reflect,
    Default,
)]
pub enum BlendMode {
    /// Addition of two operands (`Source + Dest`). This is default operation.
    #[default]
    Add,
    /// Subtraction of two operands (`Source - Dest`).
    Subtract,
    /// Reverse subtraction of two operands (`Dest - Source`).
    ReverseSubtract,
    /// Per-component min function of two operands (`min(Source, Dest)`).
    Min,
    /// Per-component max function of two operands (`max(Source, Dest)`).
    Max,
}

/// An equation used for blending a source pixel color with the destination color (the one that is
/// already in a frame buffer). This equation has different modes for rgb/alpha parts.
#[derive(
    Copy,
    Clone,
    Default,
    PartialOrd,
    PartialEq,
    Ord,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Visit,
    Debug,
    Reflect,
)]
pub struct BlendEquation {
    /// An operation for RGB part.
    pub rgb: BlendMode,
    /// An operation for alpha part.
    pub alpha: BlendMode,
}

/// Blending function defines sources of data for both operands in blending equation (separately
/// for RGB and Alpha parts). Default blending function is replacing destination values with the
/// source ones.
#[derive(
    Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Hash, Serialize, Deserialize, Visit, Debug, Reflect,
)]
pub struct BlendFunc {
    /// Data for source (the value that is produced by a shader) in the blending equation (RGB part).
    pub sfactor: BlendFactor,
    /// Data for destination (the value that is already in a frame buffer) in the blending equation
    /// (RGB part).
    pub dfactor: BlendFactor,
    /// Data for source (the value that is produced by a shader) in the blending equation (alpha part).
    pub alpha_sfactor: BlendFactor,
    /// Data for destination (the value that is already in a frame buffer) in the blending equation
    /// (alpha part).
    pub alpha_dfactor: BlendFactor,
}

impl BlendFunc {
    /// Creates a new blending function where both RGB and Alpha parts have the same blending factor.
    pub fn new(sfactor: BlendFactor, dfactor: BlendFactor) -> Self {
        Self {
            sfactor,
            dfactor,
            alpha_sfactor: sfactor,
            alpha_dfactor: dfactor,
        }
    }

    /// Creates a new blending function where RGB and Alpha parts have different blending factor.
    pub fn new_separate(
        sfactor: BlendFactor,
        dfactor: BlendFactor,
        alpha_sfactor: BlendFactor,
        alpha_dfactor: BlendFactor,
    ) -> Self {
        Self {
            sfactor,
            dfactor,
            alpha_sfactor,
            alpha_dfactor,
        }
    }
}

impl Default for BlendFunc {
    fn default() -> Self {
        Self {
            sfactor: BlendFactor::One,
            dfactor: BlendFactor::Zero,
            alpha_sfactor: BlendFactor::One,
            alpha_dfactor: BlendFactor::Zero,
        }
    }
}

/// A mask that defines which colors will be stored in a frame buffer during rendering operation.
/// By default, all colors are stored (every field is set to `true`).
#[derive(
    Copy, Clone, PartialOrd, PartialEq, Hash, Debug, Serialize, Deserialize, Visit, Eq, Reflect,
)]
pub struct ColorMask {
    /// A flag, that defines whether the red channel is written or not in a frame buffer.
    pub red: bool,
    /// A flag, that defines whether the green channel is written or not in a frame buffer.
    pub green: bool,
    /// A flag, that defines whether the blue channel is written or not in a frame buffer.
    pub blue: bool,
    /// A flag, that defines whether the alpha channel is written or not in a frame buffer.
    pub alpha: bool,
}

impl Default for ColorMask {
    fn default() -> Self {
        Self {
            red: true,
            green: true,
            blue: true,
            alpha: true,
        }
    }
}

impl ColorMask {
    /// Creates a new color mask where all the components will have the specified value.
    pub fn all(value: bool) -> Self {
        Self {
            red: value,
            green: value,
            blue: value,
            alpha: value,
        }
    }
}

/// Defines a polygon face that will be rendered. This is usually used for back face culling.
/// Default value is [`PolygonFace::FrontAndBack`].
#[derive(
    Copy,
    Clone,
    PartialOrd,
    PartialEq,
    Hash,
    Debug,
    Deserialize,
    Visit,
    Eq,
    Reflect,
    AsRefStr,
    EnumString,
    VariantNames,
    Default,
)]
pub enum PolygonFace {
    /// Only front faces will be rendered.
    Front,
    /// Only back faces will be rendered.
    Back,
    /// Both, back and front faces will be rendered.
    #[default]
    FrontAndBack,
}

/// Defines a function that used in a stencil test.
#[derive(
    Copy, Clone, PartialOrd, PartialEq, Hash, Debug, Serialize, Deserialize, Visit, Eq, Reflect,
)]
pub struct StencilFunc {
    /// A function that is used to compare two values. Default value is [`CompareFunc::Always`].
    pub func: CompareFunc,
    /// Reference value that is used to compare against the current value in the stencil buffer.
    /// Default value is 0.
    pub ref_value: u32,
    /// A mask value that is used to filter out some bits (using logical AND operation) of a value
    /// in the stencil buffer and the ref value (for example if a [`CompareFunc::Less`] is used
    /// then the entire equation will look like `(ref & mask) < (stencil & mask)`). Default value
    /// is `0xFFFFFFFF`.
    pub mask: u32,
}

impl Default for StencilFunc {
    fn default() -> Self {
        Self {
            func: CompareFunc::Always,
            ref_value: 0,
            mask: 0xFFFF_FFFF,
        }
    }
}

/// An action with the stencil value in the stencil buffer is the stencil test passed.
#[derive(
    Copy,
    Clone,
    PartialOrd,
    PartialEq,
    Hash,
    Debug,
    Serialize,
    Deserialize,
    Visit,
    Eq,
    Reflect,
    AsRefStr,
    EnumString,
    VariantNames,
    Default,
)]
pub enum StencilAction {
    /// Keeps the current value. This is the default variant.
    #[default]
    Keep,

    /// Sets the stencil buffer value to 0.
    Zero,

    /// Sets the stencil buffer value to ref value.
    Replace,

    /// Increments the current stencil buffer value.
    /// Clamps to the maximum representable unsigned value.
    Incr,

    /// Increments the current stencil buffer value.
    /// Wraps stencil buffer value to zero when incrementing the maximum representable
    /// unsigned value.
    IncrWrap,

    /// Decrements the current stencil buffer value.
    /// Clamps to 0.
    Decr,

    /// Decrements the current stencil buffer value.
    /// Wraps stencil buffer value to the maximum representable unsigned value when
    /// decrementing a stencil buffer value of zero.
    DecrWrap,

    /// Bitwise inverts the current stencil buffer value.
    Invert,
}

/// A set of actions that will be performed with the stencil buffer during various testing stages.
#[derive(
    Copy, Clone, PartialOrd, PartialEq, Hash, Debug, Serialize, Deserialize, Visit, Eq, Reflect,
)]
pub struct StencilOp {
    /// An action that happens when the stencil test has failed.
    pub fail: StencilAction,
    /// An action that happens when the depth test has failed.
    pub zfail: StencilAction,
    /// An action that happens when the depth test has passed.
    pub zpass: StencilAction,
    /// A mask that is used to filter out some bits (using `AND` logical operation) from the source
    /// value before writing it to the stencil buffer.
    pub write_mask: u32,
}

impl Default for StencilOp {
    fn default() -> Self {
        Self {
            fail: Default::default(),
            zfail: Default::default(),
            zpass: Default::default(),
            write_mask: 0xFFFF_FFFF,
        }
    }
}

/// A face side to cull.
#[derive(
    Copy,
    Clone,
    PartialOrd,
    PartialEq,
    Hash,
    Debug,
    Serialize,
    Deserialize,
    Visit,
    Eq,
    Reflect,
    Default,
)]
pub enum CullFace {
    /// Cull only back faces.
    #[default]
    Back,
    /// Cull only front faces.
    Front,
}

/// Blending parameters (such as blending function and its equation).
#[derive(Serialize, Deserialize, Default, Visit, Debug, PartialEq, Clone, Eq, Reflect)]
pub struct BlendParameters {
    /// Blending function, see [`BlendFunc`] for more info.
    pub func: BlendFunc,
    /// Blending equation, see [`BlendEquation`] for more info.
    pub equation: BlendEquation,
}

/// A rectangular area that defines which pixels will be rendered in a frame buffer or not.
#[derive(Serialize, Deserialize, Default, Visit, Debug, PartialEq, Clone, Copy, Eq, Reflect)]
pub struct ScissorBox {
    /// X coordinate of the box's origin.
    pub x: i32,
    /// Y coordinate of the box's origin. Located at the bottom of the rectangle.
    pub y: i32,
    /// Width of the box.
    pub width: i32,
    /// Height of the box.
    pub height: i32,
}

/// A set of drawing parameters, that are used during draw call. It defines pretty much all pipeline
/// settings all at once.
#[derive(Serialize, Deserialize, Visit, Debug, PartialEq, Clone, Eq, Reflect)]
pub struct DrawParameters {
    /// An optional cull face. If [`None`], then the culling is disabled.
    pub cull_face: Option<CullFace>,
    /// Color write mask.
    pub color_write: ColorMask,
    /// A flag, that defines whether the depth values should be written to the depth buffer or not.
    pub depth_write: bool,
    /// Stencil test options. If [`None`], then the stencil test is disabled.
    pub stencil_test: Option<StencilFunc>,
    /// Depth test options. If [`None`], then the depth test is disabled.
    pub depth_test: Option<CompareFunc>,
    /// Blending options. If [`None`], then the blending is disabled.
    pub blend: Option<BlendParameters>,
    /// Stencil operation.
    pub stencil_op: StencilOp,
    /// Optional scissor box. If [`None`], then the scissor test is disabled.
    pub scissor_box: Option<ScissorBox>,
}

impl Default for DrawParameters {
    fn default() -> Self {
        Self {
            cull_face: Some(CullFace::Back),
            color_write: Default::default(),
            depth_write: true,
            stencil_test: None,
            depth_test: Some(CompareFunc::Less),
            blend: None,
            stencil_op: Default::default(),
            scissor_box: None,
        }
    }
}

/// A range of elements (usually it's triangles) to draw in a draw call.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum ElementRange {
    /// All available elements. This is the default option.
    #[default]
    Full,
    /// Specific range of elements. Useful if you have a large buffer that contains multiple smaller
    /// elements at once.
    Specific {
        /// Offset (in elements) from the beginning of the buffer.
        offset: usize,
        /// Total count of elements to draw.
        count: usize,
    },
}

/// Element kind of geometry.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ElementKind {
    /// Triangles.
    Triangle,
    /// Lines.
    Line,
    /// Points.
    Point,
}

impl ElementKind {
    fn index_per_element(self) -> usize {
        match self {
            ElementKind::Triangle => 3,
            ElementKind::Line => 2,
            ElementKind::Point => 1,
        }
    }
}
