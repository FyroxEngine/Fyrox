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

use crate::core::type_traits::prelude::*;
use crate::font::FontHeight;
use std::ops::{Deref, DerefMut};

use super::*;

#[deprecated]
pub type RunBuilder = Run;

#[derive(Clone, PartialEq, Debug, Default, Reflect)]
pub struct RunSet(Vec<Run>);

impl IntoIterator for RunSet {
    type Item = Run;
    type IntoIter = std::vec::IntoIter<Run>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl From<&[Run]> for RunSet {
    fn from(value: &[Run]) -> Self {
        Self(value.to_vec())
    }
}

impl From<Vec<Run>> for RunSet {
    fn from(value: Vec<Run>) -> Self {
        Self(value)
    }
}

impl Visit for RunSet {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.0.visit(name, visitor)
    }
}

impl Deref for RunSet {
    type Target = Vec<Run>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RunSet {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl RunSet {
    /// Updates the run set with the given run, overriding any previous runs
    /// that may have set some of the same formatting attributes in the same range.
    /// Runs within this set may be merged if appropriate.
    pub fn push(&mut self, run: Run) {
        if let Some(last) = self.0.last_mut() {
            if last.range == run.range {
                *last = last.clone().with_values_from(run);
            } else {
                self.0.push(run);
            }
        } else {
            self.0.push(run);
        }
    }
    /// Find the font at the given position.
    pub fn font_at(&self, index: usize) -> Option<FontResource> {
        for run in self.0.iter().rev() {
            if run.range.contains(&(index as u32)) && run.font().is_some() {
                return Some(run.font().unwrap().clone());
            }
        }
        None
    }
    /// Find the size at the given position.
    pub fn font_size_at(&self, index: usize) -> Option<f32> {
        for run in self.0.iter().rev() {
            if run.range.contains(&(index as u32)) && run.font_size().is_some() {
                return Some(run.font_size().unwrap());
            }
        }
        None
    }
    /// Find the brush at the given position.
    pub fn brush_at(&self, index: usize) -> Option<Brush> {
        for run in self.0.iter().rev() {
            if run.range.contains(&(index as u32)) && run.brush().is_some() {
                return Some(run.brush().unwrap().clone());
            }
        }
        None
    }
    /// Find whether the text shadow is enabled at the given position.
    pub fn shadow_at(&self, index: usize) -> Option<bool> {
        for run in self.0.iter().rev() {
            if run.range.contains(&(index as u32)) && run.shadow().is_some() {
                return Some(run.shadow().unwrap());
            }
        }
        None
    }
    /// Find the shadow brush at the given position.
    pub fn shadow_brush_at(&self, index: usize) -> Option<Brush> {
        for run in self.0.iter().rev() {
            if run.range.contains(&(index as u32)) && run.shadow_brush().is_some() {
                return Some(run.shadow_brush().unwrap().clone());
            }
        }
        None
    }
    /// Find the shadow dilation at the given position.
    pub fn shadow_dilation_at(&self, index: usize) -> Option<f32> {
        for run in self.0.iter().rev() {
            if run.range.contains(&(index as u32)) && run.shadow_dilation().is_some() {
                return Some(run.shadow_dilation().unwrap());
            }
        }
        None
    }
    /// Find the shadow offset at the given position.
    pub fn shadow_offset_at(&self, index: usize) -> Option<Vector2<f32>> {
        for run in self.0.iter().rev() {
            if run.range.contains(&(index as u32)) && run.shadow_offset().is_some() {
                return Some(run.shadow_offset().unwrap());
            }
        }
        None
    }
}

/// The style of a portion of text within a range.
#[derive(Clone, PartialEq, Debug, Default, Visit, Reflect, TypeUuidProvider)]
#[type_uuid(id = "f0e5cc5d-0b82-4d6f-a505-12f890ffe7ea")]
pub struct Run {
    /// The range of characters that this run applies to within the text.
    pub range: Range<u32>,
    font: Option<FontResource>,
    brush: Option<Brush>,
    font_size: Option<f32>,
    shadow: Option<bool>,
    shadow_brush: Option<Brush>,
    shadow_dilation: Option<f32>,
    shadow_offset: Option<Vector2<f32>>,
}

impl Run {
    /// Create a run that sets no formatting values across the given range.
    pub fn new(range: Range<u32>) -> Self {
        Self {
            range,
            font: None,
            brush: None,
            font_size: None,
            shadow: None,
            shadow_brush: None,
            shadow_dilation: None,
            shadow_offset: None,
        }
    }
    /// The font of the characters in this run, or None if the font is unmodified.
    pub fn font(&self) -> Option<&FontResource> {
        self.font.as_ref()
    }
    /// The brush of the characters in this run, or None if the brush is unmodified.
    pub fn brush(&self) -> Option<&Brush> {
        self.brush.as_ref()
    }
    /// The size of the characters in this run, or None if the size is unmodified.
    pub fn font_size(&self) -> Option<f32> {
        self.font_size
    }
    /// True if the characters in this run should have a shadow. None if this run does not change
    /// whether the characters are shadowed.
    pub fn shadow(&self) -> Option<bool> {
        self.shadow
    }
    /// The brush for the shadow of the characters in this run, or None if the brush is unmodified.
    pub fn shadow_brush(&self) -> Option<&Brush> {
        self.shadow_brush.as_ref()
    }
    /// The dilation for the shadow in this run, or None if the dilation is unmodified.
    pub fn shadow_dilation(&self) -> Option<f32> {
        self.shadow_dilation
    }
    /// The offset for the shadow in this run, or None if the offset is unmodified.
    pub fn shadow_offset(&self) -> Option<Vector2<f32>> {
        self.shadow_offset
    }
    #[deprecated]
    pub fn build(self) -> Self {
        self
    }
    /// Set this run to match the values set in the given run, overwriting the values
    /// in this run only if the corresponding value is set in the given run.
    pub fn with_values_from(self, run: Run) -> Self {
        Self {
            range: self.range,
            font: run.font.or(self.font),
            brush: run.brush.or(self.brush),
            font_size: run.font_size.or(self.font_size),
            shadow: run.shadow.or(self.shadow),
            shadow_brush: run.shadow_brush.or(self.shadow_brush),
            shadow_dilation: run.shadow_dilation.or(self.shadow_dilation),
            shadow_offset: run.shadow_offset.or(self.shadow_offset),
        }
    }
    /// Set this run to modify the font of the text within the range.
    pub fn with_font(mut self, font: FontResource) -> Self {
        self.font = Some(font);
        self
    }
    /// Set this run to modify the brush of the text within the range.
    pub fn with_brush(mut self, brush: Brush) -> Self {
        self.brush = Some(brush);
        self
    }
    /// Set this run to modify the size of the text within the range.
    pub fn with_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }
    /// Set this run to enable or disable the shadow of the text within the range.
    pub fn with_shadow(mut self, shadow: bool) -> Self {
        self.shadow = Some(shadow);
        self
    }
    /// Set this run to modify the brush of the shadow within the range.
    pub fn with_shadow_brush(mut self, brush: Brush) -> Self {
        self.shadow_brush = Some(brush);
        self
    }
    /// Set this run to modify the dilation of the shadow within the range.
    pub fn with_shadow_dilation(mut self, size: f32) -> Self {
        self.shadow_dilation = Some(size);
        self
    }
    /// Set this run to modify the offset of the shadow within the range.
    pub fn with_shadow_offset(mut self, offset: Vector2<f32>) -> Self {
        self.shadow_offset = Some(offset);
        self
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum DrawValueLayer {
    Main,
    Shadow,
}

#[derive(Clone, PartialEq, Debug)]
pub struct GlyphDrawValues {
    pub atlas_page_index: usize,
    pub font: FontResource,
    pub brush: Brush,
    /// Font size scaled by super sampling scaling to pick the correct atlas page.
    pub height: FontHeight,
}
