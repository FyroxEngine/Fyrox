use crate::renderer::PipelineStatistics;
use crate::{
    core::{color::Color, math::Rect, reflect::prelude::*, visitor::prelude::*},
    renderer::framework::framebuffer::{CullFace, DrawParameters},
};
use fyrox_core::uuid_provider;
use glow::{Framebuffer, HasContext};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use strum_macros::{AsRefStr, EnumString, VariantNames};

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

#[derive(
    Copy, Clone, Hash, PartialOrd, PartialEq, Eq, Ord, Serialize, Deserialize, Visit, Debug, Reflect,
)]
#[repr(u32)]
pub enum BlendMode {
    Add = glow::FUNC_ADD,
    Subtract = glow::FUNC_SUBTRACT,
    ReverseSubtract = glow::FUNC_REVERSE_SUBTRACT,
    Min = glow::MIN,
    Max = glow::MAX,
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
    rgb: BlendMode,
    alpha: BlendMode,
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

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum GlKind {
    OpenGL,
    OpenGLES,
}

struct InnerState {
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

    polygon_face: PolygonFace,
    polygon_fill_mode: PolygonFillMode,

    framebuffer: Option<glow::Framebuffer>,
    viewport: Rect<i32>,

    blend_func: BlendFunc,
    blend_equation: BlendEquation,

    program: Option<glow::Program>,
    texture_units: [TextureUnit; 32],

    stencil_func: StencilFunc,
    stencil_op: StencilOp,

    vao: Option<glow::VertexArray>,
    vbo: Option<glow::Buffer>,

