use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use zerocopy::AsBytes;

use wgpu::*;

use crate::draw::{Command as DrawCommand, DrawList, Update, Vertex};
use crate::layout::Rectangle;
use crate::loader::Loader;
use crate::{Command, EventLoop, Model};

/// Wrapper for [`Ui`](../../struct.Ui.html) that adds wgpu rendering.
/// Requires the "wgpu" feature.
pub struct Ui<M: Model, E: EventLoop<Command<M::Message>>, L: Loader> {
    inner: crate::Ui<M, E, L>,
    pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    sampler: Sampler,
    textures: HashMap<usize, TextureEntry>,
    vertex_buffer: Option<Buffer>,
    draw_commands: Vec<DrawCommand>,
}

struct TextureEntry {
    texture: Texture,
    bind_group: BindGroup,
}

impl<M: Model, E: EventLoop<Command<M::Message>>, L: Loader> Ui<M, E, L> {
    /// Constructs a new `Ui` using the default style.
    /// This is not recommended as the default style is very empty and only renders white text.
    pub fn new(
        model: M,
        event_loop: E,
        loader: L,
        viewport: Rectangle,
        format: wgpu::TextureFormat,
        device: &Device,
    ) -> Self {
        Self::new_inner(crate::Ui::new(model, event_loop, loader, viewport), format, device)
    }

