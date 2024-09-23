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

pub use fyrox_core as core;

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
pub mod pixel_buffer;
pub mod query;
pub mod state;
pub mod stats;

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
)]
#[type_uuid(id = "47aff01a-7daa-427c-874c-87464a7ffe28")]
pub enum PolygonFillMode {
    Point,
    Line,
    Fill,
}

impl Default for PolygonFillMode {
    fn default() -> Self {
        Self::Fill
    }
}

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
)]
pub enum CompareFunc {
    /// Never passes.
    Never,

    /// Passes if the incoming value is less than the stored value.
    Less,

    /// Passes if the incoming value is equal to the stored value.
    Equal,

    /// Passes if the incoming value is less than or equal to the stored value.
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

impl Default for CompareFunc {
    fn default() -> Self {
        Self::LessOrEqual
    }
}

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
)]
pub enum BlendFactor {
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

impl Default for BlendFactor {
    fn default() -> Self {
        Self::Zero
    }
}

#[derive(
    Copy, Clone, Hash, PartialOrd, PartialEq, Eq, Ord, Serialize, Deserialize, Visit, Debug, Reflect,
)]
pub enum BlendMode {
    Add,
    Subtract,
    ReverseSubtract,
    Min,
    Max,
}

impl Default for BlendMode {
    fn default() -> Self {
        Self::Add
    }
}

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
    pub rgb: BlendMode,
    pub alpha: BlendMode,
}

#[derive(
    Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Hash, Serialize, Deserialize, Visit, Debug, Reflect,
)]
pub struct BlendFunc {
    pub sfactor: BlendFactor,
    pub dfactor: BlendFactor,
    pub alpha_sfactor: BlendFactor,
    pub alpha_dfactor: BlendFactor,
}

impl BlendFunc {
    pub fn new(sfactor: BlendFactor, dfactor: BlendFactor) -> Self {
        Self {
            sfactor,
            dfactor,
            alpha_sfactor: sfactor,
            alpha_dfactor: dfactor,
        }
    }

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

#[derive(
    Copy, Clone, PartialOrd, PartialEq, Hash, Debug, Serialize, Deserialize, Visit, Eq, Reflect,
)]
pub struct ColorMask {
    pub red: bool,
    pub green: bool,
    pub blue: bool,
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
    pub fn all(value: bool) -> Self {
        Self {
            red: value,
            green: value,
            blue: value,
            alpha: value,
        }
    }
}

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
)]
pub enum PolygonFace {
    Front,
    Back,
    FrontAndBack,
}

impl Default for PolygonFace {
    fn default() -> Self {
        Self::FrontAndBack
    }
}

#[derive(
    Copy, Clone, PartialOrd, PartialEq, Hash, Debug, Serialize, Deserialize, Visit, Eq, Reflect,
)]
pub struct StencilFunc {
    pub func: CompareFunc,
    pub ref_value: u32,
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
)]
pub enum StencilAction {
    /// Keeps the current value.
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

impl Default for StencilAction {
    fn default() -> Self {
        Self::Keep
    }
}

#[derive(
    Copy, Clone, PartialOrd, PartialEq, Hash, Debug, Serialize, Deserialize, Visit, Eq, Reflect,
)]
pub struct StencilOp {
    pub fail: StencilAction,
    pub zfail: StencilAction,
    pub zpass: StencilAction,
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

#[derive(
    Copy, Clone, PartialOrd, PartialEq, Hash, Debug, Serialize, Deserialize, Visit, Eq, Reflect,
)]
pub enum CullFace {
    Back,
    Front,
}

impl Default for CullFace {
    fn default() -> Self {
        Self::Back
    }
}

#[derive(Serialize, Deserialize, Default, Visit, Debug, PartialEq, Clone, Eq, Reflect)]
pub struct BlendParameters {
    pub func: BlendFunc,
    pub equation: BlendEquation,
}

#[derive(Serialize, Deserialize, Default, Visit, Debug, PartialEq, Clone, Copy, Eq, Reflect)]
pub struct ScissorBox {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Serialize, Deserialize, Visit, Debug, PartialEq, Clone, Eq, Reflect)]
pub struct DrawParameters {
    pub cull_face: Option<CullFace>,
    pub color_write: ColorMask,
    pub depth_write: bool,
    pub stencil_test: Option<StencilFunc>,
    pub depth_test: Option<CompareFunc>,
    pub blend: Option<BlendParameters>,
    pub stencil_op: StencilOp,
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ElementRange {
    Full,
    Specific { offset: usize, count: usize },
}

impl Default for ElementRange {
    fn default() -> Self {
        Self::Full
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ElementKind {
    Triangle,
    Line,
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
