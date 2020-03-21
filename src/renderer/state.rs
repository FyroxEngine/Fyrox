use crate::{
    renderer::{
        framebuffer::{
            CullFace,
            DrawParameters,
        },
        gl::{
            self,
            types::{
                GLuint,
                GLboolean,
                GLenum
            }
        }
    },
    core::{
        math::Rect,
        color::Color
    }
};

pub struct State {
    blend: bool,
    depth_test: bool,
    depth_write: bool,
    color_write: (bool, bool, bool, bool),
    stencil_test: bool,
    cull_face: CullFace,
    culling: bool,
    stencil_mask: u32,
    clear_color: Color,
    clear_stencil: i32,
    clear_depth: f32,

    framebuffer: GLuint,
    viewport: Rect<i32>,

    blend_src_factor: GLuint,
    blend_dst_factor: GLuint,
}

fn bool_to_gl_bool(v: bool) -> GLboolean {
    if v {
        gl::TRUE
    } else {
        gl::FALSE
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            blend: false,
            depth_test: false,
            depth_write: true,
            color_write: (true, true, true, true),
            stencil_test: false,
            cull_face: CullFace::Back,
            culling: false,
            stencil_mask: 0xFFFF_FFFF,
            clear_color: Color::from_rgba(0, 0, 0, 0),
            clear_stencil: 0,
            clear_depth: 1.0,
            framebuffer: 0,
            viewport: Rect {
                x: 0,
                y: 0,
                w: 1,
                h: 1,
            },
            blend_src_factor: gl::ONE,
            blend_dst_factor: gl::ZERO
        }
    }

    pub fn set_framebuffer(&mut self, framebuffer: GLuint) {
        unsafe {
            if self.framebuffer != framebuffer {
                self.framebuffer = framebuffer;

                gl::BindFramebuffer(gl::FRAMEBUFFER, self.framebuffer)
            }
        }
    }

    pub fn set_viewport(&mut self, viewport: Rect<i32>) {
        unsafe {
            if self.viewport != viewport {
                self.viewport = viewport;

                gl::Viewport(self.viewport.x, self.viewport.y, self.viewport.w, self.viewport.h);
            }
        }
    }

    pub fn set_blend(&mut self, blend: bool) {
        unsafe {
            if self.blend != blend {
                self.blend = blend;

                if self.blend {
                    gl::Enable(gl::BLEND);
                } else {
                    gl::Disable(gl::BLEND);
                }
            }
        }
    }

    pub fn set_depth_test(&mut self, depth_test: bool) {
        unsafe {
            if self.depth_test != depth_test {
                self.depth_test = depth_test;

                if self.depth_test {
                    gl::Enable(gl::DEPTH_TEST);
                } else {
                    gl::Disable(gl::DEPTH_TEST);
                }
            }
        }
    }

    pub fn set_depth_write(&mut self, depth_write: bool) {
        unsafe {
            if self.depth_write != depth_write {
                self.depth_write = depth_write;

                gl::DepthMask(bool_to_gl_bool(self.depth_write));
            }
        }
    }

    pub fn set_color_write(&mut self, color_write: (bool, bool, bool, bool)) {
        unsafe {
            if self.color_write != color_write {
                self.color_write = color_write;

                gl::ColorMask(bool_to_gl_bool(self.color_write.0),
                              bool_to_gl_bool(self.color_write.1),
                              bool_to_gl_bool(self.color_write.2),
                              bool_to_gl_bool(self.color_write.3));
            }
        }
    }

    pub fn set_stencil_test(&mut self, stencil_test: bool) {
        unsafe {
            if self.stencil_test != stencil_test {
                self.stencil_test = stencil_test;

                if self.stencil_test {
                    gl::Enable(gl::STENCIL_TEST);
                } else {
                    gl::Disable(gl::STENCIL_TEST);
                }
            }
        }
    }

    pub fn set_cull_face(&mut self, cull_face: CullFace) {
        unsafe {
            if self.cull_face != cull_face {
                self.cull_face = cull_face;

                gl::CullFace(self.cull_face.into_gl_value())
            }
        }
    }

    pub fn set_culling(&mut self, culling: bool) {
        unsafe {
            if self.culling != culling {
                self.culling = culling;

                if self.culling {
                    gl::Enable(gl::CULL_FACE);
                } else {
                    gl::Disable(gl::CULL_FACE);
                }
            }
        }
    }

    pub fn set_stencil_mask(&mut self, stencil_mask: u32) {
        unsafe {
            if self.stencil_mask != stencil_mask {
                self.stencil_mask = stencil_mask;

                gl::StencilMask(stencil_mask);
            }
        }
    }

    pub fn set_clear_color(&mut self, color: Color) {
        unsafe {
            if self.clear_color != color {
                self.clear_color = color;

                let rgba = color.as_frgba();
                gl::ClearColor(rgba.x, rgba.y, rgba.z, rgba.w);
            }
        }
    }

    pub fn set_clear_depth(&mut self, depth: f32) {
        unsafe {
            if self.clear_depth != depth {
                self.clear_depth = depth;

                gl::ClearDepth(depth as f64);
            }
        }
    }

    pub fn set_clear_stencil(&mut self, stencil: i32) {
        unsafe {
            if self.clear_stencil != stencil {
                self.clear_stencil = stencil;

                gl::ClearStencil(stencil);
            }
        }
    }

    pub fn set_blend_func(&mut self, sfactor: GLenum, dfactor: GLenum) {
        if self.blend_src_factor != sfactor || self.blend_dst_factor != dfactor {
            self.blend_src_factor = sfactor;
            self.blend_dst_factor = dfactor;

            unsafe {
                gl::BlendFunc(self.blend_src_factor, self.blend_dst_factor);
            }
        }
    }

    pub fn apply_draw_parameters(&mut self, draw_params: &DrawParameters) {
        self.set_blend(draw_params.blend);
        self.set_depth_test(draw_params.depth_test);
        self.set_depth_write(draw_params.depth_write);
        self.set_color_write(draw_params.color_write);
        self.set_stencil_test(draw_params.stencil_test);
        self.set_cull_face(draw_params.cull_face);
        self.set_culling(draw_params.culling);
    }
}