    frame_statistics: PipelineStatistics,
    gl_kind: GlKind,
}

impl InnerState {
    fn new(gl_kind: GlKind) -> Self {
        Self {
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
            polygon_face: Default::default(),
            polygon_fill_mode: Default::default(),
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
            blend_equation: Default::default(),
            gl_kind,
        }
    }
}

pub type SharedPipelineState = Rc<PipelineState>;

pub struct PipelineState {
    pub gl: glow::Context,
    state: RefCell<InnerState>,
    this: RefCell<Option<Weak<PipelineState>>>,
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
#[repr(u32)]
pub enum PolygonFace {
    Front = glow::FRONT,
    Back = glow::BACK,
    FrontAndBack = glow::FRONT_AND_BACK,
}

impl Default for PolygonFace {
    fn default() -> Self {
        Self::FrontAndBack
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
#[repr(u32)]
pub enum PolygonFillMode {
    Point = glow::POINT,
    Line = glow::LINE,
    Fill = glow::FILL,
}

uuid_provider!(PolygonFillMode = "47aff01a-7daa-427c-874c-87464a7ffe28");

impl Default for PolygonFillMode {
    fn default() -> Self {
        Self::Fill
    }
}

impl PipelineState {
    pub fn new(
        #[allow(unused_mut)] mut context: glow::Context,
        gl_kind: GlKind,
    ) -> SharedPipelineState {
        unsafe {
            context.depth_func(CompareFunc::default() as u32);

            #[cfg(debug_assertions)]
            {
                use crate::core::log::{Log, MessageKind};

                if context.supported_extensions().contains("GL_KHR_debug") {
                    context.debug_message_callback(|source, msg_type, id, severity, message| {
                        let message_kind = if severity == glow::DEBUG_SEVERITY_HIGH {
                            MessageKind::Error
                        } else if severity == glow::DEBUG_SEVERITY_MEDIUM
                            || severity == glow::DEBUG_SEVERITY_LOW
                        {
                            MessageKind::Warning
                        } else {
                            // Ignore any info because it tend to produce spam.
                            return;
                        };

                        let source = if source == glow::DEBUG_SOURCE_API {
                            "Calls to the OpenGL API"
                        } else if source == glow::DEBUG_SOURCE_WINDOW_SYSTEM {
                            "Calls to a window-system API"
                        } else if source == glow::DEBUG_SOURCE_SHADER_COMPILER {
                            "A compiler for a shading language"
                        } else if source == glow::DEBUG_SOURCE_THIRD_PARTY {
                            "An application associated with OpenGL"
                        } else if source == glow::DEBUG_SOURCE_APPLICATION {
                            "Generated by the user of this application"
                        } else {
                            "Other"
                        };

                        let msg_type = if msg_type == glow::DEBUG_TYPE_ERROR {
                            "An error, typically from the API"
                        } else if msg_type == glow::DEBUG_TYPE_DEPRECATED_BEHAVIOR {
                            "Some behavior marked deprecated has been used"
                        } else if msg_type == glow::DEBUG_TYPE_UNDEFINED_BEHAVIOR {
                            "Something has invoked undefined behavior"
                        } else if msg_type == glow::DEBUG_TYPE_PORTABILITY {
                            "Some functionality the user relies upon is not portable"
                        } else if msg_type == glow::DEBUG_TYPE_PERFORMANCE {
                            "Code has triggered possible performance issues"
                        } else if msg_type == glow::DEBUG_TYPE_MARKER {
                            "Command stream annotation"
                        } else if msg_type == glow::DEBUG_TYPE_PUSH_GROUP
                            || msg_type == glow::DEBUG_TYPE_POP_GROUP
                        {
                            "Group pushing"
                        } else {
                            "Other"
                        };

                        Log::writeln(
                            message_kind,
                            format!(
                                "OpenGL Message\n\
                            \tSource: {source}\n\
                            \tType: {msg_type}\n\
                            \tId: {id}\n\
                            \tMessage: {message}"
                            ),
                        );
                    })
                }
            }
        }

        let state = Self {
            gl: context,
            state: RefCell::new(InnerState::new(gl_kind)),
            this: Default::default(),
        };

        let shared = SharedPipelineState::new(state);

        *shared.this.borrow_mut() = Some(Rc::downgrade(&shared));

        shared
    }

    pub fn weak(&self) -> Weak<Self> {
        self.this.borrow().as_ref().unwrap().clone()
    }

    pub fn gl_kind(&self) -> GlKind {
        self.state.borrow().gl_kind
    }

    pub fn set_polygon_fill_mode(
        &self,
        polygon_face: PolygonFace,
        polygon_fill_mode: PolygonFillMode,
    ) {
        let mut state = self.state.borrow_mut();
        if state.polygon_fill_mode != polygon_fill_mode || state.polygon_face != polygon_face {
            state.polygon_fill_mode = polygon_fill_mode;
            state.polygon_face = polygon_face;

            unsafe {
                self.gl
                    .polygon_mode(state.polygon_face as u32, state.polygon_fill_mode as u32)
            }
        }
    }

    pub fn set_framebuffer(&self, framebuffer: Option<glow::Framebuffer>) {
        let mut state = self.state.borrow_mut();
        if state.framebuffer != framebuffer {
            state.framebuffer = framebuffer;

            state.frame_statistics.framebuffer_binding_changes += 1;

            unsafe {
                self.gl
                    .bind_framebuffer(glow::FRAMEBUFFER, state.framebuffer)
            }
        }
    }

    pub fn set_viewport(&self, viewport: Rect<i32>) {
        let mut state = self.state.borrow_mut();
        if state.viewport != viewport {
            state.viewport = viewport;

            unsafe {
                self.gl.viewport(
                    state.viewport.x(),
                    state.viewport.y(),
                    state.viewport.w(),
                    state.viewport.h(),
                );
            }
        }
    }

    pub fn set_blend(&self, blend: bool) {
        let mut state = self.state.borrow_mut();
        if state.blend != blend {
            state.blend = blend;

            state.frame_statistics.blend_state_changes += 1;

            unsafe {
                if state.blend {
                    self.gl.enable(glow::BLEND);
                } else {
                    self.gl.disable(glow::BLEND);
                }
            }
        }
    }

    pub fn set_depth_test(&self, depth_test: bool) {
        let mut state = self.state.borrow_mut();
        if state.depth_test != depth_test {
            state.depth_test = depth_test;

            unsafe {
                if state.depth_test {
                    self.gl.enable(glow::DEPTH_TEST);
                } else {
                    self.gl.disable(glow::DEPTH_TEST);
                }
            }
        }
    }

    pub fn set_depth_write(&self, depth_write: bool) {
        let mut state = self.state.borrow_mut();
        if state.depth_write != depth_write {
            state.depth_write = depth_write;

            unsafe {
                self.gl.depth_mask(state.depth_write);
            }
        }
    }

    pub fn set_color_write(&self, color_write: ColorMask) {
        let mut state = self.state.borrow_mut();
        if state.color_write != color_write {
            state.color_write = color_write;

            unsafe {
                self.gl.color_mask(
                    state.color_write.red,
                    state.color_write.green,
                    state.color_write.blue,
                    state.color_write.alpha,
                );
            }
        }
    }

    pub fn set_stencil_test(&self, stencil_test: bool) {
        let mut state = self.state.borrow_mut();
        if state.stencil_test != stencil_test {
            state.stencil_test = stencil_test;

            unsafe {
                if state.stencil_test {
                    self.gl.enable(glow::STENCIL_TEST);
                } else {
                    self.gl.disable(glow::STENCIL_TEST);
                }
            }
        }
    }

    pub fn set_cull_face(&self, cull_face: CullFace) {
        let mut state = self.state.borrow_mut();
        if state.cull_face != cull_face {
            state.cull_face = cull_face;

            unsafe { self.gl.cull_face(state.cull_face as u32) }
        }
    }

    pub fn set_culling(&self, culling: bool) {
        let mut state = self.state.borrow_mut();
        if state.culling != culling {
            state.culling = culling;

            unsafe {
                if state.culling {
                    self.gl.enable(glow::CULL_FACE);
                } else {
                    self.gl.disable(glow::CULL_FACE);
                }
            }
        }
    }

    pub fn set_stencil_mask(&self, stencil_mask: u32) {
        let mut state = self.state.borrow_mut();
        if state.stencil_mask != stencil_mask {
            state.stencil_mask = stencil_mask;

            unsafe {
                self.gl.stencil_mask(stencil_mask);
            }
        }
    }

    pub fn set_clear_color(&self, color: Color) {
        let mut state = self.state.borrow_mut();
        if state.clear_color != color {
            state.clear_color = color;

            let rgba = color.as_frgba();
            unsafe {
                self.gl.clear_color(rgba.x, rgba.y, rgba.z, rgba.w);
            }
        }
    }

    pub fn set_clear_depth(&self, depth: f32) {
        let mut state = self.state.borrow_mut();
        if (state.clear_depth - depth).abs() > f32::EPSILON {
            state.clear_depth = depth;

            unsafe {
                self.gl.clear_depth_f32(depth);
            }
        }
    }

    pub fn set_clear_stencil(&self, stencil: i32) {
        let mut state = self.state.borrow_mut();
        if state.clear_stencil != stencil {
            state.clear_stencil = stencil;

            unsafe {
                self.gl.clear_stencil(stencil);
            }
        }
    }

    pub fn set_blend_func(&self, func: BlendFunc) {
        let mut state = self.state.borrow_mut();
        if state.blend_func != func {
            state.blend_func = func;

            unsafe {
                self.gl.blend_func_separate(
                    state.blend_func.sfactor as u32,
                    state.blend_func.dfactor as u32,
                    state.blend_func.alpha_sfactor as u32,
                    state.blend_func.alpha_dfactor as u32,
                );
            }
        }
    }

    pub fn set_blend_equation(&self, equation: BlendEquation) {
        let mut state = self.state.borrow_mut();
        if state.blend_equation != equation {
            state.blend_equation = equation;

            unsafe {
                self.gl.blend_equation_separate(
                    state.blend_equation.rgb as u32,
                    state.blend_equation.alpha as u32,
                );
            }
        }
    }

    pub fn set_depth_func(&self, depth_func: CompareFunc) {
        let mut state = self.state.borrow_mut();
        if state.depth_func != depth_func {
            state.depth_func = depth_func;

            unsafe {
                self.gl.depth_func(depth_func as u32);
            }
        }
    }

    pub fn set_program(&self, program: Option<glow::Program>) {
        let mut state = self.state.borrow_mut();
        if state.program != program {
            state.program = program;

            state.frame_statistics.program_binding_changes += 1;

            unsafe {
                self.gl.use_program(state.program);
            }
        }
    }

    pub fn set_texture(&self, sampler_index: u32, target: u32, texture: Option<glow::Texture>) {
        let mut state = self.state.borrow_mut();

        // We must set active texture no matter if it's texture is bound or not.
        unsafe {
            self.gl.active_texture(glow::TEXTURE0 + sampler_index);
        }

        let unit = &mut state.texture_units[sampler_index as usize];
        if unit.target != target || unit.texture != texture {
            unit.texture = texture;
            unit.target = target;

            unsafe {
                self.gl.bind_texture(target, unit.texture);
            }
            state.frame_statistics.texture_binding_changes += 1;
        }
    }

    pub fn set_stencil_func(&self, func: StencilFunc) {
        let mut state = self.state.borrow_mut();
        if state.stencil_func != func {
            state.stencil_func = func;

            unsafe {
                self.gl.stencil_func(
                    state.stencil_func.func as u32,
                    state.stencil_func.ref_value as i32,
                    state.stencil_func.mask,
                );
            }
        }
    }

    pub fn set_stencil_op(&self, op: StencilOp) {
        let mut state = self.state.borrow_mut();
        if state.stencil_op != op {
            state.stencil_op = op;

            unsafe {
                self.gl.stencil_op(
                    state.stencil_op.fail as u32,
                    state.stencil_op.zfail as u32,
                    state.stencil_op.zpass as u32,
                );

                self.gl.stencil_mask(state.stencil_op.write_mask);
            }
        }
    }

    pub fn set_vertex_array_object(&self, vao: Option<glow::VertexArray>) {
        let mut state = self.state.borrow_mut();
        if state.vao != vao {
            state.vao = vao;

            state.frame_statistics.vao_binding_changes += 1;

            unsafe {
                self.gl.bind_vertex_array(state.vao);
            }
        }
    }

    pub fn set_vertex_buffer_object(&self, vbo: Option<glow::Buffer>) {
        let mut state = self.state.borrow_mut();
        if state.vbo != vbo {
            state.vbo = vbo;

            state.frame_statistics.vbo_binding_changes += 1;

            unsafe {
                self.gl.bind_buffer(glow::ARRAY_BUFFER, state.vbo);
            }
        }
    }

    pub fn set_scissor_test(&self, scissor_test: bool) {
        let mut state = self.state.borrow_mut();
        if state.scissor_test != scissor_test {
            state.scissor_test = scissor_test;

            unsafe {
                if scissor_test {
                    self.gl.enable(glow::SCISSOR_TEST);
                } else {
                    self.gl.disable(glow::SCISSOR_TEST);
                }
            }
        }
    }

    pub fn blit_framebuffer(
        &self,
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

    pub fn set_scissor_box(&self, x: i32, y: i32, w: i32, h: i32) {
        unsafe {
            self.gl.scissor(x, y, w, h);
        }
    }

    pub fn invalidate_resource_bindings_cache(&self) {
        let mut state = self.state.borrow_mut();
        state.texture_units = Default::default();
        state.program = Default::default();
        state.frame_statistics = Default::default();
    }

    pub fn apply_draw_parameters(&self, draw_params: &DrawParameters) {
        if let Some(ref blend_params) = draw_params.blend {
            self.set_blend_func(blend_params.func);
            self.set_blend_equation(blend_params.equation);
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
        self.state.borrow().frame_statistics
    }
}
