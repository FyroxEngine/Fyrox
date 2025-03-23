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
    core::{arrayvec::ArrayVec, log::Log, math::Rect, sstorage::ImmutableString},
    material::{
        shader::{Shader, ShaderResource},
        MaterialPropertyRef,
    },
    renderer::{
        bundle,
        cache::{uniform::UniformBufferCache, TemporaryCache},
        framework::{
            error::FrameworkError,
            framebuffer::{DrawCallStatistics, GpuFrameBuffer, ResourceBindGroup, ResourceBinding},
            geometry_buffer::GpuGeometryBuffer,
            gpu_program::{GpuProgram, ShaderResourceDefinition, ShaderResourceKind},
            gpu_texture::GpuTexture,
            server::GraphicsServer,
            DrawParameters, ElementRange,
        },
    },
};
use fxhash::FxHashMap;
use fyrox_graphics::sampler::GpuSampler;
use fyrox_graphics::uniform::StaticUniformBuffer;
use std::ops::Deref;

pub struct NamedValue<T> {
    pub name: ImmutableString,
    pub value: T,
}

impl<T> NamedValue<T> {
    pub fn new(name: impl Into<ImmutableString>, value: T) -> Self {
        Self {
            name: name.into(),
            value,
        }
    }
}

pub struct NamedValuesContainer<T, const N: usize> {
    properties: [NamedValue<T>; N],
}

fn search<'a, T>(slice: &'a [NamedValue<T>], name: &ImmutableString) -> Option<&'a NamedValue<T>> {
    slice
        .binary_search_by(|prop| prop.name.cached_hash().cmp(&name.cached_hash()))
        .ok()
        .and_then(|idx| slice.get(idx))
}

impl<T, const N: usize> NamedValuesContainer<T, N> {
    pub fn property_ref(&self, name: &ImmutableString) -> Option<&NamedValue<T>> {
        search(&self.properties, name)
    }

    pub fn data_ref(&self) -> NamedValuesContainerRef<'_, T> {
        NamedValuesContainerRef {
            properties: &self.properties,
        }
    }
}

impl<T, const N: usize> From<[NamedValue<T>; N]> for NamedValuesContainer<T, N> {
    fn from(mut value: [NamedValue<T>; N]) -> Self {
        value.sort_unstable_by_key(|prop| prop.name.cached_hash());
        Self { properties: value }
    }
}

impl<T, const N: usize> Deref for NamedValuesContainer<T, N> {
    type Target = [NamedValue<T>];

    fn deref(&self) -> &Self::Target {
        &self.properties
    }
}

pub struct NamedValuesContainerRef<'a, T> {
    properties: &'a [NamedValue<T>],
}

impl<T> NamedValuesContainerRef<'_, T> {
    pub fn property_ref(&self, name: &ImmutableString) -> Option<&NamedValue<T>> {
        search(self.properties, name)
    }
}

pub struct PropertyGroup<'a, const N: usize> {
    pub properties: NamedValuesContainer<MaterialPropertyRef<'a>, N>,
}

pub type NamedPropertyRef<'a> = NamedValue<MaterialPropertyRef<'a>>;

pub fn property<'a>(
    name: impl Into<ImmutableString>,
    value: impl Into<MaterialPropertyRef<'a>>,
) -> NamedPropertyRef<'a> {
    NamedValue::new(name, value.into())
}

pub fn binding<'a, 'b>(
    name: impl Into<ImmutableString>,
    value: impl Into<GpuResourceBinding<'a, 'b>>,
) -> NamedValue<GpuResourceBinding<'a, 'b>> {
    NamedValue::new(name, value.into())
}

impl<'a> From<(&'a GpuTexture, &'a GpuSampler)> for GpuResourceBinding<'a, '_> {
    fn from(value: (&'a GpuTexture, &'a GpuSampler)) -> Self {
        GpuResourceBinding::Texture {
            texture: value.0,
            sampler: value.1,
        }
    }
}

