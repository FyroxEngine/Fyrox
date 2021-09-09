use crate::{
    core::{color::Color, math::Rect, visitor::prelude::*},
    renderer::framework::framebuffer::{CullFace, DrawParameters},
    utils::log::{Log, MessageKind},
};
use glow::{Framebuffer, HasContext};
use serde::Deserialize;
use std::fmt::{Display, Formatter};

#[derive(Default, Copy, Clone)]
pub struct PipelineStatistics {
    pub texture_binding_changes: usize,
    pub vbo_binding_changes: usize,
    pub vao_binding_changes: usize,
    pub blend_state_changes: usize,
    pub framebuffer_binding_changes: usize,
    pub program_binding_changes: usize,
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

#[derive(Copy, Clone, PartialOrd, PartialEq, Eq, Ord, Hash, Visit, Deserialize, Debug)]
#[repr(u32)]
pub enum CompareFunc {
    /// Never passes.
    Never = glow::NEVER,

    /// Passes if the incoming value is less than the stored value.
    Less = glow::LESS,

    /// Passes if the incoming value is equal to the stored value.
    Equal = glow::EQUAL,

    /// Passes if the incoming value is less than or equal to the stored value.
    LessOrEqual = glow::LEQUAL,

    /// Passes if the incoming value is greater than the stored value.
    Greater = glow::GREATER,

    /// Passes if the incoming value is not equal to the stored value.
    NotEqual = glow::NOTEQUAL,

    /// Passes if the incoming value is greater than or equal to the stored value.
    GreaterOrEqual = glow::GEQUAL,

    /// Always passes.
    Always = glow::ALWAYS,
}

impl Default for CompareFunc {
    fn default() -> Self {
        Self::LessOrEqual
    }
}

#[derive(Copy, Clone, Hash, PartialOrd, PartialEq, Eq, Ord, Deserialize, Visit, Debug)]
#[repr(u32)]
pub enum BlendFactor {
    Zero = glow::ZERO,
    One = glow::ONE,
    SrcColor = glow::SRC_COLOR,
    OneMinusSrcColor = glow::ONE_MINUS_SRC_COLOR,
    DstColor = glow::DST_COLOR,
    OneMinusDstColor = glow::ONE_MINUS_DST_COLOR,
    SrcAlpha = glow::SRC_ALPHA,
    OneMinusSrcAlpha = glow::ONE_MINUS_SRC_ALPHA,
    DstAlpha = glow::DST_ALPHA,
    OneMinusDstAlpha = glow::ONE_MINUS_DST_ALPHA,
    ConstantColor = glow::CONSTANT_COLOR,
    OneMinusConstantColor = glow::ONE_MINUS_CONSTANT_COLOR,
    ConstantAlpha = glow::CONSTANT_ALPHA,
    OneMinusConstantAlpha = glow::ONE_MINUS_CONSTANT_ALPHA,
    SrcAlphaSaturate = glow::SRC_ALPHA_SATURATE,
    Src1Color = glow::SRC1_COLOR,
    OneMinusSrc1Color = glow::ONE_MINUS_SRC1_COLOR,
    Src1Alpha = glow::SRC1_ALPHA,
    OneMinusSrc1Alpha = glow::ONE_MINUS_SRC1_ALPHA,
}

impl Default for BlendFactor {
    fn default() -> Self {
        Self::Zero
    }
}

#[derive(Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Hash, Deserialize, Visit, Debug)]
pub struct BlendFunc {
    pub sfactor: BlendFactor,
    pub dfactor: BlendFactor,
}

impl Default for BlendFunc {
    fn default() -> Self {
        Self {
            sfactor: BlendFactor::One,
            dfactor: BlendFactor::Zero,
        }
    }
}

pub struct PipelineState {
    pub gl: glow::Context,

    blend: bool,

    depth_test: bool,
    depth_write: bool,
    depth_func: CompareFunc,

    color_write: ColorMask,
    stencil_test: bool,
    cull_face: CullFace,
    culling: bool,
    stencil_mask: u32,
    clear_color: Color,
    clear_stencil: i32,
    clear_depth: f32,
    scissor_test: bool,

    framebuffer: Option<glow::Framebuffer>,
    viewport: Rect<i32>,

    blend_func: BlendFunc,

    program: Option<glow::Program>,
    texture_units: [TextureUnit; 32],

    stencil_func: StencilFunc,
    stencil_op: StencilOp,

    vao: Option<glow::VertexArray>,
    vbo: Option<glow::Buffer>,

