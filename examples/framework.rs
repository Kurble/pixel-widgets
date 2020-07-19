use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

use pixel_widgets::prelude::*;

async fn run<T: 'static + Model>(
    model: T,
    stylesheet: impl AsRef<str>,
    event_loop: EventLoop<()>,
    window: Window,
    swapchain_format: wgpu::TextureFormat,
) {
    let size = window.inner_size();

    let surface = wgpu::Surface::create(&window);
    let adapter = wgpu::Adapter::request(
        &wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::Default,
            compatible_surface: Some(&surface),
        },
        wgpu::BackendBit::PRIMARY,
    )
    .await
    .expect("Failed to find an appropriate adapter");

    // Create the logical device and command queue
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            extensions: Default::default(),
            limits: wgpu::Limits::default(),
        })
        .await;

    let mut sc_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        format: swapchain_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Mailbox,
    };

    let mut swap_chain = device.create_swap_chain(&surface, &sc_desc);

    let mut viewport = Rectangle::from_wh(size.width as f32, size.height as f32);

    let mut ui = match WgpuUi::with_stylesheet(
        model,
        std::path::PathBuf::from("./examples/"),
        stylesheet,
        viewport,
        swapchain_format,
        &device,
    )
    .await
    {
        Ok(ui) => ui,
        Err(err) => {
            println!("{}", err);
            panic!();
        }
    };

    event_loop.run(move |event, _, control_flow| {
        let _ = &adapter;

        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                // Recreate the swap chain with the new size
                sc_desc.width = size.width;
                sc_desc.height = size.height;
                swap_chain = device.create_swap_chain(&surface, &sc_desc);
                viewport = Rectangle::from_wh(size.width as f32, size.height as f32);
                ui.resize(viewport);
            }
            Event::RedrawRequested(_) => {
                let frame = swap_chain
                    .get_next_texture()
                    .expect("Failed to acquire next swap chain texture");
                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                            attachment: &frame.view,
                            resolve_target: None,
                            load_op: wgpu::LoadOp::Clear,
                            store_op: wgpu::StoreOp::Clear,
                            clear_color: wgpu::Color::BLACK,
                        }],
                        depth_stencil_attachment: None,
                    });

                    ui.draw(&device, &queue, &mut pass);
                }

                queue.submit(&[encoder.finish()]);

                window.request_redraw();
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            other => {
                if let Some(event) = convert_event(other) {
                    ui.event(event);
                }
            }
        }
    });
}

pub fn run_model<T: 'static + Model, S: AsRef<str>>(model: T, stylesheet: S) {
    let event_loop = EventLoop::new();
    let window = winit::window::Window::new(&event_loop).unwrap();
    futures::executor::block_on(run(
        model,
        stylesheet,
        event_loop,
        window,
        wgpu::TextureFormat::Bgra8UnormSrgb,
    ));
}

#[allow(unused)]
fn main() {
    struct Dummy;
    impl Model for Dummy {
        type Message = ();

        fn update(&mut self, _message: Self::Message) {}

        fn view(&mut self) -> Node<Self::Message> {
            Space.into_node()
        }
    }
    run_model(Dummy, "tour.mss");
}
