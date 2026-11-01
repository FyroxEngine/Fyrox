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

//! A set of bits that can be used to store up to 32 individual boolean values.

use crate::core::{
    num_traits::{NumCast, One, ToPrimitive, Zero},
    reflect::prelude::*,
    visitor::prelude::*,
};
use std::{
    fmt::Write,
    ops::{Add, BitAnd, BitOr, Mul, Not, Shl},
};

/// A set of bits that can be used to store up to 32 individual boolean values.
#[derive(Clone, Copy, Default, PartialEq, Reflect, Eq)]
#[reflect(type_uuid = "f2db0c2a-921b-4728-9ce4-2506d95c60fa")]
pub struct BitMask(pub u32);

impl BitMask {
    /// BitMask with all bits set. BitMask(u32::MAX)
    pub const fn all() -> Self {
        Self(u32::MAX)
    }

    /// BitMask with no bits set. BitMask(0)
    pub const fn none() -> Self {
        Self(0)
    }

    /// Construct BitMask from this BitMask plus setting bit at the given index to 1.
    pub const fn with(self, index: usize) -> Self {
        Self(self.0 | (1 << index))
    }

    /// Construct BitMask from this BitMask plus resetting the bit at the given index to 0.
    pub const fn without(self, index: usize) -> Self {
        Self(self.0 & !(1 << index))
    }

    /// True if the bit at `index` is set.
    pub fn bit(&self, index: usize) -> bool {
        (self.0 >> index) & 1 != 0
    }

    /// Set or reset the bit at `index`.
    pub fn set_bit(&mut self, index: usize, value: bool) {
        if value {
            self.0 |= 1 << index;
        } else {
            self.0 &= !(1 << index);
        }
    }
}

impl std::fmt::Debug for BitMask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BitMask({:08x})", self.0)
    }
}

impl std::fmt::Display for BitMask {
    /// Represent the bit mask in the same way it is shown in the editor, with bit 0 on the left and bit 31 on the right.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut value = self.0;
        for i in 0..4 {
            if i != 0 {
                f.write_char(' ')?;
            }
            for _ in 0..8 {
                let bit = if value & 1 == 1 { '1' } else { '0' };
                value >>= 1;
                f.write_char(bit)?;
            }
        }
        Ok(())
    }
}

impl Visit for BitMask {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.0.visit(name, visitor)
    }
}

impl BitOr for BitMask {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitAnd for BitMask {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl Mul for BitMask {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl One for BitMask {
    fn one() -> Self {
        Self(1)
    }
}

impl Add for BitMask {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Zero for BitMask {
    fn zero() -> Self {
        Self(0)
    }

    fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

impl Shl for BitMask {
    type Output = Self;

    fn shl(self, rhs: Self) -> Self::Output {
        Self(self.0 << rhs.0)
    }
}

impl Not for BitMask {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

impl ToPrimitive for BitMask {
    fn to_i64(&self) -> Option<i64> {
        Some(self.0 as i64)
    }

    fn to_u64(&self) -> Option<u64> {
        Some(self.0 as u64)
    }
}

impl NumCast for BitMask {
    fn from<T: ToPrimitive>(n: T) -> Option<Self> {
        n.to_u32().map(Self)
    }
}
