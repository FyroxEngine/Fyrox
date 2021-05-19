use crate::{
    core::scope_profile,
    engine::resource_manager::TimedEntry,
    renderer::{
        batch::InstanceData,
        framework::{
            geometry_buffer::{
                AttributeDefinition, AttributeKind, BufferBuilder, ElementKind, GeometryBuffer,
                GeometryBufferBuilder, GeometryBufferKind,
            },
            gpu_texture::{Coordinate, GpuTexture, PixelKind},
            state::PipelineState,
        },
    },
    resource::texture::{Texture, TextureState},
    scene::mesh::surface::SurfaceData,
    utils::log::{Log, MessageKind},
};
use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    ops::Deref,
    rc::Rc,
};

#[derive(Default)]
pub(in crate) struct GeometryCache {
    map: HashMap<usize, TimedEntry<GeometryBuffer>>,
}

impl GeometryCache {
    pub fn get(&mut self, state: &mut PipelineState, data: &SurfaceData) -> &mut GeometryBuffer {
        scope_profile!();

        let key = (data as *const _) as usize;

        let geometry_buffer = self.map.entry(key).or_insert_with(|| {
            let geometry_buffer = GeometryBufferBuilder::new(ElementKind::Triangle)
                .with_buffer_builder(BufferBuilder::from_vertex_buffer(
                    data.vertex_buffer(),
                    GeometryBufferKind::StaticDraw,
                ))
                // Buffer for world and world-view-projection matrices per instance.
                .with_buffer_builder(
                    BufferBuilder::new::<InstanceData>(GeometryBufferKind::DynamicDraw, None)
                        .with_attribute(AttributeDefinition {
                            location: 7,
                            kind: AttributeKind::UnsignedByte4,
                            normalized: true,
                            divisor: 1,
                        })
                        // World Matrix
                        .with_attribute(AttributeDefinition {
                            location: 8,
                            kind: AttributeKind::Float4,
                            normalized: false,
                            divisor: 1,
                        })
                        .with_attribute(AttributeDefinition {
                            location: 9,
                            kind: AttributeKind::Float4,
                            normalized: false,
                            divisor: 1,
                        })
                        .with_attribute(AttributeDefinition {
                            location: 10,
                            kind: AttributeKind::Float4,
                            normalized: false,
                            divisor: 1,
                        })
                        .with_attribute(AttributeDefinition {
                            location: 11,
                            kind: AttributeKind::Float4,
                            normalized: false,
                            divisor: 1,
                        })
                        // Depth offset.
                        .with_attribute(AttributeDefinition {
                            location: 12,
                            kind: AttributeKind::Float,
                            normalized: false,
                            divisor: 1,
                        }),
                )
                .build(state)
                .unwrap();

            geometry_buffer.bind(state).set_triangles(data.triangles());

            TimedEntry {
                value: geometry_buffer,
                time_to_live: 20.0,
            }
        });

        geometry_buffer.time_to_live = 20.0;
        geometry_buffer
    }

    pub fn update(&mut self, dt: f32) {
        scope_profile!();

        for entry in self.map.values_mut() {
            entry.time_to_live -= dt;
        }
        self.map.retain(|_, v| v.time_to_live > 0.0);
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }
}

#[derive(Default)]
pub(in crate) struct TextureCache {
    pub(super) map: HashMap<usize, TimedEntry<Rc<RefCell<GpuTexture>>>>,
}

impl TextureCache {
    pub fn get(
        &mut self,
        state: &mut PipelineState,
        texture: &Texture,
    ) -> Option<Rc<RefCell<GpuTexture>>> {
        scope_profile!();

        let key = texture.key();
        let texture = texture.state();

        if let TextureState::Ok(texture) = texture.deref() {
            let entry = match self.map.entry(key) {
                Entry::Occupied(e) => {
                    let entry = e.into_mut();

                    // Texture won't be destroyed while it used.
                    entry.time_to_live = 20.0;

                    // Check if some value has changed in resource.
                    let mut tex = entry.borrow_mut();

                    let new_mag_filter = texture.magnification_filter().into();
                    if tex.magnification_filter() != new_mag_filter {
                        tex.bind_mut(state, 0)
                            .set_magnification_filter(new_mag_filter);
                    }

                    let new_min_filter = texture.minification_filter().into();
                    if tex.minification_filter() != new_min_filter {
                        tex.bind_mut(state, 0)
                            .set_minification_filter(new_min_filter);
                    }

                    if tex.anisotropy().ne(&texture.anisotropy_level()) {
                        tex.bind_mut(state, 0)
                            .set_anisotropy(texture.anisotropy_level());
                    }

                    let new_s_wrap_mode = texture.s_wrap_mode().into();
                    if tex.s_wrap_mode() != new_s_wrap_mode {
                        tex.bind_mut(state, 0)
                            .set_wrap(Coordinate::S, new_s_wrap_mode);
                    }

                    let new_t_wrap_mode = texture.t_wrap_mode().into();
                    if tex.t_wrap_mode() != new_t_wrap_mode {
                        tex.bind_mut(state, 0)
                            .set_wrap(Coordinate::T, new_t_wrap_mode);
                    }

                    std::mem::drop(tex);

                    entry
                }
                Entry::Vacant(e) => {
                    let gpu_texture = match GpuTexture::new(
                        state,
                        texture.kind.into(),
                        PixelKind::from(texture.pixel_kind),
                        texture.minification_filter().into(),
                        texture.magnification_filter().into(),
                        texture.mip_count() as usize,
                        Some(texture.bytes.as_slice()),
                    ) {
                        Ok(texture) => texture,
                        Err(e) => {
                            Log::writeln(
                                MessageKind::Error,
                                format!("Failed to create GPU texture. Reason: {:?}", e),
                            );
                            return None;
                        }
                    };

                    e.insert(TimedEntry {
                        value: Rc::new(RefCell::new(gpu_texture)),
                        time_to_live: 20.0,
                    })
                }
            };

            Some(entry.value.clone())
        } else {
            None
        }
    }

    pub fn update(&mut self, dt: f32) {
        scope_profile!();

        for entry in self.map.values_mut() {
            entry.time_to_live -= dt;
        }
        self.map.retain(|_, v| v.time_to_live > 0.0);
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }

    pub fn unload(&mut self, texture: Texture) {
        self.map.remove(&texture.key());
    }
}
