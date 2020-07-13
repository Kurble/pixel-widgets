use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

use gui::backend::wgpu::WgpuUi;
use gui::backend::winit::convert_event;
use gui::element::Node;
use gui::layout::Rectangle;
use gui::*;

struct Counter {
    pub value: i32,
    pub name: String,
    pub password: String,
    pub up: gui::element::button::State,
    pub down: gui::element::button::State,
    pub name_state: gui::element::input::State,
    pub password_state: gui::element::input::State,
    pub scroll_state: gui::element::scroll::State,
}

enum Message {
    UpPressed,
    DownPressed,
    NameChanged(String),
    PasswordChanged(String),
}

impl Model for Counter {
    type Message = Message;

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::UpPressed => {
                self.value += 1;
            }
            Message::DownPressed => {
                self.value -= 1;
            }
            Message::NameChanged(name) => {
                self.name = name;
            }
            Message::PasswordChanged(password) => {
                self.password = password;
            }
        }
    }

    fn view(&mut self) -> Node<Message> {
        use gui::element::*;
        Scroll::new(
            &mut self.scroll_state,
            Column::new()
                .push(Button::new(&mut self.up, Text::borrowed("Up")).on_clicked(Message::UpPressed))
                .push(Text::owned(format!("Hello {}! Count: {}", self.name, self.value)))
                .push(Button::new(&mut self.down, Text::borrowed("Down")).on_clicked(Message::DownPressed))
                .push(Input::new(&mut self.name_state, "username", Message::NameChanged))
                .push(Input::password(
                    &mut self.password_state,
                    "password",
                    Message::PasswordChanged,
                )),
        )
        .into_node()
    }
}

async fn run(event_loop: EventLoop<()>, window: Window, swapchain_format: wgpu::TextureFormat) {
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

    let mut ui = WgpuUi::with_stylesheet(
        Counter {
            name: String::new(),
            password: String::new(),
            value: 0,
            up: Default::default(),
            down: Default::default(),
            name_state: Default::default(),
            password_state: Default::default(),
            scroll_state: Default::default(),
        },
        std::path::PathBuf::from("."),
        "test_style.ron",
        swapchain_format,
        &device,
    )
    .await;

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

                    ui.render(&device, &queue, &mut pass, viewport);
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

fn main() {
    pretty_env_logger::init();
    let event_loop = EventLoop::new();
    let window = winit::window::Window::new(&event_loop).unwrap();
    futures::executor::block_on(run(event_loop, window, wgpu::TextureFormat::Bgra8UnormSrgb));
}
