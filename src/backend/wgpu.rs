use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use zerocopy::AsBytes;

use wgpu::*;

use crate::draw::{Command as DrawCommand, DrawList, Update, Vertex};
use crate::layout::Rectangle;
use crate::loader::Loader;
use crate::{Command, Component, EventLoop, UpdateComponent};
use std::num::NonZeroU32;
use wgpu::util::DeviceExt;

/// Wrapper for [`Ui`](../../struct.Ui.html) that adds wgpu rendering.
/// Requires the "wgpu" feature.
pub struct Ui<M: Component + for<'a> UpdateComponent<'a>, E: EventLoop<Command<<M as Component>::Message>>, L: Loader> {
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

impl<
        M: Component + for<'a> UpdateComponent<'a>,
        E: 'static + EventLoop<Command<<M as Component>::Message>>,
        L: 'static + Loader,
    > Ui<M, E, L>
{
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
        let shader_module = device.create_shader_module(&ShaderModuleDescriptor {
            label: Some("wgpu.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("wgpu.wgsl").into()),
            flags: ShaderFlags::VALIDATION,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler {
                        filtering: true,
                        comparison: false,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: VertexFormat::Float32x2,
                            offset: 8,
                            shader_location: 1,
                        },
                        wgpu::VertexAttribute {
                            format: VertexFormat::Float32x4,
                            offset: 16,
                            shader_location: 2,
                        },
                        wgpu::VertexAttribute {
                            format: VertexFormat::Float32,
                            offset: 32,
                            shader_location: 3,
                        },
                    ],
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                ..wgpu::PrimitiveState::default()
            },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: "fs_main",
                targets: &[wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            }),
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 0.0,
            compare: None,
            anisotropy_clamp: None,
            border_color: None,
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

            if !updates.is_empty() {
                let cmd = device.create_command_encoder(&CommandEncoderDescriptor { label: None });
                queue.submit(Some(
                    updates
                        .into_iter()
                        .fold(cmd, |mut cmd, update| {
                            match update {
                                Update::Texture {
                                    id,
                                    size,
                                    data,
                                    atlas: _,
                                } => {
                                    let texture_desc = wgpu::TextureDescriptor {
                                        label: None,
                                        size: wgpu::Extent3d {
                                            width: size[0],
                                            height: size[1],
                                            depth_or_array_layers: 1,
                                        },
                                        mip_level_count: 1,
                                        sample_count: 1,
                                        dimension: wgpu::TextureDimension::D2,
                                        format: wgpu::TextureFormat::Rgba8Unorm,
                                        usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
                                    };
                                    let texture = if data.is_empty() {
                                        device.create_texture(&texture_desc)
                                    } else {
                                        device.create_texture_with_data(queue, &texture_desc, data.as_slice())
                                    };

                                    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

                                    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                                        layout: &self.bind_group_layout,
                                        entries: &[
                                            wgpu::BindGroupEntry {
                                                binding: 0,
                                                resource: wgpu::BindingResource::TextureView(&view),
                                            },
                                            wgpu::BindGroupEntry {
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
                                    let staging = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                        label: None,
                                        contents: data.as_slice(),
                                        usage: wgpu::BufferUsage::COPY_SRC,
                                    });
                                    cmd.copy_buffer_to_texture(
                                        wgpu::ImageCopyBuffer {
                                            buffer: &staging,
                                            layout: wgpu::ImageDataLayout {
                                                offset: 0,
                                                bytes_per_row: NonZeroU32::new(size[0] * 4 + padding),
                                                rows_per_image: None,
                                            },
                                        },
                                        wgpu::ImageCopyTexture {
                                            texture: &texture,
                                            mip_level: 0,
                                            origin: wgpu::Origin3d {
                                                x: offset[0],
                                                y: offset[1],
                                                z: 0,
                                            },
                                        },
                                        wgpu::Extent3d {
                                            width: size[0],
                                            height: size[1],
                                            depth_or_array_layers: 1,
                                        },
                                    );
                                }
                            }
                            cmd
                        })
                        .finish(),
                ));
            }

            if !vertices.is_empty() {
                self.vertex_buffer
                    .replace(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        contents: vertices.as_bytes(),
                        usage: wgpu::BufferUsage::VERTEX,
                    }));
            }
        }

        if let Some(vertex_buffer) = self.vertex_buffer.as_ref() {
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.textures.values().next().unwrap().bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        }

        for command in self.draw_commands.iter() {
            match command {
                DrawCommand::Clip { scissor } => {
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
                DrawCommand::Nop => (),
            }
        }
    }
}

impl<M: Component + for<'a> UpdateComponent<'a>, E: EventLoop<Command<<M as Component>::Message>>, L: Loader> Deref
    for Ui<M, E, L>
{
    type Target = crate::Ui<M, E, L>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<M: Component + for<'a> UpdateComponent<'a>, E: EventLoop<Command<<M as Component>::Message>>, L: Loader> DerefMut
    for Ui<M, E, L>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
