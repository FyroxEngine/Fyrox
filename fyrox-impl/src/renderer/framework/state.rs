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

use crate::renderer::framework::{BlendFactor, BlendMode, StencilAction};
use crate::{
    core::{color::Color, log::Log, math::Rect},
    engine::{error::EngineError, GraphicsContextParams},
    renderer::{
        framework::{
            error::FrameworkError, BlendEquation, BlendFunc, ColorMask, CompareFunc, CullFace,
            DrawParameters, PolygonFace, PolygonFillMode, StencilFunc, StencilOp,
        },
        PipelineStatistics,
    },
};
use glow::{Framebuffer, HasContext};
#[cfg(not(target_arch = "wasm32"))]
use glutin::{
    config::ConfigTemplateBuilder,
    context::PossiblyCurrentContext,
    context::{ContextApi, ContextAttributesBuilder, GlProfile, NotCurrentGlContext, Version},
    display::{GetGlDisplay, GlDisplay},
    surface::{GlSurface, SwapInterval},
    surface::{Surface, WindowSurface},
};
#[cfg(not(target_arch = "wasm32"))]
use glutin_winit::{DisplayBuilder, GlWindow};
#[cfg(not(target_arch = "wasm32"))]
use raw_window_handle::HasRawWindowHandle;
use std::cell::RefCell;
use std::ops::DerefMut;
use std::rc::{Rc, Weak};
#[cfg(not(target_arch = "wasm32"))]
use std::{ffi::CString, num::NonZeroU32};
use winit::{
    event_loop::EventLoopWindowTarget,
    window::{Window, WindowBuilder},
};

pub trait ToGlConstant {
    fn into_gl(self) -> u32;
}

impl ToGlConstant for PolygonFace {
    fn into_gl(self) -> u32 {
        match self {
            Self::Front => glow::FRONT,
            Self::Back => glow::BACK,
            Self::FrontAndBack => glow::FRONT_AND_BACK,
        }
    }
}

impl ToGlConstant for PolygonFillMode {
    fn into_gl(self) -> u32 {
        match self {
            Self::Point => glow::POINT,
            Self::Line => glow::LINE,
            Self::Fill => glow::FILL,
        }
    }
}

impl ToGlConstant for StencilAction {
    fn into_gl(self) -> u32 {
        match self {
            StencilAction::Keep => glow::KEEP,
            StencilAction::Zero => glow::ZERO,
            StencilAction::Replace => glow::REPLACE,
            StencilAction::Incr => glow::INCR,
            StencilAction::IncrWrap => glow::INCR_WRAP,
            StencilAction::Decr => glow::DECR,
            StencilAction::DecrWrap => glow::DECR_WRAP,
            StencilAction::Invert => glow::INVERT,
        }
    }
}

impl ToGlConstant for BlendMode {
    fn into_gl(self) -> u32 {
        match self {
            Self::Add => glow::FUNC_ADD,
            Self::Subtract => glow::FUNC_SUBTRACT,
            Self::ReverseSubtract => glow::FUNC_REVERSE_SUBTRACT,
            Self::Min => glow::MIN,
            Self::Max => glow::MAX,
        }
    }
}

impl ToGlConstant for BlendFactor {
    fn into_gl(self) -> u32 {
        match self {
            Self::Zero => glow::ZERO,
            Self::One => glow::ONE,
            Self::SrcColor => glow::SRC_COLOR,
            Self::OneMinusSrcColor => glow::ONE_MINUS_SRC_COLOR,
            Self::DstColor => glow::DST_COLOR,
            Self::OneMinusDstColor => glow::ONE_MINUS_DST_COLOR,
            Self::SrcAlpha => glow::SRC_ALPHA,
            Self::OneMinusSrcAlpha => glow::ONE_MINUS_SRC_ALPHA,
            Self::DstAlpha => glow::DST_ALPHA,
            Self::OneMinusDstAlpha => glow::ONE_MINUS_DST_ALPHA,
            Self::ConstantColor => glow::CONSTANT_COLOR,
            Self::OneMinusConstantColor => glow::ONE_MINUS_CONSTANT_COLOR,
            Self::ConstantAlpha => glow::CONSTANT_ALPHA,
            Self::OneMinusConstantAlpha => glow::ONE_MINUS_CONSTANT_ALPHA,
            Self::SrcAlphaSaturate => glow::SRC_ALPHA_SATURATE,
            Self::Src1Color => glow::SRC1_COLOR,
            Self::OneMinusSrc1Color => glow::ONE_MINUS_SRC1_COLOR,
            Self::Src1Alpha => glow::SRC1_ALPHA,
            Self::OneMinusSrc1Alpha => glow::ONE_MINUS_SRC1_ALPHA,
        }
    }
}