impl<'a, 'b, const N: usize> From<&'a PropertyGroup<'b, N>> for GpuResourceBinding<'a, 'b> {
    fn from(value: &'a PropertyGroup<'b, N>) -> Self {
        GpuResourceBinding::PropertyGroup {
            properties: value.properties.data_ref(),
        }
    }
}

impl<'a, const N: usize> From<[NamedPropertyRef<'a>; N]> for PropertyGroup<'a, N> {
    fn from(value: [NamedPropertyRef<'a>; N]) -> Self {
        Self {
            properties: value.into(),
        }
    }
}

impl<'a, const N: usize> Deref for PropertyGroup<'a, N> {
    type Target = [NamedValue<MaterialPropertyRef<'a>>];

    fn deref(&self) -> &Self::Target {
        &self.properties
    }
}

pub enum GpuResourceBinding<'a, 'b> {
    Texture {
        texture: &'a GpuTexture,
        sampler: &'a GpuSampler,
    },
    PropertyGroup {
        properties: NamedValuesContainerRef<'a, MaterialPropertyRef<'b>>,
    },
}

impl<'a, 'b> GpuResourceBinding<'a, 'b> {
    pub fn texture(texture: &'a GpuTexture, sampler: &'a GpuSampler) -> Self {
        Self::Texture { texture, sampler }
    }

    pub fn property_group<const N: usize>(properties: &'a PropertyGroup<'b, N>) -> Self {
        Self::PropertyGroup {
            properties: properties.properties.data_ref(),
        }
    }
}

pub struct RenderMaterial<'a, 'b, const N: usize> {
    pub bindings: NamedValuesContainer<GpuResourceBinding<'a, 'b>, N>,
}

impl<'a, 'b, const N: usize> From<[NamedValue<GpuResourceBinding<'a, 'b>>; N]>
    for RenderMaterial<'a, 'b, N>
{
    fn from(value: [NamedValue<GpuResourceBinding<'a, 'b>>; N]) -> Self {
        Self {
            bindings: value.into(),
        }
    }
}

pub struct RenderPassData {
    pub program: GpuProgram,
    pub draw_params: DrawParameters,
}

pub struct RenderPassContainer {
    pub resources: Vec<ShaderResourceDefinition>,
    pub render_passes: FxHashMap<ImmutableString, RenderPassData>,
}

impl RenderPassContainer {
    pub fn from_str(server: &dyn GraphicsServer, str: &str) -> Result<Self, FrameworkError> {
        let shader = Shader::from_string(str).map_err(|e| FrameworkError::Custom(e.to_string()))?;
        Self::new(server, &shader)
    }

    pub fn new(server: &dyn GraphicsServer, shader: &Shader) -> Result<Self, FrameworkError> {
        let mut render_passes = FxHashMap::default();

        for render_pass in shader.definition.passes.iter() {
            let program_name = format!("{}_{}", shader.definition.name, render_pass.name);
            match server.create_program(
                &program_name,
                render_pass.vertex_shader.clone(),
                render_pass.vertex_shader_line,
                render_pass.fragment_shader.clone(),
                render_pass.fragment_shader_line,
                &shader.definition.resources,
            ) {
                Ok(gpu_program) => {
                    render_passes.insert(
                        ImmutableString::new(&render_pass.name),
                        RenderPassData {
                            program: gpu_program,
                            draw_params: render_pass.draw_parameters.clone(),
                        },
                    );
                }
                Err(e) => {
                    return Err(FrameworkError::Custom(format!(
                        "Failed to create {program_name} shader' GPU program. Reason: {e}"
                    )));
                }
            };
        }

        Ok(Self {
            render_passes,
            resources: shader.definition.resources.clone(),
        })
    }

    pub fn get(
        &self,
        render_pass_name: &ImmutableString,
    ) -> Result<&RenderPassData, FrameworkError> {
        self.render_passes.get(render_pass_name).ok_or_else(|| {
            FrameworkError::Custom(format!("No render pass with name {render_pass_name}!"))
        })
    }

