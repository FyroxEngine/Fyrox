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

use crate::framebuffer::DrawCallStatistics;
use std::fmt::{Display, Formatter};

/// Graphics pipeline statistics.
#[derive(Debug, Default, Copy, Clone)]
pub struct PipelineStatistics {
    /// Total amount of texture that was bound to the pipeline during the rendering.
    pub texture_binding_changes: usize,
    /// Total amount of VBOs was bound to the pipeline during the rendering.
    pub vbo_binding_changes: usize,
    /// Total amount of VAOs was bound to the pipeline during the rendering.
    pub vao_binding_changes: usize,
    /// Total amount of blending state changed in the pipeline during the rendering.
    pub blend_state_changes: usize,
    /// Total amount of frame buffers was used during the rendering.
    pub framebuffer_binding_changes: usize,
    /// Total amount of programs was used in the pipeline during the rendering.
    pub program_binding_changes: usize,
}

impl std::ops::AddAssign for PipelineStatistics {
    fn add_assign(&mut self, rhs: Self) {
        self.texture_binding_changes += rhs.texture_binding_changes;
        self.vbo_binding_changes += rhs.vbo_binding_changes;
        self.vao_binding_changes += rhs.vao_binding_changes;
        self.blend_state_changes += rhs.blend_state_changes;
        self.framebuffer_binding_changes += rhs.framebuffer_binding_changes;
        self.program_binding_changes += rhs.program_binding_changes;
    }
}

impl std::ops::Sub for PipelineStatistics {
    type Output = PipelineStatistics;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            texture_binding_changes: self.texture_binding_changes - rhs.texture_binding_changes,
            vbo_binding_changes: self.vbo_binding_changes - rhs.vbo_binding_changes,
            vao_binding_changes: self.vao_binding_changes - rhs.vao_binding_changes,
            blend_state_changes: self.blend_state_changes - rhs.blend_state_changes,
            framebuffer_binding_changes: self.framebuffer_binding_changes
                - rhs.framebuffer_binding_changes,
            program_binding_changes: self.program_binding_changes - rhs.program_binding_changes,
        }
    }
}

impl Display for PipelineStatistics {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Pipeline state changes:\n\
            \tTextures: {},\n\
            \tVBO: {},\n\
            \tVAO: {},\n\
            \tFBO: {},\n\
            \tShaders: {},\n\
            \tBlend: {}",
            self.texture_binding_changes,
            self.vbo_binding_changes,
            self.vao_binding_changes,
            self.framebuffer_binding_changes,
            self.program_binding_changes,
            self.blend_state_changes
        )
    }
}

/// GPU statistics for single frame.
#[derive(Debug, Copy, Clone, Default)]
pub struct RenderPassStatistics {
    /// Amount of draw calls per frame - lower the better.
    pub draw_calls: usize,
    /// Amount of triangles per frame.
    pub triangles_rendered: usize,
}

impl Display for RenderPassStatistics {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Draw Calls: {}\n\
            Triangles Rendered: {}",
            self.draw_calls, self.triangles_rendered
        )
    }
}

impl std::ops::AddAssign for RenderPassStatistics {
    fn add_assign(&mut self, rhs: Self) {
        self.draw_calls += rhs.draw_calls;
        self.triangles_rendered += rhs.triangles_rendered;
    }
}

impl std::ops::AddAssign<DrawCallStatistics> for RenderPassStatistics {
    fn add_assign(&mut self, rhs: DrawCallStatistics) {
        self.draw_calls += 1;
        self.triangles_rendered += rhs.triangles;
    }
}