impl ToGlConstant for CompareFunc {
    fn into_gl(self) -> u32 {
        match self {
            Self::Never => glow::NEVER,
            Self::Less => glow::LESS,
            Self::Equal => glow::EQUAL,
            Self::LessOrEqual => glow::LEQUAL,
            Self::Greater => glow::GREATER,
            Self::NotEqual => glow::NOTEQUAL,
            Self::GreaterOrEqual => glow::GEQUAL,
            Self::Always => glow::ALWAYS,
        }
    }
}

impl ToGlConstant for CullFace {
    fn into_gl(self) -> u32 {
        match self {
            Self::Back => glow::BACK,
            Self::Front => glow::FRONT,
        }
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum GlKind {
    OpenGL,
    OpenGLES,
}

pub(crate) struct InnerState {
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
    texture_units_storage: TextureUnitsStorage,

    stencil_func: StencilFunc,
    stencil_op: StencilOp,

    vao: Option<glow::VertexArray>,
    vbo: Option<glow::Buffer>,

    frame_statistics: PipelineStatistics,
    gl_kind: GlKind,

    pub(crate) queries: Vec<glow::Query>,

    #[cfg(not(target_arch = "wasm32"))]
    gl_context: PossiblyCurrentContext,
    #[cfg(not(target_arch = "wasm32"))]
    gl_surface: Surface<WindowSurface>,
}

impl InnerState {
    fn new(
        gl_kind: GlKind,
        #[cfg(not(target_arch = "wasm32"))] gl_context: PossiblyCurrentContext,
        #[cfg(not(target_arch = "wasm32"))] gl_surface: Surface<WindowSurface>,
    ) -> Self {
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
            texture_units_storage: TextureUnitsStorage {
                active_unit: 0,
                units: Default::default(),
            },
            stencil_func: Default::default(),
            stencil_op: Default::default(),
            vao: Default::default(),
            vbo: Default::default(),
            frame_statistics: Default::default(),
            blend_equation: Default::default(),
            gl_kind,
            queries: Default::default(),
            #[cfg(not(target_arch = "wasm32"))]
            gl_context,
            #[cfg(not(target_arch = "wasm32"))]
            gl_surface,
        }
    }
}

pub type SharedPipelineState = Rc<PipelineState>;

pub struct PipelineState {
    pub gl: glow::Context,
    pub(crate) state: RefCell<InnerState>,
    this: RefCell<Option<Weak<PipelineState>>>,
}

#[derive(Copy, Clone)]
struct TextureBinding {
    target: u32,
    texture: Option<glow::Texture>,
}

#[derive(Copy, Clone)]
struct TextureUnit {
    bindings: [TextureBinding; 4],
}

impl Default for TextureUnit {
    fn default() -> Self {
        Self {
            bindings: [
                TextureBinding {
                    target: glow::TEXTURE_2D,
                    texture: None,
                },
                TextureBinding {
                    target: glow::TEXTURE_3D,
                    texture: None,
                },
                TextureBinding {
                    target: glow::TEXTURE_1D,
                    texture: None,
                },
                TextureBinding {
                    target: glow::TEXTURE_CUBE_MAP,
                    texture: None,
                },
            ],
        }
    }
}

#[derive(Default)]
struct TextureUnitsStorage {
    active_unit: u32,
    units: [TextureUnit; 32],
}

impl PipelineState {
    pub fn new(
        #[allow(unused_variables)] params: &GraphicsContextParams,
        window_target: &EventLoopWindowTarget<()>,
        window_builder: WindowBuilder,
    ) -> Result<(Window, SharedPipelineState), EngineError> {
        #[cfg(not(target_arch = "wasm32"))]
        let (window, gl_context, gl_surface, mut context, gl_kind) = {
            let mut template = ConfigTemplateBuilder::new()
                .prefer_hardware_accelerated(Some(true))
                .with_stencil_size(8)
                .with_depth_size(24);

            if let Some(sample_count) = params.msaa_sample_count {
                template = template.with_multisampling(sample_count);
            }

            let (opt_window, gl_config) = DisplayBuilder::new()
                .with_window_builder(Some(window_builder))
                .build(window_target, template, |mut configs| {
                    configs.next().unwrap()
                })?;

            let window = opt_window.unwrap();

            let raw_window_handle = window.raw_window_handle();

            let gl_display = gl_config.display();

            #[cfg(debug_assertions)]
            let debug = true;

            #[cfg(not(debug_assertions))]
            let debug = true;

            let gl3_3_core_context_attributes = ContextAttributesBuilder::new()
                .with_debug(debug)
                .with_profile(GlProfile::Core)
                .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
                .build(Some(raw_window_handle));

            let gles3_context_attributes = ContextAttributesBuilder::new()
                .with_debug(debug)
                .with_profile(GlProfile::Core)
                .with_context_api(ContextApi::Gles(Some(Version::new(3, 0))))
                .build(Some(raw_window_handle));

            unsafe {
                let attrs = window.build_surface_attributes(Default::default());

                let gl_surface = gl_config
                    .display()
                    .create_window_surface(&gl_config, &attrs)?;

                let (non_current_gl_context, gl_kind) = if let Ok(gl3_3_core_context) =
                    gl_display.create_context(&gl_config, &gl3_3_core_context_attributes)
                {
                    (gl3_3_core_context, GlKind::OpenGL)
                } else {
                    (
                        gl_display.create_context(&gl_config, &gles3_context_attributes)?,
                        GlKind::OpenGLES,
                    )
                };

                let gl_context = non_current_gl_context.make_current(&gl_surface)?;

                if params.vsync {
                    Log::verify(gl_surface.set_swap_interval(
                        &gl_context,
                        SwapInterval::Wait(NonZeroU32::new(1).unwrap()),
                    ));
                }

                (
                    window,
                    gl_context,
                    gl_surface,
                    glow::Context::from_loader_function(|s| {
                        gl_display.get_proc_address(&CString::new(s).unwrap())
                    }),
                    gl_kind,
                )
            }
        };

        #[cfg(target_arch = "wasm32")]
        let (window, mut context, gl_kind) = {
            use crate::{
                core::wasm_bindgen::JsCast,
                dpi::{LogicalSize, PhysicalSize},
                platform::web::WindowExtWebSys,
            };
            use serde::{Deserialize, Serialize};

            let inner_size = window_builder.window_attributes().inner_size;
            let window = window_builder.build(window_target).unwrap();

            let web_window = crate::core::web_sys::window().unwrap();
            let scale_factor = web_window.device_pixel_ratio();

            let canvas = window.canvas().unwrap();

            // For some reason winit completely ignores the requested inner size. This is a quick-n-dirty fix
            // that also handles HiDPI monitors. It has one issue - if user changes DPI, it won't be handled
            // correctly.
            if let Some(inner_size) = inner_size {
                let physical_inner_size: PhysicalSize<u32> = inner_size.to_physical(scale_factor);

                canvas.set_width(physical_inner_size.width);
                canvas.set_height(physical_inner_size.height);

                let logical_inner_size: LogicalSize<f64> = inner_size.to_logical(scale_factor);
                Log::verify(
                    canvas
                        .style()
                        .set_property("width", &format!("{}px", logical_inner_size.width)),
                );
                Log::verify(
                    canvas
                        .style()
                        .set_property("height", &format!("{}px", logical_inner_size.height)),
                );
            }

            let document = web_window.document().unwrap();
            let body = document.body().unwrap();

            body.append_child(&canvas)
                .expect("Append canvas to HTML body");

            #[derive(Serialize, Deserialize)]
            #[allow(non_snake_case)]
            struct ContextAttributes {
                alpha: bool,
                premultipliedAlpha: bool,
                powerPreference: String,
            }

            let context_attributes = ContextAttributes {
                // Prevent blending with the background of the canvas. Otherwise the background
                // will "leak" and interfere with the pixels produced by the engine.
                alpha: false,
                premultipliedAlpha: false,
                // Try to use high performance GPU.
                powerPreference: "high-performance".to_string(),
            };

            let webgl2_context = canvas
                .get_context_with_context_options(
                    "webgl2",
                    &serde_wasm_bindgen::to_value(&context_attributes).unwrap(),
                )
                .unwrap()
                .unwrap()
                .dyn_into::<crate::core::web_sys::WebGl2RenderingContext>()
                .unwrap();
            (
                window,
                glow::Context::from_webgl2_context(webgl2_context),
                GlKind::OpenGLES,
            )
        };

        #[cfg(not(target_arch = "wasm32"))]
        gl_surface.resize(
            &gl_context,
            NonZeroU32::new(window.inner_size().width)
                .unwrap_or_else(|| NonZeroU32::new(1).unwrap()),
            NonZeroU32::new(window.inner_size().height)
                .unwrap_or_else(|| NonZeroU32::new(1).unwrap()),
        );

        // Dump available GL extensions to the log, this will help debugging graphical issues.
        Log::info(format!(
            "Supported GL Extensions: {:?}",
            context.supported_extensions()
        ));

        unsafe {
            context.depth_func(CompareFunc::default().into_gl());

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
            state: RefCell::new(InnerState::new(
                gl_kind,
                #[cfg(not(target_arch = "wasm32"))]
                gl_context,
                #[cfg(not(target_arch = "wasm32"))]
                gl_surface,
            )),
            this: Default::default(),
        };

        let shared = SharedPipelineState::new(state);

        *shared.this.borrow_mut() = Some(Rc::downgrade(&shared));

        Ok((window, shared))
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
                self.gl.polygon_mode(
                    state.polygon_face.into_gl(),
                    state.polygon_fill_mode.into_gl(),
                )
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

            unsafe { self.gl.cull_face(state.cull_face.into_gl()) }
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
                    state.blend_func.sfactor.into_gl(),
                    state.blend_func.dfactor.into_gl(),
                    state.blend_func.alpha_sfactor.into_gl(),
                    state.blend_func.alpha_dfactor.into_gl(),
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
                    state.blend_equation.rgb.into_gl(),
                    state.blend_equation.alpha.into_gl(),
                );
            }
        }
    }

    pub fn set_depth_func(&self, depth_func: CompareFunc) {
        let mut state = self.state.borrow_mut();
        if state.depth_func != depth_func {
            state.depth_func = depth_func;

            unsafe {
                self.gl.depth_func(depth_func.into_gl());
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

    pub fn set_texture(&self, unit_index: u32, target: u32, texture: Option<glow::Texture>) {
        unsafe fn bind_texture(
            gl: &glow::Context,
            target: u32,
            texture: Option<glow::Texture>,
            unit_index: u32,
            active_unit: &mut u32,
        ) {
            if *active_unit != unit_index {
                *active_unit = unit_index;
                gl.active_texture(glow::TEXTURE0 + unit_index);
            }
            gl.bind_texture(target, texture);
        }

        unsafe {
            let mut state_guard = self.state.borrow_mut();
            let state = state_guard.deref_mut();

            let unit = &mut state.texture_units_storage.units[unit_index as usize];
            let active_unit = &mut state.texture_units_storage.active_unit;
            for binding in unit.bindings.iter_mut() {
                if binding.target == target {
                    if binding.texture != texture {
                        binding.texture = texture;
                        bind_texture(&self.gl, binding.target, texture, unit_index, active_unit);
                        state.frame_statistics.texture_binding_changes += 1;
                    }
                } else if binding.texture.is_some() {
                    binding.texture = None;
                    bind_texture(&self.gl, binding.target, None, unit_index, active_unit);
                    state.frame_statistics.texture_binding_changes += 1;
                }
            }
        }
    }

    pub fn set_stencil_func(&self, func: StencilFunc) {
        let mut state = self.state.borrow_mut();
        if state.stencil_func != func {
            state.stencil_func = func;

            unsafe {
                self.gl.stencil_func(
                    state.stencil_func.func.into_gl(),
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
                    state.stencil_op.fail.into_gl(),
                    state.stencil_op.zfail.into_gl(),
                    state.stencil_op.zpass.into_gl(),
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

    pub fn flush(&self) {
        unsafe {
            self.gl.flush();
        }
    }

    pub fn finish(&self) {
        unsafe {
            self.gl.finish();
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
        state.texture_units_storage = Default::default();
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

    pub fn swap_buffers(&self) -> Result<(), FrameworkError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let state = self.state.borrow();
            Ok(state.gl_surface.swap_buffers(&state.gl_context)?)
        }

        #[cfg(target_arch = "wasm32")]
        {
            Ok(())
        }
    }

    pub fn set_frame_size(&self, #[allow(unused_variables)] new_size: (u32, u32)) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::num::NonZeroU32;
            let state = self.state.borrow();
            state.gl_surface.resize(
                &state.gl_context,
                NonZeroU32::new(new_size.0).unwrap_or_else(|| NonZeroU32::new(1).unwrap()),
                NonZeroU32::new(new_size.1).unwrap_or_else(|| NonZeroU32::new(1).unwrap()),
            );
        }
    }
}
