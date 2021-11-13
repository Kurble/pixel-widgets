use std::future::Future;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use crate::prelude::*;

/// Sandbox for quick prototyping of pixel widgets applications
pub struct Sandbox<M: 'static + Component> {
    /// The `Ui` being used in the sandbox
    pub ui: crate::backend::wgpu::Ui<M>,
    event_loop: Option<EventLoop<()>>,
    surface: wgpu::Surface,
    #[allow(unused)]
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    window: Window,
}

impl<T> Sandbox<T>
where
    T: 'static + Component,
{
    /// Construct a new `Sandbox` with a root component, style and window builder.
    /// The `Sandbox` will finish building the window and setup the graphics.
    /// To start the main event loop, call [`run()`](#method.run) on the result.
    pub async fn new<S, E>(root_component: T, style: S, window: WindowBuilder) -> anyhow::Result<Self>
    where
        S: TryInto<Style, Error = E>,
        anyhow::Error: From<E>,
    {
        let event_loop = EventLoop::new();
        let window = window.build(&event_loop).unwrap();
        let size = window.inner_size();

        let swapchain_format = wgpu::TextureFormat::Bgra8Unorm;

        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
        let surface = unsafe { instance.create_surface(&window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                force_fallback_adapter: false,
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

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        };

        surface.configure(&device, &surface_config);

        let ui = crate::backend::wgpu::Ui::new(
            root_component,
            Rectangle::from_wh(size.width as f32, size.height as f32),
            window.scale_factor() as f32,
            style,
            swapchain_format,
            &device,
        )?;

        Ok(Sandbox {
            ui,
            event_loop: Some(event_loop),
            surface,
            adapter,
            device,
            queue,
            surface_config,
            window,
        })
    }

    /// Update the root component with a message.
    /// Returns any output messages from the root component.
    pub fn update(&mut self, message: T::Message) {
        self.ui.update(message);
    }

    /// Create a task that will drive all ui futures.
    /// This method will panic if it's called a second time.
    pub fn task(&mut self) -> impl Future<Output = ()> {
        let proxy = self.event_loop.as_ref().unwrap().create_proxy();
        self.ui.task(move || proxy.send_event(()).unwrap())
    }

    /// Run the application
    pub async fn run(mut self) {
        let event_loop = self.event_loop.take().unwrap();
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;
            match event {
                Event::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    // Recreate the swap chain with the new size
                    self.surface_config.width = size.width;
                    self.surface_config.height = size.height;
                    self.surface.configure(&self.device, &self.surface_config);
                    self.ui.resize(
                        Rectangle::from_wh(size.width as f32, size.height as f32),
                        self.window.scale_factor() as f32,
                    );
                }
                Event::RedrawRequested(_) => {
                    let frame = self
                        .surface
                        .get_current_texture()
                        .expect("Failed to acquire next swap chain texture");
                    let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
                    let mut encoder = self
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                    {
                        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: None,
                            color_attachments: &[wgpu::RenderPassColorAttachment {
                                view: &view,
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
                    frame.present();
                }
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => *control_flow = ControlFlow::Exit,
                other => {
                    if let Some(event) = crate::backend::winit::convert_event(other) {
                        self.ui.handle_event(event);
                    }
                }
            }

            if self.ui.needs_redraw() {
                self.window.request_redraw();
            }
        });
    }
}