    fn new_inner(inner: crate::Ui<M, E, L>, format: wgpu::TextureFormat, device: &Device) -> Self {
        let vs_module = device.create_shader_module(
            wgpu::read_spirv(std::io::Cursor::new(&include_bytes!("wgpu_shader.vert.spv")[..]))
                .expect("unable to load shader module")
                .as_slice(),
        );
        let fs_module = device.create_shader_module(
            wgpu::read_spirv(std::io::Cursor::new(&include_bytes!("wgpu_shader.frag.spv")[..]))
                .expect("unable to load shader module")
                .as_slice(),
        );
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            bindings: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture {
                        dimension: wgpu::TextureViewDimension::D2,
                        component_type: wgpu::TextureComponentType::Uint,
                        multisampled: false,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler { comparison: false },
                },
            ],
            label: None,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: &pipeline_layout,
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &fs_module,
                entry_point: "main",
            }),
            // Use the default rasterizer state: no culling, no depth bias
            rasterization_state: None,
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[wgpu::ColorStateDescriptor {
                format,
                color_blend: wgpu::BlendDescriptor {
                    src_factor: BlendFactor::SrcAlpha,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                },
                alpha_blend: wgpu::BlendDescriptor {
                    src_factor: BlendFactor::Zero,
                    dst_factor: BlendFactor::Zero,
                    operation: BlendOperation::Add,
                },
                write_mask: wgpu::ColorWrite::ALL,
            }],
            depth_stencil_state: None,
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint16,
                vertex_buffers: &[wgpu::VertexBufferDescriptor {
                    stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttributeDescriptor {
                            offset: 0,
                            format: wgpu::VertexFormat::Float2,
                            shader_location: 0,
                        },
                        wgpu::VertexAttributeDescriptor {
                            offset: 8,
                            format: wgpu::VertexFormat::Float2,
                            shader_location: 1,
                        },
                        wgpu::VertexAttributeDescriptor {
                            offset: 16,
                            format: wgpu::VertexFormat::Float4,
                            shader_location: 2,
                        },
                        wgpu::VertexAttributeDescriptor {
                            offset: 32,
                            format: wgpu::VertexFormat::Uint,
                            shader_location: 3,
                        },
                    ],
                }],
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 0.0,
            compare: wgpu::CompareFunction::Undefined,
        });

        Self {
            inner,
            pipeline,
            bind_group_layout,
            sampler,
            textures: HashMap::new(),
            vertex_buffer: None,
            draw_commands: Vec::new(),
        }
    }

    /// Generate a [`DrawList`](draw/struct.DrawList.html) for the view.
    pub fn draw<'a>(&'a mut self, device: &Device, queue: &Queue, render_pass: &mut RenderPass<'a>) {
        if self.inner.needs_redraw() {
            let DrawList {
                updates,
                vertices,
                commands,
            } = self.inner.draw();

            self.vertex_buffer.take();
            self.draw_commands = commands;

            if updates.len() > 0 {
                let cmd = device.create_command_encoder(&CommandEncoderDescriptor { label: None });
                queue.submit(&[updates
                    .into_iter()
                    .fold(cmd, |mut cmd, update| {
                        match update {
                            Update::Texture { id, size, data, atlas: _ } => {
                                let texture = device.create_texture(&wgpu::TextureDescriptor {
                                    label: None,
                                    size: wgpu::Extent3d {
                                        width: size[0],
                                        height: size[1],
                                        depth: 1,
                                    },
                                    array_layer_count: 1,
                                    mip_level_count: 1,
                                    sample_count: 1,
                                    dimension: wgpu::TextureDimension::D2,
                                    format: wgpu::TextureFormat::Rgba8Unorm,
                                    usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
                                });

                                if data.len() > 0 {
                                    let staging =
                                        device.create_buffer_with_data(data.as_slice(), wgpu::BufferUsage::COPY_SRC);
                                    cmd.copy_buffer_to_texture(
                                        wgpu::BufferCopyView {
                                            buffer: &staging,
                                            offset: 0,
                                            bytes_per_row: size[0] * 4,
                                            rows_per_image: 0,
                                        },
                                        wgpu::TextureCopyView {
                                            texture: &texture,
                                            mip_level: 0,
                                            array_layer: 0,
                                            origin: Default::default(),
                                        },
                                        wgpu::Extent3d {
                                            width: size[0],
                                            height: size[1],
                                            depth: 1,
                                        },
                                    );
                                }

                                let view = texture.create_default_view();

                                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                                    layout: &self.bind_group_layout,
                                    bindings: &[
                                        wgpu::Binding {
                                            binding: 0,
                                            resource: wgpu::BindingResource::TextureView(&view),
                                        },
                                        wgpu::Binding {
                                            binding: 1,
                                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                                        },
                                    ],
                                    label: None,
                                });

                                self.textures.insert(id, TextureEntry { bind_group, texture });
                            }
                            Update::TextureSubresource { id, offset, size, data } => {
                                let texture = self
                                    .textures
                                    .get(&id)
                                    .map(|val| &val.texture)
                                    .expect("non existing texture is updated");

                                let padding = 256 - (size[0] * 4) % 256;
                                let data = if padding > 0 {
                                    data.chunks(size[0] as usize * 4).fold(Vec::new(), |mut data, row| {
                                        data.extend_from_slice(row);
                                        data.extend(std::iter::repeat(0).take(padding as _));
                                        data
                                    })
                                } else {
                                    data
                                };
                                let staging =
                                    device.create_buffer_with_data(data.as_slice(), wgpu::BufferUsage::COPY_SRC);
                                cmd.copy_buffer_to_texture(
                                    wgpu::BufferCopyView {
                                        buffer: &staging,
                                        offset: 0,
                                        bytes_per_row: size[0] * 4 + padding,
                                        rows_per_image: 0,
                                    },
                                    wgpu::TextureCopyView {
                                        texture: &texture,
                                        mip_level: 0,
                                        array_layer: 0,
                                        origin: wgpu::Origin3d {
                                            x: offset[0],
                                            y: offset[1],
                                            z: 0,
                                        },
                                    },
                                    wgpu::Extent3d {
                                        width: size[0],
                                        height: size[1],
                                        depth: 1,
                                    },
                                );
                            }
                        }
                        cmd
                    })
                    .finish()]);
            }

            if vertices.len() > 0 {
                self.vertex_buffer
                    .replace(device.create_buffer_with_data(vertices.as_bytes(), wgpu::BufferUsage::VERTEX));
            }
        }

        if let Some(vertex_buffer) = self.vertex_buffer.as_ref() {
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.textures.values().next().unwrap().bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer, 0, 0);
        }

        for command in self.draw_commands.iter() {
            match command {
                &DrawCommand::Clip { scissor } => {
                    render_pass.set_scissor_rect(
                        scissor.left as u32,
                        scissor.top as u32,
                        scissor.width() as u32,
                        scissor.height() as u32,
                    );
                }
                &DrawCommand::Colored { offset, count } => {
                    render_pass.draw(offset as u32..(offset + count) as u32, 0..1);
                }
                &DrawCommand::Textured { texture, offset, count } => {
                    render_pass.set_bind_group(0, &self.textures.get(&texture).unwrap().bind_group, &[]);
                    render_pass.draw(offset as u32..(offset + count) as u32, 0..1);
                }
                &DrawCommand::Nop => (),
            }
        }
    }
}

impl<M: Model, E: EventLoop<Command<M::Message>>, L: Loader> Deref for Ui<M, E, L> {
    type Target = crate::Ui<M, E, L>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<M: Model, E: EventLoop<Command<M::Message>>, L: Loader> DerefMut for Ui<M, E, L> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
