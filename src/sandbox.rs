use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopProxy},
    window::{Window, WindowBuilder},
};

use crate::loader::Loader;
use crate::prelude::*;

/// Sandbox for quick prototyping of pixel widgets applications
pub struct Sandbox<M: Model + for<'a> UpdateModel<'a>, L: 'static + Loader> {
    /// The `Ui` being used in the sandbox
    pub ui: crate::backend::wgpu::Ui<M, EventLoopProxy<Command<<M as Model>::Message>>, L>,
    event_loop: Option<EventLoop<Command<<M as Model>::Message>>>,
    surface: wgpu::Surface,
    #[allow(unused)]
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    swap_chain: wgpu::SwapChain,
    sc_desc: wgpu::SwapChainDescriptor,
    window: Window,
}

impl<T, L> Sandbox<T, L> where T: Model + for<'a> UpdateModel<'a, Resources=()>, L: 'static + Loader {
    /// Construct a new `Sandbox`
    pub async fn new(model: T, loader: L, builder: WindowBuilder) -> Self {
        let event_loop = EventLoop::<Command<T::Message>>::with_user_event();
        let window = builder.build(&event_loop).unwrap();
        let size = window.inner_size();

        let swapchain_format = wgpu::TextureFormat::Bgra8Unorm;

        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(&window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");

        // Create the logical device and command queue
        let trace_dir = std::env::var("WGPU_TRACE");
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: Default::default(),
                    limits: wgpu::Limits::default(),
                },
                trace_dir.ok().as_ref().map(std::path::Path::new),
            )
            .await
            .expect("Failed retrieve device and queue");

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        };

        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let ui = crate::backend::wgpu::Ui::new(
            model,
            event_loop.create_proxy(),
            loader,
            Rectangle::from_wh(size.width as f32, size.height as f32),
            swapchain_format,
            &device,
        );

        Sandbox {
            ui,
            event_loop: Some(event_loop),
            surface,
            adapter,
            device,
            queue,
            swap_chain,
            sc_desc,
            window,
        }
    }

    /// Run the application
    pub async fn run(mut self) {
        let event_loop = self.event_loop.take().unwrap();
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;
            match event {
                Event::UserEvent(command) => {
                    self.ui.command(command, ());
                }
                Event::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    // Recreate the swap chain with the new size
                    self.sc_desc.width = size.width;
                    self.sc_desc.height = size.height;
                    self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
                    self.ui
                        .resize(Rectangle::from_wh(size.width as f32, size.height as f32));
                }
                Event::RedrawRequested(_) => {
                    let frame = self
                        .swap_chain
                        .get_current_frame()
                        .expect("Failed to acquire next swap chain texture");
                    let mut encoder = self
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                    {
                        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: None,
                            color_attachments: &[wgpu::RenderPassColorAttachment {
                                view: &frame.output.view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                    store: true,
                                },
                            }],
                            depth_stencil_attachment: None,
                        });

                        self.ui.draw(&self.device, &self.queue, &mut pass);
                    }

                    self.queue.submit(Some(encoder.finish()));
                }
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => *control_flow = ControlFlow::Exit,
                other => {
                    if let Some(event) = crate::backend::winit::convert_event(other) {
                        self.ui.event(event, ());
                    }
                }
            }

            if self.ui.needs_redraw() {
                self.window.request_redraw();
            }
        });
    }
}