    pub fn run_pass<const N: usize>(
        &self,
        instance_count: usize,
        render_pass_name: &ImmutableString,
        framebuffer: &GpuFrameBuffer,
        geometry: &GpuGeometryBuffer,
        viewport: Rect<i32>,
        material: &RenderMaterial<'_, '_, N>,
        uniform_buffer_cache: &mut UniformBufferCache,
        element_range: ElementRange,
        override_params: Option<&DrawParameters>,
    ) -> Result<DrawCallStatistics, FrameworkError> {
        if instance_count == 0 {
            return Ok(Default::default());
        }

        let render_pass = self.get(render_pass_name)?;

        let mut resource_bindings = ArrayVec::<ResourceBinding, 32>::new();

        for resource in self.resources.iter() {
            // Ignore built-in groups.
            if resource.is_built_in() {
                continue;
            }

            match resource.kind {
                ShaderResourceKind::Texture { .. } => {
                    if let Some((tex, sampler)) = material
                        .bindings
                        .property_ref(&resource.name)
                        .and_then(|p| {
                            if let GpuResourceBinding::Texture { texture, sampler } = p.value {
                                Some((texture, sampler))
                            } else {
                                None
                            }
                        })
                    {
                        resource_bindings.push(ResourceBinding::texture(
                            tex,
                            sampler,
                            resource.binding,
                        ));
                    } else {
                        return Err(FrameworkError::Custom(format!(
                            "No texture bound to {} resource binding!",
                            resource.name
                        )));
                    }
                }
                ShaderResourceKind::PropertyGroup(ref shader_property_group) => {
                    let mut buf = StaticUniformBuffer::<16384>::new();

                    if let Some(material_property_group) = material
                        .bindings
                        .property_ref(&resource.name)
                        .and_then(|p| {
                            if let GpuResourceBinding::PropertyGroup { ref properties } = p.value {
                                Some(properties)
                            } else {
                                None
                            }
                        })
                    {
                        bundle::write_with_material(
                            shader_property_group,
                            material_property_group,
                            |c: &NamedValuesContainerRef<MaterialPropertyRef>, n| {
                                c.property_ref(n).map(|v| v.value)
                            },
                            &mut buf,
                        );
                    } else {
                        // No respective resource bound in the material, use shader defaults. This is very
                        // important, because some drivers will crash if uniform buffer has insufficient
                        // data.
                        bundle::write_shader_values(shader_property_group, &mut buf)
                    }

                    resource_bindings.push(ResourceBinding::buffer(
                        &uniform_buffer_cache.write(buf)?,
                        resource.binding,
                        Default::default(),
                    ));
                }
            }
        }

        let resources = [ResourceBindGroup {
            bindings: &resource_bindings,
        }];

        if instance_count == 1 {
            framebuffer.draw(
                geometry,
                viewport,
                &render_pass.program,
                override_params.unwrap_or(&render_pass.draw_params),
                &resources,
                element_range,
            )
        } else {
            framebuffer.draw_instances(
                instance_count,
                geometry,
                viewport,
                &render_pass.program,
                override_params.unwrap_or(&render_pass.draw_params),
                &resources,
                element_range,
            )
        }
    }
}

#[derive(Default)]
pub struct ShaderCache {
    pub(super) cache: TemporaryCache<RenderPassContainer>,
}

impl ShaderCache {
    pub fn remove(&mut self, shader: &ShaderResource) {
        let mut state = shader.state();
        if let Some(shader_state) = state.data() {
            self.cache.remove(&shader_state.cache_index);
        }
    }

    pub fn get(
        &mut self,
        server: &dyn GraphicsServer,
        shader: &ShaderResource,
    ) -> Option<&RenderPassContainer> {
        let mut shader_state = shader.state();

        if let Some(shader_state) = shader_state.data() {
            match self.cache.get_or_insert_with(
                &shader_state.cache_index,
                Default::default(),
                || RenderPassContainer::new(server, shader_state),
            ) {
                Ok(shader_set) => Some(shader_set),
                Err(error) => {
                    Log::err(format!("{error}"));
                    None
                }
            }
        } else {
            None
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.cache.update(dt)
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }

    pub fn alive_count(&self) -> usize {
        self.cache.alive_count()
    }
}