    frame_statistics: PipelineStatistics,
}

#[derive(Copy, Clone)]
struct TextureUnit {
    target: u32,
    texture: Option<glow::Texture>,
}

impl Default for TextureUnit {
    fn default() -> Self {
        Self {
            target: glow::TEXTURE_2D,
            texture: Default::default(),
        }
    }
}

#[derive(Copy, Clone, PartialOrd, PartialEq, Hash, Debug, Deserialize, Visit)]
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

#[derive(Copy, Clone, PartialOrd, PartialEq, Hash, Debug, Deserialize, Visit)]
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

#[derive(Copy, Clone, PartialOrd, PartialEq, Hash, Debug, Deserialize, Visit)]
#[repr(u32)]
pub enum StencilAction {
    /// Keeps the current value.
    Keep = glow::KEEP,

    /// Sets the stencil buffer value to 0.
    Zero = glow::ZERO,

    /// Sets the stencil buffer value to ref value.
    Replace = glow::REPLACE,

    /// Increments the current stencil buffer value.
    /// Clamps to the maximum representable unsigned value.
    Incr = glow::INCR,

    /// Increments the current stencil buffer value.
    /// Wraps stencil buffer value to zero when incrementing the maximum representable
    /// unsigned value.
    IncrWrap = glow::INCR_WRAP,

    /// Decrements the current stencil buffer value.
    /// Clamps to 0.
    Decr = glow::DECR,

    /// Decrements the current stencil buffer value.
    /// Wraps stencil buffer value to the maximum representable unsigned value when
    /// decrementing a stencil buffer value of zero.
    DecrWrap = glow::DECR_WRAP,

    /// Bitwise inverts the current stencil buffer value.
    Invert = glow::INVERT,
}

impl Default for StencilAction {
    fn default() -> Self {
        Self::Keep
    }
}

#[derive(Copy, Clone, PartialOrd, PartialEq, Hash, Debug, Deserialize, Visit)]
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

impl PipelineState {
    pub fn new(context: glow::Context) -> Self {
        unsafe {
            context.depth_func(CompareFunc::default() as u32);
        }

        Self {
            gl: context,
            blend: false,
            depth_test: false,
            depth_write: true,
            depth_func: Default::default(),
            color_write: Default::default(),
            stencil_test: false,
            cull_face: CullFace::Back,
            culling: false,
            stencil_mask: 0xFFFF_FFFF,
            clear_color: Color::from_rgba(0, 0, 0, 0),
            clear_stencil: 0,
            clear_depth: 1.0,
            scissor_test: false,
            framebuffer: None,
            blend_func: Default::default(),
            viewport: Rect::new(0, 0, 1, 1),
            program: Default::default(),
            texture_units: [Default::default(); 32],
            stencil_func: Default::default(),
            stencil_op: Default::default(),
            vao: Default::default(),
            vbo: Default::default(),
            frame_statistics: Default::default(),
        }
    }

    pub fn set_framebuffer(&mut self, framebuffer: Option<glow::Framebuffer>) {
        if self.framebuffer != framebuffer {
            self.framebuffer = framebuffer;

            self.frame_statistics.framebuffer_binding_changes += 1;

            unsafe {
                self.gl
                    .bind_framebuffer(glow::FRAMEBUFFER, self.framebuffer)
            }
        }
    }

    pub fn set_viewport(&mut self, viewport: Rect<i32>) {
        if self.viewport != viewport {
            self.viewport = viewport;

            unsafe {
                self.gl.viewport(
                    self.viewport.x(),
                    self.viewport.y(),
                    self.viewport.w(),
                    self.viewport.h(),
                );
            }
        }
    }

    pub fn set_blend(&mut self, blend: bool) {
        if self.blend != blend {
            self.blend = blend;

            self.frame_statistics.blend_state_changes += 1;

            unsafe {
                if self.blend {
                    self.gl.enable(glow::BLEND);
                } else {
                    self.gl.disable(glow::BLEND);
                }
            }
        }
    }

    pub fn set_depth_test(&mut self, depth_test: bool) {
        if self.depth_test != depth_test {
            self.depth_test = depth_test;

            unsafe {
                if self.depth_test {
                    self.gl.enable(glow::DEPTH_TEST);
                } else {
                    self.gl.disable(glow::DEPTH_TEST);
                }
            }
        }
    }

    pub fn set_depth_write(&mut self, depth_write: bool) {
        if self.depth_write != depth_write {
            self.depth_write = depth_write;

            unsafe {
                self.gl.depth_mask(self.depth_write);
            }
        }
    }

