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

use crate::{
    core::sstorage::ImmutableString,
    renderer::framework::{
        error::FrameworkError,
        gpu_program::{GpuProgram, UniformLocation},
        server::GraphicsServer,
    },
};

pub struct DecalShader {
    pub scene_depth: UniformLocation,
    pub diffuse_texture: UniformLocation,
    pub normal_texture: UniformLocation,
    pub decal_mask: UniformLocation,
    pub program: Box<dyn GpuProgram>,
    pub uniform_buffer_binding: usize,
}

impl DecalShader {
    pub fn new(server: &dyn GraphicsServer) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/decal_fs.glsl");
        let vertex_source = include_str!("../shaders/decal_vs.glsl");

        let program = server.create_program("DecalShader", vertex_source, fragment_source)?;
        Ok(Self {
            uniform_buffer_binding: program
                .uniform_block_index(&ImmutableString::new("Uniforms"))?,
            scene_depth: program.uniform_location(&ImmutableString::new("sceneDepth"))?,
            diffuse_texture: program.uniform_location(&ImmutableString::new("diffuseTexture"))?,
            normal_texture: program.uniform_location(&ImmutableString::new("normalTexture"))?,
            decal_mask: program.uniform_location(&ImmutableString::new("decalMask"))?,
            program,
        })
    }
}