    pub fn set_color_write(&mut self, color_write: ColorMask) {
        if self.color_write != color_write {
            self.color_write = color_write;

            unsafe {
                self.gl.color_mask(
                    self.color_write.red,
                    self.color_write.green,
                    self.color_write.blue,
                    self.color_write.alpha,
                );
            }
        }
    }

    pub fn set_stencil_test(&mut self, stencil_test: bool) {
        if self.stencil_test != stencil_test {
            self.stencil_test = stencil_test;

            unsafe {
                if self.stencil_test {
                    self.gl.enable(glow::STENCIL_TEST);
                } else {
                    self.gl.disable(glow::STENCIL_TEST);
                }
            }
        }
    }

    pub fn set_cull_face(&mut self, cull_face: CullFace) {
        if self.cull_face != cull_face {
            self.cull_face = cull_face;

            unsafe { self.gl.cull_face(self.cull_face as u32) }
        }
    }

    pub fn set_culling(&mut self, culling: bool) {
        if self.culling != culling {
            self.culling = culling;

            unsafe {
                if self.culling {
                    self.gl.enable(glow::CULL_FACE);
                } else {
                    self.gl.disable(glow::CULL_FACE);
                }
            }
        }
    }

    pub fn set_stencil_mask(&mut self, stencil_mask: u32) {
        if self.stencil_mask != stencil_mask {
            self.stencil_mask = stencil_mask;

            unsafe {
                self.gl.stencil_mask(stencil_mask);
            }
        }
    }

    pub fn set_clear_color(&mut self, color: Color) {
        if self.clear_color != color {
            self.clear_color = color;

            let rgba = color.as_frgba();
            unsafe {
                self.gl.clear_color(rgba.x, rgba.y, rgba.z, rgba.w);
            }
        }
    }

    pub fn set_clear_depth(&mut self, depth: f32) {
        if (self.clear_depth - depth).abs() > f32::EPSILON {
            self.clear_depth = depth;

            unsafe {
                self.gl.clear_depth_f32(depth);
            }
        }
    }

    pub fn set_clear_stencil(&mut self, stencil: i32) {
        if self.clear_stencil != stencil {
            self.clear_stencil = stencil;

            unsafe {
                self.gl.clear_stencil(stencil);
            }
        }
    }

    pub fn set_blend_func(&mut self, func: BlendFunc) {
        if self.blend_func != func {
            self.blend_func = func;

            unsafe {
                self.gl.blend_func(
                    self.blend_func.sfactor as u32,
                    self.blend_func.dfactor as u32,
                );
            }
        }
    }

    pub fn set_depth_func(&mut self, depth_func: CompareFunc) {
        if self.depth_func != depth_func {
            self.depth_func = depth_func;

            unsafe {
                self.gl.depth_func(depth_func as u32);
            }
        }
    }

    pub fn set_program(&mut self, program: Option<glow::Program>) {
        if self.program != program {
            self.program = program;

            self.frame_statistics.program_binding_changes += 1;

            unsafe {
                self.gl.use_program(self.program);
            }
        }
    }

    pub fn set_texture(&mut self, sampler_index: u32, target: u32, texture: Option<glow::Texture>) {
        let unit = self.texture_units.get_mut(sampler_index as usize).unwrap();

        if unit.target != target || unit.texture != texture {
            unit.texture = texture;
            unit.target = target;

            self.frame_statistics.texture_binding_changes += 1;

            unsafe {
                self.gl.active_texture(glow::TEXTURE0 + sampler_index);
                self.gl.bind_texture(target, unit.texture);
            }
        }
    }

    pub fn set_stencil_func(&mut self, func: StencilFunc) {
        if self.stencil_func != func {
            self.stencil_func = func;

            unsafe {
                self.gl.stencil_func(
                    self.stencil_func.func as u32,
                    self.stencil_func.ref_value as i32,
                    self.stencil_func.mask,
                );
            }
        }
    }

    pub fn set_stencil_op(&mut self, op: StencilOp) {
        if self.stencil_op != op {
            self.stencil_op = op;

            unsafe {
                self.gl.stencil_op(
                    self.stencil_op.fail as u32,
                    self.stencil_op.zfail as u32,
                    self.stencil_op.zpass as u32,
                );

                self.gl.stencil_mask(self.stencil_op.write_mask);
            }
        }
    }

    pub fn set_vertex_array_object(&mut self, vao: Option<glow::VertexArray>) {
        if self.vao != vao {
            self.vao = vao;

            self.frame_statistics.vao_binding_changes += 1;

            unsafe {
                self.gl.bind_vertex_array(self.vao);
            }
        }
    }

    pub fn set_vertex_buffer_object(&mut self, vbo: Option<glow::Buffer>) {
        if self.vbo != vbo {
            self.vbo = vbo;

            self.frame_statistics.vbo_binding_changes += 1;

            unsafe {
                self.gl.bind_buffer(glow::ARRAY_BUFFER, self.vbo);
            }
        }
    }

    pub fn set_scissor_test(&mut self, state: bool) {
        if self.scissor_test != state {
            self.scissor_test = state;

            unsafe {
                if state {
                    self.gl.enable(glow::SCISSOR_TEST);
                } else {
                    self.gl.disable(glow::SCISSOR_TEST);
                }
            }
        }
    }

    pub fn blit_framebuffer(
        &mut self,
        source: Option<Framebuffer>,
        dest: Option<Framebuffer>,
        src_x0: i32,
        src_y0: i32,
        src_x1: i32,
        src_y1: i32,
        dst_x0: i32,
        dst_y0: i32,
        dst_x1: i32,
        dst_y1: i32,
        copy_color: bool,
        copy_depth: bool,
        copy_stencil: bool,
    ) {
        let mut mask = 0;
        if copy_color {
            mask |= glow::COLOR_BUFFER_BIT;
        }
        if copy_depth {
            mask |= glow::DEPTH_BUFFER_BIT;
        }
        if copy_stencil {
            mask |= glow::STENCIL_BUFFER_BIT;
        }

        unsafe {
            self.gl.bind_framebuffer(glow::READ_FRAMEBUFFER, source);
            self.gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, dest);
            self.gl.blit_framebuffer(
                src_x0,
                src_y0,
                src_x1,
                src_y1,
                dst_x0,
                dst_y0,
                dst_x1,
                dst_y1,
                mask,
                glow::NEAREST,
            );
        }
    }

    pub fn set_scissor_box(&mut self, x: i32, y: i32, w: i32, h: i32) {
        unsafe {
            self.gl.scissor(x, y, w, h);
        }
    }

    pub fn invalidate_resource_bindings_cache(&mut self) {
        self.texture_units = Default::default();
        self.program = Default::default();

        self.frame_statistics = Default::default();
    }

    pub fn apply_draw_parameters(&mut self, draw_params: &DrawParameters) {
        if let Some(blend_func) = draw_params.blend {
            self.set_blend_func(blend_func);
            self.set_blend(true);
        } else {
            self.set_blend(false);
        }
        self.set_depth_test(draw_params.depth_test);
        self.set_depth_write(draw_params.depth_write);
        self.set_color_write(draw_params.color_write);

        if let Some(stencil_func) = draw_params.stencil_test {
            self.set_stencil_test(true);
            self.set_stencil_func(stencil_func);
        } else {
            self.set_stencil_test(false);
        }

        self.set_stencil_op(draw_params.stencil_op);

        if let Some(cull_face) = draw_params.cull_face {
            self.set_cull_face(cull_face);
            self.set_culling(true);
        } else {
            self.set_culling(false);
        }
    }

    pub fn pipeline_statistics(&self) -> PipelineStatistics {
        self.frame_statistics
    }

    /// Checks for errors, returns true if any error has occurred.
    pub fn check_error(&self) -> bool {
        unsafe {
            let error_code = self.gl.get_error();
            if error_code != glow::NO_ERROR {
                let code = match error_code {
                    glow::INVALID_ENUM => "GL_INVALID_ENUM",
                    glow::INVALID_VALUE => "GL_INVALID_VALUE",
                    glow::INVALID_OPERATION => "GL_INVALID_OPERATION",
                    glow::STACK_OVERFLOW => "GL_STACK_OVERFLOW",
                    glow::STACK_UNDERFLOW => "GL_STACK_UNDERFLOW",
                    glow::OUT_OF_MEMORY => "GL_OUT_OF_MEMORY",
                    _ => "Unknown",
                };

                Log::writeln(
                    MessageKind::Error,
                    format!("{} error has occurred! Stability is not guaranteed!", code),
                );

                #[cfg(not(target_arch = "wasm32"))]
                {
                    for entry in self.gl.get_debug_message_log(64) {
                        Log::writeln(MessageKind::Error, format!("OpenGL message: {:?}", entry))
                    }
                }

                true
            } else {
                false
            }
        }
    }
}
